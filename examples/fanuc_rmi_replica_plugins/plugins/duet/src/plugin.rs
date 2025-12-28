//! Bevy plugin registration for the Duet extruder system.

use bevy::prelude::*;

use crate::device::DuetCommandEvent;

#[cfg(feature = "server")]
use crate::handler::{duet_command_handler_system, duet_http_sender_system};

/// Plugin for the Duet extruder system.
///
/// This plugin registers:
/// - Events for Duet command dispatch
/// - Command handler system (converts AuxiliaryCommandEvent to DuetCommandEvent)
/// - HTTP sender system (sends commands to Duet controller)
///
/// # Usage
///
/// ```rust,ignore
/// app.add_plugins(DuetPlugin);
/// ```
///
/// # Dependencies
///
/// This plugin expects the ExecutionPlugin to be registered first,
/// as it consumes AuxiliaryCommandEvent from the orchestrator.
pub struct DuetPlugin;

impl Plugin for DuetPlugin {
    fn build(&self, app: &mut App) {
        // Register the Duet command event
        app.add_message::<DuetCommandEvent>();

        #[cfg(feature = "server")]
        {
            // Duet command handler - converts AuxiliaryCommandEvent to DuetCommandEvent
            // Should run after the orchestrator system
            app.add_systems(Update, duet_command_handler_system);

            // HTTP sender - sends commands to Duet controller
            // Should run after the command handler
            app.add_systems(Update, duet_http_sender_system.after(duet_command_handler_system));

            info!("Duet plugin loaded");
        }
    }
}

