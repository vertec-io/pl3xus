//! ExecutionPoint - a single point in the toolpath with all device commands.

use fanuc_replica_robotics::RobotPose;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::traits::AuxiliaryCommand;

/// A point in the toolpath with motion and auxiliary commands.
///
/// This is the unit of work consumed by the orchestrator.
/// Each point contains:
/// - A target pose for the motion device
/// - Motion parameters (speed, type, blending)
/// - Commands for auxiliary devices (keyed by device type)
/// - Optional metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPoint {
    /// Unique index in the toolpath (0-based)
    pub index: u32,

    /// Target pose for the motion device (quaternion-based)
    pub target_pose: RobotPose,

    /// Motion parameters for this move
    pub motion: MotionCommand,

    /// Commands for auxiliary devices, keyed by device_type
    #[serde(default)]
    pub aux_commands: HashMap<String, AuxiliaryCommand>,

    /// Optional metadata about this point
    #[serde(default)]
    pub metadata: PointMetadata,
}

impl ExecutionPoint {
    /// Create a new execution point with the given pose and default motion.
    pub fn new(index: u32, target_pose: RobotPose) -> Self {
        Self {
            index,
            target_pose,
            motion: MotionCommand::default(),
            aux_commands: HashMap::new(),
            metadata: PointMetadata::default(),
        }
    }

    /// Add an auxiliary command for a device type.
    pub fn with_aux_command(mut self, device_type: impl Into<String>, cmd: AuxiliaryCommand) -> Self {
        self.aux_commands.insert(device_type.into(), cmd);
        self
    }

    /// Set motion parameters.
    pub fn with_motion(mut self, motion: MotionCommand) -> Self {
        self.motion = motion;
        self
    }

    /// Set metadata.
    pub fn with_metadata(mut self, metadata: PointMetadata) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Motion parameters for a single move.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotionCommand {
    /// Speed in mm/s for the tool center point
    pub speed: f32,

    /// Type of motion interpolation
    pub motion_type: MotionType,

    /// Blend radius in mm (0 = stop at point, >0 = smooth through)
    pub blend_radius: f32,
}

impl Default for MotionCommand {
    fn default() -> Self {
        Self {
            speed: 100.0,
            motion_type: MotionType::Linear,
            blend_radius: 0.0,
        }
    }
}

/// Type of motion interpolation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MotionType {
    /// Linear motion - straight line in Cartesian space
    #[default]
    Linear,

    /// Joint motion - interpolated in joint space
    Joint,

    /// Circular motion - arc defined by via point
    Circular,
}

/// Metadata about a toolpath point.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PointMetadata {
    /// Layer height for this point (for 3D printing)
    pub layer_height: Option<f32>,

    /// Bead width for this point (for 3D printing)
    pub bead_width: Option<f32>,

    /// True if this is a travel move (no extrusion)
    pub is_travel: bool,

    /// Optional comment or annotation
    pub comment: Option<String>,

    /// Layer number (0-based)
    pub layer_number: Option<u32>,

    /// Point type classification
    pub point_type: PointType,
}

/// Classification of point types in a toolpath.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PointType {
    /// Normal print move with extrusion
    #[default]
    Print,

    /// Travel move without extrusion
    Travel,

    /// Retraction point
    Retract,

    /// Un-retraction point (prime)
    Prime,

    /// Z-hop up
    ZHopUp,

    /// Z-hop down
    ZHopDown,

    /// Wipe move
    Wipe,

    /// Coast move (reduced extrusion at end of line)
    Coast,
}

