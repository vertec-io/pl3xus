//! Shared types for FANUC RMI Replica
//!
//! # Philosophy
//! - **Synced Components**: Wrapped `fanuc_rmi::dto` types (Newtype pattern) to implement `Component`.
//! - **Network Messages**: Direct usages of `fanuc_rmi::dto` types where possible, custom types for App logic.
//! - **DTOs**: Data transfer objects for API communication.
//! - **Request/Response**: Use pl3xus_common::RequestMessage for correlated request/response patterns.
//!
//! # Automatic Query Invalidation
//!
//! Mutation request types use the `#[derive(Invalidates)]` macro (server feature only)
//! to declare which queries should be invalidated on success. This enables automatic
//! cache invalidation without manual broadcasting in handlers.

use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};

#[cfg(feature = "ecs")]
use bevy::prelude::*;

#[cfg(feature = "stores")]
use reactive_stores::Store;

// Re-export FANUC DTO types for easy access
// All features use the same fanuc_rmi crate with different feature flags
pub use fanuc_rmi::dto;

// Re-export RequestMessage and ErrorResponse traits for implementing request types
pub use pl3xus_common::{RequestMessage, ErrorResponse};

// Server-only: automatic query invalidation support
// Only export the derive macros - they generate `impl pl3xus_sync::Invalidates for T`
// and `impl pl3xus_common::HasSuccess for T` respectively
#[cfg(feature = "server")]
pub use pl3xus_macros::Invalidates;

// Re-export HasSuccess derive macro for response types with success: bool field
// This enables the respond_and_invalidate() auto-invalidation pattern
// Server-only because it's only used in server handlers
#[cfg(feature = "server")]
pub use pl3xus_macros::HasSuccess;

// ============================================================================
//                          SYNCED COMPONENTS (Wrapped DTOs)
// ============================================================================

/// Marker component for the active/current System entity.
///
/// This entity is the control root - clients request control of this entity
/// to gain control over the entire apparatus including all child robots, sensors, controllers, etc.
///
/// On the server, query with `With<ActiveSystem>` to find the system entity.
/// On the client, use `use_components::<ActiveSystem>()` to get the system entity ID.
///
/// When multiple systems exist, this marker can be moved to whichever is currently active.
#[cfg_attr(feature = "ecs", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct ActiveSystem;

/// Marker component for the active/current robot entity.
///
/// This is the shared marker for identifying the currently active robot on both server and client.
/// On the server, the `FanucRobot` component is server-only and used for queries.
/// This marker is synced to clients so they can identify the active robot entity.
///
/// When multiple robots exist, this marker can be moved to whichever is currently active/connected.
/// The client sync will automatically update to track the new entity.
///
/// On the server, add this marker when spawning/connecting robot entities.
/// On the client, use `use_components::<ActiveRobot>()` to get the active robot entity ID.
#[cfg_attr(feature = "ecs", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct ActiveRobot;

/// Robot cartesian position (Synced 1-way: Server -> Client)
#[cfg_attr(feature = "ecs", derive(Component))]
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
#[cfg_attr(feature = "ecs", derive(Component))]
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
#[cfg_attr(feature = "ecs", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RobotStatus {
    pub servo_ready: bool,
    pub tp_enabled: bool,
    pub in_motion: bool,
    pub speed_override: u8,
    pub error_message: Option<String>,
    /// Whether the TP program is initialized and ready for motion commands.
    /// This must be true to send motion commands. False after abort/disconnect.
    pub tp_program_initialized: bool,
    /// Active user frame number
    pub active_uframe: u8,
    /// Active user tool number
    pub active_utool: u8,
}

impl Default for RobotStatus {
    fn default() -> Self {
        Self {
            servo_ready: true,
            tp_enabled: false,
            in_motion: false,
            speed_override: 100,
            error_message: None,
            tp_program_initialized: false,
            active_uframe: 0,
            active_utool: 1,
        }
    }
}

/// Execution state for program running (Synced 1-way: Server -> Client)
/// Program execution state - server-authoritative state machine.
/// The server determines the current state and what actions are available.
/// The client simply reflects this state and enables/disables actions accordingly.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ProgramExecutionState {
    /// No program loaded.
    #[default]
    NoProgram,
    /// Program is loaded but not running (ready to start).
    Idle,
    /// Program is actively executing.
    Running,
    /// Program execution is paused (can resume or stop).
    Paused,
    /// Program completed successfully.
    Completed,
    /// Program encountered an error.
    Error,
}

/// Execution state (Synced 1-way: Server -> Client).
///
/// This component follows the **server-driven UI state pattern**:
/// - The server determines the current `state` and what actions are available via `can_*` flags
/// - The client simply reflects these values without any client-side state machine logic
/// - Button visibility is driven directly by the `can_*` flags
///
/// # Example (client)
/// ```rust,ignore
/// let exec = use_sync_component_store::<ExecutionState>();
/// // Access fields with fine-grained reactivity:
/// let can_start = move || exec.can_start().get();
/// let state = move || exec.state().get();
/// ```
#[cfg_attr(feature = "ecs", derive(Component))]
#[cfg_attr(feature = "stores", derive(Store))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ExecutionState {
    pub loaded_program_id: Option<i64>,
    pub loaded_program_name: Option<String>,
    /// Current execution state (server-authoritative state machine)
    pub state: ProgramExecutionState,
    /// Current line being executed (1-based, 0 = not started)
    pub current_line: usize,
    /// Total lines in the program
    pub total_lines: usize,
    /// The program lines for the loaded program (synced to all clients)
    pub program_lines: Vec<ProgramLineInfo>,

    // === Available Actions (server-driven) ===
    // The server determines what actions are valid based on the current state.
    // The client simply enables/disables buttons based on these flags.

    /// Can load a new program (only when no program is loaded or idle)
    pub can_load: bool,
    /// Can start/run the program
    pub can_start: bool,
    /// Can pause the running program
    pub can_pause: bool,
    /// Can resume a paused program
    pub can_resume: bool,
    /// Can stop the running/paused program
    pub can_stop: bool,
    /// Can unload the current program
    pub can_unload: bool,
}

