//! Subsystem validation for the FANUC plugin.
//!
//! This module provides:
//! - Subsystem registration on startup
//! - Validation system that checks robot connection status

use bevy::prelude::*;

use crate::connection::{FanucRobot, RobotConnectionState};
use fanuc_replica_core::ActiveSystem;
use fanuc_replica_execution::{
    BufferState, SubsystemReadiness, Subsystems, SubsystemValidation, SUBSYSTEM_FANUC,
};

/// Register the FANUC subsystem on startup.
///
/// This runs once when the System entity exists and adds the FANUC
/// subsystem to the Subsystems component.
pub fn register_fanuc_subsystem(
    mut systems: Query<&mut Subsystems, With<ActiveSystem>>,
    mut registered: Local<bool>,
) {
    if *registered {
        return;
    }

    if let Ok(mut subsystems) = systems.single_mut() {
        subsystems.register(SUBSYSTEM_FANUC);
        *registered = true;
        info!("🤖 Registered '{}' subsystem", SUBSYSTEM_FANUC);
    }
}

/// Validate the FANUC subsystem during the Validating phase.
///
/// This system:
/// - Only runs when BufferState::Validating
/// - Checks if any FANUC robot is connected
/// - Sets subsystem readiness accordingly
pub fn validate_fanuc_subsystem(
    robots: Query<&RobotConnectionState, With<FanucRobot>>,
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

    // Check if any robot is connected
    let connected = robots
        .iter()
        .any(|state| *state == RobotConnectionState::Connected);

    if connected {
        subsystems.set_readiness(SUBSYSTEM_FANUC, SubsystemReadiness::Ready);
        trace!("✅ FANUC subsystem ready (robot connected)");
    } else {
        subsystems.set_readiness(
            SUBSYSTEM_FANUC,
            SubsystemReadiness::Error("No FANUC robot connected".to_string()),
        );
        trace!("❌ FANUC subsystem not ready (no robot connected)");
    }
}

/// Plugin that adds FANUC subsystem validation.
pub struct FanucValidationPlugin;

impl Plugin for FanucValidationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, register_fanuc_subsystem);
        app.add_systems(
            Update,
            validate_fanuc_subsystem.in_set(SubsystemValidation),
        );
    }
}
