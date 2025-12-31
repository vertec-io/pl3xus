//! Main programs plugin that combines all functionality.

use bevy::prelude::*;
use crate::database::ProgramsDatabaseInit;
use crate::handlers::ProgramHandlerPlugin;
use crate::notifications::ProgramNotificationsPlugin;
use crate::validation::ProgramsValidationPlugin;
use crate::sync_plugin::ProgramsSyncPlugin;
use crate::ProgramActions;
use fanuc_replica_core::DatabaseInitRegistry;

#[cfg(feature = "server")]
use pl3xus_sync::{AppPl3xusSyncExt, ComponentSyncConfig};

/// Programs plugin - provides program management functionality.
///
/// This plugin includes:
/// - Database schema initialization (via ProgramsDatabaseInit trait)
/// - Request handlers for program CRUD operations
/// - CSV import functionality
/// - Subsystem validation for execution coordination
/// - Execution state notifications (start, complete, stop, error)
/// - Program state synchronization (syncs to ExecutionState)
pub struct ProgramsPlugin;

impl Plugin for ProgramsPlugin {
    fn build(&self, app: &mut App) {
        // Register database initializer
        let mut registry = app.world_mut().get_resource_or_insert_with(DatabaseInitRegistry::default);
        registry.register(ProgramsDatabaseInit);

        #[cfg(feature = "server")]
        {
            // Register ProgramActions as a synced component
            app.sync_component::<ProgramActions>(Some(ComponentSyncConfig::read_only_with_message(
                "ProgramActions is read-only. Use Load/Unload commands."
            )));
        }

        app.add_plugins((
            ProgramHandlerPlugin,
            ProgramsValidationPlugin,
            ProgramNotificationsPlugin,
            ProgramsSyncPlugin,
        ));
    }
}

