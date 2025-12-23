use bevy::prelude::*;
use bevy::ecs::message::MessageReader;
use bevy_tokio_tasks::TokioTasksRuntime;
use pl3xus_sync::{AuthorizedTargetedMessage, AuthorizedRequest};
use fanuc_replica_types::*;
use fanuc_rmi::dto as raw_dto;
use fanuc_rmi::{SpeedType, TermType};
use fanuc_rmi::packets::PacketPriority;
use crate::plugins::connection::{FanucRobot, RmiDriver, RobotConnectionState};
use crate::WebSocketProvider;

/// Handle authorized jog commands - uses the new AuthorizedTargetedMessage pattern.
///
/// This handler receives only messages that have passed authorization middleware.
/// The middleware checks that the client has control of the target entity (System).
/// No manual control check is needed here.
///
/// Speed and step values are read from the robot's JogSettingsState component,
/// not from the client message. This ensures jog settings are tied to the robot
/// entity, not the client, so any client that takes control uses the same settings.
pub fn handle_authorized_jog_commands(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut events: MessageReader<AuthorizedTargetedMessage<JogCommand>>,
    robot_query: Query<(Entity, &RobotConnectionState, Option<&RmiDriver>, &JogSettingsState), With<FanucRobot>>,
) {
    // Enter the Tokio runtime context so send_packet can use tokio::spawn
    let _guard = tokio_runtime.runtime().enter();

    for event in events.read() {
        // Authorization already verified by middleware - no need to check EntityControl
        let cmd = &event.message;
        let target_entity = event.target_entity;

        // Find a connected robot (in future, match by target_entity)
        let Some((entity, _, driver, jog_settings)) = robot_query.iter()
            .find(|(_, state, driver, _)| **state == RobotConnectionState::Connected && driver.is_some())
        else {
            warn!("Authorized jog rejected: No connected robot");
            continue;
        };

        let driver = driver.expect("Checked above");

        // Get speed and step from the robot's JogSettingsState
        let (speed, step) = match cmd.axis {
            JogAxis::W | JogAxis::P | JogAxis::R => {
                (jog_settings.rotation_jog_speed, jog_settings.rotation_jog_step)
            }
            JogAxis::J1 | JogAxis::J2 | JogAxis::J3 | JogAxis::J4 | JogAxis::J5 | JogAxis::J6 => {
                (jog_settings.joint_jog_speed, jog_settings.joint_jog_step)
            }
            _ => {
                (jog_settings.cartesian_jog_speed, jog_settings.cartesian_jog_step)
            }
        };

        info!(
            "Processing authorized JogCommand for entity {:?}: {:?} direction={:?} (using server settings: step={}, speed={})",
            target_entity, cmd.axis, cmd.direction, step, speed
        );

        // Build position delta (PositionDto uses f32)
        let mut pos = raw_dto::Position {
            x: 0.0, y: 0.0, z: 0.0,
            w: 0.0, p: 0.0, r: 0.0,
            ext1: 0.0, ext2: 0.0, ext3: 0.0,
        };
        let dist = if cmd.direction == JogDirection::Positive { step } else { -step };

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
            speed,
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
///
/// Authorization is handled by middleware - no manual control check needed.
pub fn handle_jog_robot_commands(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut events: MessageReader<AuthorizedTargetedMessage<JogRobot>>,
    robot_query: Query<(Entity, &RobotConnectionState, Option<&RmiDriver>), With<FanucRobot>>,
) {
    // Enter the Tokio runtime context so send_packet can use tokio::spawn
    let _guard = tokio_runtime.runtime().enter();

    for event in events.read() {
        let cmd = &event.message;
        let target_entity = event.target_entity;

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

        info!("Processing authorized JogRobot for {:?} on {:?}: {:?} dist={} speed={}",
            target_entity, entity, cmd.axis, cmd.distance, cmd.speed);

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

/// Handle InitializeRobot requests - initializes the robot for motion
///
/// Authorization is handled by middleware - no manual control check needed.
/// This is a targeted request that returns a response to the client.
pub fn handle_initialize_robot(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut events: MessageReader<AuthorizedRequest<InitializeRobot>>,
    mut robot_query: Query<(Entity, &RobotConnectionState, Option<&RmiDriver>, &mut RobotStatus), With<FanucRobot>>,
) {
    // Enter the Tokio runtime context so send_packet can use tokio::spawn
    let _guard = tokio_runtime.runtime().enter();

    for event in events.read() {
        let event = event.clone();
        let cmd = event.get_request().clone();
        let target_entity = event.target_entity;

        // Find a connected robot
        let Some((entity, _, driver, _status)) = robot_query.iter_mut()
            .find(|(_, state, driver, _)| **state == RobotConnectionState::Connected && driver.is_some())
        else {
            warn!("InitializeRobot rejected: No connected robot");
            let _ = event.respond(InitializeRobotResponse {
                success: false,
                error: Some("No connected robot".to_string()),
            });
            continue;
        };

        let driver = driver.expect("Checked above");
        let _group_mask = cmd.group_mask.unwrap_or(1);

        info!("Processing authorized InitializeRobot for {:?} on {:?}", target_entity, entity);

        // Use the async initialize() method which properly waits for response
        // and resets the sequence counter after successful initialization.
        let driver_clone = driver.0.clone();
        // Take the responder for async response handling
        let responder = event.take_responder();
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
                        // Send success response
                        let _ = responder.respond(InitializeRobotResponse { success: true, error: None });
                    } else {
                        let error_msg = format!("FRC_Initialize failed with error_id: {}", response.error_id);
                        error!("❌ {}", error_msg);
                        ctx.run_on_main_thread(move |ctx| {
                            let mut status_query = ctx.world.query_filtered::<&mut RobotStatus, With<FanucRobot>>();
                            if let Ok(mut status) = status_query.single_mut(ctx.world) {
                                status.tp_program_initialized = false;
                            }
                        }).await;
                        // Send error response
                        let _ = responder.respond(InitializeRobotResponse { success: false, error: Some(error_msg) });
                    }
                }
                Err(e) => {
                    let error_msg = format!("Failed to initialize robot: {}", e);
                    error!("❌ {}", error_msg);
                    ctx.run_on_main_thread(move |ctx| {
                        let mut status_query = ctx.world.query_filtered::<&mut RobotStatus, With<FanucRobot>>();
                        if let Ok(mut status) = status_query.single_mut(ctx.world) {
                            status.tp_program_initialized = false;
                        }
                    }).await;
                    // Send error response
                    let _ = responder.respond(InitializeRobotResponse { success: false, error: Some(error_msg) });
                }
            }
        });
    }
}

