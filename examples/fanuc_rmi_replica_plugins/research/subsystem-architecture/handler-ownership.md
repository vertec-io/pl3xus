# Handler Ownership

## Current State (What to Migrate)

### Fanuc Plugin Handlers (to be moved)

```
plugins/fanuc/src/handlers.rs:
├── handle_load_program      → Move to programs plugin (rename to Load)
├── handle_unload_program    → Move to programs plugin (rename to Unload)
├── handle_start_program     → Move to execution plugin (rename to Start)
├── handle_pause_program     → Move to execution plugin (rename to Pause)
├── handle_resume_program    → Move to execution plugin (rename to Resume)
└── handle_stop_program      → Move to execution plugin (rename to Stop)
```

### What Stays in Fanuc Plugin

```
plugins/fanuc/src/handlers.rs:
├── handle_connect           → Fanuc-specific connection
├── handle_disconnect        → Fanuc-specific disconnection
├── handle_jog               → Fanuc-specific manual control
├── handle_update_uframe     → Fanuc-specific frame config
├── handle_update_utool      → Fanuc-specific tool config
└── handle_update_tcp        → Fanuc-specific TCP config
```

## Target State

### Execution Plugin Handlers

Note: Execution plugin operates on internal BufferState, then syncs to ExecutionState.
The handlers update BufferState, and a separate sync system updates ExecutionState.

