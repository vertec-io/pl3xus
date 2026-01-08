//! CRUD handlers for program operations.
//!
//! This module contains handlers for:
//! - Listing programs
//! - Getting program details
//! - Creating programs
//! - Deleting programs
//! - Updating program settings

use bevy::prelude::*;
use bevy::ecs::message::MessageReader;
use pl3xus::managers::network_request::Request;
use pl3xus::Network;
use pl3xus_websockets::WebSocketProvider;
use pl3xus_sync::RequestInvalidateExt;

use robot_hmi_core::database::DatabaseResource;
use crate::database::queries;
use crate::types::*;

// Type alias for WebSocket network provider
type WS = WebSocketProvider;

pub fn handle_list_programs(
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

pub fn handle_get_program(
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

pub fn handle_create_program(
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

        if let Err(e) = request.clone().respond_and_invalidate(response, &net) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

pub fn handle_delete_program(
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

        if let Err(e) = request.clone().respond_and_invalidate(response, &net) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

pub fn handle_update_program_settings(
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

        if let Err(e) = request.clone().respond_and_invalidate(response, &net) {
            error!("Failed to send response: {:?}", e);
        }
    }
}