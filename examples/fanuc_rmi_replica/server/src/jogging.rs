use bevy::prelude::*;
use bevy::ecs::message::MessageReader;
use pl3xus::NetworkData;
use fanuc_replica_types::*;
use fanuc_rmi::dto as raw_dto;
use fanuc_rmi::{SpeedType, TermType};
use fanuc_rmi::packets::PacketPriority;
use pl3xus_sync::control::EntityControl;
use crate::plugins::connection::{FanucRobot, RmiDriver, RobotConnectionState};

/// Handle jog commands from clients - entity-based, uses pl3xus EntityControl
pub fn handle_jog_commands(
    mut events: MessageReader<NetworkData<JogCommand>>,
    robot_query: Query<(Entity, &RobotConnectionState, Option<&RmiDriver>, Option<&EntityControl>), With<FanucRobot>>,
) {
    use fanuc_rmi::packets::PacketPriority;

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

        let driver = driver.expect("Checked above");

        info!("Processing JogCommand on {:?}: {:?} direction={:?} dist={} speed={}",
            entity, cmd.axis, cmd.direction, cmd.distance, cmd.speed);

        // Build position delta (PositionDto uses f32)
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
                // Joint jogs not supported by this simulator - need FrcJointRelativeJRep
                warn!("Joint jogging not supported by this simulator");
                continue;
            }
        }

        // Build instruction - use FrcLinearRelative for Cartesian jogs
        let instruction = raw_dto::Instruction::FrcLinearRelative(raw_dto::FrcLinearRelative {
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
            term_type: TermType::FINE.into(), // Use FINE for step moves
            term_value: 1,
        });

        // Send instruction via driver
        let send_packet: fanuc_rmi::packets::SendPacket =
            raw_dto::SendPacket::Instruction(instruction).into();

        match driver.0.send_command(send_packet, PacketPriority::Immediate) {
            Ok(seq) => {
                info!("Sent Cartesian jog command with sequence {}", seq);
            }
            Err(e) => {
                error!("Failed to send jog instruction: {:?}", e);
            }
        }
    }
}

/// Handle JogRobot commands (simplified jog from Joint Jog panel)
/// NOTE: The simulator only supports FrcLinearRelative, so joint jogs are not supported.
/// For Cartesian jogs (X/Y/Z/W/P/R), we use FrcLinearRelative.
pub fn handle_jog_robot_commands(
    mut events: MessageReader<NetworkData<JogRobot>>,
    robot_query: Query<(Entity, &RobotConnectionState, Option<&RmiDriver>, Option<&EntityControl>), With<FanucRobot>>,
) {
    use fanuc_rmi::packets::PacketPriority;

    for event in events.read() {
        let client_id = *event.source();
        let cmd: &JogRobot = &*event;

        // Find a connected robot
        let Some((entity, _, driver, control)) = robot_query.iter()
            .find(|(_, state, driver, _)| **state == RobotConnectionState::Connected && driver.is_some())
        else {
            warn!("JogRobot rejected: No connected robot");
            continue;
        };

        // Validate control using pl3xus EntityControl
        if let Some(entity_control) = control {
            if entity_control.client_id != client_id {
                warn!("JogRobot rejected from {:?}: No control (held by {:?})", client_id, entity_control.client_id);
                continue;
            }
        } else {
            trace!("No EntityControl on robot {:?}, allowing jog", entity);
        }

        let driver = driver.expect("Checked above");

        // Determine if this is a joint jog or cartesian jog
        let is_joint_jog = matches!(cmd.axis, JogAxis::J1 | JogAxis::J2 | JogAxis::J3 | JogAxis::J4 | JogAxis::J5 | JogAxis::J6);

        if is_joint_jog {
            // Joint jogs not supported by this simulator
            warn!("JogRobot: Joint jogging ({:?}) not supported by simulator", cmd.axis);
            continue;
        }

        info!("Processing JogRobot on {:?}: {:?} dist={} speed={}",
            entity, cmd.axis, cmd.distance, cmd.speed);

        // Build position delta for Cartesian jog
        let mut pos = raw_dto::Position {
            x: 0.0, y: 0.0, z: 0.0,
            w: 0.0, p: 0.0, r: 0.0,
            ext1: 0.0, ext2: 0.0, ext3: 0.0,
        };

        match cmd.axis {
            JogAxis::X => pos.x = cmd.distance,
            JogAxis::Y => pos.y = cmd.distance,
            JogAxis::Z => pos.z = cmd.distance,
            JogAxis::W => pos.w = cmd.distance,
            JogAxis::P => pos.p = cmd.distance,
            JogAxis::R => pos.r = cmd.distance,
            _ => continue, // Joint jogs handled above
        }

        // Cartesian jog - use LinearRelative instruction (only type supported by simulator)
        let instruction = raw_dto::Instruction::FrcLinearRelative(raw_dto::FrcLinearRelative {
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
            term_type: TermType::FINE.into(), // FINE for step moves
            term_value: 1,
        });

        // Send instruction via driver
        let send_packet: fanuc_rmi::packets::SendPacket =
            raw_dto::SendPacket::Instruction(instruction).into();

        match driver.0.send_command(send_packet, PacketPriority::Immediate) {
            Ok(seq) => {
                info!("Sent JogRobot command with sequence {}", seq);
            }
            Err(e) => {
                error!("Failed to send jog instruction: {:?}", e);
            }
        }
    }
}

