//! Request handlers for program operations.

use bevy::prelude::*;
use bevy::ecs::message::MessageReader;
use pl3xus::managers::network_request::Request;
use pl3xus::Network;
use pl3xus_websockets::WebSocketProvider;
use pl3xus_sync::authorization::AppBatchRequestRegistrationExt;
use pl3xus_sync::RequestInvalidateExt;  // For respond_and_invalidate

use fanuc_replica_core::DatabaseResource;
use crate::database::queries;
use crate::csv_parser::parse_csv;
use crate::types::*;

// Type alias for WebSocket network provider
type WS = WebSocketProvider;

pub struct ProgramHandlerPlugin;

impl Plugin for ProgramHandlerPlugin {
    fn build(&self, app: &mut App) {
        // Register requests
        app.requests::<(
            ListPrograms,
            GetProgram,
            CreateProgram,
            DeleteProgram,
            UpdateProgramSettings,
            UploadCsv,
            AddSequence,
            RemoveSequence,
        ), WS>().register();

        // Add handler systems
        app.add_systems(Update, (
            handle_list_programs,
            handle_get_program,
            handle_create_program,
            handle_delete_program,
            handle_update_program_settings,
            handle_upload_csv,
            handle_add_sequence,
            handle_remove_sequence,
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
            .and_then(|db| {
                let conn = db.connection();
                let conn = conn.lock().unwrap();
                queries::get_program(&conn, program_id).ok()
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

        // Get sequence ID and insert instructions
        let result = db.as_ref()
            .map(|db| {
                let conn = db.connection();
                let conn = conn.lock().unwrap();

                // Get the appropriate sequence
                let seq_type = inner.sequence_type.unwrap_or(SequenceType::Main);
                let sequence_id = if seq_type == SequenceType::Main {
                    queries::get_main_sequence_id(&conn, inner.program_id)?
                        .ok_or_else(|| anyhow::anyhow!("Main sequence not found"))?
                } else {
                    // For approach/retreat, create a new sequence
                    queries::add_sequence(&conn, inner.program_id, seq_type, None, &parse_result.instructions)?
                };

                if seq_type == SequenceType::Main {
                    queries::insert_instructions(&conn, sequence_id, &parse_result.instructions)?;
                }

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

        let result = db.as_ref()
            .map(|db| {
                let conn = db.connection();
                let conn = conn.lock().unwrap();
                queries::add_sequence(
                    &conn,
                    inner.program_id,
                    inner.sequence_type,
                    inner.name.as_deref(),
                    &inner.instructions,
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

