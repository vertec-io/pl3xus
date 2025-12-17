use bevy::prelude::*;
use bevy::ecs::message::MessageReader;
use pl3xus::NetworkData;
use fanuc_replica_types::*;
use fanuc_rmi::dto as raw_dto;
use fanuc_rmi::{SpeedType, TermType};
use pl3xus_sync::control::EntityControl;
use crate::plugins::connection::{FanucRobot, RmiDriver, RobotConnectionState};

/// Handle jog commands from clients - entity-based, uses pl3xus EntityControl
pub fn handle_jog_commands(
    mut events: MessageReader<NetworkData<JogCommand>>,
    robot_query: Query<(Entity, &RobotConnectionState, Option<&RmiDriver>, Option<&EntityControl>), With<FanucRobot>>,
) {
    for event in events.read() {
        let client_id = *event.source();
        // NetworkData<T> implements Deref<Target=T>, so we can access fields directly
        let cmd: &JogCommand = &*event;

        // Find a connected robot (in future, match by entity ID from command)
        let Some((entity, _, driver, control)) = robot_query.iter()
            .find(|(_, state, driver, _)| **state == RobotConnectionState::Connected && driver.is_some())
        else {
            warn!("Jog rejected: No connected robot");
            continue;
        };

        // Validate control using pl3xus EntityControl
        if let Some(entity_control) = control {
            if entity_control.client_id != client_id {
                warn!("Jog rejected from {:?}: No control (held by {:?})", client_id, entity_control.client_id);
                continue;
            }
        } else {
            // No EntityControl component - allow for now (development mode)
            trace!("No EntityControl on robot {:?}, allowing jog", entity);
        }

        let _driver = driver.expect("Checked above");

        info!("Processing Jog on {:?}: {:?} direction={:?} dist={} speed={}",
            entity, cmd.axis, cmd.direction, cmd.distance, cmd.speed);

        // Build position delta
        let mut pos = raw_dto::Position {
            x: 0.0, y: 0.0, z: 0.0,
            w: 0.0, p: 0.0, r: 0.0,
            ext1: 0.0, ext2: 0.0, ext3: 0.0,
        };
        let dist = if cmd.direction == JogDirection::Positive { cmd.distance } else { -cmd.distance };

        match cmd.axis {
            JogAxis::X => pos.x = dist,
            JogAxis::Y => pos.y = dist,
            JogAxis::Z => pos.z = dist,
            JogAxis::W => pos.w = dist,
            JogAxis::P => pos.p = dist,
            JogAxis::R => pos.r = dist,
            JogAxis::J1 | JogAxis::J2 | JogAxis::J3 | JogAxis::J4 | JogAxis::J5 | JogAxis::J6 => {
                // Joint jogs would use JointRelative instruction instead
                warn!("Joint jogging not yet implemented");
                continue;
            }
        }

        // Build instruction
        let _instruction = raw_dto::Instruction::FrcLinearRelative(raw_dto::FrcLinearRelative {
            sequence_id: 0,
            configuration: raw_dto::Configuration {
                u_frame_number: 0,
                u_tool_number: 0,
                turn4: 0, turn5: 0, turn6: 0,
                front: 0, up: 0, left: 0, flip: 0,
            },
            position: pos,
            speed_type: SpeedType::MMSec.into(),
            speed: cmd.speed as f64,
            term_type: TermType::CNT.into(),
            term_value: 100,
        });

        // TODO: Send instruction via driver
        // The proper pattern would be to send through the driver's channel
        // driver.0.send_instruction(instruction);
        trace!("Would send jog instruction to driver");
    }
}
