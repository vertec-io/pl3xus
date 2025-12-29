# Type Definitions

## Design Philosophy

**Buffer-Centric Execution**: Execution is about the buffer, not the program.
Programs (and future streams/generators) are "sources" that feed the buffer.

**UI = Buffer View**: The "program table" in the UI is the execution buffer table.
- `BufferDisplayData` contains the lines to display
- `ExecutionState.current_index` determines the highlighted row
- Works for static programs, streams, and generators

## Execution Plugin Types

These types live in `execution/src/components/` and are the source of truth.

### Subsystem Readiness

```rust
// execution/src/components/subsystems.rs

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Component on System entity tracking all registered subsystems
#[derive(Component, Clone, Debug, Default, Serialize, Deserialize)]
pub struct Subsystems {
    pub entries: Vec<SubsystemEntry>,
}

impl Subsystems {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Register a new subsystem (typically called during plugin setup)
    pub fn register(&mut self, name: &'static str) {
        if !self.entries.iter().any(|e| e.name == name) {
            self.entries.push(SubsystemEntry {
                name: name.to_string(),
                readiness: SubsystemReadiness::NotReady,
            });
        }
    }

    /// Update a subsystem's readiness
    pub fn set_readiness(&mut self, name: &str, readiness: SubsystemReadiness) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.name == name) {
            entry.readiness = readiness;
        }
    }

    /// Check if all subsystems are ready
    pub fn all_ready(&self) -> bool {
        self.entries.iter().all(|e| matches!(e.readiness, SubsystemReadiness::Ready))
    }

    /// Get first error, if any
    pub fn first_error(&self) -> Option<&str> {
        self.entries.iter().find_map(|e| {
            if let SubsystemReadiness::Error(msg) = &e.readiness {
                Some(msg.as_str())
            } else {
                None
            }
        })
    }

    /// Get list of not-ready subsystems
    pub fn not_ready(&self) -> Vec<&str> {
        self.entries.iter()
            .filter(|e| matches!(e.readiness, SubsystemReadiness::NotReady))
            .map(|e| e.name.as_str())
            .collect()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubsystemEntry {
    pub name: String,
    pub readiness: SubsystemReadiness,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum SubsystemReadiness {
    #[default]
    NotReady,
    Ready,
    Error(String),
}
```

### Updated BufferState

```rust
// execution/src/components/buffer.rs (additions)

use std::time::{Duration, Instant};

/// Validation timeout - 30 seconds
pub const VALIDATION_TIMEOUT: Duration = Duration::from_secs(30);

pub enum BufferState {
    /// No execution activity
    Idle,

    /// NEW: Validating subsystem readiness before execution
    /// Subsystems should check their readiness and update Subsystems component.
    /// Includes timeout tracking - auto-fails after VALIDATION_TIMEOUT.
    /// User can cancel at any time via StopProgram.
    Validating { started_at: Instant },

    /// Buffering points before execution starts
    Buffering { points_buffered: u32 },

    /// Ready to execute (buffer has minimum points)
    Ready,

    /// Actively executing points
    Executing { current_index: u32, completed_count: u32 },

    /// Paused mid-execution
    Paused { paused_at_index: u32 },

    /// Waiting for more points (streaming mode)
    AwaitingPoints { completed_count: u32 },

    /// Waiting for device feedback
    WaitingForFeedback { expected_sequence: u32, timeout_at: Instant },

    /// Execution completed successfully
    Complete { total_executed: u32 },

    /// Error occurred during execution
    Error { message: String },

    /// Stopped by user
    Stopped { at_index: u32, completed_count: u32 },
}

impl BufferState {
    /// Check if validation has timed out
    pub fn is_validation_timed_out(&self) -> bool {
        match self {
            BufferState::Validating { started_at } => {
                started_at.elapsed() > VALIDATION_TIMEOUT
            }
            _ => false,
        }
    }
}
```

### ExecutionState (Synced to Clients)

