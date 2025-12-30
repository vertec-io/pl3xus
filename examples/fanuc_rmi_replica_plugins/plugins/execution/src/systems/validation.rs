//! Validation coordination system.
//!
//! This system coordinates the validation phase before execution starts or resumes.
//! When BufferState is Validating or ValidatingForResume, it checks all registered
//! subsystems and transitions to Executing when all are ready, or Error if any fail.
//!
//! - Validating: Used for initial Start command, starts from index 0
//! - ValidatingForResume: Used for Resume command, continues from paused index
//!
//! Includes timeout functionality to prevent stalled validations.

use bevy::prelude::*;
use std::time::{Duration, Instant};

use crate::components::{
    BufferState, ExecutionCoordinator, ExecutionState, Subsystems, SystemState,
    VALIDATION_TIMEOUT,
};
use fanuc_replica_core::ActiveSystem;

/// Resource to track when validation started.
///
/// This is separate from BufferState because `Instant` cannot be serialized.
/// The resource is created when entering Validating state and removed when exiting.
#[derive(Resource)]
pub struct ValidationStartTime(pub Instant);

impl Default for ValidationStartTime {
    fn default() -> Self {
        Self(Instant::now())
    }
}

impl ValidationStartTime {
    /// Check if validation has exceeded the timeout duration.
    pub fn is_timed_out(&self) -> bool {
        self.0.elapsed() > VALIDATION_TIMEOUT
    }

    /// Get elapsed time since validation started.
    pub fn elapsed(&self) -> Duration {
        self.0.elapsed()
    }
}

/// Coordinate validation of all subsystems before execution.
///
/// This system:
/// 1. Only runs when BufferState::Validating or BufferState::ValidatingForResume
/// 2. Checks for timeout first
/// 3. Checks for any subsystem errors
/// 4. Transitions to Executing if all ready (from index 0 or resume index)
/// 5. Stays in Validating/ValidatingForResume if still waiting
///
/// Subsystem plugins (programs, fanuc, duet) should have their own
/// `validate_*_subsystem` systems that run BEFORE this one and set
/// their readiness status.
pub fn coordinate_validation(
    mut commands: Commands,
    validation_start: Option<Res<ValidationStartTime>>,
    mut systems: Query<
        (
            &ExecutionCoordinator,
            &mut BufferState,
            &Subsystems,
            Option<&mut ExecutionState>,
        ),
        With<ActiveSystem>,
    >,
) {
    let Ok((coordinator, mut buffer_state, subsystems, exec_state)) = systems.single_mut() else {
        return;
    };

    // Extract resume index if validating for resume, or None for initial start
    let resume_from_index = match *buffer_state {
        BufferState::Validating => Some(0), // Initial start from index 0
        BufferState::ValidatingForResume { resume_from_index } => Some(resume_from_index),
        _ => None,
    };

    // Only process when in a validating state
    let Some(start_index) = resume_from_index else {
        // Clean up validation start time if we're not validating
        if validation_start.is_some() {
            commands.remove_resource::<ValidationStartTime>();
        }
        return;
    };

    let is_resume = start_index > 0;

    // Ensure validation start time exists
    let start_time = match validation_start {
        Some(start) => start,
        None => {
            // This shouldn't happen if handle_start/handle_resume inserted it, but handle gracefully
            commands.insert_resource(ValidationStartTime::default());
            warn!("ValidationStartTime was missing, created new one");
            return; // Wait for next frame
        }
    };

    // Check for timeout first
    if start_time.is_timed_out() {
        let not_ready = subsystems.not_ready();
        let timeout_msg = if not_ready.is_empty() {
            "Validation timed out without all subsystems reporting".to_string()
        } else {
            format!(
                "Validation timed out after {:?}. Not ready: {}",
                VALIDATION_TIMEOUT,
                not_ready.join(", ")
            )
        };

        *buffer_state = BufferState::Error {
            message: timeout_msg.clone(),
        };
        if let Some(mut exec) = exec_state {
            exec.state = SystemState::Error;
            exec.update_available_actions();
        }
        commands.remove_resource::<ValidationStartTime>();
        error!(
            "⏱️ Validation timeout for '{}': {}",
            coordinator.name, timeout_msg
        );
        return;
    }

    // Check for any subsystem errors
    if let Some(error_msg) = subsystems.first_error() {
        *buffer_state = BufferState::Error {
            message: error_msg.to_string(),
        };
        if let Some(mut exec) = exec_state {
            exec.state = SystemState::Error;
            exec.update_available_actions();
        }
        commands.remove_resource::<ValidationStartTime>();
        error!(
            "❌ Validation failed for '{}': {}",
            coordinator.name, error_msg
        );
        return;
    }

    // Check if all subsystems are ready
    if subsystems.all_ready() {
        *buffer_state = BufferState::Executing {
            current_index: start_index,
            completed_count: start_index, // For resume, assume all before pause point are complete
        };
        if let Some(mut exec) = exec_state {
            exec.state = SystemState::Running;
            exec.current_index = start_index as usize;
            exec.points_executed = start_index as usize;
            exec.update_available_actions();
        }
        commands.remove_resource::<ValidationStartTime>();
        if is_resume {
            info!(
                "✅ Resume validation succeeded for '{}' in {:?}, resuming from index {}",
                coordinator.name,
                start_time.elapsed(),
                start_index
            );
        } else {
            info!(
                "✅ Validation succeeded for '{}' in {:?}, starting execution",
                coordinator.name,
                start_time.elapsed()
            );
        }
        return;
    }

    // Still waiting for subsystems - log which ones are not ready (periodically)
    let elapsed = start_time.elapsed();
    if elapsed.as_secs() % 5 == 0 && elapsed.subsec_millis() < 100 {
        let not_ready = subsystems.not_ready();
        if !not_ready.is_empty() {
            let action = if is_resume { "resume" } else { "start" };
            info!(
                "⏳ Validation for {} waiting ({:?}/{:?}) for subsystems: {:?}",
                action, elapsed, VALIDATION_TIMEOUT, not_ready
            );
        }
    }
}
