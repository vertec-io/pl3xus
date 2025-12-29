//! Device-agnostic program types.
//!
//! These types are designed to be robot/device independent:
//! - Position: x, y, z (required) + optional rotations (w, p, r) + optional extra axes (ext1-3)
//! - Programs have sequences for approach, main, and retreat
//! - Each sequence can have multiple instructions

use serde::{Deserialize, Serialize};

#[cfg(feature = "ecs")]
use pl3xus_macros::{HasSuccess, Invalidates};
#[cfg(feature = "ecs")]
use pl3xus::RequestMessage;

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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramWithLines {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub lines: Vec<ProgramLineInfo>,
    pub approach_lines: Vec<ProgramLineInfo>,
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
    pub speed: Option<f64>,
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

#[cfg(feature = "ecs")]
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

#[cfg(feature = "ecs")]
impl RequestMessage for GetProgram {
    type ResponseMessage = GetProgramResponse;
}

/// Create a new program.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ecs", derive(Invalidates))]
#[cfg_attr(feature = "ecs", invalidates("ListPrograms"))]
pub struct CreateProgram {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ecs", derive(HasSuccess))]
pub struct CreateProgramResponse {
    pub success: bool,
    pub program_id: Option<i64>,
    pub error: Option<String>,
}

#[cfg(feature = "ecs")]
impl RequestMessage for CreateProgram {
    type ResponseMessage = CreateProgramResponse;
}

/// Delete a program.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ecs", derive(Invalidates))]
#[cfg_attr(feature = "ecs", invalidates("ListPrograms", "GetProgram"))]
pub struct DeleteProgram {
    pub program_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ecs", derive(HasSuccess))]
pub struct DeleteProgramResponse {
    pub success: bool,
    pub error: Option<String>,
}

#[cfg(feature = "ecs")]
impl RequestMessage for DeleteProgram {
    type ResponseMessage = DeleteProgramResponse;
}

/// Update program settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ecs", derive(Invalidates))]
#[cfg_attr(feature = "ecs", invalidates("GetProgram"))]
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
#[cfg_attr(feature = "ecs", derive(HasSuccess))]
pub struct UpdateProgramSettingsResponse {
    pub success: bool,
    pub error: Option<String>,
}

#[cfg(feature = "ecs")]
impl RequestMessage for UpdateProgramSettings {
    type ResponseMessage = UpdateProgramSettingsResponse;
}

/// Upload CSV data to a program.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ecs", derive(Invalidates))]
#[cfg_attr(feature = "ecs", invalidates("GetProgram", "ListPrograms"))]
pub struct UploadCsv {
    pub program_id: i64,
    pub csv_content: String,
    /// Which sequence to upload to (defaults to Main)
    pub sequence_type: Option<SequenceType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ecs", derive(HasSuccess))]
pub struct UploadCsvResponse {
    pub success: bool,
    pub lines_imported: Option<i32>,
    pub warnings: Vec<String>,
    pub error: Option<String>,
}

#[cfg(feature = "ecs")]
impl RequestMessage for UploadCsv {
    type ResponseMessage = UploadCsvResponse;
}

/// Add an approach/retreat sequence to a program.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ecs", derive(Invalidates))]
#[cfg_attr(feature = "ecs", invalidates("GetProgram"))]
pub struct AddSequence {
    pub program_id: i64,
    pub sequence_type: SequenceType,
    pub name: Option<String>,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ecs", derive(HasSuccess))]
pub struct AddSequenceResponse {
    pub success: bool,
    pub sequence_id: Option<i64>,
    pub error: Option<String>,
}

#[cfg(feature = "ecs")]
impl RequestMessage for AddSequence {
    type ResponseMessage = AddSequenceResponse;
}

/// Remove a sequence from a program.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ecs", derive(Invalidates))]
#[cfg_attr(feature = "ecs", invalidates("GetProgram"))]
pub struct RemoveSequence {
    pub sequence_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ecs", derive(HasSuccess))]
pub struct RemoveSequenceResponse {
    pub success: bool,
    pub error: Option<String>,
}

#[cfg(feature = "ecs")]
impl RequestMessage for RemoveSequence {
    type ResponseMessage = RemoveSequenceResponse;
}