```rust
// execution/src/components/execution_state.rs

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Execution state synced to all clients.
/// Contains current state, source info, progress, and available actions.
///
/// The UI uses this to:
/// - Display current state (Running, Paused, etc.)
/// - Know what type of source is active (Program, Stream, Generator)
/// - Highlight the current row in the buffer table (current_index)
/// - Show/hide action buttons (can_* fields)
#[derive(Component, Clone, Debug, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "stores", derive(Store))]
pub struct ExecutionState {
    /// Current execution state
    pub state: SystemState,

    /// What type of source is feeding the buffer
    pub source_type: SourceType,

    /// Source name for display (e.g., "my_print.csv", "spiral_generator")
    pub source_name: Option<String>,

    /// Current execution index (0-based)
    /// UI uses this to highlight the current row in the buffer table
    pub current_index: usize,

    /// Total points in buffer (if known)
    /// For static: known upfront. For streaming: grows as points added.
    pub total_points: Option<usize>,

    /// Points confirmed executed by the device
    pub points_executed: usize,

    /// Available actions based on current state
    pub can_load: bool,
    pub can_start: bool,
    pub can_pause: bool,
    pub can_resume: bool,
    pub can_stop: bool,
    pub can_unload: bool,
}

/// What type of source is feeding the execution buffer
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceType {
    #[default]
    None,
    StaticProgram,  // Loaded from database, all points known
    Stream,         // Future: points arriving from external source
    Generator,      // Future: points generated algorithmically
}

/// System execution state for UI display
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SystemState {
    #[default]
    NoSource,       // Nothing to execute
    Ready,          // Source loaded, ready to start
    Validating,     // Checking subsystems before execution
    Running,        // Actively executing points
    Paused,         // Paused by user
    AwaitingPoints, // Buffer empty, waiting for more (streaming only)
    Completed,      // All points executed
    Stopped,        // Stopped by user
    Error,          // Error occurred
}

impl ExecutionState {
    /// Create state for "no source loaded"
    pub fn no_source() -> Self {
        Self {
            state: SystemState::NoSource,
            source_type: SourceType::None,
            source_name: None,
            current_index: 0,
            total_points: None,
            points_executed: 0,
            can_load: true,
            can_start: false,
            can_pause: false,
            can_resume: false,
            can_stop: false,
            can_unload: false,
        }
    }

    /// Update available actions based on current state
    pub fn update_available_actions(&mut self) {
        match self.state {
            SystemState::NoSource => {
                self.can_load = true;
                self.can_start = false;
                self.can_pause = false;
                self.can_resume = false;
                self.can_stop = false;
                self.can_unload = false;
            }
            SystemState::Ready => {
                self.can_load = false;  // Already have a source
                self.can_start = true;
                self.can_pause = false;
                self.can_resume = false;
                self.can_stop = false;
                self.can_unload = true;
            }
            SystemState::Validating => {
                self.can_load = false;
                self.can_start = false;
                self.can_pause = false;
                self.can_resume = false;
                self.can_stop = true;  // Can cancel validation
                self.can_unload = false;
            }
            SystemState::Running => {
                self.can_load = false;
                self.can_start = false;
                self.can_pause = true;
                self.can_resume = false;
                self.can_stop = true;
                self.can_unload = false;
            }
            SystemState::Paused => {
                self.can_load = false;
                self.can_start = false;
                self.can_pause = false;
                self.can_resume = true;
                self.can_stop = true;
                self.can_unload = false;
            }
            SystemState::AwaitingPoints => {
                self.can_load = false;
                self.can_start = false;
                self.can_pause = true;
                self.can_resume = false;
                self.can_stop = true;
                self.can_unload = false;
            }
            SystemState::Completed | SystemState::Stopped => {
                self.can_load = true;
                self.can_start = true;  // Can restart
                self.can_pause = false;
                self.can_resume = false;
                self.can_stop = false;
                self.can_unload = true;
            }
            SystemState::Error => {
                self.can_load = true;
                self.can_start = false;
                self.can_pause = false;
                self.can_resume = false;
                self.can_stop = false;
                self.can_unload = true;
            }
        }
    }
}
```

