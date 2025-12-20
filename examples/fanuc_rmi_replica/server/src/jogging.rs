use bevy::prelude::*;
use bevy::ecs::message::MessageReader;
use bevy_tokio_tasks::TokioTasksRuntime;
use pl3xus::NetworkData;
use fanuc_replica_types::*;
use fanuc_rmi::dto as raw_dto;
use fanuc_rmi::{SpeedType, TermType};
use fanuc_rmi::packets::PacketPriority;
use pl3xus_sync::control::EntityControl;
use crate::plugins::connection::{FanucRobot, RmiDriver, RobotConnectionState};

/// Handle jog commands from clients - entity-based, uses pl3xus EntityControl
pub fn handle_jog_commands(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut events: MessageReader<NetworkData<JogCommand>>,
    robot_query: Query<(Entity, &RobotConnectionState, Option<&RmiDriver>, Option<&EntityControl>), With<FanucRobot>>,
) {
    // Enter the Tokio runtime context so send_packet can use tokio::spawn
    let _guard = tokio_runtime.runtime().enter();

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
        let dist = if cmd.direction == JogDirection::Positive { cmd.distance as f64 } else { -(cmd.distance as f64) };

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

        match driver.0.send_packet(send_packet, PacketPriority::Immediate) {
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
/// Supports both Cartesian jogs (X/Y/Z/W/P/R) using FrcLinearRelative
/// and Joint jogs (J1-J6) using FrcJointRelativeJRep.
pub fn handle_jog_robot_commands(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut events: MessageReader<NetworkData<JogRobot>>,
    robot_query: Query<(Entity, &RobotConnectionState, Option<&RmiDriver>, Option<&EntityControl>), With<FanucRobot>>,
) {
    // Enter the Tokio runtime context so send_packet can use tokio::spawn
    let _guard = tokio_runtime.runtime().enter();

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

        info!("Processing JogRobot on {:?}: {:?} dist={} speed={}",
            entity, cmd.axis, cmd.distance, cmd.speed);

        let send_packet = if is_joint_jog {
            // Joint jog - use FrcJointRelativeJRep instruction
            // Build joint angles delta (only the target joint has a non-zero value)
            let mut joint_angles = raw_dto::JointAngles {
                j1: 0.0, j2: 0.0, j3: 0.0, j4: 0.0, j5: 0.0, j6: 0.0,
                j7: 0.0, j8: 0.0, j9: 0.0,
            };

            match cmd.axis {
                JogAxis::J1 => joint_angles.j1 = cmd.distance,
                JogAxis::J2 => joint_angles.j2 = cmd.distance,
                JogAxis::J3 => joint_angles.j3 = cmd.distance,
                JogAxis::J4 => joint_angles.j4 = cmd.distance,
                JogAxis::J5 => joint_angles.j5 = cmd.distance,
                JogAxis::J6 => joint_angles.j6 = cmd.distance,
                _ => continue, // Cartesian jogs handled below
            }

            // Use Time speed type for joint motion (as per original Fanuc_RMI_API)
            let instruction = raw_dto::Instruction::FrcJointRelativeJRep(raw_dto::FrcJointRelativeJRep {
                sequence_id: 0,
                joint_angles,
                speed_type: SpeedType::Time.into(),
                speed: cmd.speed as f64,
                term_type: TermType::FINE.into(), // FINE for step moves
                term_value: 1,
            });

            let packet: fanuc_rmi::packets::SendPacket =
                raw_dto::SendPacket::Instruction(instruction).into();
            packet
        } else {
            // Cartesian jog - use FrcLinearRelative instruction
            let mut pos = raw_dto::Position {
                x: 0.0, y: 0.0, z: 0.0,
                w: 0.0, p: 0.0, r: 0.0,
                ext1: 0.0, ext2: 0.0, ext3: 0.0,
            };

            match cmd.axis {
                JogAxis::X => pos.x = cmd.distance as f64,
                JogAxis::Y => pos.y = cmd.distance as f64,
                JogAxis::Z => pos.z = cmd.distance as f64,
                JogAxis::W => pos.w = cmd.distance as f64,
                JogAxis::P => pos.p = cmd.distance as f64,
                JogAxis::R => pos.r = cmd.distance as f64,
                _ => continue, // Joint jogs handled above
            }

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

            let packet: fanuc_rmi::packets::SendPacket =
                raw_dto::SendPacket::Instruction(instruction).into();
            packet
        };

        match driver.0.send_packet(send_packet, PacketPriority::Immediate) {
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
    tokio_runtime: Res<TokioTasksRuntime>,
    mut events: MessageReader<NetworkData<InitializeRobot>>,
    mut robot_query: Query<(Entity, &RobotConnectionState, Option<&RmiDriver>, Option<&EntityControl>, &mut RobotStatus), With<FanucRobot>>,
) {
    // Enter the Tokio runtime context so send_packet can use tokio::spawn
    let _guard = tokio_runtime.runtime().enter();

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

        match driver.0.send_packet(send_packet, PacketPriority::Immediate) {
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
    tokio_runtime: Res<TokioTasksRuntime>,
    mut events: MessageReader<NetworkData<AbortMotion>>,
    mut robot_query: Query<(Entity, &RobotConnectionState, Option<&RmiDriver>, Option<&EntityControl>, &mut RobotStatus), With<FanucRobot>>,
) {
    // Enter the Tokio runtime context so send_packet can use tokio::spawn
    let _guard = tokio_runtime.runtime().enter();

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

        match driver.0.send_packet(send_packet, PacketPriority::Immediate) {
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
    tokio_runtime: Res<TokioTasksRuntime>,
    mut events: MessageReader<NetworkData<ResetRobot>>,
    robot_query: Query<(Entity, &RobotConnectionState, Option<&RmiDriver>, Option<&EntityControl>), With<FanucRobot>>,
) {
    // Enter the Tokio runtime context so send_packet can use tokio::spawn
    let _guard = tokio_runtime.runtime().enter();

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

        match driver.0.send_packet(send_packet, PacketPriority::Immediate) {
            Ok(seq) => {
                info!("Sent ResetRobot command with sequence {}", seq);
            }
            Err(e) => {
                error!("Failed to send ResetRobot command: {:?}", e);
            }
        }
    }
}

/// Handle SendPacket messages - forwards fanuc_rmi::dto::SendPacket directly to the driver
/// This is the primary way to send motion commands from the client
pub fn handle_send_packet(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut events: MessageReader<NetworkData<raw_dto::SendPacket>>,
    robot_query: Query<(Entity, &RobotConnectionState, Option<&RmiDriver>, Option<&EntityControl>), With<FanucRobot>>,
) {
    // Enter the Tokio runtime context so send_packet can use tokio::spawn
    let _guard = tokio_runtime.runtime().enter();

    for event in events.read() {
        let client_id = *event.source();
        let dto_packet: &raw_dto::SendPacket = &*event;

        // Find a connected robot
        let Some((entity, _, driver, control)) = robot_query.iter()
            .find(|(_, state, driver, _)| **state == RobotConnectionState::Connected && driver.is_some())
        else {
            warn!("SendPacket rejected: No connected robot");
            continue;
        };

        // Validate control for non-read commands
        if let Some(entity_control) = control {
            if entity_control.client_id != client_id {
                warn!("SendPacket rejected from {:?}: No control (held by {:?})", client_id, entity_control.client_id);
                continue;
            }
        }

        let driver = driver.expect("Checked above");

        // Convert DTO to protocol type using Into
        let protocol_packet: fanuc_rmi::packets::SendPacket = dto_packet.clone().into();

        info!("Processing SendPacket on {:?}: {:?}", entity, protocol_packet);

        match driver.0.send_packet(protocol_packet, PacketPriority::Immediate) {
            Ok(seq) => {
                info!("Sent packet with sequence {}", seq);
            }
            Err(e) => {
                error!("Failed to send packet: {:?}", e);
            }
        }
    }
}

/// Handle SetSpeedOverride commands - sets robot speed override
pub fn handle_set_speed_override(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut events: MessageReader<NetworkData<SetSpeedOverride>>,
    robot_query: Query<(Entity, &RobotConnectionState, Option<&RmiDriver>, Option<&EntityControl>), With<FanucRobot>>,
) {
    // Enter the Tokio runtime context so send_packet can use tokio::spawn
    let _guard = tokio_runtime.runtime().enter();

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

        match driver.0.send_packet(send_packet, PacketPriority::Immediate) {
            Ok(seq) => {
                info!("Sent SetSpeedOverride command with sequence {}", seq);
            }
            Err(e) => {
                error!("Failed to send SetSpeedOverride command: {:?}", e);
            }
        }
    }
}
