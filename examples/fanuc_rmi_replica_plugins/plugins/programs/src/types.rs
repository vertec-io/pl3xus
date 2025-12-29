//! Device-agnostic program types.
//!
//! These types are designed to be robot/device independent:
//! - Position: x, y, z (required) + optional rotations (w, p, r) + optional extra axes (ext1-3)
//! - Programs have sequences for approach, main, and retreat
//! - Each sequence can have multiple instructions

use serde::{Deserialize, Serialize};

// Server-only: derive macros for automatic query invalidation
#[cfg(feature = "server")]
use pl3xus_macros::{HasSuccess, Invalidates};

// RequestMessage trait is available on all platforms (from pl3xus_common)
use pl3xus_common::RequestMessage;

// ============================================================================
// Core Instruction Type
// ============================================================================

/// A single instruction in a program.
///
/// This is device-agnostic - x, y, z are required, everything else is optional.
/// The execution layer is responsible for mapping these to device-specific commands.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Instruction {
    /// Line number within the sequence (1-based).
    pub line_number: i32,
    
    // Required position
    pub x: f64,
    pub y: f64,
    pub z: f64,
    
    // Optional rotations (robot-specific, e.g., FANUC W/P/R)
    pub w: Option<f64>,
    pub p: Option<f64>,
    pub r: Option<f64>,
    
    // Optional extra axes (e.g., external axes, extruder)
    pub ext1: Option<f64>,
    pub ext2: Option<f64>,
    pub ext3: Option<f64>,

    // Motion parameters
    pub speed: Option<f64>,
    pub term_type: Option<String>,   // e.g., "FINE", "CNT"
    pub term_value: Option<u8>,      // e.g., 0-100 for CNT
}

// ============================================================================
// Sequence Types
// ============================================================================

/// Type of instruction sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SequenceType {
    /// Approach sequence - executed before main program.
    Approach,
    /// Main program instructions.
    Main,
    /// Retreat sequence - executed after main program.
    Retreat,
}

impl Default for SequenceType {
    fn default() -> Self {
        Self::Main
    }
}

/// A named sequence of instructions.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InstructionSequence {
    pub id: i64,
    pub sequence_type: SequenceType,
    pub name: Option<String>,
    pub order_index: i32,
    pub instructions: Vec<Instruction>,
}

// ============================================================================
// Program Types
// ============================================================================

/// Program summary for listing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProgramInfo {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub instruction_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// Full program with all sequences and instructions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramDetail {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    
    // Default values for missing instruction fields
    pub default_speed: Option<f64>,
    pub default_term_type: Option<String>,
    pub default_term_value: Option<u8>,
    
    // Approach move speed (for approach/retreat sequences)
    pub move_speed: f64,
    
    // Sequences (approach, main, retreat)
    pub approach_sequences: Vec<InstructionSequence>,
    pub main_sequence: InstructionSequence,
    pub retreat_sequences: Vec<InstructionSequence>,
    
    pub created_at: String,
    pub updated_at: String,
}

/// Program with simple line info for display.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProgramWithLines {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    /// Main program lines
    pub lines: Vec<ProgramLineInfo>,
    /// Approach sequence lines (optional)
    pub approach_lines: Vec<ProgramLineInfo>,
    /// Retreat sequence lines (optional)
    pub retreat_lines: Vec<ProgramLineInfo>,
}

/// Simplified line info for display.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProgramLineInfo {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
    pub p: f64,
    pub r: f64,
    pub speed: f64,
    pub term_type: String,
}

// ============================================================================
// Request/Response Types
// ============================================================================

/// List all programs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ListPrograms;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListProgramsResponse {
    pub programs: Vec<ProgramInfo>,
}

impl RequestMessage for ListPrograms {
    type ResponseMessage = ListProgramsResponse;
}

/// Get a single program with all details.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetProgram {
    pub program_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetProgramResponse {
    pub program: Option<ProgramDetail>,
}

impl RequestMessage for GetProgram {
    type ResponseMessage = GetProgramResponse;
}

/// Create a new program.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(Invalidates))]
#[cfg_attr(feature = "server", invalidates("ListPrograms"))]
pub struct CreateProgram {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(HasSuccess))]
pub struct CreateProgramResponse {
    pub success: bool,
    pub program_id: Option<i64>,
    pub error: Option<String>,
}

impl RequestMessage for CreateProgram {
    type ResponseMessage = CreateProgramResponse;
}

/// Delete a program.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(Invalidates))]
#[cfg_attr(feature = "server", invalidates("ListPrograms", "GetProgram"))]
pub struct DeleteProgram {
    pub program_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(HasSuccess))]
pub struct DeleteProgramResponse {
    pub success: bool,
    pub error: Option<String>,
}

impl RequestMessage for DeleteProgram {
    type ResponseMessage = DeleteProgramResponse;
}

/// Update program settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(Invalidates))]
#[cfg_attr(feature = "server", invalidates("GetProgram"))]
pub struct UpdateProgramSettings {
    pub program_id: i64,
    pub name: Option<String>,
    pub description: Option<String>,
    pub default_speed: Option<f64>,
    pub default_term_type: Option<String>,
    pub default_term_value: Option<u8>,
    pub move_speed: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(HasSuccess))]
pub struct UpdateProgramSettingsResponse {
    pub success: bool,
    pub error: Option<String>,
}

