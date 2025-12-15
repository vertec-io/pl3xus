//! Shared types for FANUC robot control using real FANUC_RMI_API
//!
//! This crate provides types that mirror FANUC_RMI_API DTO types and can be
//! used as Bevy components and synchronized via pl3xus_sync.

use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use bevy::prelude::*;

// Re-export FANUC DTO types (WASM-compatible, no tokio/mio dependencies)
pub use fanuc_rmi::dto;

/// Robot cartesian position (mirrors fanuc_rmi::dto::Position)
#[cfg_attr(feature = "server", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RobotPosition {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
    pub p: f32,
    pub r: f32,
    pub ext1: f32,
    pub ext2: f32,
    pub ext3: f32,
}

impl Default for RobotPosition {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 400.0, // Start at 400mm above origin
            w: 0.0,
            p: 0.0,
            r: 0.0,
            ext1: 0.0,
            ext2: 0.0,
            ext3: 0.0,
        }
    }
}

#[cfg(feature = "server")]
impl From<dto::Position> for RobotPosition {
    fn from(pos: dto::Position) -> Self {
        Self {
            x: pos.x,
            y: pos.y,
            z: pos.z,
            w: pos.w,
            p: pos.p,
            r: pos.r,
            ext1: pos.ext1,
            ext2: pos.ext2,
            ext3: pos.ext3,
        }
    }
}

#[cfg(feature = "server")]
impl From<RobotPosition> for dto::Position {
    fn from(pos: RobotPosition) -> Self {
        Self {
            x: pos.x,
            y: pos.y,
            z: pos.z,
            w: pos.w,
            p: pos.p,
            r: pos.r,
            ext1: pos.ext1,
            ext2: pos.ext2,
            ext3: pos.ext3,
        }
    }
}

/// Robot joint angles (mirrors fanuc_rmi::dto::JointAngles)
#[cfg_attr(feature = "server", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct JointAngles {
    pub j1: f32,
    pub j2: f32,
    pub j3: f32,
    pub j4: f32,
    pub j5: f32,
    pub j6: f32,
    pub j7: f32,
    pub j8: f32,
    pub j9: f32,
}

impl Default for JointAngles {
    fn default() -> Self {
        Self {
            j1: 0.0,
            j2: 0.0,
            j3: 0.0,
            j4: 0.0,
            j5: 0.0,
            j6: 0.0,
            j7: 0.0,
            j8: 0.0,
            j9: 0.0,
        }
    }
}

#[cfg(feature = "server")]
impl From<dto::JointAngles> for JointAngles {
    fn from(angles: dto::JointAngles) -> Self {
        Self {
            j1: angles.j1,
            j2: angles.j2,
            j3: angles.j3,
            j4: angles.j4,
            j5: angles.j5,
            j6: angles.j6,
            j7: angles.j7,
            j8: angles.j8,
            j9: angles.j9,
        }
    }
}

#[cfg(feature = "server")]
impl From<JointAngles> for dto::JointAngles {
    fn from(angles: JointAngles) -> Self {
        Self {
            j1: angles.j1,
            j2: angles.j2,
            j3: angles.j3,
            j4: angles.j4,
            j5: angles.j5,
            j6: angles.j6,
            j7: angles.j7,
            j8: angles.j8,
            j9: angles.j9,
        }
    }
}

/// Robot operational status
#[cfg_attr(feature = "server", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RobotStatus {
    /// Servos are powered and ready
    pub servo_ready: bool,
    /// Teach pendant is enabled
    pub tp_enabled: bool,
    /// Robot is currently executing motion
    pub in_motion: bool,
    /// Error message (None = no error)
    pub error_message: Option<String>,
}

impl Default for RobotStatus {
    fn default() -> Self {
        Self {
            servo_ready: true,
            tp_enabled: false,
            in_motion: false,
            error_message: None,
        }
    }
}

/// Robot identification
#[cfg_attr(feature = "server", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RobotInfo {
    pub name: String,
    pub model: String,
}

impl Default for RobotInfo {
    fn default() -> Self {
        Self {
            name: "FANUC Robot".to_string(),
            model: "LR Mate 200iD".to_string(),
        }
    }
}

/// Axis for jogging
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum JogAxis {
    X,
    Y,
    Z,
    W,
    P,
    R,
}

/// Direction for jogging
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum JogDirection {
    Positive,
    Negative,
}

/// Client-side jog command (can be sent from WASM client)
#[cfg_attr(feature = "server", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct JogCommand {
    pub axis: JogAxis,
    pub direction: JogDirection,
    /// Distance to jog in mm (for linear axes) or degrees (for rotational axes)
    pub distance: f32,
    /// Speed in mm/s (for linear axes) or deg/s (for rotational axes)
    pub speed: f32,
}

/// Motion command to be sent to the robot
/// This wraps the actual FANUC instruction (dto::Instruction is WASM-compatible)
#[cfg_attr(feature = "server", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MotionCommand {
    pub instruction: dto::Instruction,
}