/// Handle AbortMotion requests - aborts current motion
///
/// Authorization is handled by middleware - no manual control check needed.
/// This is a targeted request that returns a response to the client.
pub fn handle_abort_motion(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut events: MessageReader<AuthorizedRequest<AbortMotion>>,
    mut robot_query: Query<(Entity, &RobotConnectionState, Option<&RmiDriver>, &mut RobotStatus), With<FanucRobot>>,
) {
    // Enter the Tokio runtime context so send_packet can use tokio::spawn
    let _guard = tokio_runtime.runtime().enter();

    for event in events.read() {
        let event = event.clone();
        let target_entity = event.target_entity;

        // Find a connected robot
        let Some((entity, _, driver, mut status)) = robot_query.iter_mut()
            .find(|(_, state, driver, _)| **state == RobotConnectionState::Connected && driver.is_some())
        else {
            warn!("AbortMotion rejected: No connected robot");
            let _ = event.respond(AbortMotionResponse {
                success: false,
                error: Some("No connected robot".to_string()),
            });
            continue;
        };

        let driver = driver.expect("Checked above");

        info!("Processing authorized AbortMotion for {:?} on {:?}", target_entity, entity);

        // Send FrcAbort command (unit variant)
        let command = raw_dto::Command::FrcAbort;
        let send_packet: fanuc_rmi::packets::SendPacket = raw_dto::SendPacket::Command(command).into();

        match driver.0.send_packet(send_packet, PacketPriority::Immediate) {
            Ok(seq) => {
                info!("Sent AbortMotion command with sequence {}", seq);
                // Mark TP program as not initialized after abort
                status.tp_program_initialized = false;
                let _ = event.respond(AbortMotionResponse { success: true, error: None });
            }
            Err(e) => {
                error!("Failed to send AbortMotion command: {:?}", e);
                let _ = event.respond(AbortMotionResponse {
                    success: false,
                    error: Some(format!("Failed to send AbortMotion: {:?}", e)),
                });
            }
        }
    }
}

