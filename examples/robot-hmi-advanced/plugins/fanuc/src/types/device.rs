//! Device marker components and state types.
//!
//! This module contains ECS components that are synced between server and client,
//! representing the robot's current state.

use fanuc_rmi::dto;
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};

#[cfg(feature = "ecs")]
use bevy::prelude::*;

// ============================================================================
//                          SYNCED COMPONENTS (Wrapped DTOs)
// ============================================================================

/// Marker component for the active/current robot entity.
///
/// This is the shared marker for identifying the currently active robot on both server and client.
/// On the server, the `FanucRobot` component is server-only and used for queries.
/// This marker is synced to clients so they can identify the active robot entity.
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

/// I/O Status - contains all I/O types
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

/// Frame/Tool data state - synced component.
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
//                          COORDINATE CONVERSION UTILITIES
// ============================================================================

impl RobotPosition {
    /// Convert to a robot-agnostic RobotPose.
    #[cfg(feature = "server")]
    pub fn to_robot_pose(&self, frame_id: robot_hmi_robotics::frame::FrameId) -> robot_hmi_robotics::pose::RobotPose {
        robot_hmi_robotics::pose::RobotPose::from_xyz_wpr(
            self.0.x, self.0.y, self.0.z,
            self.0.w, self.0.p, self.0.r,
            frame_id,
        )
    }

    /// Create from a robot-agnostic RobotPose.
    #[cfg(feature = "server")]
    pub fn from_robot_pose(pose: &robot_hmi_robotics::pose::RobotPose) -> Self {
        use crate::conversion::FanucConversion;
        Self(pose.to_fanuc_position().into())
    }
}