### BufferDisplayData (Synced to Clients)

```rust
// execution/src/components/buffer_display.rs

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Display data for the execution buffer table in UI.
///
/// This is what the "program table" actually shows - it's the execution buffer!
/// - For static programs: populated once on load
/// - For streaming: updated incrementally as points arrive
/// - For generators: updated as points are generated
///
/// The UI uses ExecutionState.current_index to highlight the current row.
#[derive(Component, Clone, Debug, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "stores", derive(Store))]
pub struct BufferDisplayData {
    /// The lines/points to show in the UI table
    pub lines: Vec<BufferLineDisplay>,
}

/// A single line in the buffer table display
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BufferLineDisplay {
    /// Index in the buffer (matches ExecutionState.current_index)
    pub index: usize,

    /// Type of operation for display
    pub line_type: String,  // "Move", "Extrude", "Wait", "Dwell", etc.

    /// Human-readable description
    pub description: String,  // "Move to (100.0, 200.0, 50.0)"

    /// Optional sequence/section name
    pub sequence_name: Option<String>,  // "Infill", "Perimeter", "Skirt"

    /// Original source line number (for static programs)
    /// Useful for debugging/correlation with source file
    pub source_line: Option<usize>,
}

impl BufferDisplayData {
    pub fn clear(&mut self) {
        self.lines.clear();
    }

    pub fn push_line(&mut self, line: BufferLineDisplay) {
        self.lines.push(line);
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }
}
```

### Handler Message Types (Execution Plugin)

```rust
// execution/src/types.rs
// Simple verb names - system commands, not "program" commands

#[derive(Serialize, Deserialize)]
pub struct Start;  // Not "StartProgram"

#[derive(Serialize, Deserialize)]
pub struct StartResponse { pub success: bool, pub error: Option<String> }

#[derive(Serialize, Deserialize)]
pub struct Pause;

#[derive(Serialize, Deserialize)]
pub struct PauseResponse { pub success: bool, pub error: Option<String> }

#[derive(Serialize, Deserialize)]
pub struct Resume;

#[derive(Serialize, Deserialize)]
pub struct ResumeResponse { pub success: bool, pub error: Option<String> }

#[derive(Serialize, Deserialize)]
pub struct Stop;

#[derive(Serialize, Deserialize)]
pub struct StopResponse { pub success: bool, pub error: Option<String> }
```

## Programs Plugin Types

The Programs Plugin is a **Static Loader** - it loads programs from the database
and feeds them into the execution buffer. It does NOT own the execution state.

### Key Responsibility

When `Load` is called:
1. Load program from database
2. Convert to ExecutionPoints
3. Push ALL points to ToolpathBuffer (via execution plugin)
4. Seal the buffer (all points known)
5. Update ExecutionState.source_type = StaticProgram
6. Update BufferDisplayData with lines for UI

### LoadedProgram (Internal, Not Synced)

```rust
// programs/src/components/loaded_program.rs

use bevy::prelude::*;
use fanuc_replica_execution::ExecutionPoint;

/// Internal component on System entity tracking what program is loaded.
/// NOT synced to clients - the UI sees BufferDisplayData instead.
///
/// This is used by the programs plugin to:
/// - Know what program is currently loaded (for Unload)
/// - Correlate execution indices back to source lines (for errors/debugging)
#[derive(Component, Clone, Debug)]
pub struct LoadedProgram {
    pub program_id: i64,
    pub program_name: String,
    /// Original source for debugging/correlation
    pub sequences: Vec<LoadedSequence>,
}

#[derive(Clone, Debug)]
pub struct LoadedSequence {
    pub name: String,
    pub sequence_type: SequenceType,
    pub points: Vec<ExecutionPoint>,
}
```

### Handler Message Types (Programs Plugin)

