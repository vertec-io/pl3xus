//! Request/Response handlers plugin.
//!
//! Handles database queries like ListRobotConnections and GetRobotConfigurations.

use bevy::prelude::*;
use bevy::ecs::message::MessageReader;
use pl3xus::managers::network_request::Request;
use pl3xus::Network;
use pl3xus_websockets::WebSocketProvider;
use pl3xus_sync::{AuthorizedRequest, TargetedRequest};
use pl3xus_sync::authorization::AppBatchRequestRegistrationExt;
use pl3xus_sync::RequestInvalidateExt;
use crate::types::*;
use crate::database;
use fanuc_replica_core::DatabaseResource;

// Type alias for WebSocket network provider - reduces verbosity
type WS = WebSocketProvider;

use bevy_tokio_tasks::TokioTasksRuntime;
use crate::connection::{FanucRobot, RmiDriver, RobotConnectionState};
use fanuc_rmi::packets::PacketPriority;

pub struct RequestHandlerPlugin;

impl Plugin for RequestHandlerPlugin {
    fn build(&self, app: &mut App) {
        // =====================================================================
        // Request Registration (using pl3xus_sync builder API)
        // =====================================================================
        //
        // All requests use the new app.requests::<...>() batch registration API.
        // This is more concise and makes it easy to add targeting/authorization later.
        //
        // Note: StartProgram, PauseProgram, ResumeProgram, StopProgram, UnloadProgram
        // are registered as targeted requests in sync.rs with authorization middleware.
        //
        // Query Invalidation: Rules are defined on request types using
        // #[derive(Invalidates)] with #[invalidates("QueryName")] attribute.
        // Handlers call broadcast_invalidations_for::<RequestType, _>(&net, None)
        // after successful responses.

        // Robot Connections & Configurations
        app.requests::<(
            ListRobotConnections,
            GetRobotConfigurations,
            CreateRobotConnection,
            UpdateRobotConnection,
            DeleteRobotConnection,
            CreateConfiguration,
            UpdateConfiguration,
            DeleteConfiguration,
            SetDefaultConfiguration,
            LoadConfiguration,
            SaveCurrentConfiguration,
        ), WS>().register();

        // Note: Program CRUD operations (ListPrograms, GetProgram, CreateProgram, etc.)
        // are handled by the ProgramsPlugin in the programs crate.
        // Only execution handlers (LoadProgram, StartProgram, etc.) remain here.
        // LoadProgram is registered as a targeted request in sync.rs

        // Frame/Tool and I/O Operations are registered as targeted requests in sync.rs
        // Only non-targeted I/O config operations remain here
        app.requests::<(
            GetIoConfig,
            UpdateIoConfig,
        ), WS>().register();

        // Settings (non-targeted, global operations)
        // Note: ResetDatabase is now handled by the core plugin
        app.requests::<(
            GetSettings,
            UpdateSettings,
            UpdateJogSettings,
        ), WS>().register();

        // Add handler systems - Robot Connections & Configurations
        app.add_systems(Update, (
            handle_list_robot_connections,
            handle_get_robot_configurations,
            handle_create_robot_connection,
            handle_update_robot_connection,
            handle_delete_robot_connection,
            handle_create_configuration,
            handle_update_configuration,
            handle_delete_configuration,
            handle_set_default_configuration,
            handle_load_configuration,
            handle_save_current_configuration,
        ));

        // Note: Program execution commands (Load, Start, Pause, Resume, Stop, Unload)
        // are now handled by the execution plugin (Start/Pause/Resume/Stop) and
        // programs plugin (Load/Unload). FANUC-specific reactions to state changes
        // are in the execution_sync module.

        // Note: Frame/Tool and I/O handlers are registered in sync.rs as targeted requests
        // Only non-targeted handlers remain here

        // Add handler systems - I/O Config (non-targeted)
        app.add_systems(Update, (
            handle_get_io_config,
            handle_update_io_config,
        ));

        // Add handler systems - Settings (non-targeted)
        // Note: handle_reset_database is now in the core plugin
        app.add_systems(Update, (
            handle_get_settings,
            handle_update_settings,
            handle_update_jog_settings,
        ));
    }
}

