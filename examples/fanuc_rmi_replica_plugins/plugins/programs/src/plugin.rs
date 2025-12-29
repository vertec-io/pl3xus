//! Main programs plugin that combines all functionality.

use bevy::prelude::*;
use crate::handlers::ProgramHandlerPlugin;

/// Programs plugin - provides program management functionality.
///
/// This plugin includes:
/// - Database schema initialization (via ProgramsDatabaseInit trait)
/// - Request handlers for program CRUD operations
/// - CSV import functionality
pub struct ProgramsPlugin;

impl Plugin for ProgramsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ProgramHandlerPlugin);
    }
}