```rust
// execution/src/handlers.rs
// Uses sync ECS system pattern with MessageReader
// Simple verb names - these are SYSTEM commands

use bevy::prelude::*;
use bevy::ecs::message::MessageReader;
use pl3xus::managers::network_request::Request;
use crate::{BufferState, Subsystems, SubsystemReadiness, ToolpathBuffer, ExecutionState, SystemState};
use crate::types::{Start, StartResponse, Pause, PauseResponse, Resume, ResumeResponse, Stop, StopResponse};
use fanuc_replica_core::ActiveSystem;

/// Start execution
/// Transitions: Ready/Completed/Stopped → Validating
pub fn handle_start(
    mut requests: MessageReader<Request<Start>>,
    mut buffer_state: Query<&mut BufferState, With<ActiveSystem>>,
    mut subsystems: Query<&mut Subsystems, With<ActiveSystem>>,
    execution_state: Query<&ExecutionState, With<ActiveSystem>>,
) {
    for request in requests.read() {
        // Check via ExecutionState (synced state)
        let Ok(exec) = execution_state.single() else {
            let _ = request.respond(StartResponse {
                success: false,
                error: Some("No active system".into()),
            });
            continue;
        };

        // Can only start from Ready, Completed, or Stopped
        if !matches!(exec.state, SystemState::Ready | SystemState::Completed | SystemState::Stopped) {
            let _ = request.respond(StartResponse {
                success: false,
                error: Some(format!("Cannot start: in {:?} state", exec.state)),
            });
            continue;
        }

        // Reset all subsystems to NotReady for validation
        if let Ok(mut subs) = subsystems.single_mut() {
            for entry in &mut subs.entries {
                entry.readiness = SubsystemReadiness::NotReady;
            }
        }

        // Transition internal BufferState to Validating
        if let Ok(mut state) = buffer_state.single_mut() {
            *state = BufferState::Validating { started_at: std::time::Instant::now() };
        }

        let _ = request.respond(StartResponse {
            success: true,
            error: None,
        });
    }
}

/// Pause execution
/// Transitions: Running → Paused
pub fn handle_pause(
    mut requests: MessageReader<Request<Pause>>,
    mut buffer_state: Query<&mut BufferState, With<ActiveSystem>>,
    execution_state: Query<&ExecutionState, With<ActiveSystem>>,
) {
    for request in requests.read() {
        let Ok(exec) = execution_state.single() else {
            let _ = request.respond(PauseResponse {
                success: false,
                error: Some("No active system".into()),
            });
            continue;
        };

        if !matches!(exec.state, SystemState::Running) {
            let _ = request.respond(PauseResponse {
                success: false,
                error: Some("Cannot pause: not running".into()),
            });
            continue;
        }

        if let Ok(mut state) = buffer_state.single_mut() {
            if let BufferState::Executing { current_index, .. } = *state {
                *state = BufferState::Paused { paused_at_index: current_index };
            }
        }

        let _ = request.respond(PauseResponse { success: true, error: None });
    }
}

/// Resume execution
/// Transitions: Paused → Validating (re-validates before resuming)
pub fn handle_resume(
    mut requests: MessageReader<Request<Resume>>,
    mut buffer_state: Query<&mut BufferState, With<ActiveSystem>>,
    mut subsystems: Query<&mut Subsystems, With<ActiveSystem>>,
    execution_state: Query<&ExecutionState, With<ActiveSystem>>,
) {
    for request in requests.read() {
        let Ok(exec) = execution_state.single() else {
            let _ = request.respond(ResumeResponse {
                success: false,
                error: Some("No active system".into()),
            });
            continue;
        };

        if !matches!(exec.state, SystemState::Paused) {
            let _ = request.respond(ResumeResponse {
                success: false,
                error: Some("Cannot resume: not paused".into()),
            });
            continue;
        }

        // Reset subsystems for re-validation
        if let Ok(mut subs) = subsystems.single_mut() {
            for entry in &mut subs.entries {
                entry.readiness = SubsystemReadiness::NotReady;
            }
        }

        if let Ok(mut state) = buffer_state.single_mut() {
            *state = BufferState::Validating { started_at: std::time::Instant::now() };
        }

        let _ = request.respond(ResumeResponse { success: true, error: None });
    }
}

/// Stop execution
/// Transitions: Running/Paused/Validating → Stopped
pub fn handle_stop(
    mut requests: MessageReader<Request<Stop>>,
    mut buffer_state: Query<&mut BufferState, With<ActiveSystem>>,
    mut toolpath_buffer: Query<&mut ToolpathBuffer, With<ActiveSystem>>,
    execution_state: Query<&ExecutionState, With<ActiveSystem>>,
) {
    for request in requests.read() {
        let Ok(exec) = execution_state.single() else {
            let _ = request.respond(StopResponse {
                success: false,
                error: Some("No active system".into()),
            });
            continue;
        };

        if !matches!(exec.state,
            SystemState::Running | SystemState::Paused | SystemState::Validating | SystemState::AwaitingPoints
        ) {
            let _ = request.respond(StopResponse {
                success: false,
                error: Some("Cannot stop: not running".into()),
            });
            continue;
        }

        // Clear motion buffer (but keep buffer display data for UI)
        if let Ok(mut buffer) = toolpath_buffer.single_mut() {
            buffer.clear();
        }

        // Transition to Stopped
        if let Ok(mut state) = buffer_state.single_mut() {
            match *state {
                BufferState::Executing { current_index, completed_count } => {
                    *state = BufferState::Stopped { at_index: current_index, completed_count };
                }
                BufferState::Paused { paused_at_index } => {
                    *state = BufferState::Stopped { at_index: paused_at_index, completed_count: paused_at_index };
                }
                BufferState::Validating { .. } => {
                    *state = BufferState::Stopped { at_index: 0, completed_count: 0 };
                }
                _ => {}
            }
        }

        let _ = request.respond(StopResponse { success: true, error: None });
    }
}
```

### BufferState → ExecutionState Sync System

This system syncs internal BufferState to the client-facing ExecutionState.

