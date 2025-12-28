//! FANUC-specific conversion utilities.
//!
//! This module provides bidirectional conversion between robot-agnostic
//! types (RobotPose) and FANUC-specific types (fanuc_rmi::Position).

use fanuc_replica_robotics::{
    euler_zyx_to_quaternion, quaternion_to_euler_zyx, FrameId, RobotPose,
};
use fanuc_rmi::Position;
use nalgebra::{Isometry3, Translation3};

/// Trait for converting to/from FANUC position types.
pub trait FanucConversion {
    /// Convert to FANUC Position format.
    fn to_fanuc_position(&self) -> Position;

    /// Create from FANUC Position format.
    fn from_fanuc_position(pos: &Position, frame_id: FrameId) -> Self;
}

impl FanucConversion for RobotPose {
    fn to_fanuc_position(&self) -> Position {
        let (w, p, r) = quaternion_to_euler_zyx(&self.transform.rotation);

        Position {
            x: self.transform.translation.x,
            y: self.transform.translation.y,
            z: self.transform.translation.z,
            w,
            p,
            r,
            ext1: 0.0,
            ext2: 0.0,
            ext3: 0.0,
        }
    }

    fn from_fanuc_position(pos: &Position, frame_id: FrameId) -> Self {
        let rotation = euler_zyx_to_quaternion(pos.w, pos.p, pos.r);
        let transform = Isometry3::from_parts(Translation3::new(pos.x, pos.y, pos.z), rotation);

        RobotPose::new(transform, frame_id)
    }
}

/// Convert an Isometry3 directly to FANUC Position.
///
/// This is a convenience function for cases where you have an Isometry3
/// but don't need the full RobotPose wrapper.
pub fn isometry_to_position(iso: &Isometry3<f64>) -> Position {
    let (w, p, r) = quaternion_to_euler_zyx(&iso.rotation);

    Position {
        x: iso.translation.x,
        y: iso.translation.y,
        z: iso.translation.z,
        w,
        p,
        r,
        ext1: 0.0,
        ext2: 0.0,
        ext3: 0.0,
    }
}

/// Convert a FANUC Position to Isometry3.
pub fn position_to_isometry(pos: &Position) -> Isometry3<f64> {
    let rotation = euler_zyx_to_quaternion(pos.w, pos.p, pos.r);
    Isometry3::from_parts(Translation3::new(pos.x, pos.y, pos.z), rotation)
}

/// Convert an f32 Isometry3 to FANUC Position.
///
/// This handles the precision conversion from f32 to f64.
pub fn isometry_f32_to_position(iso: &nalgebra::Isometry3<f32>) -> Position {
    // Convert f32 quaternion to f64
    let q = iso.rotation.quaternion();
    let q64 = nalgebra::UnitQuaternion::from_quaternion(nalgebra::Quaternion::new(
        q.w as f64,
        q.i as f64,
        q.j as f64,
        q.k as f64,
    ));

    let (w, p, r) = quaternion_to_euler_zyx(&q64);

    Position {
        x: iso.translation.x as f64,
        y: iso.translation.y as f64,
        z: iso.translation.z as f64,
        w,
        p,
        r,
        ext1: 0.0,
        ext2: 0.0,
        ext3: 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f64 = 1e-6;

    #[test]
    fn test_roundtrip_identity() {
        let pos = Position {
            x: 100.0,
            y: 200.0,
            z: 300.0,
            w: 0.0,
            p: 0.0,
            r: 0.0,
            ext1: 0.0,
            ext2: 0.0,
            ext3: 0.0,
        };

        let pose = RobotPose::from_fanuc_position(&pos, FrameId::World);
        let pos2 = pose.to_fanuc_position();

        assert!((pos.x - pos2.x).abs() < TOLERANCE);
        assert!((pos.y - pos2.y).abs() < TOLERANCE);
        assert!((pos.z - pos2.z).abs() < TOLERANCE);
        assert!((pos.w - pos2.w).abs() < TOLERANCE);
        assert!((pos.p - pos2.p).abs() < TOLERANCE);
        assert!((pos.r - pos2.r).abs() < TOLERANCE);
    }

    #[test]
    fn test_roundtrip_rotated() {
        let pos = Position {
            x: 500.0,
            y: -300.0,
            z: 150.0,
            w: 45.0,
            p: 30.0,
            r: 15.0,
            ext1: 0.0,
            ext2: 0.0,
            ext3: 0.0,
        };

        let pose = RobotPose::from_fanuc_position(&pos, FrameId::UserFrame(1));
        let pos2 = pose.to_fanuc_position();

        assert!((pos.x - pos2.x).abs() < TOLERANCE, "X mismatch");
        assert!((pos.y - pos2.y).abs() < TOLERANCE, "Y mismatch");
        assert!((pos.z - pos2.z).abs() < TOLERANCE, "Z mismatch");
        assert!(
            (pos.w - pos2.w).abs() < TOLERANCE,
            "W mismatch: {} vs {}",
            pos.w,
            pos2.w
        );
        assert!(
            (pos.p - pos2.p).abs() < TOLERANCE,
            "P mismatch: {} vs {}",
            pos.p,
            pos2.p
        );
        assert!(
            (pos.r - pos2.r).abs() < TOLERANCE,
            "R mismatch: {} vs {}",
            pos.r,
            pos2.r
        );
    }
}

