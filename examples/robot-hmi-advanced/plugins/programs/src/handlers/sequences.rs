//! Sequence handlers for program operations.
//!
//! This module contains handlers for:
//! - Uploading CSV data
//! - Adding sequences
//! - Updating sequence instructions
//! - Removing sequences

use bevy::prelude::*;
use bevy::ecs::message::MessageReader;
use pl3xus::managers::network_request::Request;
use pl3xus::Network;
use pl3xus_websockets::WebSocketProvider;
use pl3xus_sync::RequestInvalidateExt;

use robot_hmi_core::database::DatabaseResource;
use crate::database::queries;
use crate::csv_parser::parse_csv;
use crate::types::*;

// Type alias for WebSocket network provider
type WS = WebSocketProvider;

pub fn handle_upload_csv(
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

        if let Err(e) = request.clone().respond_and_invalidate(response, &net) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

pub fn handle_add_sequence(
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

        if let Err(e) = request.clone().respond_and_invalidate(response, &net) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

pub fn handle_update_sequence_instructions(
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

        if let Err(e) = request.clone().respond_and_invalidate(response, &net) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

pub fn handle_remove_sequence(
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

        if let Err(e) = request.clone().respond_and_invalidate(response, &net) {
            error!("Failed to send response: {:?}", e);
        }
    }
}