```rust
// execution/src/systems/sync_execution_state.rs

pub fn sync_execution_state(
    buffer_state: Query<&BufferState, (With<ActiveSystem>, Changed<BufferState>)>,
    mut execution_state: Query<&mut ExecutionState, With<ActiveSystem>>,
) {
    let Ok(buffer) = buffer_state.single() else { return };
    let Ok(mut exec) = execution_state.single_mut() else { return };

    let new_state = match buffer {
        BufferState::Idle => {
            if exec.source_type == SourceType::None {
                SystemState::NoSource
            } else {
                SystemState::Ready
            }
        }
        BufferState::Validating { .. } => SystemState::Validating,
        BufferState::Executing { current_index, completed_count } => {
            exec.current_index = *current_index as usize;
            exec.points_executed = *completed_count as usize;
            SystemState::Running
        }
        BufferState::Paused { paused_at_index } => {
            exec.current_index = *paused_at_index as usize;
            SystemState::Paused
        }
        BufferState::AwaitingPoints { completed_count } => {
            exec.points_executed = *completed_count as usize;
            SystemState::AwaitingPoints
        }
        BufferState::Complete { total_executed } => {
            exec.points_executed = *total_executed as usize;
            SystemState::Completed
        }
        BufferState::Stopped { at_index, completed_count } => {
            exec.current_index = *at_index as usize;
            exec.points_executed = *completed_count as usize;
            SystemState::Stopped
        }
        BufferState::Error { .. } => SystemState::Error,
    };

    exec.state = new_state;
    exec.update_available_actions();
}
```

### Programs Plugin Handlers (Static Loader Pattern)

The Programs Plugin is a **Static Loader** - it loads programs from the database
and feeds them into the execution buffer. It does NOT own execution state.

Key difference from previous design:
- Programs plugin pushes points TO the ToolpathBuffer
- Programs plugin updates BufferDisplayData (for UI table)
- Programs plugin updates ExecutionState (source_type, source_name, etc.)
- Programs plugin seals the buffer (all points known for static program)

