//! Request/Response handlers plugin.
//!
//! Handles database queries like ListRobotConnections and GetRobotConfigurations.

use bevy::prelude::*;
use bevy::ecs::message::MessageReader;
use pl3xus::managers::network_request::{AppNetworkRequestMessage, Request};
use pl3xus::Network;
use pl3xus_websockets::WebSocketProvider;
use pl3xus_sync::control::EntityControl;
use pl3xus_sync::{AuthorizedRequest, QueryInvalidation, SyncServerMessage};
use fanuc_replica_types::*;

use bevy_tokio_tasks::TokioTasksRuntime;
use crate::database::DatabaseResource;
use crate::plugins::connection::{FanucRobot, RmiDriver, RobotConnectionState};
use crate::plugins::execution::{ProgramExecutor, LoadedProgramData, ExecutorRunState};
use crate::plugins::program::{Program, ProgramState, ProgramDefaults, ApproachRetreat, ExecutionBuffer, console_entry as program_console_entry};
use crate::plugins::system::ActiveSystem;
use fanuc_rmi::packets::PacketPriority;

pub struct RequestHandlerPlugin;

impl Plugin for RequestHandlerPlugin {
    fn build(&self, app: &mut App) {
        // Register request listeners - Robot Connections
        app.listen_for_request_message::<ListRobotConnections, WebSocketProvider>();
        app.listen_for_request_message::<GetRobotConfigurations, WebSocketProvider>();
        app.listen_for_request_message::<CreateRobotConnection, WebSocketProvider>();
        app.listen_for_request_message::<UpdateRobotConnection, WebSocketProvider>();
        app.listen_for_request_message::<DeleteRobotConnection, WebSocketProvider>();
        app.listen_for_request_message::<CreateConfiguration, WebSocketProvider>();
        app.listen_for_request_message::<UpdateConfiguration, WebSocketProvider>();
        app.listen_for_request_message::<DeleteConfiguration, WebSocketProvider>();
        app.listen_for_request_message::<SetDefaultConfiguration, WebSocketProvider>();
        app.listen_for_request_message::<LoadConfiguration, WebSocketProvider>();

        // Register request listeners - Programs
        app.listen_for_request_message::<ListPrograms, WebSocketProvider>();
        app.listen_for_request_message::<GetProgram, WebSocketProvider>();
        app.listen_for_request_message::<CreateProgram, WebSocketProvider>();
        app.listen_for_request_message::<DeleteProgram, WebSocketProvider>();
        app.listen_for_request_message::<UpdateProgramSettings, WebSocketProvider>();
        app.listen_for_request_message::<UploadCsv, WebSocketProvider>();
        app.listen_for_request_message::<LoadProgram, WebSocketProvider>();
        // Note: StartProgram, PauseProgram, ResumeProgram, StopProgram, UnloadProgram are registered
        // as targeted requests in sync.rs with authorization middleware

        // Register request listeners - Frame/Tool
        app.listen_for_request_message::<GetActiveFrameTool, WebSocketProvider>();
        app.listen_for_request_message::<SetActiveFrameTool, WebSocketProvider>();
        app.listen_for_request_message::<GetFrameData, WebSocketProvider>();
        app.listen_for_request_message::<WriteFrameData, WebSocketProvider>();
        app.listen_for_request_message::<GetToolData, WebSocketProvider>();
        app.listen_for_request_message::<WriteToolData, WebSocketProvider>();

        // Register request listeners - I/O
        app.listen_for_request_message::<ReadDin, WebSocketProvider>();
        app.listen_for_request_message::<ReadDinBatch, WebSocketProvider>();
        app.listen_for_request_message::<WriteDout, WebSocketProvider>();
        app.listen_for_request_message::<ReadAin, WebSocketProvider>();
        app.listen_for_request_message::<WriteAout, WebSocketProvider>();
        app.listen_for_request_message::<ReadGin, WebSocketProvider>();
        app.listen_for_request_message::<WriteGout, WebSocketProvider>();
        app.listen_for_request_message::<GetIoConfig, WebSocketProvider>();
        app.listen_for_request_message::<UpdateIoConfig, WebSocketProvider>();

        // Register request listeners - Settings
        app.listen_for_request_message::<GetSettings, WebSocketProvider>();
        app.listen_for_request_message::<UpdateSettings, WebSocketProvider>();
        app.listen_for_request_message::<ResetDatabase, WebSocketProvider>();
        app.listen_for_request_message::<GetConnectionStatus, WebSocketProvider>();
        app.listen_for_request_message::<GetExecutionState, WebSocketProvider>();
        app.listen_for_request_message::<UpdateJogSettings, WebSocketProvider>();

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
        ));

        // Add handler systems - Programs
        app.add_systems(Update, (
            handle_list_programs,
            handle_get_program,
            handle_create_program,
            handle_delete_program,
            handle_update_program_settings,
            handle_upload_csv,
            handle_load_program,
            handle_unload_program,
            handle_start_program,
            handle_pause_program,
            handle_resume_program,
            handle_stop_program,
        ));

        // Add handler systems - Frame/Tool
        app.add_systems(Update, (
            handle_get_active_frame_tool,
            handle_set_active_frame_tool,
            handle_get_frame_data,
            handle_write_frame_data,
            handle_get_tool_data,
            handle_write_tool_data,
        ));

        // Add handler systems - I/O
        app.add_systems(Update, (
            handle_read_din,
            handle_read_din_batch,
            handle_write_dout,
            handle_read_ain,
            handle_write_aout,
            handle_read_gin,
            handle_write_gout,
            handle_get_io_config,
            handle_update_io_config,
        ));

        // Add handler systems - Settings
        app.add_systems(Update, (
            handle_get_settings,
            handle_update_settings,
            handle_reset_database,
            handle_get_connection_status,
            handle_get_execution_state,
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
            .map(|db| db.list_robot_connections().unwrap_or_default())
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
            .map(|db| db.get_configurations_for_robot(robot_connection_id).unwrap_or_default())
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
            .map(|db| db.create_robot_connection(inner))
            .unwrap_or(Err(anyhow::anyhow!("Database not available")));

        let response = match result {
            Ok(robot_id) => {
                info!("✅ Created robot connection id={}", robot_id);
                // Invalidate ListRobotConnections queries on all clients
                net.broadcast(SyncServerMessage::QueryInvalidation(QueryInvalidation {
                    query_types: vec!["ListRobotConnections".to_string()],
                    keys: None,
                }));
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

        if let Err(e) = request.clone().respond(response) {
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
            .map(|db| db.update_robot_connection(inner))
            .unwrap_or(Err(anyhow::anyhow!("Database not available")));

        let response = match result {
            Ok(()) => {
                info!("✅ Updated robot connection id={}", inner.id);
                // Invalidate ListRobotConnections queries on all clients
                net.broadcast(SyncServerMessage::QueryInvalidation(QueryInvalidation {
                    query_types: vec!["ListRobotConnections".to_string()],
                    keys: None,
                }));
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

        if let Err(e) = request.clone().respond(response) {
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
            .map(|db| db.delete_robot_connection(inner.id))
            .unwrap_or(Err(anyhow::anyhow!("Database not available")));

        let response = match result {
            Ok(()) => {
                info!("✅ Deleted robot connection id={}", inner.id);
                // Invalidate ListRobotConnections queries on all clients
                net.broadcast(SyncServerMessage::QueryInvalidation(QueryInvalidation {
                    query_types: vec!["ListRobotConnections".to_string()],
                    keys: None,
                }));
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

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle CreateConfiguration request - creates a new configuration for a robot.
fn handle_create_configuration(
    mut requests: MessageReader<Request<CreateConfiguration>>,
    db: Option<Res<DatabaseResource>>,
    net: Option<Res<Network<WebSocketProvider>>>,
) {
    let mut should_invalidate = false;

    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling CreateConfiguration for robot_connection_id={}", inner.robot_connection_id);

        let result = db.as_ref()
            .map(|db| db.create_configuration(inner))
            .unwrap_or(Err(anyhow::anyhow!("Database not available")));

        let response = match result {
            Ok(id) => {
                info!("✅ Created configuration id={}", id);
                should_invalidate = true;
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

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }

    // Invalidate configuration queries so all clients refetch
    if should_invalidate {
        if let Some(net) = net {
            let invalidation = QueryInvalidation {
                query_types: vec!["GetRobotConfigurations".to_string()],
                keys: None,
            };
            net.broadcast(SyncServerMessage::QueryInvalidation(invalidation));
            info!("📢 Broadcast query invalidation for GetRobotConfigurations");
        }
    }
}

/// Handle UpdateConfiguration request - updates an existing configuration.
fn handle_update_configuration(
    mut requests: MessageReader<Request<UpdateConfiguration>>,
    db: Option<Res<DatabaseResource>>,
    net: Option<Res<Network<WebSocketProvider>>>,
) {
    let mut should_invalidate = false;

    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling UpdateConfiguration for id={}", inner.id);

        let result = db.as_ref()
            .map(|db| db.update_configuration(inner))
            .unwrap_or(Err(anyhow::anyhow!("Database not available")));

        let response = match result {
            Ok(()) => {
                info!("✅ Updated configuration id={}", inner.id);
                should_invalidate = true;
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

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }

    // Invalidate configuration queries so all clients refetch
    if should_invalidate {
        if let Some(net) = net {
            let invalidation = QueryInvalidation {
                query_types: vec!["GetRobotConfigurations".to_string()],
                keys: None,
            };
            net.broadcast(SyncServerMessage::QueryInvalidation(invalidation));
            info!("📢 Broadcast query invalidation for GetRobotConfigurations");
        }
    }
}

/// Handle DeleteConfiguration request - deletes a configuration.
fn handle_delete_configuration(
    mut requests: MessageReader<Request<DeleteConfiguration>>,
    db: Option<Res<DatabaseResource>>,
    net: Option<Res<Network<WebSocketProvider>>>,
) {
    let mut should_invalidate = false;

    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling DeleteConfiguration for id={}", inner.id);

        let result = db.as_ref()
            .map(|db| db.delete_configuration(inner.id))
            .unwrap_or(Err(anyhow::anyhow!("Database not available")));

        let response = match result {
            Ok(()) => {
                info!("✅ Deleted configuration id={}", inner.id);
                should_invalidate = true;
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

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }

    // Invalidate configuration queries so all clients refetch
    if should_invalidate {
        if let Some(net) = net {
            let invalidation = QueryInvalidation {
                query_types: vec!["GetRobotConfigurations".to_string()],
                keys: None,
            };
            net.broadcast(SyncServerMessage::QueryInvalidation(invalidation));
            info!("📢 Broadcast query invalidation for GetRobotConfigurations");
        }
    }
}

/// Handle SetDefaultConfiguration request - sets a configuration as the default.
fn handle_set_default_configuration(
    mut requests: MessageReader<Request<SetDefaultConfiguration>>,
    db: Option<Res<DatabaseResource>>,
    net: Option<Res<Network<WebSocketProvider>>>,
) {
    let mut should_invalidate = false;

    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling SetDefaultConfiguration for id={}", inner.id);

        let result = db.as_ref()
            .map(|db| db.set_default_configuration(inner.id))
            .unwrap_or(Err(anyhow::anyhow!("Database not available")));

        let response = match result {
            Ok(()) => {
                info!("✅ Set default configuration id={}", inner.id);
                should_invalidate = true;
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

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }

    // Invalidate configuration queries so all clients refetch
    if should_invalidate {
        if let Some(net) = net {
            let invalidation = QueryInvalidation {
                query_types: vec!["GetRobotConfigurations".to_string()],
                keys: None,
            };
            net.broadcast(SyncServerMessage::QueryInvalidation(invalidation));
            info!("📢 Broadcast query invalidation for GetRobotConfigurations");
        }
    }
}

/// Handle LoadConfiguration request - loads a configuration and updates the active config state.
fn handle_load_configuration(
    mut requests: MessageReader<Request<LoadConfiguration>>,
    db: Option<Res<DatabaseResource>>,
    mut robot_query: Query<&mut ActiveConfigState, With<FanucRobot>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling LoadConfiguration for id={}", inner.configuration_id);

        // Get the configuration from database
        let result = db.as_ref()
            .map(|db| db.get_configuration(inner.configuration_id))
            .unwrap_or(Err(anyhow::anyhow!("Database not available")));

        let response = match result {
            Ok(config) => {
                info!("✅ Loaded configuration id={}", inner.configuration_id);

                // Update the ActiveConfigState on the robot entity
                for mut active_config in robot_query.iter_mut() {
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
                }

                LoadConfigurationResponse {
                    success: true,
                    error: None,
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

// ============================================================================
// Programs CRUD Handlers
// ============================================================================

/// Handle ListPrograms request - returns all programs from database.
fn handle_list_programs(
    mut requests: MessageReader<Request<ListPrograms>>,
    db: Option<Res<DatabaseResource>>,
) {
    for request in requests.read() {
        info!("📋 Handling ListPrograms request");

        let programs = db.as_ref()
            .and_then(|db| db.list_programs().ok())
            .unwrap_or_default();

        // Convert ProgramInfo to ProgramWithLines
        // Use instruction_count to create placeholder lines so .len() works in UI
        let programs_with_lines: Vec<ProgramWithLines> = programs.into_iter()
            .map(|p| {
                // Create placeholder lines to represent the count
                let lines: Vec<ProgramLineInfo> = (0..p.instruction_count)
                    .map(|_| ProgramLineInfo {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                        w: 0.0,
                        p: 0.0,
                        r: 0.0,
                        speed: 0.0,
                        term_type: String::new(),
                        uframe: None,
                        utool: None,
                    })
                    .collect();
                ProgramWithLines {
                    id: p.id,
                    name: p.name,
                    description: p.description,
                    lines,
                }
            })
            .collect();

        info!("📤 Responding with {} programs", programs_with_lines.len());
        if let Err(e) = request.clone().respond(ListProgramsResponse { programs: programs_with_lines }) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle GetProgram request - returns a single program with instructions.
fn handle_get_program(
    mut requests: MessageReader<Request<GetProgram>>,
    db: Option<Res<DatabaseResource>>,
) {
    for request in requests.read() {
        let program_id = request.get_request().program_id;
        info!("📋 Handling GetProgram for id={}", program_id);

        let program = db.as_ref()
            .and_then(|db| db.get_program(program_id).ok())
            .flatten();

        info!("📤 Responding with program: {:?}", program.as_ref().map(|p| &p.name));
        if let Err(e) = request.clone().respond(GetProgramResponse { program }) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle CreateProgram request - creates a new program.
fn handle_create_program(
    mut requests: MessageReader<Request<CreateProgram>>,
    db: Option<Res<DatabaseResource>>,
    net: Res<Network<WebSocketProvider>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling CreateProgram: '{}'", inner.name);

        let result = db.as_ref()
            .map(|db| db.create_program(&inner.name, inner.description.as_deref()))
            .unwrap_or(Err(anyhow::anyhow!("Database not available")));

        let response = match result {
            Ok(program_id) => {
                info!("✅ Created program id={}", program_id);
                // Invalidate ListPrograms queries on all clients
                net.broadcast(SyncServerMessage::QueryInvalidation(QueryInvalidation {
                    query_types: vec!["ListPrograms".to_string()],
                    keys: None,
                }));
                CreateProgramResponse {
                    success: true,
                    program_id: Some(program_id),
                    error: None,
                }
            }
            Err(e) => {
                error!("❌ Failed to create program: {}", e);
                CreateProgramResponse {
                    success: false,
                    program_id: None,
                    error: Some(e.to_string()),
                }
            }
        };

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle DeleteProgram request - deletes a program.
fn handle_delete_program(
    mut requests: MessageReader<Request<DeleteProgram>>,
    db: Option<Res<DatabaseResource>>,
    net: Res<Network<WebSocketProvider>>,
) {
    for request in requests.read() {
        let program_id = request.get_request().program_id;
        info!("📋 Handling DeleteProgram for id={}", program_id);

        let result = db.as_ref()
            .map(|db| db.delete_program(program_id))
            .unwrap_or(Err(anyhow::anyhow!("Database not available")));

        let response = match result {
            Ok(()) => {
                info!("✅ Deleted program id={}", program_id);
                // Invalidate ListPrograms and GetProgram queries on all clients
                net.broadcast(SyncServerMessage::QueryInvalidation(QueryInvalidation {
                    query_types: vec!["ListPrograms".to_string(), "GetProgram".to_string()],
                    keys: None,
                }));
                DeleteProgramResponse {
                    success: true,
                    error: None,
                }
            }
            Err(e) => {
                error!("❌ Failed to delete program: {}", e);
                DeleteProgramResponse {
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

/// Handle UpdateProgramSettings request - updates program start/end positions and settings.
fn handle_update_program_settings(
    mut requests: MessageReader<Request<UpdateProgramSettings>>,
    db: Option<Res<DatabaseResource>>,
    net: Res<Network<WebSocketProvider>>,
) {
    for request in requests.read() {
        let req = request.get_request();
        info!(
            "📋 Handling UpdateProgramSettings for program_id={}, move_speed={:?}",
            req.program_id, req.move_speed
        );

        let result = db.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database not available"))
            .and_then(|db| {
                db.update_program_settings(
                    req.program_id,
                    req.start_x,
                    req.start_y,
                    req.start_z,
                    req.start_w,
                    req.start_p,
                    req.start_r,
                    req.end_x,
                    req.end_y,
                    req.end_z,
                    req.end_w,
                    req.end_p,
                    req.end_r,
                    req.move_speed,
                    req.default_term_type.clone(),
                    req.default_term_value,
                )
            });

        let response = match result {
            Ok(()) => {
                info!("✅ Updated program settings for id={}", req.program_id);
                // Invalidate GetProgram queries for this specific program
                net.broadcast(SyncServerMessage::QueryInvalidation(QueryInvalidation {
                    query_types: vec!["GetProgram".to_string()],
                    keys: Some(vec![req.program_id.to_string()]),
                }));
                UpdateProgramSettingsResponse {
                    success: true,
                    error: None,
                }
            }
            Err(e) => {
                error!("❌ Failed to update program settings: {}", e);
                UpdateProgramSettingsResponse {
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

/// Handle UploadCsv request - uploads CSV data to a program.
fn handle_upload_csv(
    mut requests: MessageReader<Request<UploadCsv>>,
    db: Option<Res<DatabaseResource>>,
    net: Res<Network<WebSocketProvider>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling UploadCsv for program_id={}", inner.program_id);

        let result = parse_and_insert_csv(&inner.csv_content, inner.program_id, &db);

        let response = match result {
            Ok(count) => {
                info!("✅ Imported {} lines to program id={}", count, inner.program_id);
                // Invalidate GetProgram queries for this specific program
                info!("📢 Broadcasting QueryInvalidation for GetProgram (program_id={})", inner.program_id);
                net.broadcast(SyncServerMessage::QueryInvalidation(QueryInvalidation {
                    query_types: vec!["GetProgram".to_string()],
                    keys: Some(vec![inner.program_id.to_string()]),
                }));
                UploadCsvResponse {
                    success: true,
                    lines_imported: Some(count),
                    error: None,
                }
            }
            Err(e) => {
                error!("❌ Failed to upload CSV: {}", e);
                UploadCsvResponse {
                    success: false,
                    lines_imported: None,
                    error: Some(e.to_string()),
                }
            }
        };

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Parse CSV content and insert instructions into database.
/// Also auto-populates start/end positions and move_speed from first/last instructions.
fn parse_and_insert_csv(
    csv_content: &str,
    program_id: i64,
    db: &Option<Res<DatabaseResource>>,
) -> anyhow::Result<i32> {
    let db = db.as_ref().ok_or_else(|| anyhow::anyhow!("Database not available"))?;

    let mut instructions = Vec::new();
    let mut line_number = 1;

    for line in csv_content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 3 {
            continue; // Skip invalid lines
        }

        // Parse X, Y, Z (required)
        let x: f64 = parts.get(0).and_then(|s| s.trim().parse().ok()).unwrap_or(0.0);
        let y: f64 = parts.get(1).and_then(|s| s.trim().parse().ok()).unwrap_or(0.0);
        let z: f64 = parts.get(2).and_then(|s| s.trim().parse().ok()).unwrap_or(0.0);

        // Parse optional fields
        let w: Option<f64> = parts.get(3).and_then(|s| s.trim().parse().ok());
        let p: Option<f64> = parts.get(4).and_then(|s| s.trim().parse().ok());
        let r: Option<f64> = parts.get(5).and_then(|s| s.trim().parse().ok());
        let speed: Option<f64> = parts.get(6).and_then(|s| s.trim().parse().ok());

        instructions.push(Instruction {
            line_number,
            x,
            y,
            z,
            w,
            p,
            r,
            speed,
            term_type: None,
            term_value: None,
            uframe: None,
            utool: None,
        });

        line_number += 1;
    }

    let count = instructions.len() as i32;
    db.insert_instructions(program_id, &instructions)?;

    // Auto-populate start position from first instruction
    let (start_x, start_y, start_z, start_w, start_p, start_r) = if let Some(first) = instructions.first() {
        (Some(first.x), Some(first.y), Some(first.z), first.w, first.p, first.r)
    } else {
        (None, None, None, None, None, None)
    };

    // Auto-populate end position from last instruction
    let (end_x, end_y, end_z, end_w, end_p, end_r) = if let Some(last) = instructions.last() {
        (Some(last.x), Some(last.y), Some(last.z), last.w, last.p, last.r)
    } else {
        (None, None, None, None, None, None)
    };

    // Auto-populate move_speed from first instruction's speed if available
    let move_speed = instructions.first().and_then(|first| first.speed);

    // Update program settings with start/end positions and move_speed
    if start_x.is_some() || end_x.is_some() || move_speed.is_some() {
        if let Err(e) = db.update_program_settings(
            program_id,
            start_x, start_y, start_z, start_w, start_p, start_r,
            end_x, end_y, end_z, end_w, end_p, end_r,
            move_speed,
            None, // Keep existing term_type
            None, // Keep existing term_value
        ) {
            warn!("Failed to update program settings after CSV upload: {}", e);
        }
    }

    Ok(count)
}

/// Handle LoadProgram request - loads a program for execution.
///
/// IMPORTANT: Requires robot connection AND client control of System.
/// Creates a Program component on the System entity with all instructions.
/// Updates the server-side ExecutionState so all clients see the loaded program.
fn handle_load_program(
    mut commands: Commands,
    mut requests: MessageReader<Request<LoadProgram>>,
    db: Option<Res<DatabaseResource>>,
    system_query: Query<(Entity, &EntityControl), With<ActiveSystem>>,
    robots: Query<(Entity, &RobotConnectionState), With<FanucRobot>>,
    mut execution_states: Query<&mut ExecutionState>,
    mut executor: ResMut<ProgramExecutor>,
) {
    for request in requests.read() {
        let program_id = request.get_request().program_id;
        let client_id = request.source();
        info!("📋 Handling LoadProgram request for program {} from {:?}", program_id, client_id);

        // Check control on System entity
        let (system_entity, has_control) = match system_query.single() {
            Ok((entity, system_control)) => (entity, system_control.client_id == *client_id),
            Err(_) => {
                let response = LoadProgramResponse {
                    success: false,
                    program: None,
                    error: Some("System not ready".to_string()),
                };
                if let Err(e) = request.clone().respond(response) {
                    error!("Failed to send response: {:?}", e);
                }
                continue;
            }
        };

        // Check robot connection
        let (robot_entity, is_connected) = match robots.single() {
            Ok((entity, state)) => (Some(entity), *state == RobotConnectionState::Connected),
            Err(_) => (None, false),
        };

        if !is_connected {
            let response = LoadProgramResponse {
                success: false,
                program: None,
                error: Some("Robot not connected".to_string()),
            };
            if let Err(e) = request.clone().respond(response) {
                error!("Failed to send response: {:?}", e);
            }
            continue;
        }

        if !has_control {
            let response = LoadProgramResponse {
                success: false,
                program: None,
                error: Some("You do not have control of the apparatus".to_string()),
            };
            if let Err(e) = request.clone().respond(response) {
                error!("Failed to send response: {:?}", e);
            }
            continue;
        }

        // Get program from database
        let Some(db) = db.as_ref() else {
            let response = LoadProgramResponse {
                success: false,
                program: None,
                error: Some("Database not available".to_string()),
            };
            if let Err(e) = request.clone().respond(response) {
                error!("Failed to send response: {:?}", e);
            }
            continue;
        };

        match db.get_program(program_id) {
            Ok(Some(program_detail)) => {
                // Convert ProgramDetail to ProgramWithLines for client display
                let lines: Vec<ProgramLineInfo> = program_detail.instructions.iter()
                    .map(|inst| ProgramLineInfo {
                        x: inst.x,
                        y: inst.y,
                        z: inst.z,
                        w: inst.w.unwrap_or(0.0),
                        p: inst.p.unwrap_or(0.0),
                        r: inst.r.unwrap_or(0.0),
                        speed: inst.speed.unwrap_or(0.0),
                        term_type: inst.term_type.clone().unwrap_or_default(),
                        uframe: inst.uframe,
                        utool: inst.utool,
                    })
                    .collect();

                // Build approach/retreat positions
                let approach = if program_detail.start_x.is_some() && program_detail.start_y.is_some() && program_detail.start_z.is_some() {
                    Some(ApproachRetreat {
                        x: program_detail.start_x.unwrap(),
                        y: program_detail.start_y.unwrap(),
                        z: program_detail.start_z.unwrap(),
                        w: program_detail.start_w,
                        p: program_detail.start_p,
                        r: program_detail.start_r,
                    })
                } else {
                    None
                };

                let retreat = if program_detail.end_x.is_some() && program_detail.end_y.is_some() && program_detail.end_z.is_some() {
                    Some(ApproachRetreat {
                        x: program_detail.end_x.unwrap(),
                        y: program_detail.end_y.unwrap(),
                        z: program_detail.end_z.unwrap(),
                        w: program_detail.end_w,
                        p: program_detail.end_p,
                        r: program_detail.end_r,
                    })
                } else {
                    None
                };

                // Build program defaults - all required fields come directly from DB
                let defaults = ProgramDefaults {
                    w: program_detail.default_w,
                    p: program_detail.default_p,
                    r: program_detail.default_r,
                    speed: program_detail.default_speed.unwrap_or(100.0),  // default_speed is optional per-instruction override
                    speed_type: program_detail.default_speed_type.clone(),
                    term_type: program_detail.default_term_type.clone(),
                    term_value: program_detail.default_term_value,
                    uframe: program_detail.default_uframe,
                    utool: program_detail.default_utool,
                };

                // Create Program component on System entity
                info!(
                    "📋 Program '{}' loaded with move_speed={}",
                    &program_detail.name,
                    program_detail.move_speed
                );

                let program_component = Program {
                    id: program_detail.id,
                    name: program_detail.name.clone(),
                    instructions: program_detail.instructions.clone(),
                    current_index: 0,
                    completed_count: 0,
                    state: ProgramState::Idle,
                    defaults,
                    approach,
                    retreat,
                    move_speed: program_detail.move_speed,
                };

                // Insert Program component on System entity
                commands.entity(system_entity).insert(program_component);
                info!("📦 Created Program component on System entity for '{}'", program_detail.name);

                // Also add ExecutionBuffer to robot entity if not present
                if let Some(robot_entity) = robot_entity {
                    commands.entity(robot_entity).insert(ExecutionBuffer::new());
                }

                // LEGACY: Also update ProgramExecutor for backward compatibility
                let loaded_data = LoadedProgramData {
                    instructions: program_detail.instructions.clone(),
                    approach: if program_detail.start_x.is_some() && program_detail.start_y.is_some() && program_detail.start_z.is_some() {
                        Some((
                            program_detail.start_x.unwrap(),
                            program_detail.start_y.unwrap(),
                            program_detail.start_z.unwrap(),
                            program_detail.start_w.unwrap_or(0.0),
                            program_detail.start_p.unwrap_or(0.0),
                            program_detail.start_r.unwrap_or(0.0),
                        ))
                    } else {
                        None
                    },
                    retreat: if program_detail.end_x.is_some() && program_detail.end_y.is_some() && program_detail.end_z.is_some() {
                        Some((
                            program_detail.end_x.unwrap(),
                            program_detail.end_y.unwrap(),
                            program_detail.end_z.unwrap(),
                            program_detail.end_w.unwrap_or(0.0),
                            program_detail.end_p.unwrap_or(0.0),
                            program_detail.end_r.unwrap_or(0.0),
                        ))
                    } else {
                        None
                    },
                    move_speed: program_detail.move_speed,
                };

                executor.loaded_program_id = Some(program_detail.id);
                executor.loaded_program_name = Some(program_detail.name.clone());
                executor.loaded_program_data = Some(loaded_data);
                executor.total_instructions = lines.len();
                executor.completed_line = 0;
                executor.run_state = ExecutorRunState::Idle;
                executor.defaults.w = program_detail.default_w;
                executor.defaults.p = program_detail.default_p;
                executor.defaults.r = program_detail.default_r;
                executor.defaults.speed = program_detail.default_speed.unwrap_or(100.0);  // default_speed is optional per-instruction override
                executor.defaults.term_type = program_detail.default_term_type.clone();
                executor.defaults.term_value = program_detail.default_term_value;
                executor.defaults.uframe = program_detail.default_uframe;
                executor.defaults.utool = program_detail.default_utool;

                // Update ExecutionState component (synced to all clients)
                if let Some(entity) = robot_entity {
                    if let Ok(mut exec_state) = execution_states.get_mut(entity) {
                        exec_state.loaded_program_id = Some(program_detail.id);
                        exec_state.loaded_program_name = Some(program_detail.name.clone());
                        exec_state.state = ProgramExecutionState::Idle;
                        exec_state.current_line = 0;
                        exec_state.total_lines = lines.len();
                        exec_state.program_lines = lines.clone();
                        // Set available actions for Idle state
                        // can_load is false because a program is now loaded
                        exec_state.can_load = false;
                        exec_state.can_start = true;
                        exec_state.can_pause = false;
                        exec_state.can_resume = false;
                        exec_state.can_stop = false;
                        exec_state.can_unload = true;
                        info!("📡 Updated ExecutionState: program '{}' with {} lines synced to all clients",
                            program_detail.name, lines.len());
                    }
                }

                let program_with_lines = ProgramWithLines {
                    id: program_detail.id,
                    name: program_detail.name.clone(),
                    description: program_detail.description.clone(),
                    lines,
                };
                info!("✅ Loaded program '{}' with {} instructions", program_detail.name, program_with_lines.lines.len());
                let response = LoadProgramResponse {
                    success: true,
                    program: Some(program_with_lines),
                    error: None,
                };
                if let Err(e) = request.clone().respond(response) {
                    error!("Failed to send response: {:?}", e);
                }
            }
            Ok(None) => {
                let response = LoadProgramResponse {
                    success: false,
                    program: None,
                    error: Some(format!("Program {} not found", program_id)),
                };
                if let Err(e) = request.clone().respond(response) {
                    error!("Failed to send response: {:?}", e);
                }
            }
            Err(e) => {
                let response = LoadProgramResponse {
                    success: false,
                    program: None,
                    error: Some(format!("Database error: {}", e)),
                };
                if let Err(e) = request.clone().respond(response) {
                    error!("Failed to send response: {:?}", e);
                }
            }
        }
    }
}

/// Handle UnloadProgram request - unloads the currently loaded program.
///
/// Authorization is handled by middleware - no manual control check needed.
/// This is a targeted request that targets the System entity.
fn handle_unload_program(
    mut commands: Commands,
    mut requests: MessageReader<AuthorizedRequest<UnloadProgram>>,
    system_query: Query<Entity, With<ActiveSystem>>,
    program_query: Query<&Program, With<ActiveSystem>>,
    mut executor: ResMut<ProgramExecutor>,
    mut execution_states: Query<&mut ExecutionState>,
    mut robots: Query<Option<&mut ExecutionBuffer>, With<FanucRobot>>,
) {
    for request in requests.read() {
        let request = request.clone();
        let client_id = request.source();
        info!("📋 Handling UnloadProgram request from {:?}", client_id);

        // Get the system entity
        let Ok(system_entity) = system_query.single() else {
            let response = UnloadProgramResponse {
                success: false,
                error: Some("System not ready".to_string()),
            };
            let _ = request.respond(response);
            continue;
        };

        // Remove Program component from System entity
        if program_query.get(system_entity).is_ok() {
            commands.entity(system_entity).remove::<Program>();
            info!("📦 Removed Program component from System entity");
        }

        // Clear ExecutionBuffer on robot entities
        for buffer_opt in robots.iter_mut() {
            if let Some(mut buffer) = buffer_opt {
                buffer.clear();
                info!("🧹 Cleared ExecutionBuffer");
            }
        }

        // LEGACY: Reset executor state for backward compatibility
        executor.reset();

        // Reset ExecutionState on all robot entities (synced to clients)
        for mut exec_state in execution_states.iter_mut() {
            exec_state.loaded_program_id = None;
            exec_state.loaded_program_name = None;
            exec_state.state = ProgramExecutionState::NoProgram;
            exec_state.current_line = 0;
            exec_state.total_lines = 0;
            exec_state.program_lines.clear();
            // Set available actions for NoProgram state
            exec_state.can_load = true;
            exec_state.can_start = false;
            exec_state.can_pause = false;
            exec_state.can_resume = false;
            exec_state.can_stop = false;
            exec_state.can_unload = false;
        }

        // Debug: Log the actual values being set
        for exec_state in execution_states.iter() {
            info!("📡 Unloaded program - ExecutionState after unload: can_load={}, can_start={}, can_unload={}, state={:?}",
                exec_state.can_load, exec_state.can_start, exec_state.can_unload, exec_state.state);
        }

        info!("📡 Unloaded program - ExecutionState cleared and synced to all clients");
        let response = UnloadProgramResponse { success: true, error: None };

        if let Err(e) = request.respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle StartProgram request - starts executing the currently loaded program.
/// A program must be loaded first via LoadProgram.
///
/// Authorization is handled by middleware - no manual control check needed.
/// This is a targeted request that targets the System entity.
///
/// NEW ARCHITECTURE: This handler just sets the Program state to Running.
/// The orchestrator system (in program.rs) handles the actual instruction dispatch.
fn handle_start_program(
    mut requests: MessageReader<AuthorizedRequest<StartProgram>>,
    mut systems: Query<&mut Program, With<ActiveSystem>>,
    robots: Query<&RobotConnectionState, With<FanucRobot>>,
    net: Res<Network<WebSocketProvider>>,
) {
    for request in requests.read() {
        let request = request.clone();
        let client_id = request.source();
        info!("📋 Handling StartProgram request from {:?}", client_id);

        // Check if a program is loaded (Program component exists on System)
        let Ok(mut program) = systems.single_mut() else {
            let response = StartProgramResponse {
                success: false,
                error: Some("No program loaded. Use LoadProgram first.".to_string()),
            };
            let _ = request.respond(response);
            continue;
        };

        // Check if program has instructions
        if program.instructions.is_empty() {
            let response = StartProgramResponse {
                success: false,
                error: Some("Program has no instructions".to_string()),
            };
            let _ = request.respond(response);
            continue;
        }

        // Check if robot is connected
        let is_connected = robots.iter().any(|state| *state == RobotConnectionState::Connected);
        if !is_connected {
            let response = StartProgramResponse {
                success: false,
                error: Some("Robot not connected".to_string()),
            };
            let _ = request.respond(response);
            continue;
        }

        // Check if already running
        if program.state == ProgramState::Running {
            let response = StartProgramResponse {
                success: false,
                error: Some("Program is already running".to_string()),
            };
            let _ = request.respond(response);
            continue;
        }

        // Reset execution counters and set state to Running
        program.current_index = 0;
        program.completed_count = 0;
        program.state = ProgramState::Running;

        let program_name = program.name.clone();
        let total = program.total_instructions();

        info!("▶ Started program '{}' with {} instructions", program_name, total);

        // Broadcast console entry for program start
        let console_msg = program_console_entry(
            format!("Program '{}' started ({} instructions)", program_name, total),
            ConsoleDirection::System,
            ConsoleMsgType::Status,
        );
        net.broadcast(console_msg);

        let response = StartProgramResponse { success: true, error: None };
        if let Err(e) = request.respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle PauseProgram request - pauses program execution.
///
/// Authorization is handled by middleware - no manual control check needed.
/// This is a targeted request that targets the System entity.
fn handle_pause_program(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut requests: MessageReader<AuthorizedRequest<PauseProgram>>,
    mut systems: Query<&mut Program, With<ActiveSystem>>,
    robots: Query<(&RmiDriver, &RobotConnectionState), With<FanucRobot>>,
) {
    // Enter the Tokio runtime context so send_packet can use tokio::spawn
    let _guard = tokio_runtime.runtime().enter();

    for request in requests.read() {
        let request = request.clone();
        let client_id = request.source();
        info!("📋 Handling PauseProgram request from {:?}", client_id);

        let Ok(mut program) = systems.single_mut() else {
            let response = PauseProgramResponse { success: false, error: Some("No program loaded".to_string()) };
            let _ = request.respond(response);
            continue;
        };

        if program.state != ProgramState::Running {
            let response = PauseProgramResponse { success: false, error: Some("No program running".to_string()) };
            let _ = request.respond(response);
            continue;
        }

        // Send pause command to robot - find connected robot first
        let connected_robot = robots.iter()
            .find(|(_, state)| **state == RobotConnectionState::Connected);

        let pause_result = if let Some((driver, _)) = connected_robot {
            // Send FRC_Pause packet (unit variant)
            let pause_packet = fanuc_rmi::packets::SendPacket::Command(
                fanuc_rmi::packets::Command::FrcPause
            );
            info!("📤 Sending FRC_Pause to robot");
            match driver.0.send_packet(pause_packet, PacketPriority::High) {
                Ok(_) => {
                    info!("✅ FRC_Pause sent successfully");
                    Ok(())
                }
                Err(e) => {
                    error!("❌ Failed to send pause command: {}", e);
                    Err(format!("Failed to pause: {}", e))
                }
            }
        } else {
            warn!("⚠️ No connected robot found to send FRC_Pause");
            // Still allow pause to succeed even without robot connection
            Ok(())
        };

        match pause_result {
            Ok(()) => {
                program.state = ProgramState::Paused;
                info!("⏸ Program '{}' paused", program.name);
                let response = PauseProgramResponse { success: true, error: None };
                if let Err(e) = request.respond(response) {
                    error!("Failed to send response: {:?}", e);
                }
            }
            Err(err) => {
                let response = PauseProgramResponse { success: false, error: Some(err) };
                let _ = request.respond(response);
            }
        }
    }
}

/// Handle ResumeProgram request - resumes program execution.
///
/// Authorization is handled by middleware - no manual control check needed.
/// This is a targeted request that targets the System entity.
fn handle_resume_program(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut requests: MessageReader<AuthorizedRequest<ResumeProgram>>,
    mut systems: Query<&mut Program, With<ActiveSystem>>,
    robots: Query<(&RmiDriver, &RobotConnectionState), With<FanucRobot>>,
) {
    // Enter the Tokio runtime context so send_packet can use tokio::spawn
    let _guard = tokio_runtime.runtime().enter();

    for request in requests.read() {
        let request = request.clone();
        let client_id = request.source();
        info!("📋 Handling ResumeProgram request from {:?}", client_id);

        let Ok(mut program) = systems.single_mut() else {
            let response = ResumeProgramResponse { success: false, error: Some("No program loaded".to_string()) };
            let _ = request.respond(response);
            continue;
        };

        if program.state != ProgramState::Paused {
            let response = ResumeProgramResponse { success: false, error: Some("Program not paused".to_string()) };
            let _ = request.respond(response);
            continue;
        }

        // Send continue command to robot - find connected robot first
        let connected_robot = robots.iter()
            .find(|(_, state)| **state == RobotConnectionState::Connected);

        let resume_result = if let Some((driver, _)) = connected_robot {
            // Send FRC_Continue packet (unit variant)
            let continue_packet = fanuc_rmi::packets::SendPacket::Command(
                fanuc_rmi::packets::Command::FrcContinue
            );
            info!("📤 Sending FRC_Continue to robot");
            match driver.0.send_packet(continue_packet, PacketPriority::High) {
                Ok(_) => {
                    info!("✅ FRC_Continue sent successfully");
                    Ok(())
                }
                Err(e) => {
                    error!("❌ Failed to send continue command: {}", e);
                    Err(format!("Failed to resume: {}", e))
                }
            }
        } else {
            warn!("⚠️ No connected robot found to send FRC_Continue");
            // Still allow resume to succeed even without robot connection
            Ok(())
        };

        match resume_result {
            Ok(()) => {
                program.state = ProgramState::Running;
                info!("▶ Program '{}' resumed", program.name);
                let response = ResumeProgramResponse { success: true, error: None };
                if let Err(e) = request.respond(response) {
                    error!("Failed to send response: {:?}", e);
                }
            }
            Err(err) => {
                let response = ResumeProgramResponse { success: false, error: Some(err) };
                let _ = request.respond(response);
            }
        }
    }
}

/// Handle StopProgram request - stops program execution.
///
/// Authorization is handled by middleware - no manual control check needed.
/// This is a targeted request that targets the System entity.
fn handle_stop_program(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut requests: MessageReader<AuthorizedRequest<StopProgram>>,
    mut systems: Query<&mut Program, With<ActiveSystem>>,
    mut robots: Query<(&RmiDriver, &RobotConnectionState, Option<&mut ExecutionBuffer>), With<FanucRobot>>,
) {
    // Enter the Tokio runtime context so send_packet can use tokio::spawn
    let _guard = tokio_runtime.runtime().enter();

    for request in requests.read() {
        let request = request.clone();
        let client_id = request.source();
        info!("📋 Handling StopProgram request from {:?}", client_id);

        let Ok(mut program) = systems.single_mut() else {
            let response = StopProgramResponse { success: false, error: Some("No program loaded".to_string()) };
            let _ = request.respond(response);
            continue;
        };

        if program.state == ProgramState::Idle {
            let response = StopProgramResponse { success: false, error: Some("No program running".to_string()) };
            let _ = request.respond(response);
            continue;
        }

        // Send abort command to robot and clear execution buffer
        for (driver, state, buffer_opt) in robots.iter_mut() {
            if *state == RobotConnectionState::Connected {
                // Send FRC_Abort packet (unit variant)
                let abort_packet = fanuc_rmi::packets::SendPacket::Command(
                    fanuc_rmi::packets::Command::FrcAbort
                );
                info!("📤 Sending FRC_Abort to robot");
                if let Err(e) = driver.0.send_packet(abort_packet, PacketPriority::High) {
                    error!("❌ Failed to send abort command: {}", e);
                    // Continue anyway to reset state
                } else {
                    info!("✅ FRC_Abort sent successfully");
                }
                // Clear execution buffer to remove in-flight packet tracking
                if let Some(mut buffer) = buffer_opt {
                    buffer.clear();
                    info!("🧹 Cleared ExecutionBuffer");
                }
                break;
            }
        }

        let program_name = program.name.clone();
        // Reset to Idle state (program stays loaded for re-running)
        program.state = ProgramState::Idle;
        program.current_index = 0;
        program.completed_count = 0;
        info!("⏹ Program '{}' stopped (still loaded)", program_name);
        let response = StopProgramResponse { success: true, error: None };
        if let Err(e) = request.respond(response) {
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
fn handle_get_active_frame_tool(
    mut requests: MessageReader<Request<GetActiveFrameTool>>,
) {
    for request in requests.read() {
        info!("📋 Handling GetActiveFrameTool request");

        // TODO: Get from connected robot driver when available
        // For now, return defaults
        let response = GetActiveFrameToolResponse { uframe: 1, utool: 1 };

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle SetActiveFrameTool request - sets active frame and tool on robot.
fn handle_set_active_frame_tool(
    mut requests: MessageReader<Request<SetActiveFrameTool>>,
    mut robots: Query<&mut FrameToolDataState, With<FanucRobot>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling SetActiveFrameTool: uframe={}, utool={}", inner.uframe, inner.utool);

        // Update synced component so all clients get the active frame/tool
        for mut ft_state in robots.iter_mut() {
            ft_state.active_frame = inner.uframe;
            ft_state.active_tool = inner.utool;
        }

        // TODO: Send to connected robot driver when available
        // For now, just acknowledge success
        let response = SetActiveFrameToolResponse { success: true, error: None };

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle GetFrameData request - reads frame data from robot and updates synced state.
fn handle_get_frame_data(
    mut requests: MessageReader<Request<GetFrameData>>,
    mut robots: Query<&mut FrameToolDataState, With<FanucRobot>>,
) {
    for request in requests.read() {
        let frame_number = request.get_request().frame_number;
        info!("📋 Handling GetFrameData for frame {}", frame_number);

        // TODO: Read from connected robot driver when available
        // For now, return mock data (zeros)
        let frame_data = FrameToolData {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 0.0,
            p: 0.0,
            r: 0.0,
        };

        // Update synced component so all clients get the data
        for mut ft_state in robots.iter_mut() {
            ft_state.frames.insert(frame_number, frame_data.clone());
        }

        let response = FrameDataResponse {
            frame_number,
            x: frame_data.x,
            y: frame_data.y,
            z: frame_data.z,
            w: frame_data.w,
            p: frame_data.p,
            r: frame_data.r,
        };

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle WriteFrameData request - writes frame data to robot and updates synced state.
fn handle_write_frame_data(
    mut requests: MessageReader<Request<WriteFrameData>>,
    mut robots: Query<&mut FrameToolDataState, With<FanucRobot>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling WriteFrameData for frame {}", inner.frame_number);

        // Update synced component
        let frame_data = FrameToolData {
            x: inner.x,
            y: inner.y,
            z: inner.z,
            w: inner.w,
            p: inner.p,
            r: inner.r,
        };
        for mut ft_state in robots.iter_mut() {
            ft_state.frames.insert(inner.frame_number, frame_data.clone());
        }

        // TODO: Write to connected robot driver when available
        // For now, just acknowledge success
        let response = WriteFrameDataResponse { success: true, error: None };

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle GetToolData request - reads tool data from robot and updates synced state.
fn handle_get_tool_data(
    mut requests: MessageReader<Request<GetToolData>>,
    mut robots: Query<&mut FrameToolDataState, With<FanucRobot>>,
) {
    for request in requests.read() {
        let tool_number = request.get_request().tool_number;
        info!("📋 Handling GetToolData for tool {}", tool_number);

        // TODO: Read from connected robot driver when available
        // For now, return mock data (zeros)
        let tool_data = FrameToolData {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 0.0,
            p: 0.0,
            r: 0.0,
        };

        // Update synced component so all clients get the data
        for mut ft_state in robots.iter_mut() {
            ft_state.tools.insert(tool_number, tool_data.clone());
        }

        let response = ToolDataResponse {
            tool_number,
            x: tool_data.x,
            y: tool_data.y,
            z: tool_data.z,
            w: tool_data.w,
            p: tool_data.p,
            r: tool_data.r,
        };

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle WriteToolData request - writes tool data to robot and updates synced state.
fn handle_write_tool_data(
    mut requests: MessageReader<Request<WriteToolData>>,
    mut robots: Query<&mut FrameToolDataState, With<FanucRobot>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling WriteToolData for tool {}", inner.tool_number);

        // Update synced component
        let tool_data = FrameToolData {
            x: inner.x,
            y: inner.y,
            z: inner.z,
            w: inner.w,
            p: inner.p,
            r: inner.r,
        };
        for mut ft_state in robots.iter_mut() {
            ft_state.tools.insert(inner.tool_number, tool_data.clone());
        }

        // TODO: Write to connected robot driver when available
        // For now, just acknowledge success
        let response = WriteToolDataResponse { success: true, error: None };

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

// ============================================================================
// I/O Handlers
// ============================================================================

/// Handle ReadDin request - reads digital input value.
fn handle_read_din(
    mut requests: MessageReader<Request<ReadDin>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling ReadDin for port {}", inner.port_number);

        // TODO: Read from connected robot driver when available
        // For now, return mock value (false)
        let response = DinValueResponse {
            port_number: inner.port_number,
            port_value: false,
        };

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle ReadDinBatch request - reads multiple digital input values.
fn handle_read_din_batch(
    mut requests: MessageReader<Request<ReadDinBatch>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling ReadDinBatch for {} ports", inner.port_numbers.len());

        // TODO: Read from connected robot driver when available
        // For now, return mock values (all false)
        let values: Vec<(u16, bool)> = inner.port_numbers.iter()
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
fn handle_write_dout(
    mut requests: MessageReader<Request<WriteDout>>,
    driver_query: Query<&RmiDriver, With<FanucRobot>>,
    runtime: Option<Res<bevy_tokio_tasks::TokioTasksRuntime>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        let port = inner.port_number;
        let value = inner.port_value;
        info!("📋 Handling WriteDout for port {} = {}", port, value);

        // Get driver or respond with error
        let driver = match driver_query.single() {
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

                        // Update IoStatus on main thread (will be synced to clients)
                        ctx.run_on_main_thread(move |ctx| {
                            let mut io_query = ctx.world.query_filtered::<&mut IoStatus, With<FanucRobot>>();
                            if let Ok(mut io_status) = io_query.single_mut(ctx.world) {
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
fn handle_read_ain(
    mut requests: MessageReader<Request<ReadAin>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling ReadAin for port {}", inner.port_number);

        // TODO: Read from connected robot driver when available
        // For now, return mock value (0.0)
        let response = AinValueResponse {
            port_number: inner.port_number,
            port_value: 0.0,
        };

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle WriteAout request - writes analog output value.
fn handle_write_aout(
    mut requests: MessageReader<Request<WriteAout>>,
    mut io_query: Query<&mut IoStatus, With<FanucRobot>>,
    driver_query: Query<&RmiDriver, With<FanucRobot>>,
    runtime: Option<Res<bevy_tokio_tasks::TokioTasksRuntime>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        let port = inner.port_number;
        let value = inner.port_value;
        info!("📋 Handling WriteAout for port {} = {}", port, value);

        // Update the IoStatus component (synced to all clients)
        if let Ok(mut io_status) = io_query.single_mut() {
            io_status.analog_outputs.insert(port, value);
            info!("✅ Updated IoStatus AOUT[{}] = {}", port, value);
        }

        // Send command to robot driver (if connected)
        if let (Ok(driver), Some(runtime)) = (driver_query.single(), runtime.as_ref()) {
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
fn handle_read_gin(
    mut requests: MessageReader<Request<ReadGin>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling ReadGin for port {}", inner.port_number);

        // TODO: Read from connected robot driver when available
        // For now, return mock value (0)
        let response = GinValueResponse {
            port_number: inner.port_number,
            port_value: 0,
        };

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle WriteGout request - writes group output value.
fn handle_write_gout(
    mut requests: MessageReader<Request<WriteGout>>,
    mut io_query: Query<&mut IoStatus, With<FanucRobot>>,
    driver_query: Query<&RmiDriver, With<FanucRobot>>,
    runtime: Option<Res<bevy_tokio_tasks::TokioTasksRuntime>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        let port = inner.port_number;
        let value = inner.port_value;
        info!("📋 Handling WriteGout for port {} = {}", port, value);

        // Update the IoStatus component (synced to all clients)
        if let Ok(mut io_status) = io_query.single_mut() {
            io_status.group_outputs.insert(port, value);
            info!("✅ Updated IoStatus GOUT[{}] = {}", port, value);
        }

        // Send command to robot driver (if connected)
        if let (Ok(driver), Some(runtime)) = (driver_query.single(), runtime.as_ref()) {
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
            Some(db) => db.get_io_config(inner.robot_connection_id).unwrap_or_default(),
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
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling UpdateIoConfig for robot_connection_id={} with {} configs",
            inner.robot_connection_id, inner.configs.len());

        let (success, error) = match db.as_ref() {
            Some(db) => match db.update_io_config(inner.robot_connection_id, &inner.configs) {
                Ok(_) => (true, None),
                Err(e) => (false, Some(e.to_string())),
            },
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
            Some(db) => db.get_settings().unwrap_or_default(),
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
            Some(db) => match db.update_settings(&settings) {
                Ok(_) => (true, None),
                Err(e) => (false, Some(e.to_string())),
            },
            None => (false, Some("Database not available".to_string())),
        };
        let response = UpdateSettingsResponse { success, error };

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle ResetDatabase request.
fn handle_reset_database(
    mut requests: MessageReader<Request<ResetDatabase>>,
    db: Option<Res<DatabaseResource>>,
) {
    for request in requests.read() {
        info!("📋 Handling ResetDatabase");

        let (success, error) = match db.as_ref() {
            Some(db) => match db.reset_database() {
                Ok(_) => (true, None),
                Err(e) => (false, Some(e.to_string())),
            },
            None => (false, Some("Database not available".to_string())),
        };
        let response = ResetDatabaseResponse { success, error };

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle GetConnectionStatus request.
fn handle_get_connection_status(
    mut requests: MessageReader<Request<GetConnectionStatus>>,
) {
    for request in requests.read() {
        info!("📋 Handling GetConnectionStatus");

        // TODO: Check actual connection status
        let response = ConnectionStatusResponse {
            connected: false,
            robot_name: None,
            ip_address: None,
            port: None,
        };

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle GetExecutionState request.
fn handle_get_execution_state(
    mut requests: MessageReader<Request<GetExecutionState>>,
) {
    for request in requests.read() {
        info!("📋 Handling GetExecutionState");

        // TODO: Get actual execution state
        let response = ExecutionStateResponse {
            status: "idle".to_string(),
            current_line: None,
            total_lines: None,
            error: None,
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