/// Handle ResetRobot requests - resets robot errors
///
/// Authorization is handled by middleware - no manual control check needed.
/// This is a targeted request that returns a response to the client.
pub fn handle_reset_robot(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut events: MessageReader<AuthorizedRequest<ResetRobot>>,
    robot_query: Query<(Entity, &RobotConnectionState, Option<&RmiDriver>), With<FanucRobot>>,
) {
    // Enter the Tokio runtime context so send_packet can use tokio::spawn
    let _guard = tokio_runtime.runtime().enter();

    for event in events.read() {
        let event = event.clone();
        let target_entity = event.target_entity;

        // Find a connected robot
        let Some((entity, _, driver)) = robot_query.iter()
            .find(|(_, state, driver)| **state == RobotConnectionState::Connected && driver.is_some())
        else {
            warn!("ResetRobot rejected: No connected robot");
            let _ = event.respond(ResetRobotResponse {
                success: false,
                error: Some("No connected robot".to_string()),
            });
            continue;
        };

        let driver = driver.expect("Checked above");

        info!("Processing authorized ResetRobot for {:?} on {:?}", target_entity, entity);

        // Send FrcReset command (unit variant)
        let command = raw_dto::Command::FrcReset;
        let send_packet: fanuc_rmi::packets::SendPacket = raw_dto::SendPacket::Command(command).into();

        match driver.0.send_packet(send_packet, PacketPriority::Immediate) {
            Ok(seq) => {
                info!("Sent ResetRobot command with sequence {}", seq);
                let _ = event.respond(ResetRobotResponse { success: true, error: None });
            }
            Err(e) => {
                error!("Failed to send ResetRobot command: {:?}", e);
                let _ = event.respond(ResetRobotResponse {
                    success: false,
                    error: Some(format!("Failed to send ResetRobot: {:?}", e)),
                });
            }
        }
    }
}

/// Handle SendPacket messages - forwards fanuc_rmi::dto::SendPacket directly to the driver
/// This is the primary way to send motion commands from the client
///
/// Authorization is handled by middleware - no manual control check needed.
pub fn handle_send_packet(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut events: MessageReader<AuthorizedTargetedMessage<raw_dto::SendPacket>>,
    robot_query: Query<(Entity, &RobotConnectionState, Option<&RmiDriver>), With<FanucRobot>>,
    net: Res<pl3xus::Network<WebSocketProvider>>,
) {
    use pl3xus_common::ServerNotification;

    // Enter the Tokio runtime context so send_packet can use tokio::spawn
    let _guard = tokio_runtime.runtime().enter();

    for event in events.read() {
        let dto_packet = &event.message;
        let target_entity = event.target_entity;
        let source = event.source;

        // Find a connected robot
        let Some((entity, _, driver)) = robot_query.iter()
            .find(|(_, state, driver)| **state == RobotConnectionState::Connected && driver.is_some())
        else {
            warn!("SendPacket rejected: No connected robot");
            let _ = net.send(
                source,
                ServerNotification::error("No connected robot available").with_context("SendPacket"),
            );
            continue;
        };

        let driver = driver.expect("Checked above");

        // Convert DTO to protocol type using Into
        let protocol_packet: fanuc_rmi::packets::SendPacket = dto_packet.clone().into();

        info!("Processing authorized SendPacket for {:?} on {:?}: {:?}", target_entity, entity, protocol_packet);

        match driver.0.send_packet(protocol_packet, PacketPriority::Immediate) {
            Ok(seq) => {
                info!("Sent packet with sequence {}", seq);
            }
            Err(e) => {
                error!("Failed to send packet: {:?}", e);
                let _ = net.send(
                    source,
                    ServerNotification::error(format!("Failed to send packet: {:?}", e))
                        .with_context("SendPacket"),
                );
            }
        }
    }
}