impl Default for ExecutionState {
    fn default() -> Self {
        // Default state is NoProgram - only load action is available
        Self {
            loaded_program_id: None,
            loaded_program_name: None,
            state: ProgramExecutionState::NoProgram,
            current_line: 0,
            total_lines: 0,
            program_lines: Vec::new(),
            // In NoProgram state, only loading is available
            can_load: true,
            can_start: false,
            can_pause: false,
            can_resume: false,
            can_stop: false,
            can_unload: false,
        }
    }
}

/// Connection state (Synced 1-way: Server -> Client)
#[cfg_attr(feature = "ecs", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct ConnectionState {
    /// Whether a robot is currently connected
    pub robot_connected: bool,
    /// Whether a robot connection is in progress
    pub robot_connecting: bool,
    /// The robot's address (IP:port)
    pub robot_addr: String,
    /// The robot's display name
    pub robot_name: String,
    /// The saved connection name (if connected via saved connection)
    pub connection_name: Option<String>,
    /// The saved connection ID (if connected via saved connection)
    pub connection_id: Option<i64>,
    /// The active connection ID for the current session
    pub active_connection_id: Option<i64>,
    /// Whether the TP (teach pendant) is initialized
    pub tp_initialized: bool,
}

/// A single change entry in the active config changelog.
/// Tracks field-level changes made since the configuration was loaded.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfigChangeEntry {
    pub field_name: String,
    pub old_value: String,
    pub new_value: String,
}

/// Active configuration state (Synced 1-way: Server -> Client)
///
/// Tracks the currently loaded configuration and any changes made since loading.
/// When `changes_count > 0`, the UI shows a warning and Save/Revert buttons.
/// The `change_log` stores detailed changes for display in the save modal.
#[cfg_attr(feature = "ecs", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct ActiveConfigState {
    /// ID of the configuration this was loaded from (None if new/unsaved)
    pub loaded_from_id: Option<i64>,
    /// Name of the configuration this was loaded from
    pub loaded_from_name: Option<String>,
    /// Number of changes made since loading (0 = no changes)
    pub changes_count: u32,
    /// Detailed log of each change made since loading
    pub change_log: Vec<ConfigChangeEntry>,
    // Current active values:
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

/// Tracks synchronization state between ActiveConfigState and the actual robot.
///
/// When a user sets active frame/tool, we need to verify the robot accepted the change.
/// Polling detects mismatches and triggers resync attempts.
#[cfg_attr(feature = "ecs", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct ActiveConfigSyncState {
    /// Current sync status
    pub status: ConfigSyncStatus,
    /// Number of consecutive sync attempts made
    pub retry_count: u32,
    /// Maximum retries before giving up (default: 3)
    pub max_retries: u32,
    /// When true, a sync operation is currently in progress (prevent concurrent syncs)
    pub sync_in_progress: bool,
    /// Human-readable error message if sync failed
    pub error_message: Option<String>,
}

impl ActiveConfigSyncState {
    pub fn new() -> Self {
        Self {
            status: ConfigSyncStatus::Synced,
            retry_count: 0,
            max_retries: 3,
            sync_in_progress: false,
            error_message: None,
        }
    }
}

/// Status of active config synchronization with robot.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub enum ConfigSyncStatus {
    /// Robot values match ActiveConfigState values
    #[default]
    Synced,
    /// Robot values differ from ActiveConfigState, will attempt resync
    Mismatch,
    /// Currently retrying to sync values to robot
    Retrying,
    /// Sync failed after max retries - user intervention required
    Failed,
}

/// Active jog settings state (Synced 1-way: Server -> Client)
#[cfg_attr(feature = "ecs", derive(Component))]
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
            cartesian_jog_speed: 10.0, // mm/s
            cartesian_jog_step: 1.0,   // mm
            joint_jog_speed: 10.0,     // °/s
            joint_jog_step: 1.0,       // degrees
            rotation_jog_speed: 5.0,   // °/s
            rotation_jog_step: 1.0,    // degrees
        }
    }
}

