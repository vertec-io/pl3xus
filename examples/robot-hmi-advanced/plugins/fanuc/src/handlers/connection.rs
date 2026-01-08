//! Robot connection and configuration handlers.
//!
//! Handles database operations for:
//! - Robot connections (CRUD)
//! - Robot configurations (CRUD)
//! - Configuration loading/saving

use bevy::prelude::*;
use bevy::ecs::message::MessageReader;
use pl3xus::managers::network_request::Request;
use pl3xus::Network;
use pl3xus_websockets::WebSocketProvider;
use pl3xus_sync::RequestInvalidateExt;
use robot_hmi_core::database::DatabaseResource;

use crate::database;
use crate::types::*;
use crate::connection::{FanucRobot, RobotConnectionState};

/// Handle ListRobotConnections request - returns saved robots from database.
pub fn handle_list_robot_connections(
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
        if let Err(e) = request.clone().respond(RobotConnectionsResponse { connections }) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle GetRobotConfigurations request - returns configurations for a robot.
pub fn handle_get_robot_configurations(
    mut requests: MessageReader<Request<GetRobotConfigurations>>,
    db: Option<Res<DatabaseResource>>,
) {
    for request in requests.read() {
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
        if let Err(e) = request.clone().respond(RobotConfigurationsResponse { robot_id: robot_connection_id, configurations }) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle CreateRobotConnection request - creates a new robot in database.
pub fn handle_create_robot_connection(
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

        if let Err(e) = request.clone().respond_and_invalidate(response, &net) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle UpdateRobotConnection request - updates an existing robot in database.
pub fn handle_update_robot_connection(
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

        if let Err(e) = request.clone().respond_and_invalidate(response, &net) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle DeleteRobotConnection request - deletes a robot from database.
pub fn handle_delete_robot_connection(
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

        if let Err(e) = request.clone().respond_and_invalidate(response, &net) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle CreateConfiguration request - creates a new configuration for a robot.
pub fn handle_create_configuration(
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

        if let Err(e) = request.clone().respond_and_invalidate(response, &net) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle UpdateConfiguration request - updates an existing configuration.
pub fn handle_update_configuration(
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

        if let Err(e) = request.clone().respond_and_invalidate(response, &net) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle DeleteConfiguration request - deletes a configuration.
pub fn handle_delete_configuration(
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

        if let Err(e) = request.clone().respond_and_invalidate(response, &net) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle SetDefaultConfiguration request - sets a configuration as the default.
pub fn handle_set_default_configuration(
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

        if let Err(e) = request.clone().respond_and_invalidate(response, &net) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle LoadConfiguration request - loads a configuration and applies it to the robot.
pub fn handle_load_configuration(
    tokio_runtime: Res<pl3xus_async::TokioTasksRuntime>,
    mut requests: MessageReader<Request<LoadConfiguration>>,
    db: Option<Res<DatabaseResource>>,
    mut robot_query: Query<(
        &mut ActiveConfigState,
        &mut FrameToolDataState,
        &RobotConnectionState,
        Option<&crate::connection::RmiDriver>,
    ), With<FanucRobot>>,
) {
    use fanuc_rmi::dto as raw_dto;
    use fanuc_rmi::packets::PacketPriority;

    let _guard = tokio_runtime.runtime().enter();

    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling LoadConfiguration for id={}", inner.configuration_id);

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

                let robot_result = robot_query.iter_mut()
                    .find(|(_, _, state, driver)| **state == RobotConnectionState::Connected && driver.is_some());

                if let Some((mut active_config, mut ft_state, _, driver)) = robot_result {
                    let driver = driver.expect("Checked above");

                    let command = raw_dto::Command::FrcSetUFrameUTool(raw_dto::FrcSetUFrameUTool {
                        group: 1,
                        u_frame_number: config.u_frame_number as u8,
                        u_tool_number: config.u_tool_number as u8,
                    });
                    let send_packet: fanuc_rmi::packets::SendPacket = raw_dto::SendPacket::Command(command).into();

                    match driver.0.send_packet(send_packet, PacketPriority::Immediate) {
                        Ok(seq) => {
                            info!("Sent FrcSetUFrameUTool (frame={}, tool={}) seq={}",
                                config.u_frame_number, config.u_tool_number, seq);
                            ft_state.active_frame = config.u_frame_number;
                            ft_state.active_tool = config.u_tool_number;
                        }
                        Err(e) => {
                            error!("Failed to send FrcSetUFrameUTool: {:?}", e);
                        }
                    }

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

                    LoadConfigurationResponse { success: true, error: None }
                } else {
                    warn!("LoadConfiguration: No connected robot");
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
                    LoadConfigurationResponse { success: true, error: None }
                }
            }
            Err(e) => {
                error!("❌ Failed to load configuration: {}", e);
                LoadConfigurationResponse { success: false, error: Some(e.to_string()) }
            }
        };

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle SaveCurrentConfiguration request - saves active config to database.
pub fn handle_save_current_configuration(
    mut requests: MessageReader<Request<SaveCurrentConfiguration>>,
    db: Option<Res<DatabaseResource>>,
    net: Res<Network<WebSocketProvider>>,
    mut robot_query: Query<(&mut ActiveConfigState, &ConnectionState), With<FanucRobot>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling SaveCurrentConfiguration, name={:?}", inner.name);

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

        let result = db.as_ref()
            .map(|db| {
                let conn = db.connection();
                let conn = conn.lock().unwrap();
                database::save_current_configuration(
                    &conn, robot_connection_id, active_config.loaded_from_id,
                    inner.name.clone(), &active_config,
                )
            })
            .unwrap_or(Err(anyhow::anyhow!("Database not available")));

        let response = match result {
            Ok((config_id, config_name)) => {
                info!("✅ Saved configuration id={}, name='{}'", config_id, config_name);
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

        if let Err(e) = request.clone().respond_and_invalidate(response, &net) {
            error!("Failed to send response: {:?}", e);
        }
    }
}