/// Handle InitializeRobot commands - initializes the robot for motion
pub fn handle_initialize_robot(
    mut events: MessageReader<NetworkData<InitializeRobot>>,
    mut robot_query: Query<(Entity, &RobotConnectionState, Option<&RmiDriver>, Option<&EntityControl>, &mut RobotStatus), With<FanucRobot>>,
) {
    for event in events.read() {
        let client_id = *event.source();
        let cmd: &InitializeRobot = &*event;

        // Find a connected robot
        let Some((entity, _, driver, control, mut status)) = robot_query.iter_mut()
            .find(|(_, state, driver, _, _)| **state == RobotConnectionState::Connected && driver.is_some())
        else {
            warn!("InitializeRobot rejected: No connected robot");
            continue;
        };

        // Validate control
        if let Some(entity_control) = control {
            if entity_control.client_id != client_id {
                warn!("InitializeRobot rejected from {:?}: No control (held by {:?})", client_id, entity_control.client_id);
                continue;
            }
        }

        let driver = driver.expect("Checked above");
        let group_mask = cmd.group_mask.unwrap_or(1);

        info!("Processing InitializeRobot on {:?} with group_mask={}", entity, group_mask);

        // Send FrcInitialize command
        let command = raw_dto::Command::FrcInitialize(raw_dto::FrcInitialize { group_mask });
        let send_packet: fanuc_rmi::packets::SendPacket = raw_dto::SendPacket::Command(command).into();

        match driver.0.send_command(send_packet, PacketPriority::Immediate) {
            Ok(seq) => {
                info!("Sent InitializeRobot command with sequence {}", seq);
                // Mark TP program as initialized
                status.tp_program_initialized = true;
            }
            Err(e) => {
                error!("Failed to send InitializeRobot command: {:?}", e);
            }
        }
    }
}

