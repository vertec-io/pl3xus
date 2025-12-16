//! Shared types for FANUC RMI Replica
//!
//! # Philosophy
//! - **Synced Components**: Wrapped `fanuc_rmi::dto` types (Newtype pattern) to implement `Component`.
//! - **Network Messages**: Direct usages of `fanuc_rmi::dto` types where possible, custom types for App logic.
//! - **DTOs**: Data transfer objects for API communication.

use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};

#[cfg(feature = "server")]
use bevy::prelude::*;

// Re-export FANUC DTO types for easy access
pub use fanuc_rmi::dto;

// ============================================================================
//                          SYNCED COMPONENTS (Wrapped DTOs)
// ============================================================================

/// Robot cartesian position (Synced 1-way: Server -> Client)
#[cfg_attr(feature = "server", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RobotPosition(pub dto::Position);

impl Default for RobotPosition {
    fn default() -> Self {
        Self(dto::Position {
            x: 0.0, y: 0.0, z: 400.0,
            w: 0.0, p: 0.0, r: 0.0,
            ext1: 0.0, ext2: 0.0, ext3: 0.0,
        })
    }
}

impl Deref for RobotPosition {
    type Target = dto::Position;
    fn deref(&self) -> &Self::Target { &self.0 }
}
impl DerefMut for RobotPosition {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

/// Robot joint angles (Synced 1-way: Server -> Client)
#[cfg_attr(feature = "server", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct JointAngles(pub dto::JointAngles);

impl Default for JointAngles {
    fn default() -> Self {
        Self(dto::JointAngles {
            j1: 0.0, j2: 0.0, j3: 0.0,
            j4: 0.0, j5: 0.0, j6: 0.0,
            j7: 0.0, j8: 0.0, j9: 0.0,
        })
    }
}

impl Deref for JointAngles {
    type Target = dto::JointAngles;
    fn deref(&self) -> &Self::Target { &self.0 }
}
impl DerefMut for JointAngles {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

/// Robot operational status (Synced 1-way: Server -> Client)
#[cfg_attr(feature = "server", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RobotStatus {
    pub servo_ready: bool,
    pub tp_enabled: bool,
    pub in_motion: bool,
    pub speed_override: u8,
    pub error_message: Option<String>,
}

impl Default for RobotStatus {
    fn default() -> Self {
        Self { servo_ready: true, tp_enabled: false, in_motion: false, speed_override: 100, error_message: None }
    }
}

/// Execution state for program running (Synced 1-way: Server -> Client)
#[cfg_attr(feature = "server", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct ExecutionState {
    pub loaded_program_id: Option<i64>,
    pub loaded_program_name: Option<String>,
    pub running: bool,
    pub paused: bool,
    pub current_line: usize,
    pub total_lines: usize,
}

/// Connection state (Synced 1-way: Server -> Client)
#[cfg_attr(feature = "server", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct ConnectionState {
    pub robot_connected: bool,
    pub robot_addr: String,
    pub connection_name: Option<String>,
    pub connection_id: Option<i64>,
    pub tp_initialized: bool,
}

/// Active configuration state (Synced 1-way: Server -> Client)
#[cfg_attr(feature = "server", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct ActiveConfigState {
    pub loaded_from_id: Option<i64>,
    pub loaded_from_name: Option<String>,
    pub changes_count: u32,
    pub u_frame_number: i32,
    pub u_tool_number: i32,
    pub front: i32,
    pub up: i32,
    pub left: i32,
    pub flip: i32,
    pub turn4: i32,
    pub turn5: i32,
    pub turn6: i32,
}

/// Active jog settings state (Synced 1-way: Server -> Client)
#[cfg_attr(feature = "server", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct JogSettingsState {
    pub cartesian_jog_speed: f64,
    pub cartesian_jog_step: f64,
    pub joint_jog_speed: f64,
    pub joint_jog_step: f64,
    pub rotation_jog_speed: f64,
    pub rotation_jog_step: f64,
}

impl Default for JogSettingsState {
    fn default() -> Self {
        Self {
            cartesian_jog_speed: 10.0,
            cartesian_jog_step: 1.0,
            joint_jog_speed: 0.1,
            joint_jog_step: 0.25,
            rotation_jog_speed: 5.0,
            rotation_jog_step: 1.0,
        }
    }
}