/// Console log entry (broadcast message for console display)
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct ConsoleLogEntry {
    pub timestamp: String,
    pub timestamp_ms: u64,
    pub direction: ConsoleDirection,
    pub msg_type: ConsoleMsgType,
    pub content: String,
    pub sequence_id: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub enum ConsoleDirection {
    #[default]
    Sent,
    Received,
    System,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub enum ConsoleMsgType {
    #[default]
    Command,
    Response,
    Error,
    Status,
    Config,
}

/// Program notification (broadcast message from server to all clients).
///
/// Used for server-initiated notifications about program events like
/// completion, errors, or other state changes. All connected clients
/// receive this message and can display appropriate UI feedback.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct ProgramNotification {
    /// Unique sequence number to distinguish identical notifications
    pub sequence: u64,
    /// The type of notification
    pub kind: ProgramNotificationKind,
}

/// Kind of program notification
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub enum ProgramNotificationKind {
    #[default]
    None,
    /// Program completed successfully
    Completed {
        program_name: String,
        total_instructions: usize,
    },
    /// Program was stopped by user
    Stopped {
        program_name: String,
        at_line: usize,
    },
    /// Program encountered an error
    Error {
        program_name: String,
        at_line: usize,
        error_message: String,
    },
}

/// I/O Status - contains all I/O types
/// Digital I/O: Each u16 represents 16 bits (ports 1-16, 17-32, etc.)
/// Analog I/O: HashMap of port number to value
/// Group I/O: HashMap of port number to value
#[cfg_attr(feature = "ecs", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct IoStatus {
    pub digital_inputs: Vec<u16>,
    pub digital_outputs: Vec<u16>,
    pub analog_inputs: std::collections::HashMap<u16, f64>,
    pub analog_outputs: std::collections::HashMap<u16, f64>,
    pub group_inputs: std::collections::HashMap<u16, u32>,
    pub group_outputs: std::collections::HashMap<u16, u32>,
}

/// I/O display configuration state - synced component.
/// Stores display names and visibility settings for I/O ports.
/// Key: (io_type, io_index) where io_type is "DIN", "DOUT", "AIN", "AOUT", "GIN", "GOUT"
#[cfg_attr(feature = "ecs", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct IoConfigState {
    pub configs: std::collections::HashMap<(String, i32), IoDisplayConfig>,
}

impl IoConfigState {
    /// Get display name for an I/O port, returns port number as string if not configured.
    pub fn get_display_name(&self, io_type: &str, port: u16) -> String {
        if let Some(cfg) = self.configs.get(&(io_type.to_string(), port as i32)) {
            if let Some(ref name) = cfg.display_name {
                return name.clone();
            }
        }
        port.to_string()
    }

    /// Check if a port is visible (defaults to true if not configured).
    pub fn is_port_visible(&self, io_type: &str, port: u16) -> bool {
        if let Some(cfg) = self.configs.get(&(io_type.to_string(), port as i32)) {
            return cfg.is_visible;
        }
        true
    }
}

/// Frame/Tool data state - synced component.
/// Stores all frame (1-9) and tool (1-10) data read from the robot.
/// Also tracks active frame/tool indices.
#[cfg_attr(feature = "ecs", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct FrameToolDataState {
    /// Active user frame (0-9)
    pub active_frame: i32,
    /// Active user tool (1-10)
    pub active_tool: i32,
    /// Frame data: key is frame number (1-9), value is (x, y, z, w, p, r)
    pub frames: std::collections::HashMap<i32, FrameToolData>,
    /// Tool data: key is tool number (1-10), value is (x, y, z, w, p, r)
    pub tools: std::collections::HashMap<i32, FrameToolData>,
}

/// Frame or tool position/orientation data.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct FrameToolData {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
    pub p: f64,
    pub r: f64,
}

impl FrameToolDataState {
    /// Get frame data by number, returns zeros if not loaded.
    pub fn get_frame(&self, frame_num: i32) -> FrameToolData {
        self.frames.get(&frame_num).cloned().unwrap_or_default()
    }

    /// Get tool data by number, returns zeros if not loaded.
    pub fn get_tool(&self, tool_num: i32) -> FrameToolData {
        self.tools.get(&tool_num).cloned().unwrap_or_default()
    }
}

// ============================================================================
//                          DATA TRANSFER OBJECTS (DTOs)
// ============================================================================

/// Saved robot connection from database.
/// Motion defaults (speed, term_type, w/p/r) and jog defaults are stored here.
/// Frame/tool/arm configuration is stored in robot_configurations table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RobotConnection {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub ip_address: String,
    pub port: u32,
    // Motion defaults (required - no global fallback)
    pub default_speed: f64,
    pub default_speed_type: String,  // mmSec, InchMin, Time, mSec
    pub default_term_type: String,
    pub default_w: f64,
    pub default_p: f64,
    pub default_r: f64,
    // Jog defaults
    pub default_cartesian_jog_speed: f64,
    pub default_cartesian_jog_step: f64,
    pub default_joint_jog_speed: f64,
    pub default_joint_jog_step: f64,
    pub default_rotation_jog_speed: f64,
    pub default_rotation_jog_step: f64,
}

/// Named configuration for a robot (frame, tool, arm config).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RobotConfiguration {
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

/// New robot configuration (for creating without ID).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewRobotConfiguration {
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
    pub instructions: Vec<Instruction>,
    // Program defaults for motion - these are required, non-optional fields
    pub default_w: f64,
    pub default_p: f64,
    pub default_r: f64,
    pub default_speed: Option<f64>,
    pub default_speed_type: String,        // Required: "mmSec" or "percent"
    pub default_term_type: String,         // Required: "CNT" or "FINE"
    pub default_term_value: u8,            // Required: 0-100 for CNT, 0 for FINE
    pub default_uframe: Option<i32>,
    pub default_utool: Option<i32>,
    // Approach/retreat positions (optional - not all programs have approach/retreat)
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
    pub move_speed: f64,                   // Required: speed for approach/retreat moves
    pub created_at: String,
    pub updated_at: String,
}