/// Handle AbortMotion commands - aborts current motion
pub fn handle_abort_motion(
    mut events: MessageReader<NetworkData<AbortMotion>>,
    mut robot_query: Query<(Entity, &RobotConnectionState, Option<&RmiDriver>, Option<&EntityControl>, &mut RobotStatus), With<FanucRobot>>,
) {
    for event in events.read() {
        let client_id = *event.source();

        // Find a connected robot
        let Some((entity, _, driver, control, mut status)) = robot_query.iter_mut()
            .find(|(_, state, driver, _, _)| **state == RobotConnectionState::Connected && driver.is_some())
        else {
            warn!("AbortMotion rejected: No connected robot");
            continue;
        };

        // Validate control
        if let Some(entity_control) = control {
            if entity_control.client_id != client_id {
                warn!("AbortMotion rejected from {:?}: No control (held by {:?})", client_id, entity_control.client_id);
                continue;
            }
        }

        let driver = driver.expect("Checked above");

        info!("Processing AbortMotion on {:?}", entity);

        // Send FrcAbort command (unit variant)
        let command = raw_dto::Command::FrcAbort;
        let send_packet: fanuc_rmi::packets::SendPacket = raw_dto::SendPacket::Command(command).into();

        match driver.0.send_command(send_packet, PacketPriority::Immediate) {
            Ok(seq) => {
                info!("Sent AbortMotion command with sequence {}", seq);
                // Mark TP program as not initialized after abort
                status.tp_program_initialized = false;
            }
            Err(e) => {
                error!("Failed to send AbortMotion command: {:?}", e);
            }
        }
    }
}

/// Handle ResetRobot commands - resets robot errors
pub fn handle_reset_robot(
    mut events: MessageReader<NetworkData<ResetRobot>>,
    robot_query: Query<(Entity, &RobotConnectionState, Option<&RmiDriver>, Option<&EntityControl>), With<FanucRobot>>,
) {
    for event in events.read() {
        let client_id = *event.source();

        // Find a connected robot
        let Some((entity, _, driver, control)) = robot_query.iter()
            .find(|(_, state, driver, _)| **state == RobotConnectionState::Connected && driver.is_some())
        else {
            warn!("ResetRobot rejected: No connected robot");
            continue;
        };

        // Validate control
        if let Some(entity_control) = control {
            if entity_control.client_id != client_id {
                warn!("ResetRobot rejected from {:?}: No control (held by {:?})", client_id, entity_control.client_id);
                continue;
            }
        }

        let driver = driver.expect("Checked above");

        info!("Processing ResetRobot on {:?}", entity);

        // Send FrcReset command (unit variant)
        let command = raw_dto::Command::FrcReset;
        let send_packet: fanuc_rmi::packets::SendPacket = raw_dto::SendPacket::Command(command).into();

        match driver.0.send_command(send_packet, PacketPriority::Immediate) {
            Ok(seq) => {
                info!("Sent ResetRobot command with sequence {}", seq);
            }
            Err(e) => {
                error!("Failed to send ResetRobot command: {:?}", e);
            }
        }
    }
}

/// Handle SetSpeedOverride commands - sets robot speed override
pub fn handle_set_speed_override(
    mut events: MessageReader<NetworkData<SetSpeedOverride>>,
    robot_query: Query<(Entity, &RobotConnectionState, Option<&RmiDriver>, Option<&EntityControl>), With<FanucRobot>>,
) {
    for event in events.read() {
        let client_id = *event.source();
        let cmd: &SetSpeedOverride = &*event;

        // Find a connected robot
        let Some((entity, _, driver, control)) = robot_query.iter()
            .find(|(_, state, driver, _)| **state == RobotConnectionState::Connected && driver.is_some())
        else {
            warn!("SetSpeedOverride rejected: No connected robot");
            continue;
        };

        // Validate control
        if let Some(entity_control) = control {
            if entity_control.client_id != client_id {
                warn!("SetSpeedOverride rejected from {:?}: No control (held by {:?})", client_id, entity_control.client_id);
                continue;
            }
        }

        let driver = driver.expect("Checked above");

        info!("Processing SetSpeedOverride on {:?}: speed={}", entity, cmd.speed);

        // Send FrcSetOverRide command
        let command = raw_dto::Command::FrcSetOverRide(raw_dto::FrcSetOverRide { value: cmd.speed });
        let send_packet: fanuc_rmi::packets::SendPacket = raw_dto::SendPacket::Command(command).into();

        match driver.0.send_command(send_packet, PacketPriority::Immediate) {
            Ok(seq) => {
                info!("Sent SetSpeedOverride command with sequence {}", seq);
            }
            Err(e) => {
                error!("Failed to send SetSpeedOverride command: {:?}", e);
            }
        }
    }
}
