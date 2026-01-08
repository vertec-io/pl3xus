//! Subsystem readiness tracking for validation before execution.

use serde::{Deserialize, Serialize};

#[cfg(feature = "ecs")]
use bevy::prelude::*;

/// Component on System entity tracking all registered subsystems.
///
/// Subsystems (e.g., fanuc robot, duet extruder) register themselves during plugin setup.
/// During the Validating phase, each subsystem updates its readiness status.
/// Execution proceeds only when all subsystems report Ready.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ecs", derive(Component))]
pub struct Subsystems {
    pub entries: Vec<SubsystemEntry>,
}

impl Subsystems {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Register a new subsystem (typically called during plugin setup)
    pub fn register(&mut self, name: &str) {
        if !self.entries.iter().any(|e| e.name == name) {
            self.entries.push(SubsystemEntry {
                name: name.to_string(),
                readiness: SubsystemReadiness::NotReady,
            });
        }
    }

    /// Update a subsystem's readiness
    pub fn set_readiness(&mut self, name: &str, readiness: SubsystemReadiness) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.name == name) {
            entry.readiness = readiness;
        }
    }

    /// Check if all subsystems are ready
    pub fn all_ready(&self) -> bool {
        self.entries.is_empty() || self.entries.iter().all(|e| matches!(e.readiness, SubsystemReadiness::Ready))
    }

    /// Get first error, if any
    pub fn first_error(&self) -> Option<&str> {
        self.entries.iter().find_map(|e| {
            if let SubsystemReadiness::Error(msg) = &e.readiness {
                Some(msg.as_str())
            } else {
                None
            }
        })
    }

    /// Get list of not-ready subsystems
    pub fn not_ready(&self) -> Vec<&str> {
        self.entries.iter()
            .filter(|e| matches!(e.readiness, SubsystemReadiness::NotReady))
            .map(|e| e.name.as_str())
            .collect()
    }

    /// Reset all subsystems to NotReady (called before validation)
    pub fn reset_all(&mut self) {
        for entry in &mut self.entries {
            entry.readiness = SubsystemReadiness::NotReady;
        }
    }
}

/// A single subsystem entry tracking its name and readiness.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubsystemEntry {
    pub name: String,
    pub readiness: SubsystemReadiness,
}

/// Readiness status of a subsystem.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum SubsystemReadiness {
    /// Subsystem has not yet reported readiness
    #[default]
    NotReady,
    /// Subsystem is ready for execution
    Ready,
    /// Subsystem encountered an error that prevents execution
    Error(String),
}

// ============================================================================
// Subsystem Name Constants
// ============================================================================

/// Subsystem name for the execution coordinator itself
pub const SUBSYSTEM_EXECUTION: &str = "execution";

/// Subsystem name for the programs plugin
pub const SUBSYSTEM_PROGRAMS: &str = "programs";

/// Subsystem name for FANUC robot
pub const SUBSYSTEM_FANUC: &str = "fanuc_robot";

/// Subsystem name for Duet extruder
pub const SUBSYSTEM_DUET: &str = "duet_extruder";

