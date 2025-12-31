//! Request handlers for program operations.

use bevy::prelude::*;
use bevy::ecs::message::MessageReader;
use pl3xus::managers::network_request::Request;
use pl3xus::Network;
use pl3xus_websockets::WebSocketProvider;
use pl3xus_sync::AppBatchRequestRegistrationExt;
use pl3xus_sync::RequestInvalidateExt;  // For respond_and_invalidate
use pl3xus_sync::AuthorizedRequest;

use fanuc_replica_core::{ActiveSystem, DatabaseResource};
use fanuc_replica_execution::{
    BufferDisplayData, BufferLineDisplay, BufferState, ExecutionCoordinator,
    ExecutionPoint, ExecutionState, MotionCommand, MotionType, SourceType, SystemState,
    ToolpathBuffer,
};
use fanuc_replica_robotics::{FrameId, RobotPose};
use crate::database::queries;
use crate::csv_parser::parse_csv;
use crate::{types::*, ProgramActions};

// Type alias for WebSocket network provider
type WS = WebSocketProvider;

pub struct ProgramHandlerPlugin;

impl Plugin for ProgramHandlerPlugin {
    fn build(&self, app: &mut App) {
        // Register CRUD requests (non-targeted, no authorization needed)
        app.requests::<(
            ListPrograms,
            GetProgram,
            CreateProgram,
            DeleteProgram,
            UpdateProgramSettings,
            UploadCsv,
            AddSequence,
            UpdateSequenceInstructions,
            RemoveSequence,
        ), WS>().register();

        // Register Load/Unload as targeted requests (require entity control)
        // These target the ActiveSystem entity and need authorization
        app.requests::<(
            Load,
            Unload,
        ), WS>()
            .targeted()
            .with_default_entity_policy()
            .with_error_response();

        // Add CRUD handler systems
        app.add_systems(Update, (
            handle_list_programs,
            handle_get_program,
            handle_create_program,
            handle_delete_program,
            handle_update_program_settings,
            handle_upload_csv,
            handle_add_sequence,
            handle_update_sequence_instructions,
            handle_remove_sequence,
        ));

        // Add Load/Unload handler systems
        app.add_systems(Update, (
            handle_load,
            handle_unload,
        ));
    }
}

fn handle_list_programs(
    mut requests: MessageReader<Request<ListPrograms>>,
    db: Option<Res<DatabaseResource>>,
) {
    for request in requests.read() {
        info!("📋 Handling ListPrograms");

        let programs = db.as_ref()
            .and_then(|db| {
                let conn = db.connection();
                let conn = conn.lock().unwrap();
                queries::list_programs(&conn).ok()
            })
            .unwrap_or_default();

        info!("📤 Responding with {} programs", programs.len());
        let _ = request.clone().respond(ListProgramsResponse { programs });
    }
}

fn handle_get_program(
    mut requests: MessageReader<Request<GetProgram>>,
    db: Option<Res<DatabaseResource>>,
) {
    for request in requests.read() {
        let program_id = request.get_request().program_id;
        info!("📋 Handling GetProgram for id={}", program_id);

        let program = db.as_ref()
            .map(|db| {
                let conn = db.connection();
                let conn = conn.lock().unwrap();
                match queries::get_program(&conn, program_id) {
                    Ok(p) => p,
                    Err(e) => {
                        tracing::error!("❌ Error getting program {}: {:?}", program_id, e);
                        None
                    }
                }
            })
            .flatten();

        info!("📤 Responding with program: {:?}", program.as_ref().map(|p| &p.name));
        let _ = request.clone().respond(GetProgramResponse { program });
    }
}

