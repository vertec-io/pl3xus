//! ExecutionState - synced component representing system execution state.
//!
//! This is the primary state component for the UI to display execution status.
//! It's buffer-centric: execution is about the buffer, not the program.

use serde::{Deserialize, Serialize};

#[cfg(feature = "ecs")]
use bevy::prelude::*;

#[cfg(feature = "stores")]
use reactive_stores::Store;

use super::buffer::ExecutionActions;

/// Execution state synced to all clients.
///
/// Contains current state, source info, progress, and available actions.
///
/// The UI uses this to:
/// - Display current state (Running, Paused, etc.)
/// - Know what type of source is active (Program, Stream, Generator)
/// - Highlight the current row in the buffer table (current_index)
/// - Show/hide action buttons (can_* fields)
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ecs", derive(Component))]
#[cfg_attr(feature = "stores", derive(Store))]
pub struct ExecutionState {
    /// Current execution state
    pub state: SystemState,

    /// What type of source is feeding the buffer
    pub source_type: SourceType,

    /// Source name for display (e.g., "my_print.csv", "spiral_generator")
    pub source_name: Option<String>,

    /// Source ID (e.g., program_id from database)
    pub source_id: Option<i64>,

    /// Current execution index (0-based)
    /// UI uses this to highlight the current row in the buffer table
    pub current_index: usize,

    /// Total points in buffer (if known)
    /// For static: known upfront. For streaming: grows as points added.
    pub total_points: Option<usize>,

    /// Points confirmed executed by the device
    pub points_executed: usize,

    // === Available Actions (server-driven) ===
    
    /// Execution-related actions (start, pause, resume, stop)
    /// Computed by the execution system based on buffer state
    pub execution_actions: ExecutionActions,
}

/// What type of source is feeding the execution buffer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceType {
    #[default]
    None,
    /// Loaded from database, all points known upfront
    StaticProgram,
    /// Points arriving from external source (future)
    Stream,
    /// Points generated algorithmically (future)
    Generator,
}

/// System execution state for UI display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SystemState {
    #[default]
    /// Nothing loaded, no source
    NoSource,
    /// Source loaded, ready to start
    Ready,
    /// Checking subsystems before execution
    Validating,
    /// Actively executing points
    Running,
    /// Paused by user
    Paused,
    /// Buffer empty, waiting for more (streaming only)
    AwaitingPoints,
    /// All points executed successfully
    Completed,
    /// Stopped by user
    Stopped,
    /// Error occurred
    Error,
}

impl ExecutionState {
    /// Create state for "no source loaded"
    pub fn no_source() -> Self {
        Self {
            state: SystemState::NoSource,
            source_type: SourceType::None,
            source_name: None,
            source_id: None,
            current_index: 0,
            total_points: None,
            points_executed: 0,
            execution_actions: ExecutionActions::default(),
        }
    }

    /// Update execution actions based on current state.
    ///
    /// This updates only the execution-related actions.
    /// Program actions (can_load, can_unload) must be set by the program plugin.
    pub fn update_execution_actions(&mut self) {
        self.execution_actions = match self.state {
            SystemState::NoSource => ExecutionActions {
                can_start: false,
                can_pause: false,
                can_resume: false,
                can_stop: false,
            },
            SystemState::Ready => ExecutionActions {
                can_start: true,
                can_pause: false,
                can_resume: false,
                can_stop: false,
            },
            SystemState::Validating => ExecutionActions {
                can_start: false,
                can_pause: false,
                can_resume: false,
                can_stop: true, // Can cancel validation
            },
            SystemState::Running | SystemState::AwaitingPoints => ExecutionActions {
                can_start: false,
                can_pause: true,
                can_resume: false,
                can_stop: true,
            },
            SystemState::Paused => ExecutionActions {
                can_start: false,
                can_pause: false,
                can_resume: true,
                can_stop: true,
            },
            SystemState::Completed | SystemState::Stopped => ExecutionActions {
                can_start: true, // Can restart
                can_pause: false,
                can_resume: false,
                can_stop: false,
            },
            SystemState::Error => ExecutionActions {
                can_start: true,
                can_pause: false,
                can_resume: false,
                can_stop: false,
            },
        };
    }
}

