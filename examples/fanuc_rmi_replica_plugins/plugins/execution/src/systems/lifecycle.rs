//! Lifecycle systems for execution state management.
//!
//! These systems handle cleanup and state transitions based on external events
//! such as robot disconnection.

use bevy::prelude::*;

use crate::components::{BufferState, ExecutionCoordinator, ToolpathBuffer};

/// Marker trait that device plugins should implement to indicate a device is connected.
///
/// This is used by the lifecycle systems to detect disconnection.
/// Device plugins should add this component when connected and remove when disconnected.
#[derive(Component, Debug, Clone, Default)]
pub struct DeviceConnected;

/// System that resets execution state when devices disconnect.
///
/// When no devices are connected but execution is in progress, this system:
/// - Transitions BufferState to Idle
/// - Clears the ToolpathBuffer
/// - Logs the reset
///
/// Device plugins are responsible for adding/removing the `DeviceConnected` marker
/// on their device entities.
pub fn reset_on_disconnect_system(
    mut coordinator_query: Query<
        (Entity, &mut BufferState, &mut ToolpathBuffer),
        With<ExecutionCoordinator>,
    >,
    connected_devices: Query<Entity, With<DeviceConnected>>,
) {
    // If any device is still connected, do nothing
    if !connected_devices.is_empty() {
        return;
    }

    // No devices connected - reset any active execution
    for (entity, mut state, mut buffer) in coordinator_query.iter_mut() {
        match &*state {
            BufferState::Executing { .. } | BufferState::Paused { .. } => {
                info!(
                    "🔌 All devices disconnected while executing - resetting coordinator {:?}",
                    entity
                );
                buffer.clear();
                *state = BufferState::Idle;
            }
            BufferState::Ready | BufferState::Buffering { .. } => {
                // Also reset ready/buffering state since devices are gone
                info!(
                    "🔌 All devices disconnected with program loaded - resetting coordinator {:?}",
                    entity
                );
                buffer.clear();
                *state = BufferState::Idle;
            }
            BufferState::Stopped { .. } => {
                // Stopped state should transition to Idle on disconnect
                // (user already stopped, now we're cleaning up)
                info!(
                    "🔌 All devices disconnected after stop - resetting coordinator {:?}",
                    entity
                );
                buffer.clear();
                *state = BufferState::Idle;
            }
            _ => {} // Idle, Complete, Error - nothing to reset
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn test_reset_on_disconnect_with_connected_device() {
        let mut world = World::new();

        // Spawn a coordinator in Executing state
        world.spawn((
            ExecutionCoordinator::new("test"),
            BufferState::Executing {
                current_index: 0,
                completed_count: 0,
            },
            ToolpathBuffer::new(),
        ));

        // Spawn a connected device
        world.spawn(DeviceConnected);

        // Run the system
        let _ = world.run_system_once(reset_on_disconnect_system);

        // State should NOT be reset (device still connected)
        let state = world
            .query::<&BufferState>()
            .single(&world)
            .expect("Should have BufferState");
        assert!(matches!(state, BufferState::Executing { .. }));
    }

    #[test]
    fn test_reset_on_disconnect_no_devices() {
        let mut world = World::new();

        // Spawn a coordinator in Executing state
        world.spawn((
            ExecutionCoordinator::new("test"),
            BufferState::Executing {
                current_index: 5,
                completed_count: 3,
            },
            ToolpathBuffer::new(),
        ));

        // No connected devices

        // Run the system
        let _ = world.run_system_once(reset_on_disconnect_system);

        // State should be reset to Idle
        let state = world
            .query::<&BufferState>()
            .single(&world)
            .expect("Should have BufferState");
        assert!(matches!(state, BufferState::Idle));
    }

    #[test]
    fn test_reset_stopped_state_on_disconnect() {
        let mut world = World::new();

        // Spawn a coordinator in Stopped state
        world.spawn((
            ExecutionCoordinator::new("test"),
            BufferState::Stopped {
                at_index: 10,
                completed_count: 8,
            },
            ToolpathBuffer::new(),
        ));

        // No connected devices

        // Run the system
        let _ = world.run_system_once(reset_on_disconnect_system);

        // State should be reset to Idle
        let state = world
            .query::<&BufferState>()
            .single(&world)
            .expect("Should have BufferState");
        assert!(matches!(state, BufferState::Idle));
    }

    #[test]
    fn test_is_stopped_helper() {
        let stopped = BufferState::Stopped {
            at_index: 5,
            completed_count: 3,
        };
        assert!(stopped.is_stopped());
        assert!(stopped.is_terminal());
        assert!(!stopped.is_active());
        assert!(!stopped.is_complete());
        assert!(!stopped.is_error());
    }
}

