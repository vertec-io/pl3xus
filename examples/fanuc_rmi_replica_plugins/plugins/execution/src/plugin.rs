//! Bevy plugin registration for the execution system.

use bevy::prelude::*;

#[cfg(feature = "server")]
use crate::systems::{
    orchestrator_system, reset_on_disconnect_system, update_buffer_state_system,
    AuxiliaryCommandEvent, MotionCommandEvent,
};

/// Plugin for the execution system.
///
/// This plugin registers:
/// - Events for device command dispatch
/// - State management systems (buffer state transitions)
/// - Orchestrator system (command dispatch to devices)
///
/// # Usage
///
/// ```rust,ignore
/// app.add_plugins(ExecutionPlugin);
/// ```
///
/// # Schedule
///
/// Systems run in Update schedule in this order:
/// 1. `update_buffer_state_system` - Handle state transitions
/// 2. `orchestrator_system` - Dispatch commands to devices
///
/// Device plugins should add their own systems that run after these
/// to consume the `MotionCommandEvent` and `AuxiliaryCommandEvent` events.
///
/// # Device Plugins
///
/// Device-specific handlers are in their respective plugin crates:
/// - FANUC: `fanuc_replica_plugins::RobotPlugin` (includes motion handler)
/// - Duet: `fanuc_replica_duet::DuetPlugin`
pub struct ExecutionPlugin;

impl Plugin for ExecutionPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "server")]
        {
            // Register messages (events)
            app.add_message::<MotionCommandEvent>();
            app.add_message::<AuxiliaryCommandEvent>();

            // Register systems - update_buffer_state runs first, then orchestrator, then lifecycle
            app.add_systems(Update, update_buffer_state_system);
            app.add_systems(Update, orchestrator_system.after(update_buffer_state_system));
            app.add_systems(Update, reset_on_disconnect_system.after(orchestrator_system));

            info!("Execution plugin loaded");
        }
    }
}

