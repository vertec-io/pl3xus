//! Program state synchronization plugin.
//!
//! Handles syncing ProgramActions based on program and execution state.

use bevy::prelude::*;
use crate::systems::sync_program_actions;
use crate::types::ProgramActions;
use robot_hmi_core::types::ActiveSystem;

/// Plugin for program action synchronization.
pub struct ProgramsSyncPlugin;

impl Plugin for ProgramsSyncPlugin {
    fn build(&self, app: &mut App) {
        // Initialize ProgramActions on System entity
        app.add_systems(First, add_program_actions_to_system);
        
        // Sync program actions based on execution state
        app.add_systems(Update, sync_program_actions);
    }
}

/// Add ProgramActions component to the System entity.
///
/// This runs in First schedule and waits until the System entity exists.
/// ProgramActions is synced to all clients for UI action availability.
fn add_program_actions_to_system(
    mut commands: Commands,
    system_query: Query<Entity, (With<ActiveSystem>, Without<ProgramActions>)>,
    mut initialized: Local<bool>,
) {
    if *initialized {
        return;
    }

    if let Ok(system_entity) = system_query.single() {
        commands.entity(system_entity).insert(ProgramActions {
            can_load: true,
            can_unload: false,
        });
        *initialized = true;
        info!("📡 Added ProgramActions to System entity");
    }
}
