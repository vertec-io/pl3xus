//! Synchronization systems for execution state.
//!
//! These systems sync internal buffer state to the synced ExecutionState component.

use bevy::prelude::*;

use crate::components::{BufferState, ExecutionCoordinator, ExecutionState, SystemState, ToolpathBuffer};
use crate::systems::DeviceStatus;

#[cfg(feature = "server")]
use fanuc_replica_core::ActiveSystem;

/// Sync BufferState to ExecutionState (both on System entity).
///
/// This system bridges the internal buffer state with the synced ExecutionState.
/// It uses BufferState's `to_system_state()` and `available_actions()` methods
/// to derive the UI-facing state and available actions.
///
/// The system:
/// 1. Reads BufferState from the System entity
/// 2. Uses `to_system_state()` to get the SystemState enum
/// 3. Uses `available_actions()` to get the action flags
/// 4. Uses `completed_count()` for progress tracking
/// 5. Updates ExecutionState on System entity (synced to clients)
#[cfg(feature = "server")]
pub fn sync_buffer_state_to_execution_state(
    mut system_query: Query<(&BufferState, &mut ExecutionState), With<ActiveSystem>>,
) {
    let Ok((buffer_state, mut exec_state)) = system_query.single_mut() else {
        return; // No BufferState/ExecutionState on System entity
    };

    // Skip if in NoSource state (nothing loaded)
    if exec_state.state == SystemState::NoSource {
        return;
    }

    // Use the consolidated methods from BufferState
    let new_state = buffer_state.to_system_state();
    let completed_count = buffer_state.completed_count().unwrap_or(0) as usize;
    let actions = buffer_state.available_actions();

    // Only update if something changed
    let needs_update = exec_state.state != new_state
        || exec_state.points_executed != completed_count
        || exec_state.can_load != actions.can_load
        || exec_state.can_start != actions.can_start
        || exec_state.can_pause != actions.can_pause
        || exec_state.can_resume != actions.can_resume
        || exec_state.can_stop != actions.can_stop
        || exec_state.can_unload != actions.can_unload;

    if needs_update {
        exec_state.state = new_state;
        exec_state.points_executed = completed_count;
        exec_state.current_index = completed_count;
        exec_state.can_load = actions.can_load;
        exec_state.can_start = actions.can_start;
        exec_state.can_pause = actions.can_pause;
        exec_state.can_resume = actions.can_resume;
        exec_state.can_stop = actions.can_stop;
        exec_state.can_unload = actions.can_unload;
    }
}

/// Sync DeviceStatus changes back to BufferState.
///
/// This system handles:
/// - Updating BufferState.completed_count from DeviceStatus.completed_count
/// - Transitioning BufferState to Error if device has an error
/// - Transitioning BufferState to Complete when all points are executed
///
/// Note: Device plugins update DeviceStatus, and this system syncs it to BufferState.
/// Notifications are handled separately by device-specific plugins.
#[cfg(feature = "server")]
pub fn sync_device_status_to_buffer_state(
    mut system_query: Query<(&mut BufferState, &ToolpathBuffer, &ExecutionCoordinator)>,
    device_query: Query<&DeviceStatus, With<crate::components::PrimaryMotion>>,
) {
    // Get the device status from the primary motion device
    let Ok(device_status) = device_query.single() else {
        return; // No primary motion device
    };

    for (mut buffer_state, toolpath_buffer, coordinator) in system_query.iter_mut() {
        // Extract current state info before matching to avoid borrow issues
        let (is_executing, current_index) = match &*buffer_state {
            BufferState::Executing { current_index, .. } => (true, *current_index),
            _ => (false, 0),
        };

        if !is_executing {
            continue;
        }

        // Check for device error
        if let Some(ref error_msg) = device_status.error {
            let error_msg_clone = error_msg.clone();
            *buffer_state = BufferState::Error {
                message: error_msg_clone.clone(),
            };
            error!(
                "📛 BufferState -> Error for '{}': {}",
                coordinator.name, error_msg_clone
            );
            continue;
        }

        // Update completed count from device status
        let new_completed = device_status.completed_count;

        // Check if execution is complete using the sealed buffer pattern
        if toolpath_buffer.is_execution_complete(new_completed) {
            *buffer_state = BufferState::Complete {
                total_executed: new_completed,
            };
            info!(
                "✅ BufferState -> Complete for '{}' ({} points)",
                coordinator.name, new_completed
            );
        } else if new_completed != current_index {
            // Update the state with new counts
            *buffer_state = BufferState::Executing {
                current_index,
                completed_count: new_completed,
            };
        }
    }
}

