//! Robot pose types using Isometry3 (SE3) representation.

use nalgebra::Isometry3;
use serde::{Deserialize, Serialize};
use crate::frame::FrameId;
use crate::conversion::{quaternion_to_euler_zyx, euler_zyx_to_quaternion};

/// Robot-agnostic pose in a named frame.
///
/// Uses nalgebra's `Isometry3<f64>` internally, which stores:
/// - Translation as Vector3<f64> (x, y, z in mm)
/// - Rotation as UnitQuaternion<f64> (singularity-free)
///
/// # Benefits over Euler Angles
/// - No gimbal lock
/// - Proper interpolation via slerp
/// - Composable transforms
/// - Robot-vendor agnostic
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RobotPose {
    /// The SE3 transform (position + orientation)
    pub transform: Isometry3<f64>,
    
    /// The frame this pose is expressed in
    pub frame_id: FrameId,
}

impl RobotPose {
    /// Create a new robot pose.
    pub fn new(transform: Isometry3<f64>, frame_id: FrameId) -> Self {
        Self { transform, frame_id }
    }
    
    /// Create a pose from translation only (identity rotation).
    pub fn from_translation(x: f64, y: f64, z: f64, frame_id: FrameId) -> Self {
        Self {
            transform: Isometry3::translation(x, y, z),
            frame_id,
        }
    }
    
    /// Get translation components.
    pub fn translation(&self) -> (f64, f64, f64) {
        let t = &self.transform.translation;
        (t.x, t.y, t.z)
    }
    
    /// Get rotation as Euler ZYX angles in degrees (W, P, R for FANUC).
    /// 
    /// Returns (yaw, pitch, roll) which maps to FANUC's (W, P, R).
    pub fn to_wpr_degrees(&self) -> (f64, f64, f64) {
        quaternion_to_euler_zyx(&self.transform.rotation)
    }
    
    /// Create a pose from XYZ position and WPR rotation (FANUC format).
    pub fn from_xyz_wpr(
        x: f64, y: f64, z: f64,
        w: f64, p: f64, r: f64,
        frame_id: FrameId,
    ) -> Self {
        let rotation = euler_zyx_to_quaternion(w, p, r);
        let transform = Isometry3::from_parts(
            nalgebra::Translation3::new(x, y, z),
            rotation,
        );
        Self { transform, frame_id }
    }
}

/// Motion termination type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerminationType {
    /// Fine motion - robot stops at position
    Fine,
    /// Continuous motion with blending value 0-100
    Continuous(u8),
}

impl Default for TerminationType {
    fn default() -> Self {
        TerminationType::Fine
    }
}

/// Toolpath point with motion parameters.
///
/// Combines a robot pose with motion-specific parameters like speed
/// and termination type.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolpathPoint {
    /// The target pose
    pub pose: RobotPose,
    
    /// Speed in mm/sec for linear motion
    pub speed: f64,
    
    /// Termination type (FINE or CNT)
    pub termination: TerminationType,
    
    /// External axis positions (optional, up to 6 axes)
    pub external_axes: Option<[f64; 6]>,
}

impl ToolpathPoint {
    /// Create a new toolpath point with default motion parameters.
    pub fn new(pose: RobotPose) -> Self {
        Self {
            pose,
            speed: 100.0, // Default 100 mm/sec
            termination: TerminationType::Fine,
            external_axes: None,
        }
    }
    
    /// Set the speed and return self for chaining.
    pub fn with_speed(mut self, speed: f64) -> Self {
        self.speed = speed;
        self
    }
    
    /// Set continuous termination with blend value.
    pub fn with_continuous(mut self, blend: u8) -> Self {
        self.termination = TerminationType::Continuous(blend.min(100));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_pose_from_translation() {
        let pose = RobotPose::from_translation(100.0, 200.0, 300.0, FrameId::World);
        let (x, y, z) = pose.translation();
        assert!((x - 100.0).abs() < 1e-10);
        assert!((y - 200.0).abs() < 1e-10);
        assert!((z - 300.0).abs() < 1e-10);
    }
    
    #[test]
    fn test_identity_rotation_wpr() {
        let pose = RobotPose::from_translation(0.0, 0.0, 0.0, FrameId::World);
        let (w, p, r) = pose.to_wpr_degrees();
        assert!(w.abs() < 1e-10, "W should be 0, got {}", w);
        assert!(p.abs() < 1e-10, "P should be 0, got {}", p);
        assert!(r.abs() < 1e-10, "R should be 0, got {}", r);
    }
}

