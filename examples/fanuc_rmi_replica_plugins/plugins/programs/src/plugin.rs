//! Main programs plugin that combines all functionality.

use bevy::prelude::*;
use crate::database::ProgramsDatabaseInit;
use crate::handlers::ProgramHandlerPlugin;
use crate::notifications::ProgramNotificationsPlugin;
use crate::validation::ProgramsValidationPlugin;
use fanuc_replica_core::DatabaseInitRegistry;

/// Programs plugin - provides program management functionality.
///
/// This plugin includes:
/// - Database schema initialization (via ProgramsDatabaseInit trait)
/// - Request handlers for program CRUD operations
/// - CSV import functionality
/// - Subsystem validation for execution coordination
/// - Execution state notifications (start, complete, stop, error)
pub struct ProgramsPlugin;

impl Plugin for ProgramsPlugin {
    fn build(&self, app: &mut App) {
        // Register database initializer
        let mut registry = app.world_mut().get_resource_or_insert_with(DatabaseInitRegistry::default);
        registry.register(ProgramsDatabaseInit);

        app.add_plugins((
            ProgramHandlerPlugin,
            ProgramsValidationPlugin,
            ProgramNotificationsPlugin,
        ));
    }
}

