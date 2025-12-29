//! Program types and utilities.
//!
//! This module provides:
//! - `Program` component for tracking loaded program state
//! - `ProgramState` enum for execution state
//! - `ProgramDefaults` for motion packet defaults
//! - Notification utilities (`new_notification`, `console_entry`)
//!
//! Note: The actual execution logic is now in the ExecutionPlugin.
//! This module only provides data types and utilities.

use bevy::prelude::*;
use crate::types::{
    ProgramNotification, ProgramNotificationKind,
    ConsoleLogEntry, ConsoleDirection, ConsoleMsgType,
    Instruction,
};
use std::sync::atomic::{AtomicU64, Ordering};

/// Global sequence counter for program notifications.
static NOTIFICATION_SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// Create a new ProgramNotification with a unique sequence number.
pub fn new_notification(kind: ProgramNotificationKind) -> ProgramNotification {
    ProgramNotification {
        sequence: NOTIFICATION_SEQUENCE.fetch_add(1, Ordering::Relaxed),
        kind,
    }
}

/// Create a console log entry with current timestamp.
pub fn console_entry(content: impl Into<String>, direction: ConsoleDirection, msg_type: ConsoleMsgType) -> ConsoleLogEntry {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let ms = now.as_millis() as u64;
    let secs = now.as_secs();
    let hours = (secs / 3600) % 24;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;
    let millis = (ms % 1000) as u32;

    ConsoleLogEntry {
        timestamp: format!("{:02}:{:02}:{:02}.{:03}", hours, minutes, seconds, millis),
        timestamp_ms: ms,
        direction,
        msg_type,
        content: content.into(),
        sequence_id: None,
    }
}

// ============================================================================
// Program State Machine
// ============================================================================

/// Program execution state (like meteorite's PrinterStatus).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProgramState {
    /// Program is loaded but not running.
    #[default]
    Idle,
    /// Program is actively executing.
    Running,
    /// Program is paused (can be resumed).
    Paused,
    /// Program completed successfully.
    Completed,
    /// Program encountered an error.
    Error,
}

/// Program defaults for building motion packets.
/// Required fields (term_type, term_value) are non-optional - they come from DB.
#[derive(Clone, Default)]
pub struct ProgramDefaults {
    pub w: f64,
    pub p: f64,
    pub r: f64,
    pub speed: f64,
    pub speed_type: String,
    pub term_type: String,
    pub term_value: u8,  // Required: 0-100 for CNT, 0 for FINE
    pub uframe: Option<i32>,
    pub utool: Option<i32>,
}

/// Approach/retreat position.
#[derive(Clone, Default)]
pub struct ApproachRetreat {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: Option<f64>,
    pub p: Option<f64>,
    pub r: Option<f64>,
}

/// Program component - attached to System entity when a program is loaded.
/// This is the single source of truth for program execution state.
#[derive(Component)]
pub struct Program {
    /// Program ID from database.
    pub id: i64,
    /// Program name for display.
    pub name: String,
    /// All instructions (including approach/retreat as first/last).
    pub instructions: Vec<Instruction>,
    /// Current instruction index (next to send).
    pub current_index: usize,
    /// Number of instructions completed (responses received).
    pub completed_count: usize,
    /// Current execution state.
    pub state: ProgramState,
    /// Program defaults for motion.
    pub defaults: ProgramDefaults,
    /// Approach position (optional).
    pub approach: Option<ApproachRetreat>,
    /// Retreat position (optional).
    pub retreat: Option<ApproachRetreat>,
    /// Move speed for approach/retreat.
    pub move_speed: f64,
}

impl Program {
    /// Total number of instructions (including approach/retreat).
    pub fn total_instructions(&self) -> usize {
        let mut count = self.instructions.len();
        if self.approach.is_some() { count += 1; }
        if self.retreat.is_some() { count += 1; }
        count
    }

    /// Check if all instructions have been sent.
    pub fn all_sent(&self) -> bool {
        self.current_index >= self.total_instructions()
    }

    /// Check if all instructions have been completed.
    #[allow(dead_code)]
    pub fn all_completed(&self) -> bool {
        self.completed_count >= self.total_instructions()
    }
}