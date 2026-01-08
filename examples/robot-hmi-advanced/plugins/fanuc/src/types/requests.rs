//! Network request/response types.
//!
//! This module contains all RPC request and response types for communication
//! between the client and server, including:
//! - Connection management
//! - Motion commands
//! - Frame/tool management
//! - I/O operations
//! - Configuration management

use pl3xus_common::{ErrorResponse, RequestMessage};
use serde::{Deserialize, Serialize};

// Server-only: automatic query invalidation support
#[cfg(feature = "server")]
use pl3xus_macros::{HasSuccess, Invalidates};

use super::config::{
    IoDisplayConfig, NewRobotConfiguration, RobotConfiguration, RobotConnection, RobotSettings,
};

// ============================================================================
//                          JOG TYPES
// ============================================================================

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum JogAxis {
    X, Y, Z, W, P, R, J1, J2, J3, J4, J5, J6,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum JogDirection {
    Positive,
    Negative,
}

/// Jog command sent from client to server.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JogCommand {
    pub axis: JogAxis,
    pub direction: JogDirection,
}

// ============================================================================
//                          CONNECTION REQUESTS
// ============================================================================

/// Request to list all saved robot connections from the database.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ListRobotConnections;

impl RequestMessage for ListRobotConnections {
    type ResponseMessage = RobotConnectionsResponse;
}

/// Request to create a new robot connection with configurations.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "server", derive(Invalidates))]
#[cfg_attr(feature = "server", invalidates("ListRobotConnections", "GetRobotConfigurations"))]
pub struct CreateRobotConnection {
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
    pub default_rotation_jog_speed: f64,
    pub default_rotation_jog_step: f64,
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
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConnectToRobot {
    pub connection_id: Option<i64>,
    pub addr: String,
    pub port: u32,
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateJogSettingsResponse {
    pub success: bool,
    pub error: Option<String>,
}

impl RequestMessage for UpdateJogSettings {
    type ResponseMessage = UpdateJogSettingsResponse;
}

// ============================================================================
//                          ROBOT CONTROL COMMANDS
// ============================================================================

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
    pub speed: u8,
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConnectToRobotResponse {
    pub success: bool,
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

// ============================================================================
//                          MOTION COMMANDS
// ============================================================================

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
    pub speed_type: String,
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
    pub via_x: f64,
    pub via_y: f64,
    pub via_z: f64,
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

// ============================================================================
//                          FRAME/TOOL MANAGEMENT
// ============================================================================

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GetActiveFrameTool;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GetActiveFrameToolResponse {
    pub uframe: i32,
    pub utool: i32,
}

impl RequestMessage for GetActiveFrameTool {
    type ResponseMessage = GetActiveFrameToolResponse;
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SetActiveFrameTool {
    pub uframe: i32,
    pub utool: i32,
}

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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GetFrameData {
    pub frame_number: i32,
}

impl RequestMessage for GetFrameData {
    type ResponseMessage = FrameDataResponse;
}

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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GetToolData {
    pub tool_number: i32,
}

impl RequestMessage for GetToolData {
    type ResponseMessage = ToolDataResponse;
}

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

// ============================================================================
//                          ROBOT CONNECTION MANAGEMENT
// ============================================================================

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

// ============================================================================
//                          CONFIGURATION MANAGEMENT
// ============================================================================

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GetRobotConfigurations {
    pub robot_connection_id: i64,
}

impl RequestMessage for GetRobotConfigurations {
    type ResponseMessage = RobotConfigurationsResponse;
}

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

#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "server", derive(Invalidates))]
#[cfg_attr(feature = "server", invalidates("GetRobotConfigurations"))]
pub struct SaveCurrentConfiguration {
    pub name: Option<String>,
}

impl RequestMessage for SaveCurrentConfiguration {
    type ResponseMessage = SaveCurrentConfigurationResponse;
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "server", derive(HasSuccess))]
pub struct SaveCurrentConfigurationResponse {
    pub success: bool,
    pub configuration_id: Option<i64>,
    pub configuration_name: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RevertConfiguration;

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

// ============================================================================
//                          RESPONSE MESSAGES
// ============================================================================

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

// ============================================================================
//                          COMMAND LOG
// ============================================================================

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CommandLogEntry {
    pub timestamp: String,
    pub timestamp_ms: u64,
    pub command_type: CommandType,
    pub description: String,
    pub command_data: String,
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
    System,
}

// ============================================================================
//                          I/O MESSAGES
// ============================================================================

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
//                          SETTINGS MESSAGES
// ============================================================================

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct GetSettings;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SettingsResponse {
    pub settings: RobotSettings,
}

impl RequestMessage for GetSettings {
    type ResponseMessage = SettingsResponse;
}

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