/// Console log entry (broadcast message for console display)
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ConsoleLogEntry {
    pub timestamp: String,
    pub timestamp_ms: u64,
    pub direction: ConsoleDirection,
    pub msg_type: ConsoleMsgType,
    pub content: String,
    pub sequence_id: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum ConsoleDirection {
    Sent,
    Received,
    System,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum ConsoleMsgType {
    Command,
    Response,
    Error,
    Status,
    Config,
}

/// Control ownership state
///
/// **DEPRECATED**: Use `pl3xus_sync::control::EntityControl` on server
/// or `RobotControlState` in this crate for the client.
#[cfg_attr(feature = "server", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[deprecated(note = "Use RobotControlState or pl3xus_sync::control::EntityControl")]
pub struct ControlStatus {
    pub holder_id: Option<String>,
    pub locked_at: Option<u64>,
}

/// **DEPRECATED**: Use `pl3xus_common::EntityControl` directly.
/// It's available on both server (with ecs feature) and client.
#[cfg_attr(feature = "server", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[deprecated(note = "Use pl3xus_common::EntityControl directly")]
pub struct RobotControlState {
    /// The client ID that currently has control (matches ConnectionId structure)
    pub client_id: ClientId,
    /// Timestamp of last activity
    pub last_activity: f32,
}

/// **DEPRECATED**: Use `pl3xus_common::ConnectionId` directly.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Default)]
#[deprecated(note = "Use pl3xus_common::ConnectionId directly")]
pub struct ClientId {
    pub id: u32,
}

/// **DEPRECATED**: Use `pl3xus_common::ControlRequest::Take(entity_bits)` directly.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[deprecated(note = "Use pl3xus_common::ControlRequest::Take(entity_bits) directly")]
pub struct ControlTakeRequest {
    /// Entity bits (from Entity::to_bits())
    pub entity_bits: u64,
}

/// **DEPRECATED**: Use `pl3xus_common::ControlRequest::Release(entity_bits)` directly.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[deprecated(note = "Use pl3xus_common::ControlRequest::Release(entity_bits) directly")]
pub struct ControlReleaseRequest {
    /// Entity bits (from Entity::to_bits())
    pub entity_bits: u64,
}

/// I/O Status
#[cfg_attr(feature = "server", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct IoStatus {
    pub digital_inputs: Vec<u16>,
    pub digital_outputs: Vec<u16>,
}

// ============================================================================
//                          DATA TRANSFER OBJECTS (DTOs)
// ============================================================================

/// Robot connection DTO (for saved connections).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RobotConnectionDto {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub ip_address: String,
    pub port: u32,
    pub default_speed: f64,
    pub default_speed_type: String,
    pub default_term_type: String,
    pub default_w: f64,
    pub default_p: f64,
    pub default_r: f64,
    pub default_cartesian_jog_speed: f64,
    pub default_cartesian_jog_step: f64,
    pub default_joint_jog_speed: f64,
    pub default_joint_jog_step: f64,
}

/// Robot configuration DTO (named configurations per robot).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobotConfigurationDto {
    pub id: i64,
    pub robot_connection_id: i64,
    pub name: String,
    pub is_default: bool,
    pub u_frame_number: i32,
    pub u_tool_number: i32,
    pub front: i32,
    pub up: i32,
    pub left: i32,
    pub flip: i32,
    pub turn4: i32,
    pub turn5: i32,
    pub turn6: i32,
}

/// New robot configuration DTO (for creating without ID).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewRobotConfigurationDto {
    pub name: String,
    pub is_default: bool,
    pub u_frame_number: i32,
    pub u_tool_number: i32,
    pub front: i32,
    pub up: i32,
    pub left: i32,
    pub flip: i32,
    pub turn4: i32,
    pub turn5: i32,
    pub turn6: i32,
}

/// Optional start position for program execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartPosition {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Program summary info for listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramInfo {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub instruction_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// Full program detail including instructions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramDetail {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub instructions: Vec<InstructionDto>,
    pub default_term_type: String,
    pub default_term_value: Option<u8>,
    pub start_x: Option<f64>,
    pub start_y: Option<f64>,
    pub start_z: Option<f64>,
    pub start_w: Option<f64>,
    pub start_p: Option<f64>,
    pub start_r: Option<f64>,
    pub end_x: Option<f64>,
    pub end_y: Option<f64>,
    pub end_z: Option<f64>,
    pub end_w: Option<f64>,
    pub end_p: Option<f64>,
    pub end_r: Option<f64>,
    pub move_speed: Option<f64>,
    pub created_at: String,
    pub updated_at: String,
}