impl RequestMessage for UpdateProgramSettings {
    type ResponseMessage = UpdateProgramSettingsResponse;
}

/// Upload CSV data to a program.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(Invalidates))]
#[cfg_attr(feature = "server", invalidates("GetProgram", "ListPrograms"))]
pub struct UploadCsv {
    pub program_id: i64,
    pub csv_content: String,
    /// Which sequence to upload to (defaults to Main)
    pub sequence_type: Option<SequenceType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(HasSuccess))]
pub struct UploadCsvResponse {
    pub success: bool,
    pub lines_imported: Option<i32>,
    pub warnings: Vec<String>,
    pub error: Option<String>,
}

impl RequestMessage for UploadCsv {
    type ResponseMessage = UploadCsvResponse;
}

/// Add an approach/retreat sequence to a program.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(Invalidates))]
#[cfg_attr(feature = "server", invalidates("GetProgram"))]
pub struct AddSequence {
    pub program_id: i64,
    pub sequence_type: SequenceType,
    pub name: Option<String>,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(HasSuccess))]
pub struct AddSequenceResponse {
    pub success: bool,
    pub sequence_id: Option<i64>,
    pub error: Option<String>,
}

impl RequestMessage for AddSequence {
    type ResponseMessage = AddSequenceResponse;
}

/// Remove a sequence from a program.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(Invalidates))]
#[cfg_attr(feature = "server", invalidates("GetProgram"))]
pub struct RemoveSequence {
    pub sequence_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(HasSuccess))]
pub struct RemoveSequenceResponse {
    pub success: bool,
    pub error: Option<String>,
}

impl RequestMessage for RemoveSequence {
    type ResponseMessage = RemoveSequenceResponse;
}

// ============================================================================
// Load/Unload Types
// ============================================================================

use pl3xus_common::ErrorResponse;

/// Load a program into the execution buffer.
///
/// This fetches the program from the database and populates:
/// - ToolpathBuffer with ExecutionPoints
/// - BufferDisplayData for UI display
/// - ExecutionState with source info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Load {
    pub program_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadResponse {
    pub success: bool,
    pub program: Option<ProgramWithLines>,
    pub error: Option<String>,
}

impl RequestMessage for Load {
    type ResponseMessage = LoadResponse;
}

impl ErrorResponse for Load {
    fn error_response(error: String) -> Self::ResponseMessage {
        LoadResponse {
            success: false,
            program: None,
            error: Some(error),
        }
    }
}

/// Unload the currently loaded program.
///
/// Clears the execution buffer and resets state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Unload;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnloadResponse {
    pub success: bool,
    pub error: Option<String>,
}

impl RequestMessage for Unload {
    type ResponseMessage = UnloadResponse;
}

impl ErrorResponse for Unload {
    fn error_response(error: String) -> Self::ResponseMessage {
        UnloadResponse {
            success: false,
            error: Some(error),
        }
    }
}

// ============================================================================
// Program Notifications
// ============================================================================

use std::sync::atomic::{AtomicU64, Ordering};

/// Global sequence counter for program notifications.
static NOTIFICATION_SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// Program notification (broadcast message from server to all clients).
///
/// This is sent to notify clients about program execution events like
/// completion, errors, or other state changes. All connected clients
/// receive this message and can display appropriate UI feedback.
#[cfg_attr(feature = "ecs", derive(bevy::prelude::Message))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct ProgramNotification {
    /// Unique sequence number to distinguish identical notifications
    pub sequence: u64,
    /// The type of notification
    pub kind: ProgramNotificationKind,
}

impl ProgramNotification {
    /// Create a new ProgramNotification with a unique sequence number.
    pub fn new(kind: ProgramNotificationKind) -> Self {
        Self {
            sequence: NOTIFICATION_SEQUENCE.fetch_add(1, Ordering::Relaxed),
            kind,
        }
    }

    /// Create a "started" notification.
    pub fn started(program_name: impl Into<String>, total_instructions: usize) -> Self {
        Self::new(ProgramNotificationKind::Started {
            program_name: program_name.into(),
            total_instructions,
        })
    }

    /// Create a "completed" notification.
    pub fn completed(program_name: impl Into<String>, total_instructions: usize) -> Self {
        Self::new(ProgramNotificationKind::Completed {
            program_name: program_name.into(),
            total_instructions,
        })
    }

    /// Create a "stopped" notification.
    pub fn stopped(program_name: impl Into<String>, at_line: usize, completed: usize) -> Self {
        Self::new(ProgramNotificationKind::Stopped {
            program_name: program_name.into(),
            at_line,
            completed,
        })
    }

    /// Create an "error" notification.
    pub fn error(program_name: impl Into<String>, at_line: usize, error_message: impl Into<String>) -> Self {
        Self::new(ProgramNotificationKind::Error {
            program_name: program_name.into(),
            at_line,
            error_message: error_message.into(),
        })
    }
}

/// Kind of program notification.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub enum ProgramNotificationKind {
    #[default]
    None,
    /// Program execution started.
    Started {
        program_name: String,
        total_instructions: usize,
    },
    /// Program completed successfully.
    Completed {
        program_name: String,
        total_instructions: usize,
    },
    /// Program was stopped by user.
    Stopped {
        program_name: String,
        at_line: usize,
        completed: usize,
    },
    /// Program encountered an error.
    Error {
        program_name: String,
        at_line: usize,
        error_message: String,
    },
}