/// Handle ListRobotConnections request - returns saved robots from database.
fn handle_list_robot_connections(
    mut requests: MessageReader<Request<ListRobotConnections>>,
    db: Option<Res<DatabaseResource>>,
) {
    for request in requests.read() {
        info!("📋 Handling ListRobotConnections request");

        let connections = db.as_ref()
            .map(|db| {
                let conn = db.connection();
                let conn = conn.lock().unwrap();
                database::list_robot_connections(&conn).unwrap_or_default()
            })
            .unwrap_or_default();

        info!("📤 Responding with {} robot connections", connections.len());
        // Clone request since respond() takes ownership
        if let Err(e) = request.clone().respond(RobotConnectionsResponse { connections }) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle GetRobotConfigurations request - returns configurations for a robot.
fn handle_get_robot_configurations(
    mut requests: MessageReader<Request<GetRobotConfigurations>>,
    db: Option<Res<DatabaseResource>>,
) {
    for request in requests.read() {
        // Access inner request data via get_request()
        let inner = request.get_request();
        let robot_connection_id = inner.robot_connection_id;
        info!("📋 Handling GetRobotConfigurations for robot_connection_id={}", robot_connection_id);

        let configurations = db.as_ref()
            .map(|db| {
                let conn = db.connection();
                let conn = conn.lock().unwrap();
                database::get_configurations_for_robot(&conn, robot_connection_id).unwrap_or_default()
            })
            .unwrap_or_default();

        info!("📤 Responding with {} configurations", configurations.len());
        // Clone request since respond() takes ownership
        if let Err(e) = request.clone().respond(RobotConfigurationsResponse { robot_id: robot_connection_id, configurations }) {
            error!("Failed to send response: {:?}", e);
        }
    }
}


/// Handle CreateRobotConnection request - creates a new robot in database.
fn handle_create_robot_connection(
    mut requests: MessageReader<Request<CreateRobotConnection>>,
    db: Option<Res<DatabaseResource>>,
    net: Res<Network<WebSocketProvider>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling CreateRobotConnection for '{}'", inner.name);

        let result = db.as_ref()
            .map(|db| {
                let conn = db.connection();
                let conn = conn.lock().unwrap();
                database::create_robot_connection(&conn, inner)
            })
            .unwrap_or(Err(anyhow::anyhow!("Database not available")));

        let response = match result {
            Ok(robot_id) => {
                info!("✅ Created robot connection id={}", robot_id);
                CreateRobotConnectionResponse {
                    robot_id,
                    success: true,
                    error: None,
                }
            }
            Err(e) => {
                error!("❌ Failed to create robot connection: {}", e);
                CreateRobotConnectionResponse {
                    robot_id: 0,
                    success: false,
                    error: Some(e.to_string()),
                }
            }
        };

        // respond_and_invalidate automatically broadcasts invalidations on success
        if let Err(e) = request.clone().respond_and_invalidate(response, &net) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle UpdateRobotConnection request - updates an existing robot in database.
fn handle_update_robot_connection(
    mut requests: MessageReader<Request<UpdateRobotConnection>>,
    db: Option<Res<DatabaseResource>>,
    net: Res<Network<WebSocketProvider>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling UpdateRobotConnection for id={}", inner.id);

        let result = db.as_ref()
            .map(|db| {
                let conn = db.connection();
                let conn = conn.lock().unwrap();
                database::update_robot_connection(&conn, inner)
            })
            .unwrap_or(Err(anyhow::anyhow!("Database not available")));

        let response = match result {
            Ok(()) => {
                info!("✅ Updated robot connection id={}", inner.id);
                UpdateRobotConnectionResponse {
                    success: true,
                    error: None,
                }
            }
            Err(e) => {
                error!("❌ Failed to update robot connection: {}", e);
                UpdateRobotConnectionResponse {
                    success: false,
                    error: Some(e.to_string()),
                }
            }
        };

        // respond_and_invalidate automatically broadcasts invalidations on success
        if let Err(e) = request.clone().respond_and_invalidate(response, &net) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle DeleteRobotConnection request - deletes a robot from database.
fn handle_delete_robot_connection(
    mut requests: MessageReader<Request<DeleteRobotConnection>>,
    db: Option<Res<DatabaseResource>>,
    net: Res<Network<WebSocketProvider>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling DeleteRobotConnection for id={}", inner.id);

        let result = db.as_ref()
            .map(|db| {
                let conn = db.connection();
                let conn = conn.lock().unwrap();
                database::delete_robot_connection(&conn, inner.id)
            })
            .unwrap_or(Err(anyhow::anyhow!("Database not available")));

        let response = match result {
            Ok(()) => {
                info!("✅ Deleted robot connection id={}", inner.id);
                DeleteRobotConnectionResponse {
                    success: true,
                    error: None,
                }
            }
            Err(e) => {
                error!("❌ Failed to delete robot connection: {}", e);
                DeleteRobotConnectionResponse {
                    success: false,
                    error: Some(e.to_string()),
                }
            }
        };

        // respond_and_invalidate automatically broadcasts invalidations on success
        if let Err(e) = request.clone().respond_and_invalidate(response, &net) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle CreateConfiguration request - creates a new configuration for a robot.
fn handle_create_configuration(
    mut requests: MessageReader<Request<CreateConfiguration>>,
    db: Option<Res<DatabaseResource>>,
    net: Res<Network<WebSocketProvider>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling CreateConfiguration for robot_connection_id={}", inner.robot_connection_id);

        let result = db.as_ref()
            .map(|db| {
                let conn = db.connection();
                let conn = conn.lock().unwrap();
                database::create_configuration(&conn, inner)
            })
            .unwrap_or(Err(anyhow::anyhow!("Database not available")));

        let response = match result {
            Ok(id) => {
                info!("✅ Created configuration id={}", id);
                CreateConfigurationResponse {
                    success: true,
                    configuration_id: id,
                    error: None,
                }
            }
            Err(e) => {
                error!("❌ Failed to create configuration: {}", e);
                CreateConfigurationResponse {
                    success: false,
                    configuration_id: 0,
                    error: Some(e.to_string()),
                }
            }
        };

        // respond_and_invalidate automatically broadcasts invalidations on success
        if let Err(e) = request.clone().respond_and_invalidate(response, &net) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle UpdateConfiguration request - updates an existing configuration.
fn handle_update_configuration(
    mut requests: MessageReader<Request<UpdateConfiguration>>,
    db: Option<Res<DatabaseResource>>,
    net: Res<Network<WebSocketProvider>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling UpdateConfiguration for id={}", inner.id);

        let result = db.as_ref()
            .map(|db| {
                let conn = db.connection();
                let conn = conn.lock().unwrap();
                database::update_configuration(&conn, inner)
            })
            .unwrap_or(Err(anyhow::anyhow!("Database not available")));

        let response = match result {
            Ok(()) => {
                info!("✅ Updated configuration id={}", inner.id);
                UpdateConfigurationResponse {
                    success: true,
                    error: None,
                }
            }
            Err(e) => {
                error!("❌ Failed to update configuration: {}", e);
                UpdateConfigurationResponse {
                    success: false,
                    error: Some(e.to_string()),
                }
            }
        };

        // respond_and_invalidate automatically broadcasts invalidations on success
        if let Err(e) = request.clone().respond_and_invalidate(response, &net) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle DeleteConfiguration request - deletes a configuration.
fn handle_delete_configuration(
    mut requests: MessageReader<Request<DeleteConfiguration>>,
    db: Option<Res<DatabaseResource>>,
    net: Res<Network<WebSocketProvider>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling DeleteConfiguration for id={}", inner.id);

        let result = db.as_ref()
            .map(|db| {
                let conn = db.connection();
                let conn = conn.lock().unwrap();
                database::delete_configuration(&conn, inner.id)
            })
            .unwrap_or(Err(anyhow::anyhow!("Database not available")));

        let response = match result {
            Ok(()) => {
                info!("✅ Deleted configuration id={}", inner.id);
                DeleteConfigurationResponse {
                    success: true,
                    error: None,
                }
            }
            Err(e) => {
                error!("❌ Failed to delete configuration: {}", e);
                DeleteConfigurationResponse {
                    success: false,
                    error: Some(e.to_string()),
                }
            }
        };

        // respond_and_invalidate automatically broadcasts invalidations on success
        if let Err(e) = request.clone().respond_and_invalidate(response, &net) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle SetDefaultConfiguration request - sets a configuration as the default.
fn handle_set_default_configuration(
    mut requests: MessageReader<Request<SetDefaultConfiguration>>,
    db: Option<Res<DatabaseResource>>,
    net: Res<Network<WebSocketProvider>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling SetDefaultConfiguration for id={}", inner.id);

        let result = db.as_ref()
            .map(|db| {
                let conn = db.connection();
                let conn = conn.lock().unwrap();
                database::set_default_configuration(&conn, inner.id)
            })
            .unwrap_or(Err(anyhow::anyhow!("Database not available")));

        let response = match result {
            Ok(()) => {
                info!("✅ Set default configuration id={}", inner.id);
                SetDefaultConfigurationResponse {
                    success: true,
                    error: None,
                }
            }
            Err(e) => {
                error!("❌ Failed to set default configuration: {}", e);
                SetDefaultConfigurationResponse {
                    success: false,
                    error: Some(e.to_string()),
                }
            }
        };

        // respond_and_invalidate automatically broadcasts invalidations on success
        if let Err(e) = request.clone().respond_and_invalidate(response, &net) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle LoadConfiguration request - loads a configuration and applies it to the robot.
///
/// This handler:
/// 1. Loads the configuration from the database
/// 2. Sends FrcSetUFrameUTool command to the robot to apply frame/tool settings
/// 3. Updates the ActiveConfigState to track the loaded configuration
fn handle_load_configuration(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut requests: MessageReader<Request<LoadConfiguration>>,
    db: Option<Res<DatabaseResource>>,
    mut robot_query: Query<(
        &mut ActiveConfigState,
        &mut FrameToolDataState,
        &RobotConnectionState,
        Option<&RmiDriver>,
    ), With<FanucRobot>>,
) {
    use fanuc_rmi::dto as raw_dto;

    // Enter the Tokio runtime context so send_packet can use tokio::spawn
    let _guard = tokio_runtime.runtime().enter();

    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling LoadConfiguration for id={}", inner.configuration_id);

        // Get the configuration from database
        let result = db.as_ref()
            .map(|db| {
                let conn = db.connection();
                let conn = conn.lock().unwrap();
                database::get_configuration(&conn, inner.configuration_id)
            })
            .unwrap_or(Err(anyhow::anyhow!("Database not available")));

        let response = match result {
            Ok(config) => {
                info!("✅ Loaded configuration id={}", inner.configuration_id);

                // Find connected robot with driver
                let robot_result = robot_query.iter_mut()
                    .find(|(_, _, state, driver)| **state == RobotConnectionState::Connected && driver.is_some());

                if let Some((mut active_config, mut ft_state, _, driver)) = robot_result {
                    let driver = driver.expect("Checked above");

                    // Send FrcSetUFrameUTool command to robot
                    let command = raw_dto::Command::FrcSetUFrameUTool(raw_dto::FrcSetUFrameUTool {
                        group: 1,
                        u_frame_number: config.u_frame_number as u8,
                        u_tool_number: config.u_tool_number as u8,
                    });
                    let send_packet: fanuc_rmi::packets::SendPacket = raw_dto::SendPacket::Command(command).into();

                    match driver.0.send_packet(send_packet, PacketPriority::Immediate) {
                        Ok(seq) => {
                            info!("Sent FrcSetUFrameUTool command (frame={}, tool={}) with sequence {}",
                                config.u_frame_number, config.u_tool_number, seq);

                            // Update FrameToolDataState (will be confirmed by next poll)
                            ft_state.active_frame = config.u_frame_number;
                            ft_state.active_tool = config.u_tool_number;
                        }
                        Err(e) => {
                            error!("Failed to send FrcSetUFrameUTool command: {:?}", e);
                            // Continue anyway - we still update the config state
                        }
                    }

                    // Update the ActiveConfigState on the robot entity
                    active_config.loaded_from_id = Some(config.id);
                    active_config.u_frame_number = config.u_frame_number;
                    active_config.u_tool_number = config.u_tool_number;
                    active_config.front = config.front;
                    active_config.up = config.up;
                    active_config.left = config.left;
                    active_config.flip = config.flip;
                    active_config.turn4 = config.turn4;
                    active_config.turn5 = config.turn5;
                    active_config.turn6 = config.turn6;
                    active_config.changes_count = 0; // Reset changes count
                    active_config.change_log.clear(); // Clear change log

                    LoadConfigurationResponse {
                        success: true,
                        error: None,
                    }
                } else {
                    // No connected robot - still update config state but warn
                    warn!("LoadConfiguration: No connected robot - configuration loaded but not applied");

                    for (mut active_config, _, _, _) in robot_query.iter_mut() {
                        active_config.loaded_from_id = Some(config.id);
                        active_config.u_frame_number = config.u_frame_number;
                        active_config.u_tool_number = config.u_tool_number;
                        active_config.front = config.front;
                        active_config.up = config.up;
                        active_config.left = config.left;
                        active_config.flip = config.flip;
                        active_config.turn4 = config.turn4;
                        active_config.turn5 = config.turn5;
                        active_config.turn6 = config.turn6;
                        active_config.changes_count = 0;
                        active_config.change_log.clear();
                    }

                    LoadConfigurationResponse {
                        success: true,
                        error: None,
                    }
                }
            }
            Err(e) => {
                error!("❌ Failed to load configuration: {}", e);
                LoadConfigurationResponse {
                    success: false,
                    error: Some(e.to_string()),
                }
            }
        };

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle SaveCurrentConfiguration request - saves the active config to database.
/// If `name` is provided, creates a new configuration.
/// If `name` is None, updates the currently loaded configuration.
/// Resets `changes_count` and `change_log` to empty after successful save.
fn handle_save_current_configuration(
    mut requests: MessageReader<Request<SaveCurrentConfiguration>>,
    db: Option<Res<DatabaseResource>>,
    net: Res<Network<WebSocketProvider>>,
    mut robot_query: Query<(&mut ActiveConfigState, &ConnectionState), With<FanucRobot>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling SaveCurrentConfiguration, name={:?}", inner.name);

        // Get the active config and connection state from the robot entity
        let query_result: Option<(ActiveConfigState, i64)> = robot_query.iter().next().and_then(|(config, conn_state)| {
            conn_state.active_connection_id.map(|id| (config.clone(), id))
        });

        let (active_config, robot_connection_id) = match query_result {
            Some((config, id)) => (config, id),
            None => {
                let response = SaveCurrentConfigurationResponse {
                    success: false,
                    configuration_id: None,
                    configuration_name: None,
                    error: Some("No robot connection active".to_string()),
                };
                if let Err(e) = request.clone().respond(response) {
                    error!("Failed to send response: {:?}", e);
                }
                continue;
            }
        };

        // Save to database
        let result = db.as_ref()
            .map(|db| {
                let conn = db.connection();
                let conn = conn.lock().unwrap();
                database::save_current_configuration(
                    &conn,
                    robot_connection_id,
                    active_config.loaded_from_id,
                    inner.name.clone(),
                    &active_config,
                )
            })
            .unwrap_or(Err(anyhow::anyhow!("Database not available")));

        let response = match result {
            Ok((config_id, config_name)) => {
                info!("✅ Saved configuration id={}, name='{}'", config_id, config_name);

                // Update the ActiveConfigState on the robot entity - reset changes tracking
                for (mut active, _) in robot_query.iter_mut() {
                    active.loaded_from_id = Some(config_id);
                    active.loaded_from_name = Some(config_name.clone());
                    active.changes_count = 0;
                    active.change_log.clear();
                }

                SaveCurrentConfigurationResponse {
                    success: true,
                    configuration_id: Some(config_id),
                    configuration_name: Some(config_name),
                    error: None,
                }
            }
            Err(e) => {
                error!("❌ Failed to save configuration: {}", e);
                SaveCurrentConfigurationResponse {
                    success: false,
                    configuration_id: None,
                    configuration_name: None,
                    error: Some(e.to_string()),
                }
            }
        };

        // respond_and_invalidate automatically broadcasts invalidations on success
        if let Err(e) = request.clone().respond_and_invalidate(response, &net) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

// ============================================================================
// Frame/Tool Handlers
// ============================================================================
// Note: These handlers return mock data for now. The FanucDriver doesn't have
// direct frame/tool read/write methods yet - they would need to be implemented
// using the packet system (FrcReadFrameData, FrcWriteFrameData, etc.)

/// Handle GetActiveFrameTool request - returns current active frame and tool.
/// This is a targeted query (no authorization required).
pub fn handle_get_active_frame_tool(
    mut requests: MessageReader<Request<TargetedRequest<GetActiveFrameTool>>>,
    robots: Query<&FrameToolDataState, With<FanucRobot>>,
) {
    for request in requests.read() {
        let targeted = request.get_request();
        info!("📋 Handling GetActiveFrameTool request for target {}", targeted.target_id);

        // Parse target entity from bits string
        let target = match targeted.target_id.parse::<u64>() {
            Ok(bits) => Entity::from_bits(bits),
            Err(_) => {
                error!("Invalid target entity: {}", targeted.target_id);
                continue;
            }
        };

        // Get from target entity
        let (uframe, utool) = if let Ok(ft_state) = robots.get(target) {
            (ft_state.active_frame, ft_state.active_tool)
        } else {
            (1, 1) // Default if entity not found
        };

        let response = GetActiveFrameToolResponse { uframe, utool };

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle SetActiveFrameTool request - sets active frame and tool on robot.
/// This is a targeted request that requires entity control.
///
/// This handler:
/// 1. Sends FrcSetUFrameUTool command to the robot
/// 2. Updates ActiveConfigState to track changes for the save functionality
/// 3. Adds entries to the change log for display in the save modal
///
/// Note: FrameToolDataState is updated by polling - we don't update it directly here
/// so that the UI reflects the actual robot state after the command is executed.
pub fn handle_set_active_frame_tool(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut requests: MessageReader<AuthorizedRequest<SetActiveFrameTool>>,
    mut robots: Query<(
        &mut FrameToolDataState,
        &mut ActiveConfigState,
        &RobotConnectionState,
        Option<&RmiDriver>,
    ), With<FanucRobot>>,
) {
    use fanuc_rmi::dto as raw_dto;

    // Enter the Tokio runtime context so send_packet can use tokio::spawn
    let _guard = tokio_runtime.runtime().enter();

    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling SetActiveFrameTool REQUEST: uframe={}, utool={}", inner.uframe, inner.utool);

        // Find connected robot with driver
        let Some((ft_state, mut active_config, _, driver)) = robots.iter_mut()
            .find(|(_, _, state, driver)| **state == RobotConnectionState::Connected && driver.is_some())
        else {
            warn!("SetActiveFrameTool rejected: No connected robot");
            let _ = request.clone().respond(SetActiveFrameToolResponse {
                success: false,
                error: Some("No connected robot".to_string()),
            });
            continue;
        };

        let driver = driver.expect("Checked above");
        let old_frame = ft_state.active_frame;
        let old_tool = ft_state.active_tool;

        // Send FrcSetUFrameUTool command to robot
        // IMPORTANT: Verify the values being sent match the request
        let uframe_to_send = inner.uframe as u8;
        let utool_to_send = inner.utool as u8;
        
        info!("📤 Before FrcSetUFrameUTool construction: uframe_to_send={}, utool_to_send={}", uframe_to_send, utool_to_send);
        
        let command = raw_dto::Command::FrcSetUFrameUTool(raw_dto::FrcSetUFrameUTool {
            group: 1,
            u_frame_number: uframe_to_send,
            u_tool_number: utool_to_send,
        });
        
        // Log the command after construction
        if let raw_dto::Command::FrcSetUFrameUTool(ref frc_cmd) = command {
            info!("📤 After FrcSetUFrameUTool construction: u_frame_number={}, u_tool_number={}", 
                frc_cmd.u_frame_number, frc_cmd.u_tool_number);
        }
        
        let send_packet: fanuc_rmi::packets::SendPacket = raw_dto::SendPacket::Command(command).into();

        info!("📤 SetActiveFrameTool: Sending FrcSetUFrameUTool with UFrame={}, UTool={}", inner.uframe, inner.utool);

        match driver.0.send_packet(send_packet, PacketPriority::Immediate) {
            Ok(seq) => {
                info!("✅ Sent FrcSetUFrameUTool command with sequence {}", seq);

                // Update ActiveConfigState to track the intended state
                // DO NOT update FrameToolDataState here - wait for polling to confirm from robot
                // This prevents showing stale/intermediate states on the UI
                if old_frame != inner.uframe {
                    active_config.u_frame_number = inner.uframe;
                    active_config.changes_count += 1;
                    active_config.change_log.push(ConfigChangeEntry {
                        field_name: "UFrame".to_string(),
                        old_value: format!("{}", old_frame),
                        new_value: format!("{}", inner.uframe),
                    });
                    info!("📊 UFrame change requested: {} -> {} (will be confirmed by poll)", old_frame, inner.uframe);
                }
                if old_tool != inner.utool {
                    active_config.u_tool_number = inner.utool;
                    active_config.changes_count += 1;
                    active_config.change_log.push(ConfigChangeEntry {
                        field_name: "UTool".to_string(),
                        old_value: format!("{}", old_tool),
                        new_value: format!("{}", inner.utool),
                    });
                    info!("📊 UTool change requested: {} -> {} (will be confirmed by poll)", old_tool, inner.utool);
                }

                let _ = request.clone().respond(SetActiveFrameToolResponse { success: true, error: None });
            }
            Err(e) => {
                error!("Failed to send FrcSetUFrameUTool command: {:?}", e);
                let _ = request.clone().respond(SetActiveFrameToolResponse {
                    success: false,
                    error: Some(format!("Failed to send command: {:?}", e)),
                });
            }
        }
    }
}

/// Handle GetFrameData request - reads frame data from robot and updates synced state.
/// This is a targeted query (no authorization required).
///
/// GAP-002: UFrame 0 represents world coordinates and cannot be queried.
/// GAP-007: Now reads actual frame data from robot via FrcReadUFrameData.
pub fn handle_get_frame_data(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut requests: MessageReader<Request<TargetedRequest<GetFrameData>>>,
    robots: Query<(&FrameToolDataState, Option<&RmiDriver>, &RobotConnectionState), With<FanucRobot>>,
) {
    use fanuc_rmi::packets::{SendPacket, Command, ResponsePacket, CommandResponse};
    use fanuc_rmi::commands::FrcReadUFrameData;
    use std::time::Duration;

    // Enter the Tokio runtime context
    let _guard = tokio_runtime.runtime().enter();

    for request in requests.read() {
        let targeted = request.get_request();
        let frame_number = targeted.request.frame_number;
        info!("📋 Handling GetFrameData for frame {} on target {}", frame_number, targeted.target_id);

        // GAP-002: UFrame 0 cannot be queried - it represents world coordinates
        if frame_number == 0 {
            warn!("Cannot read UFrame 0 - world coordinates have no frame data");
            let response = FrameDataResponse {
                frame_number: 0,
                x: 0.0, y: 0.0, z: 0.0,
                w: 0.0, p: 0.0, r: 0.0,
            };
            if let Err(e) = request.clone().respond(response) {
                error!("Failed to send response: {:?}", e);
            }
            continue;
        }

        // Parse target entity from bits string (used for entity matching in future)
        let _target = match targeted.target_id.parse::<u64>() {
            Ok(bits) => Entity::from_bits(bits),
            Err(_) => {
                error!("Invalid target entity: {}", targeted.target_id);
                continue;
            }
        };

        // Find connected robot with driver
        let robot_info = robots.iter()
            .find(|(_, driver, state)| **state == RobotConnectionState::Connected && driver.is_some());

        if let Some((_, Some(driver), _)) = robot_info {
            let driver = driver.0.clone();
            let request = request.clone();

            // GAP-007: Read actual frame data from robot via async call
            tokio_runtime.spawn_background_task(move |mut ctx| async move {
                let packet = SendPacket::Command(Command::FrcReadUFrameData(FrcReadUFrameData {
                    frame_number: frame_number as i8,
                    group: 1,
                }));

                // Subscribe before sending to avoid race condition
                let mut response_rx = driver.response_tx.subscribe();

                if let Err(e) = driver.send_packet(packet, PacketPriority::Standard) {
                    bevy::log::error!("Failed to send FrcReadUFrameData: {}", e);
                    let response = FrameDataResponse {
                        frame_number,
                        x: 0.0, y: 0.0, z: 0.0,
                        w: 0.0, p: 0.0, r: 0.0,
                    };
                    let _ = request.respond(response);
                    return;
                }

                // Wait for response with timeout
                let result = tokio::time::timeout(Duration::from_secs(5), async {
                    while let Ok(response) = response_rx.recv().await {
                        if let ResponsePacket::CommandResponse(CommandResponse::FrcReadUFrameData(resp)) = response {
                            return Some(resp);
                        }
                    }
                    None
                }).await;

                match result {
                    Ok(Some(resp)) => {
                        if resp.error_id != 0 {
                            bevy::log::error!("Robot error reading UFrame {}: error_id={}", frame_number, resp.error_id);
                            let response = FrameDataResponse {
                                frame_number,
                                x: 0.0, y: 0.0, z: 0.0,
                                w: 0.0, p: 0.0, r: 0.0,
                            };
                            let _ = request.respond(response);
                        } else {
                            bevy::log::info!("✅ Read UFrame {} data from robot", frame_number);

                            // Update FrameToolDataState on main thread
                            let frame_data = FrameToolData {
                                x: resp.frame.x,
                                y: resp.frame.y,
                                z: resp.frame.z,
                                w: resp.frame.w,
                                p: resp.frame.p,
                                r: resp.frame.r,
                            };
                            let frame_data_clone = frame_data.clone();
                            let frame_num = frame_number;
                            ctx.run_on_main_thread(move |ctx| {
                                let mut query = ctx.world.query_filtered::<&mut FrameToolDataState, With<FanucRobot>>();
                                for mut ft_state in query.iter_mut(ctx.world) {
                                    ft_state.frames.insert(frame_num, frame_data_clone.clone());
                                }
                            }).await;

                            let response = FrameDataResponse {
                                frame_number,
                                x: frame_data.x,
                                y: frame_data.y,
                                z: frame_data.z,
                                w: frame_data.w,
                                p: frame_data.p,
                                r: frame_data.r,
                            };
                            let _ = request.respond(response);
                        }
                    }
                    Ok(None) => {
                        bevy::log::error!("No response received for UFrame {}", frame_number);
                        let response = FrameDataResponse {
                            frame_number,
                            x: 0.0, y: 0.0, z: 0.0,
                            w: 0.0, p: 0.0, r: 0.0,
                        };
                        let _ = request.respond(response);
                    }
                    Err(_) => {
                        bevy::log::error!("Timeout waiting for UFrame {} response", frame_number);
                        let response = FrameDataResponse {
                            frame_number,
                            x: 0.0, y: 0.0, z: 0.0,
                            w: 0.0, p: 0.0, r: 0.0,
                        };
                        let _ = request.respond(response);
                    }
                }
            });
        } else {
            // No connected robot - return zeros
            warn!("GetFrameData: No connected robot");
            let response = FrameDataResponse {
                frame_number,
                x: 0.0, y: 0.0, z: 0.0,
                w: 0.0, p: 0.0, r: 0.0,
            };
            if let Err(e) = request.clone().respond(response) {
                error!("Failed to send response: {:?}", e);
            }
        }
    }
}

/// Handle WriteFrameData request - writes frame data to robot and updates synced state.
/// This is a targeted request that requires entity control.
///
/// GAP-008: Now writes actual frame data to robot via FrcWriteUFrameData.
pub fn handle_write_frame_data(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut requests: MessageReader<AuthorizedRequest<WriteFrameData>>,
    robots: Query<(Option<&RmiDriver>, &RobotConnectionState), With<FanucRobot>>,
) {
    use fanuc_rmi::packets::{SendPacket, Command, ResponsePacket, CommandResponse};
    use fanuc_rmi::commands::FrcWriteUFrameData;
    use fanuc_rmi::FrameData;
    use std::time::Duration;

    // Enter the Tokio runtime context
    let _guard = tokio_runtime.runtime().enter();

    for request in requests.read() {
        let inner = request.get_request();
        let target = request.target_entity;
        let frame_number = inner.frame_number;
        info!("📋 Handling WriteFrameData for frame {} on entity {:?}", frame_number, target);

        // UFrame 0 cannot be written - it represents world coordinates
        if frame_number == 0 {
            warn!("Cannot write UFrame 0 - world coordinates cannot be modified");
            let response = WriteFrameDataResponse {
                success: false,
                error: Some("UFrame 0 (world coordinates) cannot be modified".to_string()),
            };
            if let Err(e) = request.clone().respond(response) {
                error!("Failed to send response: {:?}", e);
            }
            continue;
        }

        // Find connected robot with driver
        let robot_info = robots.iter()
            .find(|(driver, state)| **state == RobotConnectionState::Connected && driver.is_some());

        if let Some((Some(driver), _)) = robot_info {
            let driver = driver.0.clone();
            let request = request.clone();
            let x = inner.x;
            let y = inner.y;
            let z = inner.z;
            let w = inner.w;
            let p = inner.p;
            let r = inner.r;

            // Write frame data to robot via async call
            tokio_runtime.spawn_background_task(move |mut ctx| async move {
                let packet = SendPacket::Command(Command::FrcWriteUFrameData(FrcWriteUFrameData {
                    frame_number: frame_number as i8,
                    frame: FrameData { x, y, z, w, p, r },
                    group: 1,
                }));

                // Subscribe before sending to avoid race condition
                let mut response_rx = driver.response_tx.subscribe();

                if let Err(e) = driver.send_packet(packet, PacketPriority::Standard) {
                    bevy::log::error!("Failed to send FrcWriteUFrameData: {}", e);
                    let response = WriteFrameDataResponse {
                        success: false,
                        error: Some(format!("Failed to send command: {}", e)),
                    };
                    let _ = request.respond(response);
                    return;
                }

                // Wait for response with timeout
                let result = tokio::time::timeout(Duration::from_secs(5), async {
                    while let Ok(response) = response_rx.recv().await {
                        if let ResponsePacket::CommandResponse(CommandResponse::FrcWriteUFrameData(resp)) = response {
                            return Some(resp);
                        }
                    }
                    None
                }).await;

                match result {
                    Ok(Some(resp)) => {
                        if resp.error_id != 0 {
                            bevy::log::error!("Robot error writing UFrame {}: error_id={}", frame_number, resp.error_id);
                            let response = WriteFrameDataResponse {
                                success: false,
                                error: Some(format!("Robot error: {}", resp.error_id)),
                            };
                            let _ = request.respond(response);
                        } else {
                            bevy::log::info!("✅ Wrote UFrame {} data to robot", frame_number);

                            // Update FrameToolDataState on main thread
                            let frame_data = FrameToolData { x, y, z, w, p, r };
                            let frame_num = frame_number;
                            ctx.run_on_main_thread(move |ctx| {
                                let mut query = ctx.world.query_filtered::<&mut FrameToolDataState, With<FanucRobot>>();
                                for mut ft_state in query.iter_mut(ctx.world) {
                                    ft_state.frames.insert(frame_num, frame_data.clone());
                                }
                            }).await;

                            let response = WriteFrameDataResponse { success: true, error: None };
                            let _ = request.respond(response);
                        }
                    }
                    Ok(None) => {
                        bevy::log::error!("No response received for WriteUFrame {}", frame_number);
                        let response = WriteFrameDataResponse {
                            success: false,
                            error: Some("No response received".to_string()),
                        };
                        let _ = request.respond(response);
                    }
                    Err(_) => {
                        bevy::log::error!("Timeout waiting for WriteUFrame {} response", frame_number);
                        let response = WriteFrameDataResponse {
                            success: false,
                            error: Some("Timeout waiting for response".to_string()),
                        };
                        let _ = request.respond(response);
                    }
                }
            });
        } else {
            // No connected robot
            warn!("WriteFrameData: No connected robot");
            let response = WriteFrameDataResponse {
                success: false,
                error: Some("No connected robot".to_string()),
            };
            if let Err(e) = request.clone().respond(response) {
                error!("Failed to send response: {:?}", e);
            }
        }
    }
}

/// Handle GetToolData request - reads tool data from robot and updates synced state.
/// This is a targeted query (no authorization required).
///
/// GAP-009: Now reads actual tool data from robot via FrcReadUToolData.
pub fn handle_get_tool_data(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut requests: MessageReader<Request<TargetedRequest<GetToolData>>>,
    robots: Query<(&FrameToolDataState, Option<&RmiDriver>, &RobotConnectionState), With<FanucRobot>>,
) {
    use fanuc_rmi::packets::{SendPacket, Command, ResponsePacket, CommandResponse};
    use fanuc_rmi::commands::FrcReadUToolData;
    use std::time::Duration;

    // Enter the Tokio runtime context
    let _guard = tokio_runtime.runtime().enter();

    for request in requests.read() {
        let targeted = request.get_request();
        let tool_number = targeted.request.tool_number;
        info!("📋 Handling GetToolData for tool {} on target {}", tool_number, targeted.target_id);

        // Tool 0 is typically not valid on FANUC (tools are 1-10)
        if tool_number <= 0 {
            warn!("Tool number {} is invalid (must be 1-10)", tool_number);
            let response = ToolDataResponse {
                tool_number,
                x: 0.0, y: 0.0, z: 0.0,
                w: 0.0, p: 0.0, r: 0.0,
            };
            if let Err(e) = request.clone().respond(response) {
                error!("Failed to send response: {:?}", e);
            }
            continue;
        }

        // Find connected robot with driver
        let robot_info = robots.iter()
            .find(|(_, driver, state)| **state == RobotConnectionState::Connected && driver.is_some());

        if let Some((_, Some(driver), _)) = robot_info {
            let driver = driver.0.clone();
            let request = request.clone();

            // Read tool data from robot via async call
            tokio_runtime.spawn_background_task(move |mut ctx| async move {
                let packet = SendPacket::Command(Command::FrcReadUToolData(FrcReadUToolData {
                    tool_number: tool_number as i8,
                    group: 1,
                }));

                // Subscribe before sending to avoid race condition
                let mut response_rx = driver.response_tx.subscribe();

                if let Err(e) = driver.send_packet(packet, PacketPriority::Standard) {
                    bevy::log::error!("Failed to send FrcReadUToolData: {}", e);
                    let response = ToolDataResponse {
                        tool_number,
                        x: 0.0, y: 0.0, z: 0.0,
                        w: 0.0, p: 0.0, r: 0.0,
                    };
                    let _ = request.respond(response);
                    return;
                }

                // Wait for response with timeout
                let result = tokio::time::timeout(Duration::from_secs(5), async {
                    while let Ok(response) = response_rx.recv().await {
                        if let ResponsePacket::CommandResponse(CommandResponse::FrcReadUToolData(resp)) = response {
                            return Some(resp);
                        }
                    }
                    None
                }).await;

                match result {
                    Ok(Some(resp)) => {
                        if resp.error_id != 0 {
                            bevy::log::error!("Robot error reading UTool {}: error_id={}", tool_number, resp.error_id);
                            let response = ToolDataResponse {
                                tool_number,
                                x: 0.0, y: 0.0, z: 0.0,
                                w: 0.0, p: 0.0, r: 0.0,
                            };
                            let _ = request.respond(response);
                        } else {
                            bevy::log::info!("✅ Read UTool {} data from robot", tool_number);

                            // Update FrameToolDataState on main thread
                            let tool_data = FrameToolData {
                                x: resp.frame.x,
                                y: resp.frame.y,
                                z: resp.frame.z,
                                w: resp.frame.w,
                                p: resp.frame.p,
                                r: resp.frame.r,
                            };
                            let tool_data_clone = tool_data.clone();
                            let tool_num = tool_number;
                            ctx.run_on_main_thread(move |ctx| {
                                let mut query = ctx.world.query_filtered::<&mut FrameToolDataState, With<FanucRobot>>();
                                for mut ft_state in query.iter_mut(ctx.world) {
                                    ft_state.tools.insert(tool_num, tool_data_clone.clone());
                                }
                            }).await;

                            let response = ToolDataResponse {
                                tool_number,
                                x: tool_data.x,
                                y: tool_data.y,
                                z: tool_data.z,
                                w: tool_data.w,
                                p: tool_data.p,
                                r: tool_data.r,
                            };
                            let _ = request.respond(response);
                        }
                    }
                    Ok(None) => {
                        bevy::log::error!("No response received for UTool {}", tool_number);
                        let response = ToolDataResponse {
                            tool_number,
                            x: 0.0, y: 0.0, z: 0.0,
                            w: 0.0, p: 0.0, r: 0.0,
                        };
                        let _ = request.respond(response);
                    }
                    Err(_) => {
                        bevy::log::error!("Timeout waiting for UTool {} response", tool_number);
                        let response = ToolDataResponse {
                            tool_number,
                            x: 0.0, y: 0.0, z: 0.0,
                            w: 0.0, p: 0.0, r: 0.0,
                        };
                        let _ = request.respond(response);
                    }
                }
            });
        } else {
            // No connected robot - return zeros
            warn!("GetToolData: No connected robot");
            let response = ToolDataResponse {
                tool_number,
                x: 0.0, y: 0.0, z: 0.0,
                w: 0.0, p: 0.0, r: 0.0,
            };
            if let Err(e) = request.clone().respond(response) {
                error!("Failed to send response: {:?}", e);
            }
        }
    }
}

/// Handle WriteToolData request - writes tool data to robot and updates synced state.
/// This is a targeted request that requires entity control.
///
/// GAP-009: Now writes actual tool data to robot via FrcWriteUToolData.
pub fn handle_write_tool_data(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut requests: MessageReader<AuthorizedRequest<WriteToolData>>,
    robots: Query<(Option<&RmiDriver>, &RobotConnectionState), With<FanucRobot>>,
) {
    use fanuc_rmi::packets::{SendPacket, Command, ResponsePacket, CommandResponse};
    use fanuc_rmi::commands::FrcWriteUToolData;
    use fanuc_rmi::FrameData;
    use std::time::Duration;

    // Enter the Tokio runtime context
    let _guard = tokio_runtime.runtime().enter();

    for request in requests.read() {
        let inner = request.get_request();
        let target = request.target_entity;
        let tool_number = inner.tool_number;
        info!("📋 Handling WriteToolData for tool {} on entity {:?}", tool_number, target);

        // Tool 0 is typically not valid on FANUC (tools are 1-10)
        if tool_number <= 0 {
            warn!("Tool number {} is invalid (must be 1-10)", tool_number);
            let response = WriteToolDataResponse {
                success: false,
                error: Some("Tool number must be 1-10".to_string()),
            };
            if let Err(e) = request.clone().respond(response) {
                error!("Failed to send response: {:?}", e);
            }
            continue;
        }

        // Find connected robot with driver
        let robot_info = robots.iter()
            .find(|(driver, state)| **state == RobotConnectionState::Connected && driver.is_some());

        if let Some((Some(driver), _)) = robot_info {
            let driver = driver.0.clone();
            let request = request.clone();
            let x = inner.x;
            let y = inner.y;
            let z = inner.z;
            let w = inner.w;
            let p = inner.p;
            let r = inner.r;

            // Write tool data to robot via async call
            tokio_runtime.spawn_background_task(move |mut ctx| async move {
                let packet = SendPacket::Command(Command::FrcWriteUToolData(FrcWriteUToolData {
                    tool_number: tool_number as i8,
                    frame: FrameData { x, y, z, w, p, r },
                    group: 1,
                }));

                // Subscribe before sending to avoid race condition
                let mut response_rx = driver.response_tx.subscribe();

                if let Err(e) = driver.send_packet(packet, PacketPriority::Standard) {
                    bevy::log::error!("Failed to send FrcWriteUToolData: {}", e);
                    let response = WriteToolDataResponse {
                        success: false,
                        error: Some(format!("Failed to send command: {}", e)),
                    };
                    let _ = request.respond(response);
                    return;
                }

                // Wait for response with timeout
                let result = tokio::time::timeout(Duration::from_secs(5), async {
                    while let Ok(response) = response_rx.recv().await {
                        if let ResponsePacket::CommandResponse(CommandResponse::FrcWriteUToolData(resp)) = response {
                            return Some(resp);
                        }
                    }
                    None
                }).await;

                match result {
                    Ok(Some(resp)) => {
                        if resp.error_id != 0 {
                            bevy::log::error!("Robot error writing UTool {}: error_id={}", tool_number, resp.error_id);
                            let response = WriteToolDataResponse {
                                success: false,
                                error: Some(format!("Robot error: {}", resp.error_id)),
                            };
                            let _ = request.respond(response);
                        } else {
                            bevy::log::info!("✅ Wrote UTool {} data to robot", tool_number);

                            // Update FrameToolDataState on main thread
                            let tool_data = FrameToolData { x, y, z, w, p, r };
                            let tool_num = tool_number;
                            ctx.run_on_main_thread(move |ctx| {
                                let mut query = ctx.world.query_filtered::<&mut FrameToolDataState, With<FanucRobot>>();
                                for mut ft_state in query.iter_mut(ctx.world) {
                                    ft_state.tools.insert(tool_num, tool_data.clone());
                                }
                            }).await;

                            let response = WriteToolDataResponse { success: true, error: None };
                            let _ = request.respond(response);
                        }
                    }
                    Ok(None) => {
                        bevy::log::error!("No response received for WriteUTool {}", tool_number);
                        let response = WriteToolDataResponse {
                            success: false,
                            error: Some("No response received".to_string()),
                        };
                        let _ = request.respond(response);
                    }
                    Err(_) => {
                        bevy::log::error!("Timeout waiting for WriteUTool {} response", tool_number);
                        let response = WriteToolDataResponse {
                            success: false,
                            error: Some("Timeout waiting for response".to_string()),
                        };
                        let _ = request.respond(response);
                    }
                }
            });
        } else {
            // No connected robot
            warn!("WriteToolData: No connected robot");
            let response = WriteToolDataResponse {
                success: false,
                error: Some("No connected robot".to_string()),
            };
            if let Err(e) = request.clone().respond(response) {
                error!("Failed to send response: {:?}", e);
            }
        }
    }
}

// ============================================================================
// I/O Handlers
// ============================================================================

/// Handle ReadDin request - reads digital input value.
/// This is a targeted query (no authorization required).
pub fn handle_read_din(
    mut requests: MessageReader<Request<TargetedRequest<ReadDin>>>,
) {
    for request in requests.read() {
        let targeted = request.get_request();
        let port_number = targeted.request.port_number;
        info!("📋 Handling ReadDin for port {} on target {}", port_number, targeted.target_id);

        // TODO: Read from connected robot driver when available
        // For now, return mock value (false)
        let response = DinValueResponse {
            port_number,
            port_value: false,
        };

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle ReadDinBatch request - reads multiple digital input values.
/// This is a targeted query (no authorization required).
pub fn handle_read_din_batch(
    mut requests: MessageReader<Request<TargetedRequest<ReadDinBatch>>>,
) {
    for request in requests.read() {
        let targeted = request.get_request();
        let port_numbers = &targeted.request.port_numbers;
        info!("📋 Handling ReadDinBatch for {} ports on target {}", port_numbers.len(), targeted.target_id);

        // TODO: Read from connected robot driver when available
        // For now, return mock values (all false)
        let values: Vec<(u16, bool)> = port_numbers.iter()
            .map(|&port| (port, false))
            .collect();

        let response = DinBatchResponse { values };

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle WriteDout request - writes digital output value.
/// Waits for robot confirmation before updating IoStatus and responding.
/// This is a targeted request that requires entity control.
pub fn handle_write_dout(
    mut requests: MessageReader<AuthorizedRequest<WriteDout>>,
    driver_query: Query<&RmiDriver, With<FanucRobot>>,
    runtime: Option<Res<bevy_tokio_tasks::TokioTasksRuntime>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        let target = request.target_entity;
        let port = inner.port_number;
        let value = inner.port_value;
        info!("📋 Handling WriteDout for port {} = {} on entity {:?}", port, value, target);

        // Get driver from target entity or respond with error
        let driver = match driver_query.get(target) {
            Ok(d) => d.0.clone(),
            Err(_) => {
                let response = DoutValueResponse {
                    port_number: port,
                    port_value: value,
                    success: false,
                    error: Some("No robot connected".to_string()),
                };
                let _ = request.clone().respond(response);
                continue;
            }
        };

        let Some(runtime) = runtime.as_ref() else {
            let response = DoutValueResponse {
                port_number: port,
                port_value: value,
                success: false,
                error: Some("Runtime not available".to_string()),
            };
            let _ = request.clone().respond(response);
            continue;
        };

        // Clone request so we can respond from async task
        let request = request.clone();

        // Send command and wait for confirmation
        runtime.spawn_background_task(move |mut ctx| async move {
            use fanuc_rmi::packets::{SendPacket, Command, ResponsePacket, CommandResponse};
            use fanuc_rmi::commands::FrcWriteDOUT;
            use std::time::Duration;

            let packet = SendPacket::Command(Command::FrcWriteDOUT(FrcWriteDOUT {
                port_number: port,
                port_value: if value { 1 } else { 0 },
            }));

            // Subscribe before sending to avoid race condition
            let mut response_rx = driver.response_tx.subscribe();

            if let Err(e) = driver.send_packet(packet, PacketPriority::Standard) {
                bevy::log::error!("Failed to send WriteDout to robot: {}", e);
                let response = DoutValueResponse {
                    port_number: port,
                    port_value: value,
                    success: false,
                    error: Some(format!("Failed to send command: {}", e)),
                };
                let _ = request.respond(response);
                return;
            }

            // Wait for response with timeout
            let result = tokio::time::timeout(Duration::from_secs(5), async {
                while let Ok(response) = response_rx.recv().await {
                    if let ResponsePacket::CommandResponse(CommandResponse::FrcWriteDOUT(resp)) = response {
                        return Some(resp);
                    }
                }
                None
            }).await;

            match result {
                Ok(Some(resp)) => {
                    if resp.error_id != 0 {
                        bevy::log::error!("Robot error writing DOUT[{}]: error_id={}", port, resp.error_id);
                        let response = DoutValueResponse {
                            port_number: port,
                            port_value: value,
                            success: false,
                            error: Some(format!("Robot error: {}", resp.error_id)),
                        };
                        let _ = request.respond(response);
                    } else {
                        bevy::log::info!("✅ DOUT[{}] set to {} confirmed by robot", port, value);

                        // Update IoStatus on target entity (will be synced to clients)
                        ctx.run_on_main_thread(move |ctx| {
                            if let Some(mut io_status) = ctx.world.get_mut::<IoStatus>(target) {
                                let word_index = (port as usize - 1) / 16;
                                let bit_index = (port as usize - 1) % 16;
                                while io_status.digital_outputs.len() <= word_index {
                                    io_status.digital_outputs.push(0);
                                }
                                if value {
                                    io_status.digital_outputs[word_index] |= 1 << bit_index;
                                } else {
                                    io_status.digital_outputs[word_index] &= !(1 << bit_index);
                                }
                            }
                        }).await;

                        let response = DoutValueResponse {
                            port_number: port,
                            port_value: value,
                            success: true,
                            error: None,
                        };
                        let _ = request.respond(response);
                    }
                }
                Ok(None) => {
                    bevy::log::error!("No response received for DOUT[{}]", port);
                    let response = DoutValueResponse {
                        port_number: port,
                        port_value: value,
                        success: false,
                        error: Some("No response received".to_string()),
                    };
                    let _ = request.respond(response);
                }
                Err(_) => {
                    bevy::log::error!("Timeout waiting for DOUT[{}] response", port);
                    let response = DoutValueResponse {
                        port_number: port,
                        port_value: value,
                        success: false,
                        error: Some("Timeout waiting for response".to_string()),
                    };
                    let _ = request.respond(response);
                }
            }
        });
    }
}

/// Handle ReadAin request - reads analog input value.
/// This is a targeted query (no authorization required).
pub fn handle_read_ain(
    mut requests: MessageReader<Request<TargetedRequest<ReadAin>>>,
) {
    for request in requests.read() {
        let targeted = request.get_request();
        let port_number = targeted.request.port_number;
        info!("📋 Handling ReadAin for port {} on target {}", port_number, targeted.target_id);

        // TODO: Read from connected robot driver when available
        // For now, return mock value (0.0)
        let response = AinValueResponse {
            port_number,
            port_value: 0.0,
        };

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle WriteAout request - writes analog output value.
/// This is a targeted request that requires entity control.
pub fn handle_write_aout(
    mut requests: MessageReader<AuthorizedRequest<WriteAout>>,
    mut io_query: Query<&mut IoStatus, With<FanucRobot>>,
    driver_query: Query<&RmiDriver, With<FanucRobot>>,
    runtime: Option<Res<bevy_tokio_tasks::TokioTasksRuntime>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        let target = request.target_entity;
        let port = inner.port_number;
        let value = inner.port_value;
        info!("📋 Handling WriteAout for port {} = {} on entity {:?}", port, value, target);

        // Update the IoStatus component on target entity (synced to all clients)
        if let Ok(mut io_status) = io_query.get_mut(target) {
            io_status.analog_outputs.insert(port, value);
            info!("✅ Updated IoStatus AOUT[{}] = {}", port, value);
        }

        // Send command to robot driver (if connected)
        if let (Ok(driver), Some(runtime)) = (driver_query.get(target), runtime.as_ref()) {
            let driver = driver.0.clone();
            runtime.spawn_background_task(move |_ctx| async move {
                use fanuc_rmi::packets::{SendPacket, Command};
                use fanuc_rmi::commands::FrcWriteAOUT;
                let packet = SendPacket::Command(Command::FrcWriteAOUT(FrcWriteAOUT {
                    port_number: port,
                    port_value: value,
                }));
                if let Err(e) = driver.send_packet(packet, PacketPriority::Standard) {
                    bevy::log::error!("Failed to send WriteAout to robot: {}", e);
                }
            });
        }

        let response = AoutValueResponse {
            port_number: port,
            port_value: value,
            success: true,
            error: None,
        };

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle ReadGin request - reads group input value.
/// This is a targeted query (no authorization required).
pub fn handle_read_gin(
    mut requests: MessageReader<Request<TargetedRequest<ReadGin>>>,
) {
    for request in requests.read() {
        let targeted = request.get_request();
        let port_number = targeted.request.port_number;
        info!("📋 Handling ReadGin for port {} on target {}", port_number, targeted.target_id);

        // TODO: Read from connected robot driver when available
        // For now, return mock value (0)
        let response = GinValueResponse {
            port_number,
            port_value: 0,
        };

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle WriteGout request - writes group output value.
/// This is a targeted request that requires entity control.
pub fn handle_write_gout(
    mut requests: MessageReader<AuthorizedRequest<WriteGout>>,
    mut io_query: Query<&mut IoStatus, With<FanucRobot>>,
    driver_query: Query<&RmiDriver, With<FanucRobot>>,
    runtime: Option<Res<bevy_tokio_tasks::TokioTasksRuntime>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        let target = request.target_entity;
        let port = inner.port_number;
        let value = inner.port_value;
        info!("📋 Handling WriteGout for port {} = {} on entity {:?}", port, value, target);

        // Update the IoStatus component on target entity (synced to all clients)
        if let Ok(mut io_status) = io_query.get_mut(target) {
            io_status.group_outputs.insert(port, value);
            info!("✅ Updated IoStatus GOUT[{}] = {}", port, value);
        }

        // Send command to robot driver (if connected)
        if let (Ok(driver), Some(runtime)) = (driver_query.get(target), runtime.as_ref()) {
            let driver = driver.0.clone();
            runtime.spawn_background_task(move |_ctx| async move {
                use fanuc_rmi::packets::{SendPacket, Command};
                use fanuc_rmi::commands::FrcWriteGOUT;
                let packet = SendPacket::Command(Command::FrcWriteGOUT(FrcWriteGOUT {
                    port_number: port,
                    port_value: value,
                }));
                if let Err(e) = driver.send_packet(packet, PacketPriority::Standard) {
                    bevy::log::error!("Failed to send WriteGout to robot: {}", e);
                }
            });
        }

        let response = GoutValueResponse {
            port_number: port,
            port_value: value,
            success: true,
            error: None,
        };

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle GetIoConfig request - gets I/O display configuration.
fn handle_get_io_config(
    mut requests: MessageReader<Request<GetIoConfig>>,
    db: Option<Res<DatabaseResource>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling GetIoConfig for robot_connection_id={}", inner.robot_connection_id);

        let configs = match db.as_ref() {
            Some(db_res) => {
                let conn = db_res.connection();
                let conn = conn.lock().unwrap();
                database::get_io_config(&conn, inner.robot_connection_id).unwrap_or_default()
            }
            None => vec![],
        };
        let response = IoConfigResponse { configs };

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle UpdateIoConfig request - updates I/O display configuration.
fn handle_update_io_config(
    mut requests: MessageReader<Request<UpdateIoConfig>>,
    db: Option<Res<DatabaseResource>>,
    mut robot_query: Query<(&ConnectionState, &mut IoConfigState)>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling UpdateIoConfig for robot_connection_id={} with {} configs",
            inner.robot_connection_id, inner.configs.len());

        let (success, error) = match db.as_ref() {
            Some(db_res) => {
                let conn = db_res.connection();
                let conn = conn.lock().unwrap();
                match database::update_io_config(&conn, inner.robot_connection_id, &inner.configs) {
                    Ok(_) => {
                        // Find the robot entity with matching active_connection_id and update its IoConfigState
                        for (conn_state, mut io_config) in robot_query.iter_mut() {
                            if conn_state.active_connection_id == Some(inner.robot_connection_id) {
                                // Build the new IoConfigState from the configs
                                let mut new_configs = std::collections::HashMap::new();
                                for cfg in &inner.configs {
                                    new_configs.insert(
                                        (cfg.io_type.clone(), cfg.io_index),
                                        cfg.clone(),
                                    );
                                }
                                io_config.configs = new_configs;
                                info!("✅ Updated IoConfigState on robot entity");
                                break;
                            }
                        }
                        (true, None)
                    },
                    Err(e) => (false, Some(e.to_string())),
                }
            }
            None => (false, Some("Database not available".to_string())),
        };
        let response = UpdateIoConfigResponse { success, error };

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

// ============================================================================
// Settings Handlers
// ============================================================================

/// Handle GetSettings request.
fn handle_get_settings(
    mut requests: MessageReader<Request<GetSettings>>,
    db: Option<Res<DatabaseResource>>,
) {
    for request in requests.read() {
        info!("📋 Handling GetSettings");

        let settings = match db.as_ref() {
            Some(db_res) => {
                let conn = db_res.connection();
                let conn = conn.lock().unwrap();
                database::get_settings(&conn).unwrap_or_default()
            }
            None => RobotSettings::default(),
        };
        let response = SettingsResponse { settings };

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle UpdateSettings request.
fn handle_update_settings(
    mut requests: MessageReader<Request<UpdateSettings>>,
    db: Option<Res<DatabaseResource>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling UpdateSettings: speed={}", inner.default_speed);

        let settings = RobotSettings {
            default_w: inner.default_w,
            default_p: inner.default_p,
            default_r: inner.default_r,
            default_speed: inner.default_speed,
            default_term_type: inner.default_term_type.clone(),
            default_uframe: inner.default_uframe,
            default_utool: inner.default_utool,
        };

        let (success, error) = match db.as_ref() {
            Some(db_res) => {
                let conn = db_res.connection();
                let conn = conn.lock().unwrap();
                match database::update_settings(&conn, &settings) {
                    Ok(_) => (true, None),
                    Err(e) => (false, Some(e.to_string())),
                }
            }
            None => (false, Some("Database not available".to_string())),
        };
        let response = UpdateSettingsResponse { success, error };

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

// handle_reset_database moved to fanuc_replica_core

/// Handle GetConnectionStatus request.
/// This is a targeted query (no authorization required).
pub fn handle_get_connection_status(
    mut requests: MessageReader<Request<TargetedRequest<GetConnectionStatus>>>,
    robots: Query<(&RobotConnectionState, &ConnectionState), With<FanucRobot>>,
) {
    for request in requests.read() {
        let targeted = request.get_request();
        info!("📋 Handling GetConnectionStatus for target {}", targeted.target_id);

        // Parse target entity from bits string
        let target = match targeted.target_id.parse::<u64>() {
            Ok(bits) => Entity::from_bits(bits),
            Err(_) => {
                error!("Invalid target entity: {}", targeted.target_id);
                continue;
            }
        };

        // Get connection status from target entity
        let response = if let Ok((conn_state, conn_details)) = robots.get(target) {
            ConnectionStatusResponse {
                connected: *conn_state == RobotConnectionState::Connected,
                robot_name: Some(conn_details.robot_name.clone()),
                ip_address: Some(conn_details.robot_addr.clone()),
                port: None, // Port is embedded in robot_addr
            }
        } else {
            ConnectionStatusResponse {
                connected: false,
                robot_name: None,
                ip_address: None,
                port: None,
            }
        };

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle UpdateJogSettings request.
fn handle_update_jog_settings(
    mut requests: MessageReader<Request<UpdateJogSettings>>,
    db: Option<Res<DatabaseResource>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling UpdateJogSettings: cartesian_speed={}", inner.cartesian_jog_speed);

        // TODO: Save to database when available
        let _ = db;
        let response = UpdateJogSettingsResponse { success: true, error: None };

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}