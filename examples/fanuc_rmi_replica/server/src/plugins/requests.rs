//! Request/Response handlers plugin.
//!
//! Handles database queries like ListRobotConnections and GetRobotConfigurations.

use bevy::prelude::*;
use bevy::ecs::message::MessageReader;
use pl3xus::managers::network_request::{AppNetworkRequestMessage, Request};
use pl3xus_websockets::WebSocketProvider;
use fanuc_replica_types::*;

use crate::database::DatabaseResource;
use crate::plugins::connection::FanucRobot;

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
        app.listen_for_request_message::<UploadCsv, WebSocketProvider>();
        app.listen_for_request_message::<UnloadProgram, WebSocketProvider>();
        app.listen_for_request_message::<StartProgram, WebSocketProvider>();
        app.listen_for_request_message::<PauseProgram, WebSocketProvider>();
        app.listen_for_request_message::<ResumeProgram, WebSocketProvider>();
        app.listen_for_request_message::<StopProgram, WebSocketProvider>();

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
        app.listen_for_request_message::<GetActiveJogSettings, WebSocketProvider>();
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
            handle_upload_csv,
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
            handle_get_active_jog_settings,
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
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling CreateConfiguration for robot_connection_id={}", inner.robot_connection_id);

        let result = db.as_ref()
            .map(|db| db.create_configuration(inner))
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

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle UpdateConfiguration request - updates an existing configuration.
fn handle_update_configuration(
    mut requests: MessageReader<Request<UpdateConfiguration>>,
    db: Option<Res<DatabaseResource>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling UpdateConfiguration for id={}", inner.id);

        let result = db.as_ref()
            .map(|db| db.update_configuration(inner))
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

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle DeleteConfiguration request - deletes a configuration.
fn handle_delete_configuration(
    mut requests: MessageReader<Request<DeleteConfiguration>>,
    db: Option<Res<DatabaseResource>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling DeleteConfiguration for id={}", inner.id);

        let result = db.as_ref()
            .map(|db| db.delete_configuration(inner.id))
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

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle SetDefaultConfiguration request - sets a configuration as the default.
fn handle_set_default_configuration(
    mut requests: MessageReader<Request<SetDefaultConfiguration>>,
    db: Option<Res<DatabaseResource>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling SetDefaultConfiguration for id={}", inner.id);

        let result = db.as_ref()
            .map(|db| db.set_default_configuration(inner.id))
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

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
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

        // Convert ProgramInfo to ProgramWithLines (empty lines for list view)
        let programs_with_lines: Vec<ProgramWithLines> = programs.into_iter()
            .map(|p| ProgramWithLines {
                id: p.id,
                name: p.name,
                description: p.description,
                lines: Vec::new(),
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

/// Handle UploadCsv request - uploads CSV data to a program.
fn handle_upload_csv(
    mut requests: MessageReader<Request<UploadCsv>>,
    db: Option<Res<DatabaseResource>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling UploadCsv for program_id={}", inner.program_id);

        let result = parse_and_insert_csv(&inner.csv_content, inner.program_id, &db);

        let response = match result {
            Ok(count) => {
                info!("✅ Imported {} lines to program id={}", count, inner.program_id);
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

    Ok(count)
}

/// Handle UnloadProgram request - unloads the currently loaded program.
fn handle_unload_program(
    mut requests: MessageReader<Request<UnloadProgram>>,
) {
    for request in requests.read() {
        info!("📋 Handling UnloadProgram request");

        // TODO: Clear loaded program state from server
        // For now, just acknowledge success
        let response = UnloadProgramResponse { success: true, error: None };

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle StartProgram request - starts executing a program.
fn handle_start_program(
    mut requests: MessageReader<Request<StartProgram>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling StartProgram for program_id={}", inner.program_id);

        // TODO: Start program execution on robot
        // For now, just acknowledge success
        let response = StartProgramResponse { success: true, error: None };

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle PauseProgram request - pauses program execution.
fn handle_pause_program(
    mut requests: MessageReader<Request<PauseProgram>>,
) {
    for request in requests.read() {
        info!("📋 Handling PauseProgram request");

        // TODO: Pause program execution on robot
        // For now, just acknowledge success
        let response = PauseProgramResponse { success: true, error: None };

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle ResumeProgram request - resumes program execution.
fn handle_resume_program(
    mut requests: MessageReader<Request<ResumeProgram>>,
) {
    for request in requests.read() {
        info!("📋 Handling ResumeProgram request");

        // TODO: Resume program execution on robot
        // For now, just acknowledge success
        let response = ResumeProgramResponse { success: true, error: None };

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle StopProgram request - stops program execution.
fn handle_stop_program(
    mut requests: MessageReader<Request<StopProgram>>,
) {
    for request in requests.read() {
        info!("📋 Handling StopProgram request");

        // TODO: Stop program execution on robot
        // For now, just acknowledge success
        let response = StopProgramResponse { success: true, error: None };

        if let Err(e) = request.clone().respond(response) {
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
fn handle_write_dout(
    mut requests: MessageReader<Request<WriteDout>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling WriteDout for port {} = {}", inner.port_number, inner.port_value);

        // TODO: Write to connected robot driver when available
        // For now, just acknowledge success
        let response = DoutValueResponse {
            port_number: inner.port_number,
            port_value: inner.port_value,
            success: true,
            error: None,
        };

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
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
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling WriteAout for port {} = {}", inner.port_number, inner.port_value);

        // TODO: Write to connected robot driver when available
        // For now, just acknowledge success
        let response = AoutValueResponse {
            port_number: inner.port_number,
            port_value: inner.port_value,
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
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling WriteGout for port {} = {}", inner.port_number, inner.port_value);

        // TODO: Write to connected robot driver when available
        // For now, just acknowledge success
        let response = GoutValueResponse {
            port_number: inner.port_number,
            port_value: inner.port_value,
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

/// Handle GetActiveJogSettings request.
fn handle_get_active_jog_settings(
    mut requests: MessageReader<Request<GetActiveJogSettings>>,
    db: Option<Res<DatabaseResource>>,
) {
    for request in requests.read() {
        info!("📋 Handling GetActiveJogSettings");

        // TODO: Load from database when available
        let _ = db;
        let response = ActiveJogSettingsResponse::default();

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