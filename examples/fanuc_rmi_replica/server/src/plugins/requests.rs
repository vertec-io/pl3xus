//! Request/Response handlers plugin.
//!
//! Handles database queries like ListRobotConnections and GetRobotConfigurations.

use bevy::prelude::*;
use bevy::ecs::message::MessageReader;
use pl3xus::managers::network_request::{AppNetworkRequestMessage, Request};
use pl3xus_websockets::WebSocketProvider;
use fanuc_replica_types::*;

use bevy_tokio_tasks::TokioTasksRuntime;
use crate::database::DatabaseResource;
use crate::plugins::connection::{FanucRobot, RmiDriver, RobotConnectionState};
use crate::plugins::execution::ProgramExecutor;
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

/// Handle UpdateProgramSettings request - updates program start/end positions and settings.
fn handle_update_program_settings(
    mut requests: MessageReader<Request<UpdateProgramSettings>>,
    db: Option<Res<DatabaseResource>>,
) {
    for request in requests.read() {
        let req = request.get_request();
        info!("📋 Handling UpdateProgramSettings for program_id={}", req.program_id);

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
/// IMPORTANT: Requires robot connection AND client control.
/// Updates the server-side ExecutionState so all clients see the loaded program.
fn handle_load_program(
    mut requests: MessageReader<Request<LoadProgram>>,
    db: Option<Res<DatabaseResource>>,
    robots: Query<(Entity, &RobotConnectionState, Option<&pl3xus_sync::control::EntityControl>), With<FanucRobot>>,
    mut execution_states: Query<&mut ExecutionState>,
    mut executor: ResMut<ProgramExecutor>,
) {
    for request in requests.read() {
        let program_id = request.get_request().program_id;
        let client_id = request.source();
        info!("📋 Handling LoadProgram request for program {} from {:?}", program_id, client_id);

        // Check robot connection and control
        let (robot_entity, is_connected, has_control) = match robots.single() {
            Ok((entity, state, control)) => {
                let connected = *state == RobotConnectionState::Connected;
                let has_ctrl = control.map(|c| c.client_id == *client_id).unwrap_or(true);
                (Some(entity), connected, has_ctrl)
            }
            Err(_) => (None, false, false),
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
                error: Some("You do not have control of the robot".to_string()),
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
            Ok(Some(program)) => {
                // Convert ProgramDetail to ProgramWithLines
                let lines: Vec<ProgramLineInfo> = program.instructions.iter()
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

                // Update ProgramExecutor state (server-side)
                executor.loaded_program_id = Some(program.id);
                executor.loaded_program_name = Some(program.name.clone());
                executor.total_instructions = lines.len();
                executor.completed_line = 0;
                executor.running = false;
                executor.paused = false;

                // Update ExecutionState component (synced to all clients)
                if let Some(entity) = robot_entity {
                    if let Ok(mut exec_state) = execution_states.get_mut(entity) {
                        exec_state.loaded_program_id = Some(program.id);
                        exec_state.loaded_program_name = Some(program.name.clone());
                        exec_state.running = false;
                        exec_state.paused = false;
                        exec_state.current_line = 0;
                        exec_state.total_lines = lines.len();
                        exec_state.program_lines = lines.clone();
                        info!("📡 Updated ExecutionState: program '{}' with {} lines synced to all clients",
                            program.name, lines.len());
                    }
                }

                let program_with_lines = ProgramWithLines {
                    id: program.id,
                    name: program.name.clone(),
                    description: program.description.clone(),
                    lines,
                };
                info!("✅ Loaded program '{}' with {} lines", program.name, program_with_lines.lines.len());
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
fn handle_unload_program(
    mut requests: MessageReader<Request<UnloadProgram>>,
    mut executor: ResMut<ProgramExecutor>,
    mut execution_states: Query<&mut ExecutionState>,
) {
    for request in requests.read() {
        info!("📋 Handling UnloadProgram request");

        // Reset executor state
        executor.reset();

        // Reset ExecutionState on all robot entities (synced to clients)
        for mut exec_state in execution_states.iter_mut() {
            exec_state.loaded_program_id = None;
            exec_state.loaded_program_name = None;
            exec_state.running = false;
            exec_state.paused = false;
            exec_state.current_line = 0;
            exec_state.total_lines = 0;
            exec_state.program_lines.clear();
        }

        info!("📡 Unloaded program - ExecutionState cleared and synced to all clients");
        let response = UnloadProgramResponse { success: true, error: None };

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle StartProgram request - starts executing a program.
fn handle_start_program(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut requests: MessageReader<Request<StartProgram>>,
    db: Option<Res<DatabaseResource>>,
    mut executor: ResMut<ProgramExecutor>,
    robots: Query<(&RmiDriver, &RobotConnectionState), With<FanucRobot>>,
) {
    // Enter the Tokio runtime context so send_packet can use tokio::spawn
    let _guard = tokio_runtime.runtime().enter();

    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling StartProgram for program_id={}", inner.program_id);

        // Get database
        let db = match &db {
            Some(db) => db,
            None => {
                let response = StartProgramResponse { success: false, error: Some("Database not available".to_string()) };
                let _ = request.clone().respond(response);
                continue;
            }
        };

        // Get program from database
        let program = match db.get_program(inner.program_id) {
            Ok(Some(prog)) => prog,
            Ok(None) => {
                let response = StartProgramResponse { success: false, error: Some("Program not found".to_string()) };
                let _ = request.clone().respond(response);
                continue;
            }
            Err(e) => {
                let response = StartProgramResponse { success: false, error: Some(format!("Database error: {}", e)) };
                let _ = request.clone().respond(response);
                continue;
            }
        };

        if program.instructions.is_empty() {
            let response = StartProgramResponse { success: false, error: Some("Program has no instructions".to_string()) };
            let _ = request.clone().respond(response);
            continue;
        }

        // Check if robot is connected
        let mut robot_driver = None;
        for (driver, state) in robots.iter() {
            if *state == RobotConnectionState::Connected {
                robot_driver = Some(driver);
                break;
            }
        }

        let driver = match robot_driver {
            Some(d) => d,
            None => {
                let response = StartProgramResponse { success: false, error: Some("Robot not connected".to_string()) };
                let _ = request.clone().respond(response);
                continue;
            }
        };

        // Reset executor and load program
        executor.reset();
        executor.loaded_program_id = Some(inner.program_id);
        executor.loaded_program_name = Some(program.name.clone());

        // Set defaults from program
        executor.defaults.w = program.default_w;
        executor.defaults.p = program.default_p;
        executor.defaults.r = program.default_r;
        executor.defaults.speed = program.default_speed.unwrap_or(100.0);
        executor.defaults.term_type = program.default_term_type.clone();
        executor.defaults.term_value = program.default_term_value;
        executor.defaults.uframe = program.default_uframe;
        executor.defaults.utool = program.default_utool;

        // Build pending queue with all instructions
        let total = program.instructions.len();
        let has_retreat = program.end_x.is_some() && program.end_y.is_some() && program.end_z.is_some();
        let has_approach = program.start_x.is_some() && program.start_y.is_some() && program.start_z.is_some();

        // Add approach move if defined
        if let (Some(start_x), Some(start_y), Some(start_z)) = (program.start_x, program.start_y, program.start_z) {
            let speed = program.move_speed.unwrap_or(100.0);
            let packet = executor.build_approach_retreat_packet(
                start_x, start_y, start_z,
                program.start_w, program.start_p, program.start_r,
                0, speed, false,
            );
            executor.pending_queue.push_back((0, packet));
            info!("Added approach move to ({:.2}, {:.2}, {:.2})", start_x, start_y, start_z);
        }

        // Add program instructions
        for (i, instr) in program.instructions.iter().enumerate() {
            let line_number = i + 1;
            let is_last_overall = !has_retreat && (i == total - 1);
            let packet = executor.build_motion_packet(instr, is_last_overall);
            executor.pending_queue.push_back((line_number, packet));
        }

        // Add retreat move if defined
        if let (Some(end_x), Some(end_y), Some(end_z)) = (program.end_x, program.end_y, program.end_z) {
            let speed = program.move_speed.unwrap_or(100.0);
            let packet = executor.build_approach_retreat_packet(
                end_x, end_y, end_z,
                program.end_w, program.end_p, program.end_r,
                total + 1, speed, true,
            );
            executor.pending_queue.push_back((total + 1, packet));
            info!("Added retreat move to ({:.2}, {:.2}, {:.2})", end_x, end_y, end_z);
        }

        // Calculate total lines
        executor.total_instructions = total + (if has_approach { 1 } else { 0 }) + (if has_retreat { 1 } else { 0 });
        executor.running = true;
        executor.paused = false;

        // Send initial batch of instructions
        let initial_batch = executor.get_next_batch();
        for (line_number, packet) in initial_batch {
            match driver.0.send_packet(packet, PacketPriority::Standard) {
                Ok(request_id) => {
                    // Record by request_id - will be mapped to sequence_id when SentInstructionInfo arrives
                    executor.record_sent(request_id, line_number);
                    info!("📤 Sent instruction {} (request_id {})", line_number, request_id);
                }
                Err(e) => {
                    error!("Failed to send instruction {}: {}", line_number, e);
                    executor.reset();
                    let response = StartProgramResponse { success: false, error: Some(format!("Failed to send instruction: {}", e)) };
                    let _ = request.clone().respond(response);
                    continue;
                }
            }
        }

        info!("Started program {} with {} instructions", program.name, executor.total_instructions);
        let response = StartProgramResponse { success: true, error: None };
        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle PauseProgram request - pauses program execution.
fn handle_pause_program(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut requests: MessageReader<Request<PauseProgram>>,
    mut executor: ResMut<ProgramExecutor>,
    robots: Query<(&RmiDriver, &RobotConnectionState), With<FanucRobot>>,
) {
    // Enter the Tokio runtime context so send_packet can use tokio::spawn
    let _guard = tokio_runtime.runtime().enter();

    for request in requests.read() {
        info!("📋 Handling PauseProgram request");

        if !executor.running {
            let response = PauseProgramResponse { success: false, error: Some("No program running".to_string()) };
            let _ = request.clone().respond(response);
            continue;
        }

        if executor.paused {
            let response = PauseProgramResponse { success: false, error: Some("Program already paused".to_string()) };
            let _ = request.clone().respond(response);
            continue;
        }

        // Send pause command to robot
        for (driver, state) in robots.iter() {
            if *state == RobotConnectionState::Connected {
                // Send FRC_Pause packet (unit variant)
                let pause_packet = fanuc_rmi::packets::SendPacket::Command(
                    fanuc_rmi::packets::Command::FrcPause
                );
                if let Err(e) = driver.0.send_packet(pause_packet, PacketPriority::High) {
                    error!("Failed to send pause command: {}", e);
                    let response = PauseProgramResponse { success: false, error: Some(format!("Failed to pause: {}", e)) };
                    let _ = request.clone().respond(response);
                    continue;
                }
                break;
            }
        }

        executor.paused = true;
        info!("Program paused");
        let response = PauseProgramResponse { success: true, error: None };
        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle ResumeProgram request - resumes program execution.
fn handle_resume_program(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut requests: MessageReader<Request<ResumeProgram>>,
    mut executor: ResMut<ProgramExecutor>,
    robots: Query<(&RmiDriver, &RobotConnectionState), With<FanucRobot>>,
) {
    // Enter the Tokio runtime context so send_packet can use tokio::spawn
    let _guard = tokio_runtime.runtime().enter();

    for request in requests.read() {
        info!("📋 Handling ResumeProgram request");

        if !executor.running {
            let response = ResumeProgramResponse { success: false, error: Some("No program running".to_string()) };
            let _ = request.clone().respond(response);
            continue;
        }

        if !executor.paused {
            let response = ResumeProgramResponse { success: false, error: Some("Program not paused".to_string()) };
            let _ = request.clone().respond(response);
            continue;
        }

        // Send continue command to robot
        for (driver, state) in robots.iter() {
            if *state == RobotConnectionState::Connected {
                // Send FRC_Continue packet (unit variant)
                let continue_packet = fanuc_rmi::packets::SendPacket::Command(
                    fanuc_rmi::packets::Command::FrcContinue
                );
                if let Err(e) = driver.0.send_packet(continue_packet, PacketPriority::High) {
                    error!("Failed to send continue command: {}", e);
                    let response = ResumeProgramResponse { success: false, error: Some(format!("Failed to resume: {}", e)) };
                    let _ = request.clone().respond(response);
                    continue;
                }
                break;
            }
        }

        executor.paused = false;
        info!("Program resumed");
        let response = ResumeProgramResponse { success: true, error: None };
        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle StopProgram request - stops program execution.
fn handle_stop_program(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut requests: MessageReader<Request<StopProgram>>,
    mut executor: ResMut<ProgramExecutor>,
    robots: Query<(&RmiDriver, &RobotConnectionState), With<FanucRobot>>,
) {
    // Enter the Tokio runtime context so send_packet can use tokio::spawn
    let _guard = tokio_runtime.runtime().enter();

    for request in requests.read() {
        info!("📋 Handling StopProgram request");

        if !executor.running {
            let response = StopProgramResponse { success: false, error: Some("No program running".to_string()) };
            let _ = request.clone().respond(response);
            continue;
        }

        // Send abort command to robot
        for (driver, state) in robots.iter() {
            if *state == RobotConnectionState::Connected {
                // Send FRC_Abort packet (unit variant)
                let abort_packet = fanuc_rmi::packets::SendPacket::Command(
                    fanuc_rmi::packets::Command::FrcAbort
                );
                if let Err(e) = driver.0.send_packet(abort_packet, PacketPriority::High) {
                    error!("Failed to send abort command: {}", e);
                    // Continue anyway to reset executor
                }
                break;
            }
        }

        let program_name = executor.loaded_program_name.clone().unwrap_or_default();
        executor.reset();
        info!("Program {} stopped", program_name);
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
    mut io_query: Query<&mut IoStatus, With<FanucRobot>>,
    driver_query: Query<&RmiDriver, With<FanucRobot>>,
    runtime: Option<Res<bevy_tokio_tasks::TokioTasksRuntime>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        let port = inner.port_number;
        let value = inner.port_value;
        info!("📋 Handling WriteDout for port {} = {}", port, value);

        // Update the IoStatus component (synced to all clients)
        if let Ok(mut io_status) = io_query.single_mut() {
            // Calculate word and bit index (1-based port numbers)
            let word_index = (port as usize - 1) / 16;
            let bit_index = (port as usize - 1) % 16;

            // Ensure vector is large enough
            while io_status.digital_outputs.len() <= word_index {
                io_status.digital_outputs.push(0);
            }

            // Set or clear the bit
            if value {
                io_status.digital_outputs[word_index] |= 1 << bit_index;
            } else {
                io_status.digital_outputs[word_index] &= !(1 << bit_index);
            }
            info!("✅ Updated IoStatus DOUT[{}] = {}", port, value);
        }

        // Send command to robot driver (if connected)
        if let (Ok(driver), Some(runtime)) = (driver_query.single(), runtime.as_ref()) {
            let driver = driver.0.clone();
            runtime.spawn_background_task(move |_ctx| async move {
                use fanuc_rmi::packets::{SendPacket, Command};
                use fanuc_rmi::commands::FrcWriteDOUT;
                let packet = SendPacket::Command(Command::FrcWriteDOUT(FrcWriteDOUT {
                    port_number: port,
                    port_value: if value { 1 } else { 0 },
                }));
                if let Err(e) = driver.send_packet(packet, PacketPriority::Standard) {
                    bevy::log::error!("Failed to send WriteDout to robot: {}", e);
                }
            });
        }

        let response = DoutValueResponse {
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