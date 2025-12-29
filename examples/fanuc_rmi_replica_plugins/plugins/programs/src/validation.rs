//! Subsystem validation for the programs plugin.
//!
//! This module provides:
//! - Subsystem registration on startup
//! - Validation system that checks if a program is loaded

use bevy::prelude::*;

use fanuc_replica_core::ActiveSystem;
use fanuc_replica_execution::{
    BufferState, ExecutionCoordinator, SubsystemReadiness, Subsystems, SubsystemValidation,
    SUBSYSTEM_PROGRAMS,
};

/// Register the programs subsystem on startup.
///
/// This runs once when the System entity exists and adds the programs
/// subsystem to the Subsystems component.
pub fn register_programs_subsystem(
    mut systems: Query<&mut Subsystems, With<ActiveSystem>>,
    mut registered: Local<bool>,
) {
    if *registered {
        return;
    }

    if let Ok(mut subsystems) = systems.single_mut() {
        subsystems.register(SUBSYSTEM_PROGRAMS);
        *registered = true;
        info!("📋 Registered '{}' subsystem", SUBSYSTEM_PROGRAMS);
    }
}

/// Validate the programs subsystem during the Validating phase.
///
/// This system:
/// - Only runs when BufferState::Validating
/// - Checks if an ExecutionCoordinator exists (program is loaded)
/// - Sets subsystem readiness accordingly
pub fn validate_programs_subsystem(
    coordinator_query: Query<Option<&ExecutionCoordinator>, With<ActiveSystem>>,
    buffer_state_query: Query<&BufferState, With<ActiveSystem>>,
    mut subsystems_query: Query<&mut Subsystems, With<ActiveSystem>>,
) {
    let Ok(buffer_state) = buffer_state_query.single() else {
        return;
    };

    // Only validate when in Validating state
    if !matches!(buffer_state, BufferState::Validating) {
        return;
    }

    let Ok(mut subsystems) = subsystems_query.single_mut() else {
        return;
    };

    let Ok(coordinator_opt) = coordinator_query.single() else {
        return;
    };

    if coordinator_opt.is_some() {
        subsystems.set_readiness(SUBSYSTEM_PROGRAMS, SubsystemReadiness::Ready);
        trace!("✅ Programs subsystem ready (coordinator exists)");
    } else {
        subsystems.set_readiness(
            SUBSYSTEM_PROGRAMS,
            SubsystemReadiness::Error("No program loaded".to_string()),
        );
        trace!("❌ Programs subsystem not ready (no coordinator)");
    }
}

/// Plugin that adds programs subsystem validation.
pub struct ProgramsValidationPlugin;

impl Plugin for ProgramsValidationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, register_programs_subsystem);
        app.add_systems(
            Update,
            validate_programs_subsystem.in_set(SubsystemValidation),
        );
    }
}