/// Instruction DTO for client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionDto {
    pub line_number: i32,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: Option<f64>,
    pub p: Option<f64>,
    pub r: Option<f64>,
    pub speed: Option<f64>,
    pub term_type: Option<String>,
    pub term_value: Option<u8>,
    pub uframe: Option<i32>,
    pub utool: Option<i32>,
}

/// Robot settings DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobotSettingsDto {
    pub default_w: f64,
    pub default_p: f64,
    pub default_r: f64,
    pub default_speed: f64,
    pub default_term_type: String,
    pub default_uframe: i32,
    pub default_utool: i32,
}

/// A single change entry in the changelog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeLogEntryDto {
    pub field_name: String,
    pub old_value: String,
    pub new_value: String,
}

/// I/O display configuration DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoDisplayConfigDto {
    pub io_type: String,
    pub io_index: i32,
    pub display_name: Option<String>,
    pub is_visible: bool,
    pub display_order: Option<i32>,
}

/// Active jog settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveJogSettings {
    pub cartesian_jog_speed: f64,
    pub cartesian_jog_step: f64,
    pub joint_jog_speed: f64,
    pub joint_jog_step: f64,
}

/// Active configuration state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveConfiguration {
    pub loaded_from_id: Option<i64>,
    pub loaded_from_name: Option<String>,
    pub changes_count: u32,
    pub change_log: Vec<ChangeLogEntryDto>,
    pub u_frame_number: i32,
    pub u_tool_number: i32,
    pub front: i32,
    pub up: i32,
    pub left: i32,
    pub flip: i32,
    pub turn4: i32,
    pub turn5: i32,
    pub turn6: i32,
}

// ============================================================================
//                          NETWORK MESSAGES (RPC)
// ============================================================================

/// **DEPRECATED**: Use `pl3xus_sync::control::ControlRequest::Take` instead.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[deprecated(note = "Use pl3xus_sync::control::ControlRequest instead")]
pub struct RequestControl;

