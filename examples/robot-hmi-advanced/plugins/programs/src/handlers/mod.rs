//! Request handlers for program operations.
//!
//! This module contains:
//! - CRUD handlers for programs
//! - Sequence handlers for CSV upload and sequence management
//! - Execution handlers for loading/unloading programs

mod crud;
mod execution;
mod sequences;

use bevy::prelude::*;
use pl3xus_websockets::WebSocketProvider;
use pl3xus_sync::AppBatchRequestRegistrationExt;

use crate::types::*;

// Re-export all handlers for external use
pub use crud::{
    handle_list_programs,
    handle_get_program,
    handle_create_program,
    handle_delete_program,
    handle_update_program_settings,
};
pub use sequences::{
    handle_upload_csv,
    handle_add_sequence,
    handle_update_sequence_instructions,
    handle_remove_sequence,
};
pub use execution::{
    handle_load,
    handle_unload,
};

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
        ));

        // Add sequence handler systems
        app.add_systems(Update, (
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