/// Program instruction (motion command).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instruction {
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

/// Robot default settings.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RobotSettings {
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
pub struct ChangeLogEntry {
    pub field_name: String,
    pub old_value: String,
    pub new_value: String,
}

/// I/O display configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IoDisplayConfig {
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
    pub change_log: Vec<ChangeLogEntry>,
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

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum JogAxis { X, Y, Z, W, P, R, J1, J2, J3, J4, J5, J6 }

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum JogDirection { Positive, Negative }

/// Jog command sent from client to server.
///
/// Only contains axis and direction - the server uses its own JogSettingsState
/// component for speed and step values. This ensures jog settings are tied to
/// the robot entity, not the client, so any client that takes control uses
/// the same settings.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JogCommand {
    pub axis: JogAxis,
    pub direction: JogDirection,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ListPrograms;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ListProgramsResponse {
    pub programs: Vec<ProgramWithLines>,
}

/// Program with lines for loading into the program display
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ProgramWithLines {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub lines: Vec<ProgramLineInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ProgramLineInfo {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
    pub p: f64,
    pub r: f64,
    pub speed: f64,
    pub term_type: String,
    pub uframe: Option<i32>,
    pub utool: Option<i32>,
}

impl RequestMessage for ListPrograms {
    type ResponseMessage = ListProgramsResponse;
}

/// Request to get a single program with all its instructions.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GetProgram {
    pub program_id: i64,
}

/// Response for getting a single program.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GetProgramResponse {
    pub program: Option<ProgramDetail>,
}

impl RequestMessage for GetProgram {
    type ResponseMessage = GetProgramResponse;
}

/// Request to create a new program.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "server", derive(Invalidates))]
#[cfg_attr(feature = "server", invalidates("ListPrograms"))]
pub struct CreateProgram {
    pub name: String,
    pub description: Option<String>,
}

/// Response for creating a program.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "server", derive(HasSuccess))]
pub struct CreateProgramResponse {
    pub success: bool,
    pub program_id: Option<i64>,
    pub error: Option<String>,
}

impl RequestMessage for CreateProgram {
    type ResponseMessage = CreateProgramResponse;
}

/// Request to delete a program.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "server", derive(Invalidates))]
#[cfg_attr(feature = "server", invalidates("ListPrograms", "GetProgram"))]
pub struct DeleteProgram {
    pub program_id: i64,
}

/// Response for deleting a program.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "server", derive(HasSuccess))]
pub struct DeleteProgramResponse {
    pub success: bool,
    pub error: Option<String>,
}

impl RequestMessage for DeleteProgram {
    type ResponseMessage = DeleteProgramResponse;
}

/// Request to update program settings (start/end positions, speed, termination).
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "server", derive(Invalidates))]
#[cfg_attr(feature = "server", invalidates("GetProgram"))]
pub struct UpdateProgramSettings {
    pub program_id: i64,
    // Start position (approach move before toolpath)
    pub start_x: Option<f64>,
    pub start_y: Option<f64>,
    pub start_z: Option<f64>,
    pub start_w: Option<f64>,
    pub start_p: Option<f64>,
    pub start_r: Option<f64>,
    // End position (retreat move after toolpath)
    pub end_x: Option<f64>,
    pub end_y: Option<f64>,
    pub end_z: Option<f64>,
    pub end_w: Option<f64>,
    pub end_p: Option<f64>,
    pub end_r: Option<f64>,
    // Motion defaults
    pub move_speed: Option<f64>,
    pub default_term_type: Option<String>,
    pub default_term_value: Option<u8>,
}

/// Response for updating program settings.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "server", derive(HasSuccess))]
pub struct UpdateProgramSettingsResponse {
    pub success: bool,
    pub error: Option<String>,
}

impl RequestMessage for UpdateProgramSettings {
    type ResponseMessage = UpdateProgramSettingsResponse;
}

/// Request to upload CSV content to a program.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "server", derive(Invalidates))]
#[cfg_attr(feature = "server", invalidates("GetProgram"))]
pub struct UploadCsv {
    pub program_id: i64,
    pub csv_content: String,
    /// Optional start position to prepend before CSV points
    pub start_position: Option<CsvStartPosition>,
}

/// Start position for CSV upload.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CsvStartPosition {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
    pub p: f64,
    pub r: f64,
}

/// Response for uploading CSV.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "server", derive(HasSuccess))]
pub struct UploadCsvResponse {
    pub success: bool,
    pub lines_imported: Option<i32>,
    pub error: Option<String>,
}

impl RequestMessage for UploadCsv {
    type ResponseMessage = UploadCsvResponse;
}

/// Request to unload the currently loaded program.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UnloadProgram;

/// Response for unloading a program.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UnloadProgramResponse {
    pub success: bool,
    pub error: Option<String>,
}

impl RequestMessage for UnloadProgram {
    type ResponseMessage = UnloadProgramResponse;
}

impl ErrorResponse for UnloadProgram {
    fn error_response(error: String) -> Self::ResponseMessage {
        UnloadProgramResponse { success: false, error: Some(error) }
    }
}

/// Request to list all saved robot connections from the database.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ListRobotConnections;

impl RequestMessage for ListRobotConnections {
    type ResponseMessage = RobotConnectionsResponse;
}