/// **DEPRECATED**: Use `pl3xus_sync::control::ControlRequest::Release` instead.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[deprecated(note = "Use pl3xus_sync::control::ControlRequest instead")]
pub struct ReleaseControl;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum JogAxis { X, Y, Z, W, P, R, J1, J2, J3, J4, J5, J6 }

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum JogDirection { Positive, Negative }

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JogCommand {
    pub axis: JogAxis,
    pub direction: JogDirection,
    pub distance: f32,
    pub speed: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExecuteProgram {
    pub program_id: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StopExecution;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ListPrograms;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GetProgram {
    pub program_id: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ListRobotConnections;

/// Request to connect to a robot.
/// Can either provide a saved connection_id or direct connection details.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConnectToRobot {
    /// Database ID of saved robot connection (optional)
    pub connection_id: Option<i64>,
    /// Direct connection address (used if connection_id is None)
    pub addr: String,
    /// Connection port (default: 16001)
    pub port: u32,
    /// Optional name for this connection
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DisconnectRobot;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GetActiveConfiguration;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LoadConfiguration {
    pub configuration_id: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateJogSettings {
    pub cartesian_jog_speed: f64,
    pub cartesian_jog_step: f64,
    pub joint_jog_speed: f64,
    pub joint_jog_step: f64,
    pub rotation_jog_speed: f64,
    pub rotation_jog_step: f64,
}

// ============================================================================
//                    ADDITIONAL NETWORK MESSAGES (Complete API)
// ============================================================================

// Robot Control Commands
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InitializeRobot {
    pub group_mask: Option<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ResetRobot;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AbortMotion;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SetSpeedOverride {
    pub speed: u8, // 0-100%
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContinueMotion;

// Motion Commands (Command Composer)
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum MotionType {
    Absolute,
    Relative,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum TermType {
    CNT,
    FINE,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LinearMotionCommand {
    pub motion_type: MotionType,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
    pub p: f64,
    pub r: f64,
    pub speed: f64,
    pub speed_type: String, // mmSec, InchMin, Time, mSec
    pub term_type: TermType,
    pub term_value: u8,
    pub uframe: Option<i32>,
    pub utool: Option<i32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JointMotionCommand {
    pub motion_type: MotionType,
    pub j1: f64,
    pub j2: f64,
    pub j3: f64,
    pub j4: f64,
    pub j5: f64,
    pub j6: f64,
    pub speed: f64,
    pub term_type: TermType,
    pub term_value: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CircularMotionCommand {
    pub motion_type: MotionType,
    // Via point
    pub via_x: f64,
    pub via_y: f64,
    pub via_z: f64,
    // End point
    pub end_x: f64,
    pub end_y: f64,
    pub end_z: f64,
    pub w: f64,
    pub p: f64,
    pub r: f64,
    pub speed: f64,
    pub speed_type: String,
    pub term_type: TermType,
    pub term_value: u8,
}

// Rotation Jog (separate from cartesian jog)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RotationJogCommand {
    pub axis: u8, // 0=W, 1=P, 2=R
    pub direction: JogDirection,
    pub distance: f64,
    pub speed: f64,
}

// Program Management
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CreateProgram {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateProgram {
    pub program_id: i64,
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DeleteProgram {
    pub program_id: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LoadProgram {
    pub program_id: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UnloadProgram;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StartProgram {
    pub program_id: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PauseProgram;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ResumeProgram;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StopProgram;

// CSV Upload
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UploadCsv {
    pub program_id: i64,
    pub csv_content: String,
}

// Frame/Tool Management
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SetActiveFrameTool {
    pub uframe: i32,
    pub utool: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GetFrameData {
    pub frame_number: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GetToolData {
    pub tool_number: i32,
}

// Robot Connection Management
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CreateRobotConnection {
    pub name: String,
    pub description: Option<String>,
    pub ip_address: String,
    pub port: u32,
    pub configurations: Vec<NewRobotConfigurationDto>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateRobotConnection {
    pub id: i64,
    pub name: Option<String>,
    pub description: Option<String>,
    pub ip_address: Option<String>,
    pub port: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DeleteRobotConnection {
    pub id: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GetRobotConfigurations {
    pub robot_connection_id: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SaveConfiguration {
    pub name: Option<String>, // If provided, saves as new config
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RevertConfiguration;

// Configuration updates
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateActiveConfig {
    pub u_frame_number: Option<i32>,
    pub u_tool_number: Option<i32>,
    pub front: Option<i32>,
    pub up: Option<i32>,
    pub left: Option<i32>,
    pub flip: Option<i32>,
    pub turn4: Option<i32>,
    pub turn5: Option<i32>,
    pub turn6: Option<i32>,
}

// Response Messages (from server to client)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ApiResponse {
    pub success: bool,
    pub message: String,
    pub request_type: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProgramListResponse {
    pub programs: Vec<ProgramInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProgramDetailResponse {
    pub program: ProgramDetail,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RobotConnectionsResponse {
    pub connections: Vec<RobotConnectionDto>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RobotConfigurationsResponse {
    pub robot_id: i64,
    pub configurations: Vec<RobotConfigurationDto>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FrameDataResponse {
    pub frame_number: i32,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
    pub p: f64,
    pub r: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ToolDataResponse {
    pub tool_number: i32,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
    pub p: f64,
    pub r: f64,
}

// Command log entry (for Command Log panel)
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CommandLogEntry {
    pub timestamp: String,
    pub timestamp_ms: u64,
    pub command_type: CommandType,
    pub description: String,
    pub command_data: String, // JSON serialized command for re-run
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandType {
    LinearAbsolute,
    LinearRelative,
    JointAbsolute,
    JointRelative,
    CircularAbsolute,
    CircularRelative,
    Jog,
    RotationJog,
    System, // Initialize, Reset, Abort
}

// I/O Messages
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ReadDin {
    pub index: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WriteDout {
    pub index: i32,
    pub value: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ReadAin {
    pub index: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WriteAout {
    pub index: i32,
    pub value: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IoValueResponse {
    pub io_type: String,
    pub index: i32,
    pub value: String, // Serialized value
}
