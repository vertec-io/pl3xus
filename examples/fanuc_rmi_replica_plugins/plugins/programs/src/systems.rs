//! Program plugin systems.
//!
//! Systems for syncing ProgramActions based on execution state.

#[cfg(feature = "server")]
use bevy::prelude::*;

#[cfg(all(feature = "server", feature = "ecs"))]
use fanuc_replica_core::ActiveSystem;

#[cfg(all(feature = "server", feature = "ecs"))]
use fanuc_replica_execution::{ExecutionCoordinator, ExecutionState, SystemState};

#[cfg(all(feature = "server", feature = "ecs"))]
use crate::ProgramActions;

/// Sync ProgramActions based on execution state.
///
/// This system is the source of truth for ProgramActions by observing ExecutionState:
/// - can_load = true only when no program is loaded (state == NoSource)
/// - can_unload = true when program is loaded AND execution is not active
///
/// The handlers don't set ProgramActions directly; they only update ExecutionState.
/// This system then derives the correct action flags from that state.
#[cfg(all(feature = "server", feature = "ecs"))]
pub fn sync_program_actions(
    mut system_query: Query<(&ExecutionState, &mut ProgramActions, Option<&ExecutionCoordinator>), With<ActiveSystem>>,
) {
    let Ok((exec_state, mut actions, coordinator)) = system_query.single_mut() else {
        return; // No ExecutionState/ProgramActions on System entity
    };

    // Determine if a program is loaded
    let program_is_loaded = (exec_state.state != SystemState::NoSource) | coordinator.is_some();

    // Determine if execution is active (pause or stop available)
    let execution_is_active = exec_state.execution_actions.can_pause 
        || exec_state.execution_actions.can_stop;

    let new_actions = ProgramActions {
        can_load: !program_is_loaded,
        can_unload: program_is_loaded && !execution_is_active,
    };

    // Only update if changed to avoid constant updates
    if *actions != new_actions {
        *actions = new_actions;
    }
}