/// Handle SetSpeedOverride requests - sets robot speed override
///
/// Authorization is handled by middleware - no manual control check needed.
/// This is a targeted request that returns a response to the client.
pub fn handle_set_speed_override(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut events: MessageReader<AuthorizedRequest<SetSpeedOverride>>,
    robot_query: Query<(Entity, &RobotConnectionState, Option<&RmiDriver>), With<FanucRobot>>,
) {
    // Enter the Tokio runtime context so send_packet can use tokio::spawn
    let _guard = tokio_runtime.runtime().enter();

    for event in events.read() {
        let event = event.clone();
        let cmd = event.get_request().clone();
        let target_entity = event.target_entity;

        // Find a connected robot
        let Some((entity, _, driver)) = robot_query.iter()
            .find(|(_, state, driver)| **state == RobotConnectionState::Connected && driver.is_some())
        else {
            warn!("SetSpeedOverride rejected: No connected robot");
            let _ = event.respond(SetSpeedOverrideResponse {
                success: false,
                error: Some("No connected robot".to_string()),
            });
            continue;
        };

        let driver = driver.expect("Checked above");

        info!("Processing authorized SetSpeedOverride for {:?} on {:?}: speed={}", target_entity, entity, cmd.speed);

        // Send FrcSetOverRide command
        let command = raw_dto::Command::FrcSetOverRide(raw_dto::FrcSetOverRide { value: cmd.speed });
        let send_packet: fanuc_rmi::packets::SendPacket = raw_dto::SendPacket::Command(command).into();

        match driver.0.send_packet(send_packet, PacketPriority::Immediate) {
            Ok(seq) => {
                info!("Sent SetSpeedOverride command with sequence {}", seq);
                let _ = event.respond(SetSpeedOverrideResponse { success: true, error: None });
            }
            Err(e) => {
                error!("Failed to send SetSpeedOverride command: {:?}", e);
                let _ = event.respond(SetSpeedOverrideResponse {
                    success: false,
                    error: Some(format!("Failed to send SetSpeedOverride: {:?}", e)),
                });
            }
        }
    }
}

/// Handle JogSettingsState component mutations.
///
/// This handler is called when a client mutates the JogSettingsState component.
/// It validates the new settings and applies them to the robot entity.
/// The mutation response is sent back to the client via MutationResponseQueue.
pub fn handle_jog_settings_mutation(
    mut events: MessageReader<pl3xus_sync::ComponentMutation<JogSettingsState>>,
    mut jog_settings_query: Query<&mut JogSettingsState, With<FanucRobot>>,
    mut response_queue: ResMut<pl3xus_sync::MutationResponseQueue>,
) {
    for event in events.read() {
        let new_settings = &event.new_value;

        // Validate settings
        let validation_error = validate_jog_settings(new_settings);

        if let Some(error) = validation_error {
            // Send error response
            response_queue.respond_error(event.connection_id, event.request_id, error);
            continue;
        }

        // Apply the new settings to the robot entity
        if let Ok(mut settings) = jog_settings_query.get_mut(event.entity) {
            *settings = new_settings.clone();

            info!(
                "JogSettingsState updated for entity {:?}: cart_speed={}, cart_step={}, joint_speed={}, joint_step={}",
                event.entity,
                new_settings.cartesian_jog_speed,
                new_settings.cartesian_jog_step,
                new_settings.joint_jog_speed,
                new_settings.joint_jog_step
            );

            // Send success response
            response_queue.respond_ok(event.connection_id, event.request_id);
        } else {
            // Entity not found or doesn't have JogSettingsState
            response_queue.respond_error(event.connection_id, event.request_id, "Robot entity not found");
        }
    }
}

/// Validate jog settings values.
fn validate_jog_settings(settings: &JogSettingsState) -> Option<String> {
    if settings.cartesian_jog_speed <= 0.0 || settings.cartesian_jog_speed > 1000.0 {
        return Some("Cartesian jog speed must be between 0 and 1000 mm/s".to_string());
    }
    if settings.cartesian_jog_step <= 0.0 || settings.cartesian_jog_step > 100.0 {
        return Some("Cartesian jog step must be between 0 and 100 mm".to_string());
    }
    if settings.joint_jog_speed <= 0.0 || settings.joint_jog_speed > 100.0 {
        return Some("Joint jog speed must be between 0 and 100 °/s".to_string());
    }
    if settings.joint_jog_step <= 0.0 || settings.joint_jog_step > 90.0 {
        return Some("Joint jog step must be between 0 and 90 °".to_string());
    }
    if settings.rotation_jog_speed <= 0.0 || settings.rotation_jog_speed > 100.0 {
        return Some("Rotation jog speed must be between 0 and 100 °/s".to_string());
    }
    if settings.rotation_jog_step <= 0.0 || settings.rotation_jog_step > 90.0 {
        return Some("Rotation jog step must be between 0 and 90 °".to_string());
    }
    None
}