```rust
// programs/src/types.rs

/// Load a program from the database and feed it to the execution buffer
#[derive(Serialize, Deserialize)]
pub struct Load { pub program_id: i64 }

#[derive(Serialize, Deserialize)]
pub struct LoadResponse {
    pub success: bool,
    pub error: Option<String>,
    // Execution state is synced via ExecutionState component
    // Buffer display is synced via BufferDisplayData component
}

/// Unload the current program and clear the execution buffer
#[derive(Serialize, Deserialize)]
pub struct Unload;

#[derive(Serialize, Deserialize)]
pub struct UnloadResponse {
    pub success: bool,
    pub error: Option<String>,
}
```

### What Happens on Load (Pseudocode)

```rust
fn handle_load(
    mut commands: Commands,
    mut requests: MessageReader<Request<Load>>,
    system: Query<Entity, With<ActiveSystem>>,
    mut execution_state: Query<&mut ExecutionState, With<ActiveSystem>>,
    mut buffer_display: Query<&mut BufferDisplayData, With<ActiveSystem>>,
    mut toolpath_buffer: Query<&mut ToolpathBuffer, With<ActiveSystem>>,
    db: Res<DatabaseResource>,
) {
    for request in requests.read() {
        let entity = system.single();

        // 1. Load program from database
        let program_data = db.get_program(request.message.program_id)?;

        // 2. Convert to execution points and display lines
        let points: Vec<ExecutionPoint> = convert_to_points(&program_data);
        let display_lines: Vec<BufferLineDisplay> = convert_to_display(&program_data);

        // 3. Push ALL points to buffer and seal (static loader pattern)
        let mut buffer = toolpath_buffer.get_mut(entity)?;
        buffer.clear();
        for point in &points {
            buffer.push(point.clone());
        }
        buffer.seal();  // All points known for static program

        // 4. Update BufferDisplayData (synced to clients for UI table)
        let mut display = buffer_display.get_mut(entity)?;
        display.lines = display_lines;

        // 5. Update ExecutionState (synced to clients)
        let mut exec = execution_state.get_mut(entity)?;
        exec.state = SystemState::Ready;
        exec.source_type = SourceType::StaticProgram;
        exec.source_name = Some(program_data.name.clone());
        exec.total_points = Some(points.len());
        exec.current_index = 0;
        exec.points_executed = 0;
        exec.update_available_actions();

        // 6. Store internal LoadedProgram for tracking
        commands.entity(entity).insert(LoadedProgram {
            program_id: request.message.program_id,
            program_name: program_data.name,
            sequences: program_data.sequences,
        });
    }
}
```

### Programs Subsystem Validation

The programs plugin registers a subsystem that validates:
- A program is loaded (LoadedProgram component exists)

```rust
fn validate_programs_subsystem(
    programs: Query<Option<&LoadedProgram>, With<ActiveSystem>>,
    mut subsystems: Query<&mut Subsystems, With<ActiveSystem>>,
    buffer_state: Query<&BufferState, With<ActiveSystem>>,
) {
    // Only validate when execution is in Validating state
    let state = buffer_state.single();
    if !matches!(state, BufferState::Validating { .. }) {
        return;
    }

    let program = programs.single();
    let mut subs = subsystems.single_mut();

    if program.is_some() {
        subs.set_readiness("programs", SubsystemReadiness::Ready);
    } else {
        subs.set_readiness("programs", SubsystemReadiness::Error(
            "No program loaded".to_string()
        ));
    }
}
```

## Subsystem Names (Constants)

```rust
// Each plugin defines its subsystem name as a constant
// execution/src/lib.rs
pub const SUBSYSTEM_EXECUTION: &str = "execution";

// programs/src/lib.rs  
pub const SUBSYSTEM_PROGRAMS: &str = "programs";

// fanuc/src/lib.rs
pub const SUBSYSTEM_FANUC: &str = "fanuc_robot";

// duet/src/lib.rs
pub const SUBSYSTEM_DUET: &str = "duet_extruder";
```

