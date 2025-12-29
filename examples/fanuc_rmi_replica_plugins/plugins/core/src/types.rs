//! Core types - ActiveSystem marker component, console logging, and shared utilities.

use serde::{Deserialize, Serialize};

#[cfg(feature = "ecs")]
use bevy::prelude::Component;

/// Marker component for the active/current System entity.
///
/// This entity is the control root - clients request control of this entity
/// to gain control over the entire apparatus including all child robots.
/// The System represents the overall application/cell and is the parent
/// entity in the hierarchy.
#[cfg_attr(feature = "ecs", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct ActiveSystem;

// ============================================================================
// Console Log Types
// ============================================================================

/// Console log entry (broadcast message for console display).
///
/// Used to send timestamped log messages to clients for display in a console UI.
#[cfg_attr(feature = "ecs", derive(bevy::prelude::Message))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct ConsoleLogEntry {
    /// Formatted timestamp string (HH:MM:SS.mmm)
    pub timestamp: String,
    /// Unix timestamp in milliseconds
    pub timestamp_ms: u64,
    /// Direction of the message (sent, received, system)
    pub direction: ConsoleDirection,
    /// Type of message (command, response, error, etc.)
    pub msg_type: ConsoleMsgType,
    /// The actual log content
    pub content: String,
    /// Optional sequence ID for correlation
    pub sequence_id: Option<u32>,
}

/// Direction of a console message.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub enum ConsoleDirection {
    /// Message sent to a device/service
    #[default]
    Sent,
    /// Message received from a device/service
    Received,
    /// Internal system message
    System,
}

/// Type of console message.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub enum ConsoleMsgType {
    /// A command or request
    #[default]
    Command,
    /// A response to a command
    Response,
    /// An error message
    Error,
    /// A status update
    Status,
    /// Configuration-related message
    Config,
}

/// Create a console log entry with current timestamp.
///
/// # Example
/// ```ignore
/// use fanuc_replica_core::{console_entry, ConsoleDirection, ConsoleMsgType};
/// let entry = console_entry("Robot connected", ConsoleDirection::System, ConsoleMsgType::Status);
/// ```
pub fn console_entry(
    content: impl Into<String>,
    direction: ConsoleDirection,
    msg_type: ConsoleMsgType,
) -> ConsoleLogEntry {
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
// Database Management Messages
// ============================================================================

/// Reset the database to initial state.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ResetDatabase;

/// Response for ResetDatabase.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ResetDatabaseResponse {
    pub success: bool,
    pub error: Option<String>,
}

impl pl3xus_common::RequestMessage for ResetDatabase {
    type ResponseMessage = ResetDatabaseResponse;
}