/// Request to create a new robot connection with configurations.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "server", derive(Invalidates))]
#[cfg_attr(feature = "server", invalidates("ListRobotConnections"))]
pub struct CreateRobotConnection {
    // Connection details
    pub name: String,
    pub description: Option<String>,
    pub ip_address: String,
    pub port: u32,
    // Motion defaults
    pub default_speed: f64,
    pub default_speed_type: String,
    pub default_term_type: String,
    pub default_w: f64,
    pub default_p: f64,
    pub default_r: f64,
    // Jog defaults
    pub default_cartesian_jog_speed: f64,
    pub default_cartesian_jog_step: f64,
    pub default_joint_jog_speed: f64,
    pub default_joint_jog_step: f64,
    pub default_rotation_jog_speed: f64,
    pub default_rotation_jog_step: f64,
    // Initial configuration
    pub configuration: NewRobotConfiguration,
}

impl RequestMessage for CreateRobotConnection {
    type ResponseMessage = CreateRobotConnectionResponse;
}

/// Response for creating a robot connection.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "server", derive(HasSuccess))]
pub struct CreateRobotConnectionResponse {
    pub robot_id: i64,
    pub success: bool,
    pub error: Option<String>,
}

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

// Robot Control Commands (Targeted Requests)
// These are sent to a specific robot entity and require authorization.

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InitializeRobot {
    pub group_mask: Option<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InitializeRobotResponse {
    pub success: bool,
    pub error: Option<String>,
}

impl RequestMessage for InitializeRobot {
    type ResponseMessage = InitializeRobotResponse;
}

impl ErrorResponse for InitializeRobot {
    fn error_response(error: String) -> Self::ResponseMessage {
        InitializeRobotResponse { success: false, error: Some(error) }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ResetRobot;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ResetRobotResponse {
    pub success: bool,
    pub error: Option<String>,
}

impl RequestMessage for ResetRobot {
    type ResponseMessage = ResetRobotResponse;
}

impl ErrorResponse for ResetRobot {
    fn error_response(error: String) -> Self::ResponseMessage {
        ResetRobotResponse { success: false, error: Some(error) }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AbortMotion;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AbortMotionResponse {
    pub success: bool,
    pub error: Option<String>,
}

impl RequestMessage for AbortMotion {
    type ResponseMessage = AbortMotionResponse;
}

impl ErrorResponse for AbortMotion {
    fn error_response(error: String) -> Self::ResponseMessage {
        AbortMotionResponse { success: false, error: Some(error) }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SetSpeedOverride {
    pub speed: u8, // 0-100%
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SetSpeedOverrideResponse {
    pub success: bool,
    pub error: Option<String>,
}

impl RequestMessage for SetSpeedOverride {
    type ResponseMessage = SetSpeedOverrideResponse;
}

impl ErrorResponse for SetSpeedOverride {
    fn error_response(error: String) -> Self::ResponseMessage {
        SetSpeedOverrideResponse { success: false, error: Some(error) }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContinueMotion;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContinueMotionResponse {
    pub success: bool,
    pub error: Option<String>,
}

impl RequestMessage for ContinueMotion {
    type ResponseMessage = ContinueMotionResponse;
}

// ConnectToRobot is a request/response that returns the robot entity ID on success
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConnectToRobotResponse {
    pub success: bool,
    /// The robot entity ID (available immediately when connection starts, not when it completes)
    pub entity_id: Option<u64>,
    pub error: Option<String>,
}

impl RequestMessage for ConnectToRobot {
    type ResponseMessage = ConnectToRobotResponse;
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DisconnectRobotResponse {
    pub success: bool,
    pub error: Option<String>,
}

impl RequestMessage for DisconnectRobot {
    type ResponseMessage = DisconnectRobotResponse;
}

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

// Simple motion commands for Command Composer
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MoveLinear {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
    pub p: f32,
    pub r: f32,
    pub speed: f32,
    pub uframe: Option<u8>,
    pub utool: Option<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MoveJoint {
    pub j1: f32,
    pub j2: f32,
    pub j3: f32,
    pub j4: f32,
    pub j5: f32,
    pub j6: f32,
    pub speed: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MoveRelative {
    pub dx: f32,
    pub dy: f32,
    pub dz: f32,
    pub dw: f32,
    pub dp: f32,
    pub dr: f32,
    pub speed: f32,
}

// Program Execution (defined with RequestMessage impls above for CRUD types)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LoadProgram {
    pub program_id: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LoadProgramResponse {
    pub success: bool,
    /// The loaded program info (if successful)
    pub program: Option<ProgramWithLines>,
    pub error: Option<String>,
}

impl RequestMessage for LoadProgram {
    type ResponseMessage = LoadProgramResponse;
}

impl ErrorResponse for LoadProgram {
    fn error_response(error: String) -> Self::ResponseMessage {
        LoadProgramResponse { success: false, program: None, error: Some(error) }
    }
}

/// Start executing the currently loaded program.
/// A program must be loaded first via LoadProgram.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct StartProgram;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StartProgramResponse {
    pub success: bool,
    pub error: Option<String>,
}

impl RequestMessage for StartProgram {
    type ResponseMessage = StartProgramResponse;
}

impl ErrorResponse for StartProgram {
    fn error_response(error: String) -> Self::ResponseMessage {
        StartProgramResponse { success: false, error: Some(error) }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PauseProgram;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PauseProgramResponse {
    pub success: bool,
    pub error: Option<String>,
}

impl RequestMessage for PauseProgram {
    type ResponseMessage = PauseProgramResponse;
}

impl ErrorResponse for PauseProgram {
    fn error_response(error: String) -> Self::ResponseMessage {
        PauseProgramResponse { success: false, error: Some(error) }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ResumeProgram;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ResumeProgramResponse {
    pub success: bool,
    pub error: Option<String>,
}

impl RequestMessage for ResumeProgram {
    type ResponseMessage = ResumeProgramResponse;
}

impl ErrorResponse for ResumeProgram {
    fn error_response(error: String) -> Self::ResponseMessage {
        ResumeProgramResponse { success: false, error: Some(error) }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StopProgram;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StopProgramResponse {
    pub success: bool,
    pub error: Option<String>,
}

impl RequestMessage for StopProgram {
    type ResponseMessage = StopProgramResponse;
}

impl ErrorResponse for StopProgram {
    fn error_response(error: String) -> Self::ResponseMessage {
        StopProgramResponse { success: false, error: Some(error) }
    }
}

// Frame/Tool Management

/// Request to get the currently active frame and tool numbers.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GetActiveFrameTool;

/// Response for GetActiveFrameTool.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GetActiveFrameToolResponse {
    pub uframe: i32,
    pub utool: i32,
}

impl RequestMessage for GetActiveFrameTool {
    type ResponseMessage = GetActiveFrameToolResponse;
}

/// Request to set the active frame and tool numbers.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SetActiveFrameTool {
    pub uframe: i32,
    pub utool: i32,
}

/// Response for SetActiveFrameTool.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SetActiveFrameToolResponse {
    pub success: bool,
    pub error: Option<String>,
}

impl RequestMessage for SetActiveFrameTool {
    type ResponseMessage = SetActiveFrameToolResponse;
}

impl ErrorResponse for SetActiveFrameTool {
    fn error_response(error: String) -> Self::ResponseMessage {
        SetActiveFrameToolResponse { success: false, error: Some(error) }
    }
}

/// Request to read frame data from the robot.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GetFrameData {
    pub frame_number: i32,
}

impl RequestMessage for GetFrameData {
    type ResponseMessage = FrameDataResponse;
}

/// Request to write frame data to the robot.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WriteFrameData {
    pub frame_number: i32,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
    pub p: f64,
    pub r: f64,
}

/// Response for WriteFrameData.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WriteFrameDataResponse {
    pub success: bool,
    pub error: Option<String>,
}

impl RequestMessage for WriteFrameData {
    type ResponseMessage = WriteFrameDataResponse;
}

#[cfg(feature = "ecs")]
impl ErrorResponse for WriteFrameData {
    fn error_response(error: String) -> Self::ResponseMessage {
        WriteFrameDataResponse { success: false, error: Some(error) }
    }
}

/// Request to read tool data from the robot.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GetToolData {
    pub tool_number: i32,
}

impl RequestMessage for GetToolData {
    type ResponseMessage = ToolDataResponse;
}

/// Request to write tool data to the robot.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WriteToolData {
    pub tool_number: i32,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
    pub p: f64,
    pub r: f64,
}

/// Response for WriteToolData.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WriteToolDataResponse {
    pub success: bool,
    pub error: Option<String>,
}

impl RequestMessage for WriteToolData {
    type ResponseMessage = WriteToolDataResponse;
}

#[cfg(feature = "ecs")]
impl ErrorResponse for WriteToolData {
    fn error_response(error: String) -> Self::ResponseMessage {
        WriteToolDataResponse { success: false, error: Some(error) }
    }
}

// Robot Connection Management - CreateRobotConnection is defined above with RequestMessage impl

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[cfg_attr(feature = "server", derive(Invalidates))]
#[cfg_attr(feature = "server", invalidates("ListRobotConnections"))]
pub struct UpdateRobotConnection {
    pub id: i64,
    pub name: Option<String>,
    pub description: Option<String>,
    pub ip_address: Option<String>,
    pub port: Option<i32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "server", derive(HasSuccess))]
pub struct UpdateRobotConnectionResponse {
    pub success: bool,
    pub error: Option<String>,
}

impl RequestMessage for UpdateRobotConnection {
    type ResponseMessage = UpdateRobotConnectionResponse;
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "server", derive(Invalidates))]
#[cfg_attr(feature = "server", invalidates("ListRobotConnections"))]
pub struct DeleteRobotConnection {
    pub id: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "server", derive(HasSuccess))]
pub struct DeleteRobotConnectionResponse {
    pub success: bool,
    pub error: Option<String>,
}

impl RequestMessage for DeleteRobotConnection {
    type ResponseMessage = DeleteRobotConnectionResponse;
}

/// Request to get configurations for a specific robot.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GetRobotConfigurations {
    pub robot_connection_id: i64,
}

impl RequestMessage for GetRobotConfigurations {
    type ResponseMessage = RobotConfigurationsResponse;
}

/// Request to create a new configuration for a robot.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "server", derive(Invalidates))]
#[cfg_attr(feature = "server", invalidates("GetRobotConfigurations"))]
pub struct CreateConfiguration {
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

impl RequestMessage for CreateConfiguration {
    type ResponseMessage = CreateConfigurationResponse;
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "server", derive(HasSuccess))]
pub struct CreateConfigurationResponse {
    pub success: bool,
    pub configuration_id: i64,
    pub error: Option<String>,
}

/// Request to update an existing configuration.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "server", derive(Invalidates))]
#[cfg_attr(feature = "server", invalidates("GetRobotConfigurations"))]
pub struct UpdateConfiguration {
    pub id: i64,
    pub name: Option<String>,
    pub is_default: Option<bool>,
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

impl RequestMessage for UpdateConfiguration {
    type ResponseMessage = UpdateConfigurationResponse;
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "server", derive(HasSuccess))]
pub struct UpdateConfigurationResponse {
    pub success: bool,
    pub error: Option<String>,
}

/// Request to delete a configuration.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "server", derive(Invalidates))]
#[cfg_attr(feature = "server", invalidates("GetRobotConfigurations"))]
pub struct DeleteConfiguration {
    pub id: i64,
}

impl RequestMessage for DeleteConfiguration {
    type ResponseMessage = DeleteConfigurationResponse;
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "server", derive(HasSuccess))]
pub struct DeleteConfigurationResponse {
    pub success: bool,
    pub error: Option<String>,
}

impl RequestMessage for LoadConfiguration {
    type ResponseMessage = LoadConfigurationResponse;
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LoadConfigurationResponse {
    pub success: bool,
    pub error: Option<String>,
}

/// Request to set a configuration as the default for its robot.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "server", derive(Invalidates))]
#[cfg_attr(feature = "server", invalidates("GetRobotConfigurations"))]
pub struct SetDefaultConfiguration {
    pub id: i64,
}

impl RequestMessage for SetDefaultConfiguration {
    type ResponseMessage = SetDefaultConfigurationResponse;
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "server", derive(HasSuccess))]
pub struct SetDefaultConfigurationResponse {
    pub success: bool,
    pub error: Option<String>,
}

/// Request to save the current active configuration to the database.
/// If `name` is provided, creates a new configuration with that name.
/// If `name` is None and `loaded_from_id` exists, updates the existing configuration.
/// Resets `changes_count` to 0 after successful save.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "server", derive(Invalidates))]
#[cfg_attr(feature = "server", invalidates("GetRobotConfigurations"))]
pub struct SaveCurrentConfiguration {
    /// If provided, saves as a new configuration with this name.
    /// If None, updates the currently loaded configuration.
    pub name: Option<String>,
}

impl RequestMessage for SaveCurrentConfiguration {
    type ResponseMessage = SaveCurrentConfigurationResponse;
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "server", derive(HasSuccess))]
pub struct SaveCurrentConfigurationResponse {
    pub success: bool,
    /// The ID of the saved configuration (new or updated)
    pub configuration_id: Option<i64>,
    /// The name of the saved configuration
    pub configuration_name: Option<String>,
    pub error: Option<String>,
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
    pub connections: Vec<RobotConnection>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RobotConfigurationsResponse {
    pub robot_id: i64,
    pub configurations: Vec<RobotConfiguration>,
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

// ============================================================================
// I/O Messages
// ============================================================================

// --- Digital Input (Read Only) ---

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ReadDin {
    pub port_number: u16,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DinValueResponse {
    pub port_number: u16,
    pub port_value: bool,
}

impl RequestMessage for ReadDin {
    type ResponseMessage = DinValueResponse;
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ReadDinBatch {
    pub port_numbers: Vec<u16>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DinBatchResponse {
    pub values: Vec<(u16, bool)>,
}

impl RequestMessage for ReadDinBatch {
    type ResponseMessage = DinBatchResponse;
}

// --- Digital Output (Read/Write) ---

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WriteDout {
    pub port_number: u16,
    pub port_value: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DoutValueResponse {
    pub port_number: u16,
    pub port_value: bool,
    pub success: bool,
    pub error: Option<String>,
}

impl RequestMessage for WriteDout {
    type ResponseMessage = DoutValueResponse;
}

#[cfg(feature = "ecs")]
impl ErrorResponse for WriteDout {
    fn error_response(error: String) -> Self::ResponseMessage {
        DoutValueResponse { port_number: 0, port_value: false, success: false, error: Some(error) }
    }
}

// --- Analog Input (Read Only) ---

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ReadAin {
    pub port_number: u16,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AinValueResponse {
    pub port_number: u16,
    pub port_value: f64,
}

impl RequestMessage for ReadAin {
    type ResponseMessage = AinValueResponse;
}

// --- Analog Output (Read/Write) ---

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WriteAout {
    pub port_number: u16,
    pub port_value: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AoutValueResponse {
    pub port_number: u16,
    pub port_value: f64,
    pub success: bool,
    pub error: Option<String>,
}

impl RequestMessage for WriteAout {
    type ResponseMessage = AoutValueResponse;
}

#[cfg(feature = "ecs")]
impl ErrorResponse for WriteAout {
    fn error_response(error: String) -> Self::ResponseMessage {
        AoutValueResponse { port_number: 0, port_value: 0.0, success: false, error: Some(error) }
    }
}

// --- Group Input (Read Only) ---

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ReadGin {
    pub port_number: u16,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GinValueResponse {
    pub port_number: u16,
    pub port_value: u32,
}

impl RequestMessage for ReadGin {
    type ResponseMessage = GinValueResponse;
}

// --- Group Output (Read/Write) ---

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WriteGout {
    pub port_number: u16,
    pub port_value: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GoutValueResponse {
    pub port_number: u16,
    pub port_value: u32,
    pub success: bool,
    pub error: Option<String>,
}

impl RequestMessage for WriteGout {
    type ResponseMessage = GoutValueResponse;
}

#[cfg(feature = "ecs")]
impl ErrorResponse for WriteGout {
    fn error_response(error: String) -> Self::ResponseMessage {
        GoutValueResponse { port_number: 0, port_value: 0, success: false, error: Some(error) }
    }
}

// --- I/O Configuration ---

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GetIoConfig {
    pub robot_connection_id: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IoConfigResponse {
    pub configs: Vec<IoDisplayConfig>,
}

impl RequestMessage for GetIoConfig {
    type ResponseMessage = IoConfigResponse;
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateIoConfig {
    pub robot_connection_id: i64,
    pub configs: Vec<IoDisplayConfig>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateIoConfigResponse {
    pub success: bool,
    pub error: Option<String>,
}

impl RequestMessage for UpdateIoConfig {
    type ResponseMessage = UpdateIoConfigResponse;
}

// ============================================================================
// Settings Messages
// ============================================================================

// Note: RobotSettings is already defined above

/// Get current settings.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct GetSettings;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SettingsResponse {
    pub settings: RobotSettings,
}

impl RequestMessage for GetSettings {
    type ResponseMessage = SettingsResponse;
}

/// Update settings.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateSettings {
    pub default_w: f64,
    pub default_p: f64,
    pub default_r: f64,
    pub default_speed: f64,
    pub default_term_type: String,
    pub default_uframe: i32,
    pub default_utool: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateSettingsResponse {
    pub success: bool,
    pub error: Option<String>,
}

impl RequestMessage for UpdateSettings {
    type ResponseMessage = UpdateSettingsResponse;
}

/// Reset the database.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ResetDatabase;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ResetDatabaseResponse {
    pub success: bool,
    pub error: Option<String>,
}

impl RequestMessage for ResetDatabase {
    type ResponseMessage = ResetDatabaseResponse;
}

/// Get connection status.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct GetConnectionStatus;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConnectionStatusResponse {
    pub connected: bool,
    pub robot_name: Option<String>,
    pub ip_address: Option<String>,
    pub port: Option<i32>,
}

impl RequestMessage for GetConnectionStatus {
    type ResponseMessage = ConnectionStatusResponse;
}

/// Get execution state.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct GetExecutionState;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExecutionStateResponse {
    pub status: String,
    pub current_line: Option<usize>,
    pub total_lines: Option<usize>,
    pub error: Option<String>,
}

impl RequestMessage for GetExecutionState {
    type ResponseMessage = ExecutionStateResponse;
}

// Note: GetActiveJogSettings was removed - use JogSettingsState synced component instead.
// UpdateJogSettings is used to persist settings to database.

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateJogSettingsResponse {
    pub success: bool,
    pub error: Option<String>,
}

impl RequestMessage for UpdateJogSettings {
    type ResponseMessage = UpdateJogSettingsResponse;
}

// ============================================================================
//                          COORDINATE CONVERSION UTILITIES
// ============================================================================

/// Re-export robot-agnostic coordinate types for toolpath integration.
///
/// These types use `Isometry3<f64>` (SE3) internally for singularity-free
/// rotation representation. Use these for:
/// - Toolpath generation and storage
/// - Motion planning and interpolation
/// - Frame transformations
///
/// The existing `Instruction` and `RobotPosition` types remain for:
/// - Database storage (Euler angles are human-readable)
/// - UI display (users expect W/P/R)
/// - Robot feedback (vendor-specific format)
pub mod robotics {
    pub use fanuc_replica_robotics::{
        RobotPose, ToolpathPoint, TerminationType, FrameId,
        quaternion_to_euler_zyx, euler_zyx_to_quaternion,
    };

    #[cfg(feature = "server")]
    pub use crate::{
        FanucConversion, isometry_to_position, position_to_isometry,
    };
}

/// Extension trait for converting RobotPosition to/from RobotPose.
impl RobotPosition {
    /// Convert to a robot-agnostic RobotPose.
    ///
    /// This creates an Isometry3 from the Euler angles, which is useful for:
    /// - Interpolation (slerp)
    /// - Frame transformations
    /// - Toolpath integration
    #[cfg(feature = "server")]
    pub fn to_robot_pose(&self, frame_id: robotics::FrameId) -> robotics::RobotPose {
        robotics::RobotPose::from_xyz_wpr(
            self.0.x, self.0.y, self.0.z,
            self.0.w, self.0.p, self.0.r,
            frame_id,
        )
    }

    /// Create from a robot-agnostic RobotPose.
    ///
    /// This extracts Euler angles from the quaternion for FANUC compatibility.
    #[cfg(feature = "server")]
    pub fn from_robot_pose(pose: &robotics::RobotPose) -> Self {
        use robotics::FanucConversion;
        Self(pose.to_fanuc_position().into())
    }
}

/// Extension trait for converting Instruction to RobotPose.
impl Instruction {
    /// Convert to a robot-agnostic RobotPose.
    ///
    /// Uses default rotation (0,0,0) if W/P/R are not specified.
    #[cfg(feature = "server")]
    pub fn to_robot_pose(&self, frame_id: robotics::FrameId) -> robotics::RobotPose {
        robotics::RobotPose::from_xyz_wpr(
            self.x, self.y, self.z,
            self.w.unwrap_or(0.0),
            self.p.unwrap_or(0.0),
            self.r.unwrap_or(0.0),
            frame_id,
        )
    }
}
