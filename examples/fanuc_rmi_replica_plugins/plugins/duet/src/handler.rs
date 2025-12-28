//! Duet Extruder Command Handler System
//!
//! This system processes AuxiliaryCommandEvents for Duet extruders,
//! converting them to HTTP requests to the Duet controller.

use bevy::prelude::*;

use crate::device::{
    DuetCommandEvent, DuetConnectionState, DuetExtruder, DuetExtruderConfig,
    DuetPositionState, format_extrusion_gcode,
};
use fanuc_replica_execution::{AuxiliaryCommand, AuxiliaryCommandEvent};

/// System that processes AuxiliaryCommandEvents for Duet extruders.
///
/// This system:
/// 1. Listens for AuxiliaryCommandEvents with device_type "duet_extruder"
/// 2. Looks up the Duet configuration for the target entity
/// 3. Converts the command to G-code
/// 4. Sends DuetCommandEvent for actual HTTP transmission
pub fn duet_command_handler_system(
    mut aux_events: MessageReader<AuxiliaryCommandEvent>,
    mut duet_events: MessageWriter<DuetCommandEvent>,
    duet_query: Query<(&DuetExtruderConfig, &DuetPositionState), With<DuetExtruder>>,
) {
    for event in aux_events.read() {
        // Only handle duet_extruder device types
        if event.device_type != "duet_extruder" {
            continue;
        }

        // Look up the Duet configuration
        let Ok((config, current_pos)) = duet_query.get(event.device) else {
            warn!(
                "AuxiliaryCommandEvent for duet_extruder but entity {:?} has no DuetExtruderConfig",
                event.device
            );
            continue;
        };

        // Parse the command - expecting extrusion commands
        match &event.command {
            AuxiliaryCommand::Extruder { distance, speed } => {
                // Convert relative distance to absolute position
                let target_position = current_pos.position + distance;
                // Convert speed from mm/s to mm/min
                let feedrate = (speed * 60.0).min(config.max_feedrate);

                debug!(
                    "Duet extrusion command: pos={:.4} feedrate={:.0} (point {})",
                    target_position, feedrate, event.point_index
                );

                duet_events.write(DuetCommandEvent {
                    extruder: event.device,
                    target_position,
                    feedrate,
                    point_index: event.point_index,
                });
            }
            AuxiliaryCommand::Gcode(gcode) => {
                // Direct G-code pass-through - parse for position
                // For now, just log it
                debug!(
                    "Duet G-code pass-through: {} (point {})",
                    gcode, event.point_index
                );
            }
            AuxiliaryCommand::None => {
                // No-op, skip this device for this point
            }
            _ => {
                warn!(
                    "Unsupported auxiliary command for duet_extruder: {:?}",
                    event.command
                );
            }
        }
    }
}

/// System that would send HTTP requests to Duet controllers.
///
/// In a real implementation, this would use reqwest or similar to send
/// HTTP requests. For now, it just logs the commands.
pub fn duet_http_sender_system(
    mut events: MessageReader<DuetCommandEvent>,
    mut duet_query: Query<
        (&DuetExtruderConfig, &mut DuetConnectionState, &mut DuetPositionState),
        With<DuetExtruder>,
    >,
) {
    for event in events.read() {
        let Ok((config, mut connection, mut position)) = duet_query.get_mut(event.extruder) else {
            warn!("DuetCommandEvent for unknown entity {:?}", event.extruder);
            continue;
        };

        // Format the G-code command
        let gcode = format_extrusion_gcode(config.axis, event.target_position, event.feedrate);

        info!(
            "Duet HTTP: http://{}:{}/rr_gcode?gcode={} (point {})",
            config.host,
            config.port,
            urlencoding::encode(&gcode),
            event.point_index
        );

        // Update local state (in real impl, this would be after HTTP response)
        position.position = event.target_position;
        position.feedrate = event.feedrate;
        connection.commands_sent += 1;
    }
}