```rust
// programs/src/handlers.rs
// Uses sync ECS system pattern with MessageReader
// Simple verb names: Load, Unload

use bevy::prelude::*;
use bevy::ecs::message::MessageReader;
use pl3xus::managers::network_request::Request;
use fanuc_replica_execution::{
    BufferState, ToolpathBuffer, ExecutionState, BufferDisplayData,
    BufferLineDisplay, SystemState, SourceType,
};
use fanuc_replica_core::{ActiveSystem, DatabaseResource};
use crate::{LoadedProgram, queries as programs_db};
use crate::types::{Load, LoadResponse, Unload, UnloadResponse};

/// Load a program from database and feed it to the execution buffer
///
/// This is the "Static Loader" pattern:
/// 1. Load program from database
/// 2. Convert to execution points
/// 3. Push ALL points to ToolpathBuffer
/// 4. Seal the buffer (all points known)
/// 5. Update ExecutionState and BufferDisplayData
pub fn handle_load(
    mut commands: Commands,
    mut requests: MessageReader<Request<Load>>,
    system: Query<Entity, With<ActiveSystem>>,
    buffer_state: Query<&BufferState, With<ActiveSystem>>,
    mut toolpath_buffer: Query<&mut ToolpathBuffer, With<ActiveSystem>>,
    mut execution_state: Query<&mut ExecutionState, With<ActiveSystem>>,
    mut buffer_display: Query<&mut BufferDisplayData, With<ActiveSystem>>,
    db: Res<DatabaseResource>,
) {
    for request in requests.read() {
        let program_id = request.message.program_id;

        // Check if we can load (must be NoSource or Ready)
        if let Ok(exec) = execution_state.single() {
            if !matches!(exec.state, SystemState::NoSource | SystemState::Ready) {
                let _ = request.respond(LoadResponse {
                    success: false,
                    error: Some("Cannot load: execution in progress".into()),
                });
                continue;
            }
        }

        // Get system entity
        let Ok(system_entity) = system.single() else {
            let _ = request.respond(LoadResponse {
                success: false,
                error: Some("No active system".into()),
            });
            continue;
        };

        // Fetch program from database
        let conn = db.connection.lock().unwrap();
        match programs_db::get_program(&conn, program_id) {
            Ok(Some(program_detail)) => {
                // 1. Convert to execution points
                let points = program_detail.to_execution_points();
                let total_points = points.len();

                // 2. Convert to display lines for UI table
                let display_lines = program_detail.to_display_lines();

                // 3. Push ALL points to ToolpathBuffer and seal
                if let Ok(mut buffer) = toolpath_buffer.single_mut() {
                    buffer.clear();
                    for point in &points {
                        buffer.push(point.clone());
                    }
                    buffer.seal();  // Static program - all points known
                }

                // 4. Update BufferDisplayData (synced to clients for UI table)
                if let Ok(mut display) = buffer_display.single_mut() {
                    display.lines = display_lines;
                }

                // 5. Update ExecutionState (synced to clients)
                if let Ok(mut exec) = execution_state.single_mut() {
                    exec.state = SystemState::Ready;
                    exec.source_type = SourceType::StaticProgram;
                    exec.source_name = Some(program_detail.name.clone());
                    exec.total_points = Some(total_points);
                    exec.current_index = 0;
                    exec.points_executed = 0;
                    exec.update_available_actions();
                }

                // 6. Store internal LoadedProgram for tracking/unload
                let loaded = LoadedProgram::from_program_detail(&program_detail);
                commands.entity(system_entity).insert(loaded);

                let _ = request.respond(LoadResponse {
                    success: true,
                    error: None,
                });
            }
            Ok(None) => {
                let _ = request.respond(LoadResponse {
                    success: false,
                    error: Some(format!("Program {} not found", program_id)),
                });
            }
            Err(e) => {
                let _ = request.respond(LoadResponse {
                    success: false,
                    error: Some(format!("Database error: {}", e)),
                });
            }
        }
    }
}

/// Unload program from System entity and clear execution buffer
pub fn handle_unload(
    mut commands: Commands,
    mut requests: MessageReader<Request<Unload>>,
    system: Query<Entity, With<ActiveSystem>>,
    execution_state: Query<&ExecutionState, With<ActiveSystem>>,
    mut toolpath_buffer: Query<&mut ToolpathBuffer, With<ActiveSystem>>,
    mut buffer_display: Query<&mut BufferDisplayData, With<ActiveSystem>>,
    mut exec_state_mut: Query<&mut ExecutionState, With<ActiveSystem>>,
) {
    for request in requests.read() {
        // Can't unload during execution
        if let Ok(exec) = execution_state.single() {
            if matches!(exec.state,
                SystemState::Running | SystemState::Validating | SystemState::Paused
            ) {
                let _ = request.respond(UnloadResponse {
                    success: false,
                    error: Some("Cannot unload: execution in progress".into()),
                });
                continue;
            }
        }

        let Ok(system_entity) = system.single() else {
            let _ = request.respond(UnloadResponse {
                success: false,
                error: Some("No active system".into()),
            });
            continue;
        };

        // 1. Remove LoadedProgram component
        commands.entity(system_entity).remove::<LoadedProgram>();

        // 2. Clear ToolpathBuffer
        if let Ok(mut buffer) = toolpath_buffer.single_mut() {
            buffer.clear();
        }

        // 3. Clear BufferDisplayData
        if let Ok(mut display) = buffer_display.single_mut() {
            display.clear();
        }

        // 4. Reset ExecutionState to NoSource
        if let Ok(mut exec) = exec_state_mut.single_mut() {
            *exec = ExecutionState::no_source();
        }

        let _ = request.respond(UnloadResponse { success: true, error: None });
    }
}
```

### Conversion Methods (Programs Plugin)

```rust
// programs/src/conversion.rs

impl ProgramDetail {
    /// Convert program to execution points (internal format)
    pub fn to_execution_points(&self) -> Vec<ExecutionPoint> {
        // ... conversion logic
    }

    /// Convert program to display lines (for UI table)
    pub fn to_display_lines(&self) -> Vec<BufferLineDisplay> {
        let mut lines = Vec::new();
        for (seq_idx, seq) in self.sequences.iter().enumerate() {
            for (point_idx, point) in seq.points.iter().enumerate() {
                lines.push(BufferLineDisplay {
                    index: lines.len(),
                    line_type: point.point_type_display(),
                    description: point.description(),
                    sequence_name: Some(seq.name.clone()),
                    source_line: Some(point_idx + 1),
                });
            }
        }
        lines
    }
}
```