fn handle_create_program(
    mut requests: MessageReader<Request<CreateProgram>>,
    db: Option<Res<DatabaseResource>>,
    net: Res<Network<WS>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling CreateProgram: '{}'", inner.name);

        let result = db.as_ref()
            .map(|db| {
                let conn = db.connection();
                let conn = conn.lock().unwrap();
                queries::create_program(&conn, &inner.name, inner.description.as_deref())
            })
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

        // respond_and_invalidate automatically broadcasts invalidations on success
        if let Err(e) = request.clone().respond_and_invalidate(response, &net) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

fn handle_delete_program(
    mut requests: MessageReader<Request<DeleteProgram>>,
    db: Option<Res<DatabaseResource>>,
    net: Res<Network<WS>>,
) {
    for request in requests.read() {
        let program_id = request.get_request().program_id;
        info!("📋 Handling DeleteProgram id={}", program_id);

        let result = db.as_ref()
            .map(|db| {
                let conn = db.connection();
                let conn = conn.lock().unwrap();
                queries::delete_program(&conn, program_id)
            })
            .unwrap_or(Err(anyhow::anyhow!("Database not available")));

        let response = match result {
            Ok(()) => {
                info!("✅ Deleted program id={}", program_id);
                DeleteProgramResponse { success: true, error: None }
            }
            Err(e) => {
                error!("❌ Failed to delete program: {}", e);
                DeleteProgramResponse { success: false, error: Some(e.to_string()) }
            }
        };

        // respond_and_invalidate automatically broadcasts invalidations on success
        if let Err(e) = request.clone().respond_and_invalidate(response, &net) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

fn handle_update_program_settings(
    mut requests: MessageReader<Request<UpdateProgramSettings>>,
    db: Option<Res<DatabaseResource>>,
    net: Res<Network<WS>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling UpdateProgramSettings id={}", inner.program_id);

        let result = db.as_ref()
            .map(|db| {
                let conn = db.connection();
                let conn = conn.lock().unwrap();
                queries::update_program_settings(
                    &conn,
                    inner.program_id,
                    inner.name.as_deref(),
                    inner.description.as_deref(),
                    inner.default_speed,
                    inner.default_term_type.as_deref(),
                    inner.default_term_value,
                    inner.move_speed,
                )
            })
            .unwrap_or(Err(anyhow::anyhow!("Database not available")));

        let response = match result {
            Ok(()) => {
                info!("✅ Updated program settings id={}", inner.program_id);
                UpdateProgramSettingsResponse { success: true, error: None }
            }
            Err(e) => {
                error!("❌ Failed to update program settings: {}", e);
                UpdateProgramSettingsResponse { success: false, error: Some(e.to_string()) }
            }
        };

        // respond_and_invalidate automatically broadcasts invalidations on success
        if let Err(e) = request.clone().respond_and_invalidate(response, &net) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

fn handle_upload_csv(
    mut requests: MessageReader<Request<UploadCsv>>,
    db: Option<Res<DatabaseResource>>,
    net: Res<Network<WS>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling UploadCsv for program id={}", inner.program_id);

        // Parse CSV
        let parse_result = match parse_csv(&inner.csv_content) {
            Ok(result) => result,
            Err(e) => {
                error!("❌ CSV parse error: {}", e);
                let _ = request.clone().respond(UploadCsvResponse {
                    success: false,
                    lines_imported: None,
                    warnings: vec![],
                    error: Some(e.to_string()),
                });
                continue;
            }
        };

        let warnings: Vec<String> = parse_result.warnings.iter()
            .map(|w| format!("Line {}: {}", w.line, w.message))
            .collect();

        // Get sequence ID and append instructions
        let result = db.as_ref()
            .map(|db| {
                let conn = db.connection();
                let conn = conn.lock().unwrap();

                // Determine which sequence to append to
                let sequence_id = if let Some(seq_id) = inner.sequence_id {
                    // Use the provided sequence ID
                    seq_id
                } else {
                    // Fall back to sequence_type logic (for backwards compatibility)
                    let seq_type = inner.sequence_type.unwrap_or(SequenceType::Main);
                    if seq_type == SequenceType::Main {
                        queries::get_main_sequence_id(&conn, inner.program_id)?
                            .ok_or_else(|| anyhow::anyhow!("Main sequence not found"))?
                    } else {
                        // For approach/retreat without sequence_id, create a new sequence
                        queries::add_sequence(&conn, inner.program_id, seq_type, None, &parse_result.instructions)?
                    }
                };

                // Append instructions to the sequence
                queries::append_instructions(&conn, sequence_id, &parse_result.instructions)?;

                Ok::<_, anyhow::Error>(parse_result.instructions.len())
            })
            .unwrap_or(Err(anyhow::anyhow!("Database not available")));

        let response = match result {
            Ok(count) => {
                info!("✅ Imported {} lines", count);
                UploadCsvResponse {
                    success: true,
                    lines_imported: Some(count as i32),
                    warnings,
                    error: None,
                }
            }
            Err(e) => {
                error!("❌ Failed to import CSV: {}", e);
                UploadCsvResponse {
                    success: false,
                    lines_imported: None,
                    warnings,
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

fn handle_add_sequence(
    mut requests: MessageReader<Request<AddSequence>>,
    db: Option<Res<DatabaseResource>>,
    net: Res<Network<WS>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling AddSequence for program id={}", inner.program_id);

        // Parse CSV if provided, otherwise use the instructions field
        let instructions = if let Some(ref csv_content) = inner.csv_content {
            match parse_csv(csv_content) {
                Ok(parse_result) => parse_result.instructions,
                Err(e) => {
                    error!("❌ CSV parse error: {}", e);
                    let _ = request.clone().respond(AddSequenceResponse {
                        success: false,
                        sequence_id: None,
                        error: Some(e.to_string()),
                    });
                    continue;
                }
            }
        } else {
            inner.instructions.clone()
        };

        let result = db.as_ref()
            .map(|db| {
                let conn = db.connection();
                let conn = conn.lock().unwrap();
                queries::add_sequence(
                    &conn,
                    inner.program_id,
                    inner.sequence_type,
                    inner.name.as_deref(),
                    &instructions,
                )
            })
            .unwrap_or(Err(anyhow::anyhow!("Database not available")));

        let response = match result {
            Ok(sequence_id) => {
                info!("✅ Added sequence id={}", sequence_id);
                AddSequenceResponse {
                    success: true,
                    sequence_id: Some(sequence_id),
                    error: None,
                }
            }
            Err(e) => {
                error!("❌ Failed to add sequence: {}", e);
                AddSequenceResponse {
                    success: false,
                    sequence_id: None,
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

fn handle_update_sequence_instructions(
    mut requests: MessageReader<Request<UpdateSequenceInstructions>>,
    db: Option<Res<DatabaseResource>>,
    net: Res<Network<WS>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling UpdateSequenceInstructions for sequence id={}", inner.sequence_id);

        let result = db.as_ref()
            .map(|db| {
                let conn = db.connection();
                let conn = conn.lock().unwrap();
                queries::insert_instructions(&conn, inner.sequence_id, &inner.instructions)
            })
            .unwrap_or(Err(anyhow::anyhow!("Database not available")));

        let response = match result {
            Ok(()) => {
                info!("✅ Updated {} instructions for sequence id={}", inner.instructions.len(), inner.sequence_id);
                UpdateSequenceInstructionsResponse { success: true, error: None }
            }
            Err(e) => {
                error!("❌ Failed to update sequence instructions: {}", e);
                UpdateSequenceInstructionsResponse { success: false, error: Some(e.to_string()) }
            }
        };

        // respond_and_invalidate automatically broadcasts invalidations on success
        if let Err(e) = request.clone().respond_and_invalidate(response, &net) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

fn handle_remove_sequence(
    mut requests: MessageReader<Request<RemoveSequence>>,
    db: Option<Res<DatabaseResource>>,
    net: Res<Network<WS>>,
) {
    for request in requests.read() {
        let sequence_id = request.get_request().sequence_id;
        info!("📋 Handling RemoveSequence id={}", sequence_id);

        let result = db.as_ref()
            .map(|db| {
                let conn = db.connection();
                let conn = conn.lock().unwrap();
                queries::remove_sequence(&conn, sequence_id)
            })
            .unwrap_or(Err(anyhow::anyhow!("Database not available")));

        let response = match result {
            Ok(()) => {
                info!("✅ Removed sequence id={}", sequence_id);
                RemoveSequenceResponse { success: true, error: None }
            }
            Err(e) => {
                error!("❌ Failed to remove sequence: {}", e);
                RemoveSequenceResponse { success: false, error: Some(e.to_string()) }
            }
        };

        // respond_and_invalidate automatically broadcasts invalidations on success
        if let Err(e) = request.clone().respond_and_invalidate(response, &net) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

// ============================================================================
// Load/Unload Handlers
// ============================================================================

/// Handle Load request - loads a program into the execution buffer.
///
/// This is the static program loader that:
/// 1. Fetches program from database
/// 2. Converts instructions to ExecutionPoints
/// 3. Populates ToolpathBuffer
/// 4. Updates BufferDisplayData for UI
/// 5. Updates ExecutionState with source info
fn handle_load(
    mut commands: Commands,
    mut requests: MessageReader<AuthorizedRequest<Load>>,
    db: Option<Res<DatabaseResource>>,
    system_query: Query<Entity, With<ActiveSystem>>,
    coordinator_query: Query<&ExecutionCoordinator, With<ActiveSystem>>,
    mut execution_states: Query<&mut ExecutionState, With<ActiveSystem>>,
    _buffer_displays: Query<&mut BufferDisplayData, With<ActiveSystem>>,
) {
    for request in requests.read() {
        let request = request.clone();
        let program_id = request.get_request().program_id;
        info!("📋 Handling Load request for program {}", program_id);

        // Get system entity
        let Ok(system_entity) = system_query.single() else {
            let _ = request.respond(LoadResponse {
                success: false,
                program: None,
                error: Some("System not ready".to_string()),
            });
            continue;
        };

        // Check if something is already loaded
        if coordinator_query.get(system_entity).is_ok() {
            let _ = request.respond(LoadResponse {
                success: false,
                program: None,
                error: Some("A program is already loaded. Unload first.".to_string()),
            });
            continue;
        }

        // Get database
        let Some(db_res) = db.as_ref() else {
            let _ = request.respond(LoadResponse {
                success: false,
                program: None,
                error: Some("Database not available".to_string()),
            });
            continue;
        };

        let conn = db_res.connection();
        let conn = conn.lock().unwrap();

        // Fetch program from database
        match queries::get_program(&conn, program_id) {
            Ok(Some(program_detail)) => {
                // Build defaults for missing instruction fields
                let default_speed = program_detail.default_speed.unwrap_or(100.0);
                let default_term_type = program_detail.default_term_type.clone()
                    .unwrap_or_else(|| "FINE".to_string());

                // Count total instructions
                let approach_count: usize = program_detail.approach_sequences.iter()
                    .map(|s| s.instructions.len()).sum();
                let main_count: usize = program_detail.main_sequences.iter()
                    .map(|s| s.instructions.len()).sum();
                let retreat_count: usize = program_detail.retreat_sequences.iter()
                    .map(|s| s.instructions.len()).sum();
                let total_points = approach_count + main_count + retreat_count;

                // Create static buffer with known total
                let mut toolpath_buffer = ToolpathBuffer::new_static(total_points as u32);
                let mut display_data = BufferDisplayData::new();
                let mut lines: Vec<ProgramLineInfo> = Vec::with_capacity(total_points);
                let mut point_index: u32 = 0;

                // Helper to process an instruction
                let mut process_instruction = |instruction: &Instruction, seq_name: Option<&str>| {
                    let speed = instruction.speed.unwrap_or(default_speed);
                    let term_type = instruction.term_type.clone()
                        .unwrap_or_else(|| default_term_type.clone());

                    // Create ExecutionPoint with RobotPose
                    let pose = RobotPose::from_xyz_wpr(
                        instruction.x,
                        instruction.y,
                        instruction.z,
                        instruction.w.unwrap_or(0.0),
                        instruction.p.unwrap_or(0.0),
                        instruction.r.unwrap_or(0.0),
                        FrameId::World,
                    );

                    let motion = MotionCommand {
                        speed: speed as f32,
                        motion_type: MotionType::Linear,
                        blend_radius: if term_type == "FINE" { 0.0 } else { 5.0 },
                    };

                    let exec_point = ExecutionPoint::new(point_index, pose)
                        .with_motion(motion);
                    toolpath_buffer.push(exec_point);

                    // Add to display data
                    display_data.push_line(BufferLineDisplay {
                        index: point_index as usize,
                        line_type: "Move".to_string(),
                        description: format!("({:.1}, {:.1}, {:.1})",
                            instruction.x, instruction.y, instruction.z),
                        sequence_name: seq_name.map(|s| s.to_string()),
                        source_line: Some(instruction.line_number as usize),
                        x: instruction.x,
                        y: instruction.y,
                        z: instruction.z,
                        w: instruction.w.unwrap_or(0.0),
                        p: instruction.p.unwrap_or(0.0),
                        r: instruction.r.unwrap_or(0.0),
                        speed,
                        term_type: term_type.clone(),
                    });

                    // Add to program lines for response
                    lines.push(ProgramLineInfo {
                        x: instruction.x,
                        y: instruction.y,
                        z: instruction.z,
                        w: instruction.w.unwrap_or(0.0),
                        p: instruction.p.unwrap_or(0.0),
                        r: instruction.r.unwrap_or(0.0),
                        speed,
                        term_type,
                    });

                    point_index += 1;
                };

                // Process approach sequences
                for seq in &program_detail.approach_sequences {
                    let seq_name = seq.name.as_deref().unwrap_or("Approach");
                    for instruction in &seq.instructions {
                        process_instruction(instruction, Some(seq_name));
                    }
                }

                // Process main sequences (concatenate in order)
                for seq in &program_detail.main_sequences {
                    let seq_name = seq.name.as_deref().unwrap_or("Main");
                    for instruction in &seq.instructions {
                        process_instruction(instruction, Some(seq_name));
                    }
                }

                // Process retreat sequences
                for seq in &program_detail.retreat_sequences {
                    let seq_name = seq.name.as_deref().unwrap_or("Retreat");
                    for instruction in &seq.instructions {
                        process_instruction(instruction, Some(seq_name));
                    }
                }

                info!("📋 Program '{}' loaded: {} points in buffer",
                    &program_detail.name, toolpath_buffer.len());

                // Add execution components to System entity
                commands.entity(system_entity).insert((
                    ExecutionCoordinator::with_name(
                        format!("program_{}", program_detail.id),
                        program_detail.name.clone(),
                    ),
                    toolpath_buffer,
                    BufferState::Ready,
                    display_data,
                ));

                // Update ExecutionState
                if let Ok(mut exec_state) = execution_states.single_mut() {
                    exec_state.state = SystemState::Ready;
                    exec_state.source_type = SourceType::StaticProgram;
                    exec_state.source_name = Some(program_detail.name.clone());
                    exec_state.source_id = Some(program_detail.id);
                    exec_state.current_index = 0;
                    exec_state.total_points = Some(total_points);
                    exec_state.points_executed = 0;
                    exec_state.update_execution_actions();
                    info!("📡 ExecutionState updated: source='{}', {} points",
                        program_detail.name, total_points);
                }

                // ProgramActions will be synced by sync_program_actions system
                // based on the ExecutionState we just updated

                // Build response
                let program_with_lines = ProgramWithLines {
                    id: program_detail.id,
                    name: program_detail.name.clone(),
                    description: program_detail.description.clone(),
                    lines,
                    approach_lines: Vec::new(),
                    retreat_lines: Vec::new(),
                };

                let _ = request.respond(LoadResponse {
                    success: true,
                    program: Some(program_with_lines),
                    error: None,
                });
            }
            Ok(None) => {
                let _ = request.respond(LoadResponse {
                    success: false,
                    program: None,
                    error: Some(format!("Program {} not found", program_id)),
                });
            }
            Err(e) => {
                error!("❌ Database error loading program: {}", e);
                let _ = request.respond(LoadResponse {
                    success: false,
                    program: None,
                    error: Some(format!("Database error: {}", e)),
                });
            }
        }
    }
}

/// Handle Unload request - unloads the currently loaded program.
fn handle_unload(
    mut commands: Commands,
    mut requests: MessageReader<AuthorizedRequest<Unload>>,
    system_query: Query<Entity, With<ActiveSystem>>,
    coordinator_query: Query<&ExecutionCoordinator, With<ActiveSystem>>,
    mut execution_states: Query<&mut ExecutionState, With<ActiveSystem>>,
    mut buffer_displays: Query<&mut BufferDisplayData, With<ActiveSystem>>,
) {
    for request in requests.read() {
        let request = request.clone();
        info!("📋 Handling Unload request");

        // Get system entity
        let Ok(system_entity) = system_query.single() else {
            let _ = request.respond(UnloadResponse {
                success: false,
                error: Some("System not ready".to_string()),
            });
            continue;
        };

        // Check if a program is loaded
        if coordinator_query.get(system_entity).is_err() {
            let _ = request.respond(UnloadResponse {
                success: false,
                error: Some("No program loaded".to_string()),
            });
            continue;
        };

        // Remove execution components (but not synced components like ExecutionState/BufferDisplayData)
        commands.entity(system_entity).remove::<ExecutionCoordinator>();
        commands.entity(system_entity).remove::<ToolpathBuffer>();
        commands.entity(system_entity).remove::<BufferState>();
        info!("📦 Removed execution components from System entity");

        // Reset ExecutionState
        if let Ok(mut exec_state) = execution_states.single_mut() {
            *exec_state = ExecutionState::no_source();
            info!("📡 ExecutionState reset to NoSource");
        }

        // ProgramActions will be synced by sync_program_actions system
        // based on the ExecutionState we just reset

        // Clear BufferDisplayData
        if let Ok(mut buffer_display) = buffer_displays.single_mut() {
            buffer_display.clear();
            info!("📡 BufferDisplayData cleared");
        }

        let _ = request.respond(UnloadResponse {
            success: true,
            error: None,
        });
    }
}
