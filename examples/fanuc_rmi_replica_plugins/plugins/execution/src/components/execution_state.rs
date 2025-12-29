//! ExecutionState - synced component representing system execution state.
//!
//! This is the primary state component for the UI to display execution status.
//! It's buffer-centric: execution is about the buffer, not the program.

use serde::{Deserialize, Serialize};

#[cfg(feature = "ecs")]
use bevy::prelude::*;

#[cfg(feature = "stores")]
use reactive_stores::Store;

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
    
    /// Can load a new source (program, stream, etc.)
    pub can_load: bool,
    /// Can start execution
    pub can_start: bool,
    /// Can pause execution
    pub can_pause: bool,
    /// Can resume execution
    pub can_resume: bool,
    /// Can stop execution (or cancel validation)
    pub can_stop: bool,
    /// Can unload the current source
    pub can_unload: bool,
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
        let mut s = Self {
            state: SystemState::NoSource,
            source_type: SourceType::None,
            source_name: None,
            source_id: None,
            current_index: 0,
            total_points: None,
            points_executed: 0,
            can_load: true,
            can_start: false,
            can_pause: false,
            can_resume: false,
            can_stop: false,
            can_unload: false,
        };
        s.update_available_actions();
        s
    }

    /// Update available actions based on current state.
    pub fn update_available_actions(&mut self) {
        match self.state {
            SystemState::NoSource => {
                self.can_load = true;
                self.can_start = false;
                self.can_pause = false;
                self.can_resume = false;
                self.can_stop = false;
                self.can_unload = false;
            }
            SystemState::Ready => {
                self.can_load = false;
                self.can_start = true;
                self.can_pause = false;
                self.can_resume = false;
                self.can_stop = false;
                self.can_unload = true;
            }
            SystemState::Validating => {
                self.can_load = false;
                self.can_start = false;
                self.can_pause = false;
                self.can_resume = false;
                self.can_stop = true; // Can cancel validation
                self.can_unload = false;
            }
            SystemState::Running | SystemState::AwaitingPoints => {
                self.can_load = false;
                self.can_start = false;
                self.can_pause = true;
                self.can_resume = false;
                self.can_stop = true;
                self.can_unload = false;
            }
            SystemState::Paused => {
                self.can_load = false;
                self.can_start = false;
                self.can_pause = false;
                self.can_resume = true;
                self.can_stop = true;
                self.can_unload = false;
            }
            SystemState::Completed | SystemState::Stopped => {
                self.can_load = true;
                self.can_start = true; // Can restart
                self.can_pause = false;
                self.can_resume = false;
                self.can_stop = false;
                self.can_unload = true;
            }
            SystemState::Error => {
                self.can_load = true;
                self.can_start = false;
                self.can_pause = false;
                self.can_resume = false;
                self.can_stop = false;
                self.can_unload = true;
            }
        }
    }
}

