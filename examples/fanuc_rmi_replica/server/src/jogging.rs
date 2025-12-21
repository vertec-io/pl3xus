use bevy::prelude::*;
use bevy::ecs::message::MessageReader;
use bevy_tokio_tasks::TokioTasksRuntime;
use pl3xus::NetworkData;
use pl3xus_sync::AuthorizedMessage;
use fanuc_replica_types::*;
use fanuc_rmi::dto as raw_dto;
use fanuc_rmi::{SpeedType, TermType};
use fanuc_rmi::packets::PacketPriority;
use pl3xus_sync::control::EntityControl;
use crate::plugins::connection::{FanucRobot, RmiDriver, RobotConnectionState};
use crate::plugins::system::SystemMarker;
use crate::WebSocketProvider;

/// Handle jog commands from clients - entity-based, uses pl3xus EntityControl on System
pub fn handle_jog_commands(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut events: MessageReader<NetworkData<JogCommand>>,
    system_query: Query<&EntityControl, With<SystemMarker>>,
    robot_query: Query<(Entity, &RobotConnectionState, Option<&RmiDriver>), With<FanucRobot>>,
) {
    // Enter the Tokio runtime context so send_packet can use tokio::spawn
    let _guard = tokio_runtime.runtime().enter();

    for event in events.read() {
        let client_id = *event.source();
        // NetworkData<T> implements Deref<Target=T>, so we can access fields directly
        let cmd: &JogCommand = &*event;

        // Check control on System entity
        let Ok(system_control) = system_query.single() else {
            warn!("Jog rejected: No System entity found");
            continue;
        };

        if system_control.client_id != client_id {
            warn!("Jog rejected from {:?}: No control of System (held by {:?})", client_id, system_control.client_id);
            continue;
        }

        // Find a connected robot (in future, match by entity ID from command)
        let Some((entity, _, driver)) = robot_query.iter()
            .find(|(_, state, driver)| **state == RobotConnectionState::Connected && driver.is_some())
        else {
            warn!("Jog rejected: No connected robot");
            continue;
        };

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

/// Handle authorized jog commands - uses the new AuthorizedMessage pattern.
///
/// This handler receives only messages that have passed authorization middleware.
/// The middleware checks that the client has control of the target entity (System).
/// No manual control check is needed here.
pub fn handle_authorized_jog_commands(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut events: MessageReader<AuthorizedMessage<JogCommand>>,
    robot_query: Query<(Entity, &RobotConnectionState, Option<&RmiDriver>), With<FanucRobot>>,
) {
    // Enter the Tokio runtime context so send_packet can use tokio::spawn
    let _guard = tokio_runtime.runtime().enter();

    for event in events.read() {
        // Authorization already verified by middleware - no need to check EntityControl
        let cmd = &event.message;
        let target_entity = event.target_entity;

        info!(
            "Processing authorized JogCommand for entity {:?}: {:?} direction={:?} dist={} speed={}",
            target_entity, cmd.axis, cmd.direction, cmd.distance, cmd.speed
        );

        // Find a connected robot (in future, match by target_entity)
        let Some((entity, _, driver)) = robot_query.iter()
            .find(|(_, state, driver)| **state == RobotConnectionState::Connected && driver.is_some())
        else {
            warn!("Authorized jog rejected: No connected robot");
            continue;
        };

        let driver = driver.expect("Checked above");

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
                info!("Sent authorized Cartesian jog command on {:?} with sequence {}", entity, seq);
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
    system_query: Query<&EntityControl, With<SystemMarker>>,
    robot_query: Query<(Entity, &RobotConnectionState, Option<&RmiDriver>), With<FanucRobot>>,
) {
    // Enter the Tokio runtime context so send_packet can use tokio::spawn
    let _guard = tokio_runtime.runtime().enter();

    for event in events.read() {
        let client_id = *event.source();
        let cmd: &JogRobot = &*event;

        // Check control on System entity
        let Ok(system_control) = system_query.single() else {
            warn!("JogRobot rejected: No System entity found");
            continue;
        };

        if system_control.client_id != client_id {
            warn!("JogRobot rejected from {:?}: No control of System (held by {:?})", client_id, system_control.client_id);
            continue;
        }

        // Find a connected robot
        let Some((entity, _, driver)) = robot_query.iter()
            .find(|(_, state, driver)| **state == RobotConnectionState::Connected && driver.is_some())
        else {
            warn!("JogRobot rejected: No connected robot");
            continue;
        };

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
    system_query: Query<&EntityControl, With<SystemMarker>>,
    mut robot_query: Query<(Entity, &RobotConnectionState, Option<&RmiDriver>, &mut RobotStatus), With<FanucRobot>>,
) {
    // Enter the Tokio runtime context so send_packet can use tokio::spawn
    let _guard = tokio_runtime.runtime().enter();

    for event in events.read() {
        let client_id = *event.source();
        let cmd: &InitializeRobot = &*event;

        // Check control on System entity
        let Ok(system_control) = system_query.single() else {
            warn!("InitializeRobot rejected: No System entity found");
            continue;
        };

        if system_control.client_id != client_id {
            warn!("InitializeRobot rejected from {:?}: No control of System (held by {:?})", client_id, system_control.client_id);
            continue;
        }

        // Find a connected robot
        let Some((entity, _, driver, _status)) = robot_query.iter_mut()
            .find(|(_, state, driver, _)| **state == RobotConnectionState::Connected && driver.is_some())
        else {
            warn!("InitializeRobot rejected: No connected robot");
            continue;
        };

        let driver = driver.expect("Checked above");
        let _group_mask = cmd.group_mask.unwrap_or(1);

        info!("Processing InitializeRobot on {:?}", entity);

        // Use the async initialize() method which properly waits for response
        // and resets the sequence counter after successful initialization.
        let driver_clone = driver.0.clone();
        tokio_runtime.spawn_background_task(move |mut ctx| async move {
            match driver_clone.initialize().await {
                Ok(response) => {
                    if response.error_id == 0 {
                        info!("✅ FRC_Initialize successful, group_mask: {}", response.group_mask);
                        // Sequence counter is reset inside initialize() on success
                        // Update RobotStatus on main thread
                        ctx.run_on_main_thread(move |ctx| {
                            let mut status_query = ctx.world.query_filtered::<&mut RobotStatus, With<FanucRobot>>();
                            if let Ok(mut status) = status_query.single_mut(ctx.world) {
                                status.tp_program_initialized = true;
                            }
                        }).await;
                    } else {
                        error!("❌ FRC_Initialize failed with error_id: {}", response.error_id);
                        ctx.run_on_main_thread(move |ctx| {
                            let mut status_query = ctx.world.query_filtered::<&mut RobotStatus, With<FanucRobot>>();
                            if let Ok(mut status) = status_query.single_mut(ctx.world) {
                                status.tp_program_initialized = false;
                            }
                        }).await;
                    }
                }
                Err(e) => {
                    error!("❌ Failed to initialize robot: {}", e);
                    ctx.run_on_main_thread(move |ctx| {
                        let mut status_query = ctx.world.query_filtered::<&mut RobotStatus, With<FanucRobot>>();
                        if let Ok(mut status) = status_query.single_mut(ctx.world) {
                            status.tp_program_initialized = false;
                        }
                    }).await;
                }
            }
        });
    }
}

/// Handle AbortMotion commands - aborts current motion
pub fn handle_abort_motion(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut events: MessageReader<NetworkData<AbortMotion>>,
    system_query: Query<&EntityControl, With<SystemMarker>>,
    mut robot_query: Query<(Entity, &RobotConnectionState, Option<&RmiDriver>, &mut RobotStatus), With<FanucRobot>>,
) {
    // Enter the Tokio runtime context so send_packet can use tokio::spawn
    let _guard = tokio_runtime.runtime().enter();

    for event in events.read() {
        let client_id = *event.source();

        // Check control on System entity
        let Ok(system_control) = system_query.single() else {
            warn!("AbortMotion rejected: No System entity found");
            continue;
        };

        if system_control.client_id != client_id {
            warn!("AbortMotion rejected from {:?}: No control of System (held by {:?})", client_id, system_control.client_id);
            continue;
        }

        // Find a connected robot
        let Some((entity, _, driver, mut status)) = robot_query.iter_mut()
            .find(|(_, state, driver, _)| **state == RobotConnectionState::Connected && driver.is_some())
        else {
            warn!("AbortMotion rejected: No connected robot");
            continue;
        };

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
    system_query: Query<&EntityControl, With<SystemMarker>>,
    robot_query: Query<(Entity, &RobotConnectionState, Option<&RmiDriver>), With<FanucRobot>>,
) {
    // Enter the Tokio runtime context so send_packet can use tokio::spawn
    let _guard = tokio_runtime.runtime().enter();

    for event in events.read() {
        let client_id = *event.source();

        // Check control on System entity
        let Ok(system_control) = system_query.single() else {
            warn!("ResetRobot rejected: No System entity found");
            continue;
        };

        if system_control.client_id != client_id {
            warn!("ResetRobot rejected from {:?}: No control of System (held by {:?})", client_id, system_control.client_id);
            continue;
        }

        // Find a connected robot
        let Some((entity, _, driver)) = robot_query.iter()
            .find(|(_, state, driver)| **state == RobotConnectionState::Connected && driver.is_some())
        else {
            warn!("ResetRobot rejected: No connected robot");
            continue;
        };

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
    system_query: Query<&EntityControl, With<SystemMarker>>,
    robot_query: Query<(Entity, &RobotConnectionState, Option<&RmiDriver>), With<FanucRobot>>,
    net: Res<pl3xus::Network<WebSocketProvider>>,
) {
    use pl3xus_common::ServerNotification;

    // Enter the Tokio runtime context so send_packet can use tokio::spawn
    let _guard = tokio_runtime.runtime().enter();

    for event in events.read() {
        let client_id = *event.source();
        let dto_packet: &raw_dto::SendPacket = &*event;

        // Check control on System entity
        let Ok(system_control) = system_query.single() else {
            warn!("SendPacket rejected: No System entity found");
            let _ = net.send(
                client_id,
                ServerNotification::error("No system entity found").with_context("SendPacket"),
            );
            continue;
        };

        if system_control.client_id != client_id {
            let holder_msg = if system_control.client_id.id == 0 {
                "No client has control"
            } else {
                "Another client has control"
            };
            warn!("SendPacket rejected from {:?}: No control of System (held by {:?})", client_id, system_control.client_id);
            let _ = net.send(
                client_id,
                ServerNotification::warning(format!("Motion command rejected: {}", holder_msg))
                    .with_context("SendPacket"),
            );
            continue;
        }

        // Find a connected robot
        let Some((entity, _, driver)) = robot_query.iter()
            .find(|(_, state, driver)| **state == RobotConnectionState::Connected && driver.is_some())
        else {
            warn!("SendPacket rejected: No connected robot");
            let _ = net.send(
                client_id,
                ServerNotification::error("No connected robot available").with_context("SendPacket"),
            );
            continue;
        };

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
                let _ = net.send(
                    client_id,
                    ServerNotification::error(format!("Failed to send packet: {:?}", e))
                        .with_context("SendPacket"),
                );
            }
        }
    }
}

/// Handle SetSpeedOverride commands - sets robot speed override
pub fn handle_set_speed_override(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut events: MessageReader<NetworkData<SetSpeedOverride>>,
    system_query: Query<&EntityControl, With<SystemMarker>>,
    robot_query: Query<(Entity, &RobotConnectionState, Option<&RmiDriver>), With<FanucRobot>>,
) {
    // Enter the Tokio runtime context so send_packet can use tokio::spawn
    let _guard = tokio_runtime.runtime().enter();

    for event in events.read() {
        let client_id = *event.source();
        let cmd: &SetSpeedOverride = &*event;

        // Check control on System entity
        let Ok(system_control) = system_query.single() else {
            warn!("SetSpeedOverride rejected: No System entity found");
            continue;
        };

        if system_control.client_id != client_id {
            warn!("SetSpeedOverride rejected from {:?}: No control of System (held by {:?})", client_id, system_control.client_id);
            continue;
        }

        // Find a connected robot
        let Some((entity, _, driver)) = robot_query.iter()
            .find(|(_, state, driver)| **state == RobotConnectionState::Connected && driver.is_some())
        else {
            warn!("SetSpeedOverride rejected: No connected robot");
            continue;
        };

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
