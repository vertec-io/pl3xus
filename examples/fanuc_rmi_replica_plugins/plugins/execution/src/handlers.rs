//! Execution control handlers.
//!
//! These handlers manage execution state transitions:
//! - Start: Ready/Completed/Stopped → Validating → Executing
//! - Pause: Running → Paused
//! - Resume: Paused → Running
//! - Stop: Running/Paused/Validating → Stopped

use bevy::ecs::message::MessageReader;
use bevy::prelude::*;
use fanuc_replica_core::ActiveSystem;
use pl3xus_sync::AuthorizedRequest;

use crate::components::{
    BufferState, ExecutionCoordinator, ExecutionState, Subsystems, SystemState, ToolpathBuffer,
};
use crate::systems::{DeviceStatus, ValidationStartTime};
use crate::types::{Pause, PauseResponse, Resume, ResumeResponse, Start, StartResponse, Stop, StopResponse};

/// Handle Start request - begins execution.
///
/// Transitions: Ready/Completed/Stopped → Validating
/// The validation system will then check subsystems and transition to Executing.
pub fn handle_start(
    mut commands: Commands,
    mut requests: MessageReader<AuthorizedRequest<Start>>,
    mut systems: Query<
        (
            &ExecutionCoordinator,
            &mut BufferState,
            &mut ToolpathBuffer,
            &mut Subsystems,
            Option<&mut ExecutionState>,
        ),
        With<ActiveSystem>,
    >,
    mut devices: Query<&mut DeviceStatus>,
) {
    for request in requests.read() {
        let request = request.clone();
        info!("📋 Handling Start request");

        let Ok((coordinator, mut buffer_state, mut toolpath_buffer, mut subsystems, exec_state)) =
            systems.single_mut()
        else {
            let response = StartResponse {
                success: false,
                error: Some("No source loaded. Load a program first.".into()),
            };
            let _ = request.respond(response);
            continue;
        };

        // Check if we can start
        let can_start = matches!(
            *buffer_state,
            BufferState::Ready | BufferState::Complete { .. } | BufferState::Stopped { .. }
        );

        if !can_start {
            let response = StartResponse {
                success: false,
                error: Some(format!("Cannot start from state: {:?}", *buffer_state)),
            };
            let _ = request.respond(response);
            continue;
        }

        // Reset buffer for restart if needed
        if matches!(*buffer_state, BufferState::Complete { .. } | BufferState::Stopped { .. }) {
            toolpath_buffer.reset_for_rerun();
        }

        // Reset device status
        for mut device_status in devices.iter_mut() {
            device_status.completed_count = 0;
            device_status.ready_for_next = true;
        }

        // Reset subsystems for validation
        subsystems.reset_all();

        // Transition to Validating and start timeout timer
        *buffer_state = BufferState::Validating;
        commands.insert_resource(ValidationStartTime::default());
        info!("📦 Set BufferState to Validating for '{}'", coordinator.name);

        // Update ExecutionState if present
        if let Some(mut exec) = exec_state {
            exec.state = SystemState::Validating;
            exec.update_available_actions();
        }

        let response = StartResponse {
            success: true,
            error: None,
        };
        if let Err(e) = request.respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle Pause request - pauses execution.
///
/// Transitions: Running → Paused
pub fn handle_pause(
    mut requests: MessageReader<AuthorizedRequest<Pause>>,
    mut systems: Query<
        (&ExecutionCoordinator, &mut BufferState, Option<&mut ExecutionState>),
        With<ActiveSystem>,
    >,
) {
    for request in requests.read() {
        let request = request.clone();
        info!("📋 Handling Pause request");

        let Ok((coordinator, mut buffer_state, exec_state)) = systems.single_mut() else {
            let response = PauseResponse {
                success: false,
                error: Some("No source loaded".into()),
            };
            let _ = request.respond(response);
            continue;
        };

        // Check if currently executing
        let current_idx = match *buffer_state {
            BufferState::Executing { current_index, .. } => current_index,
            _ => {
                let response = PauseResponse {
                    success: false,
                    error: Some("Cannot pause: not running".into()),
                };
                let _ = request.respond(response);
                continue;
            }
        };

        // Transition to Paused
        *buffer_state = BufferState::Paused {
            paused_at_index: current_idx,
        };
        info!("⏸ Paused '{}' at index {}", coordinator.name, current_idx);

        // Update ExecutionState if present
        if let Some(mut exec) = exec_state {
            exec.state = SystemState::Paused;
            exec.update_available_actions();
        }

        let response = PauseResponse {
            success: true,
            error: None,
        };
        if let Err(e) = request.respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle Resume request - resumes paused execution.
///
/// Transitions: Paused → Running
pub fn handle_resume(
    mut requests: MessageReader<AuthorizedRequest<Resume>>,
    mut systems: Query<
        (&ExecutionCoordinator, &mut BufferState, Option<&mut ExecutionState>),
        With<ActiveSystem>,
    >,
) {
    for request in requests.read() {
        let request = request.clone();
        info!("📋 Handling Resume request");

        let Ok((coordinator, mut buffer_state, exec_state)) = systems.single_mut() else {
            let response = ResumeResponse {
                success: false,
                error: Some("No source loaded".into()),
            };
            let _ = request.respond(response);
            continue;
        };

        // Check if paused
        let paused_at = match *buffer_state {
            BufferState::Paused { paused_at_index } => paused_at_index,
            _ => {
                let response = ResumeResponse {
                    success: false,
                    error: Some("Cannot resume: not paused".into()),
                };
                let _ = request.respond(response);
                continue;
            }
        };

        // Transition back to Executing
        *buffer_state = BufferState::Executing {
            current_index: paused_at,
            completed_count: paused_at, // Assume all before pause point are complete
        };
        info!("▶ Resumed '{}' from index {}", coordinator.name, paused_at);

        // Update ExecutionState if present
        if let Some(mut exec) = exec_state {
            exec.state = SystemState::Running;
            exec.update_available_actions();
        }

        let response = ResumeResponse {
            success: true,
            error: None,
        };
        if let Err(e) = request.respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle Stop request - stops execution.
///
/// Transitions: Running/Paused/Validating → Stopped
pub fn handle_stop(
    mut requests: MessageReader<AuthorizedRequest<Stop>>,
    mut systems: Query<
        (
            &ExecutionCoordinator,
            &mut BufferState,
            Option<&mut ExecutionState>,
        ),
        With<ActiveSystem>,
    >,
    devices: Query<&DeviceStatus>,
) {
    for request in requests.read() {
        let request = request.clone();
        info!("📋 Handling Stop request");

        let Ok((coordinator, mut buffer_state, exec_state)) = systems.single_mut() else {
            let response = StopResponse {
                success: false,
                error: Some("No source loaded".into()),
            };
            let _ = request.respond(response);
            continue;
        };

        // Get current position and completed count
        let (stopped_at_index, completed_before_stop) = match *buffer_state {
            BufferState::Executing {
                current_index,
                completed_count,
            } => (current_index, completed_count),
            BufferState::Paused { paused_at_index } => {
                let completed = devices.single().map(|d| d.completed_count).unwrap_or(0);
                (paused_at_index, completed)
            }
            BufferState::Validating => (0, 0),
            _ => {
                let response = StopResponse {
                    success: false,
                    error: Some("Cannot stop: not running, paused, or validating".into()),
                };
                let _ = request.respond(response);
                continue;
            }
        };

        // Transition to Stopped
        *buffer_state = BufferState::Stopped {
            at_index: stopped_at_index,
            completed_count: completed_before_stop,
        };
        info!(
            "⏹ Stopped '{}' at index {} ({} completed)",
            coordinator.name, stopped_at_index, completed_before_stop
        );

        // Update ExecutionState if present
        if let Some(mut exec) = exec_state {
            exec.state = SystemState::Stopped;
            exec.points_executed = completed_before_stop as usize;
            exec.update_available_actions();
        }

        let response = StopResponse {
            success: true,
            error: None,
        };
        if let Err(e) = request.respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}
