//! BufferDisplayData - synced component for UI buffer table display.
//!
//! This is what the "program table" actually shows - it's the execution buffer!
//! The UI renders these lines as table rows and uses ExecutionState.current_index
//! to highlight the current row.

use serde::{Deserialize, Serialize};

#[cfg(feature = "ecs")]
use bevy::prelude::*;

#[cfg(feature = "stores")]
use reactive_stores::Store;

/// Display data for the execution buffer table in UI.
///
/// This is what the "program table" actually shows - it's the execution buffer!
/// - For static programs: populated once on load
/// - For streaming: updated incrementally as points arrive
/// - For generators: updated as points are generated
///
/// The UI uses ExecutionState.current_index to highlight the current row.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ecs", derive(Component))]
#[cfg_attr(feature = "stores", derive(Store))]
pub struct BufferDisplayData {
    /// The lines/points to show in the UI table
    pub lines: Vec<BufferLineDisplay>,
}

/// A single line in the buffer table display.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BufferLineDisplay {
    /// Index in the buffer (matches ExecutionState.current_index)
    pub index: usize,

    /// Type of operation for display (e.g., "Move", "Extrude", "Wait", "Dwell")
    pub line_type: String,

    /// Human-readable description (e.g., "Move to (100.0, 200.0, 50.0)")
    pub description: String,

    /// Optional sequence/section name (e.g., "Approach", "Main", "Retreat")
    pub sequence_name: Option<String>,

    /// Original source line number (for static programs)
    /// Useful for debugging/correlation with source file
    pub source_line: Option<usize>,

    /// Position data for table columns
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
    pub p: f64,
    pub r: f64,
    pub speed: f64,
    pub term_type: String,
}

impl Default for BufferLineDisplay {
    fn default() -> Self {
        Self {
            index: 0,
            line_type: "Move".to_string(),
            description: String::new(),
            sequence_name: None,
            source_line: None,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 0.0,
            p: 0.0,
            r: 0.0,
            speed: 0.0,
            term_type: "FINE".to_string(),
        }
    }
}

impl BufferDisplayData {
    pub fn new() -> Self {
        Self { lines: Vec::new() }
    }

    pub fn clear(&mut self) {
        self.lines.clear();
    }

    pub fn push_line(&mut self, line: BufferLineDisplay) {
        self.lines.push(line);
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }
}

