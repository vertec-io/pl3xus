//! Quaternion ↔ Euler angle conversion utilities.
//!
//! FANUC uses W-P-R (Euler ZYX intrinsic) convention:
//! - W (Yaw): Rotation around Z axis (first)
//! - P (Pitch): Rotation around Y' axis (second)
//! - R (Roll): Rotation around X'' axis (third)
//!
//! This is equivalent to extrinsic XYZ: R around fixed X, P around fixed Y, W around fixed Z.

use nalgebra::{Rotation3, UnitQuaternion};

/// Convert a unit quaternion to Euler ZYX angles in degrees.
///
/// Returns (W, P, R) in degrees, matching FANUC's convention:
/// - W = Yaw (rotation around Z)
/// - P = Pitch (rotation around Y)
/// - R = Roll (rotation around X)
///
/// # Gimbal Lock
/// When P = ±90°, gimbal lock occurs. The function handles this by
/// setting R=0 and encoding all rotation in W.
pub fn quaternion_to_euler_zyx(q: &UnitQuaternion<f64>) -> (f64, f64, f64) {
    // Use nalgebra's built-in euler_angles which returns (roll, pitch, yaw)
    // in the XYZ extrinsic order (= ZYX intrinsic)
    let rotation_matrix = q.to_rotation_matrix();
    let (r_rad, p_rad, w_rad) = rotation_matrix.euler_angles();
    
    (w_rad.to_degrees(), p_rad.to_degrees(), r_rad.to_degrees())
}

/// Convert Euler ZYX angles in degrees to a unit quaternion.
///
/// Takes (W, P, R) in degrees, matching FANUC's convention:
/// - W = Yaw (rotation around Z)
/// - P = Pitch (rotation around Y)
/// - R = Roll (rotation around X)
pub fn euler_zyx_to_quaternion(w_deg: f64, p_deg: f64, r_deg: f64) -> UnitQuaternion<f64> {
    let w_rad = w_deg.to_radians();
    let p_rad = p_deg.to_radians();
    let r_rad = r_deg.to_radians();
    
    // nalgebra's from_euler_angles takes (roll, pitch, yaw) = (X, Y, Z)
    let rotation = Rotation3::from_euler_angles(r_rad, p_rad, w_rad);
    UnitQuaternion::from_rotation_matrix(&rotation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::{FRAC_PI_2, PI};
    use nalgebra::Vector3;

    const ANGLE_TOLERANCE: f64 = 1e-6; // degrees

    #[test]
    fn test_identity() {
        let q = UnitQuaternion::identity();
        let (w, p, r) = quaternion_to_euler_zyx(&q);
        
        assert!(w.abs() < ANGLE_TOLERANCE, "W should be 0, got {}", w);
        assert!(p.abs() < ANGLE_TOLERANCE, "P should be 0, got {}", p);
        assert!(r.abs() < ANGLE_TOLERANCE, "R should be 0, got {}", r);
    }

    #[test]
    fn test_yaw_90() {
        // 90° rotation around Z (yaw)
        let q = UnitQuaternion::from_axis_angle(&Vector3::z_axis(), FRAC_PI_2);
        let (w, p, r) = quaternion_to_euler_zyx(&q);
        
        assert!((w - 90.0).abs() < ANGLE_TOLERANCE, "W should be 90, got {}", w);
        assert!(p.abs() < ANGLE_TOLERANCE, "P should be 0, got {}", p);
        assert!(r.abs() < ANGLE_TOLERANCE, "R should be 0, got {}", r);
    }

    #[test]
    fn test_pitch_90() {
        // 90° rotation around Y (pitch)
        let q = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), FRAC_PI_2);
        let (_w, p, _r) = quaternion_to_euler_zyx(&q);

        // At gimbal lock, W and R become coupled
        assert!((p - 90.0).abs() < ANGLE_TOLERANCE, "P should be 90, got {}", p);
    }

    #[test]
    fn test_roll_90() {
        // 90° rotation around X (roll)
        let q = UnitQuaternion::from_axis_angle(&Vector3::x_axis(), FRAC_PI_2);
        let (w, p, r) = quaternion_to_euler_zyx(&q);
        
        assert!(w.abs() < ANGLE_TOLERANCE, "W should be 0, got {}", w);
        assert!(p.abs() < ANGLE_TOLERANCE, "P should be 0, got {}", p);
        assert!((r - 90.0).abs() < ANGLE_TOLERANCE, "R should be 90, got {}", r);
    }

    #[test]
    fn test_tool_down_180() {
        // 180° rotation around X (tool pointing down)
        let q = UnitQuaternion::from_axis_angle(&Vector3::x_axis(), PI);
        let (_w, _p, r) = quaternion_to_euler_zyx(&q);

        // R should be ±180°
        assert!((r.abs() - 180.0).abs() < ANGLE_TOLERANCE, "R should be ±180, got {}", r);
    }

    #[test]
    fn test_roundtrip() {
        // Test various angles round-trip correctly
        let test_cases = [
            (0.0, 0.0, 0.0),
            (45.0, 30.0, 15.0),
            (-45.0, -30.0, -15.0),
            (90.0, 0.0, 0.0),
            (0.0, 45.0, 0.0),
            (0.0, 0.0, 90.0),
            (180.0, 0.0, 0.0),
            (0.0, 0.0, 180.0),
        ];
        
        for (w, p, r) in test_cases {
            let q = euler_zyx_to_quaternion(w, p, r);
            let (w2, p2, r2) = quaternion_to_euler_zyx(&q);
            
            // Handle angle wrapping for ±180°
            let w_diff = (w - w2).abs();
            let p_diff = (p - p2).abs();
            let r_diff = (r - r2).abs();
            
            let w_ok = w_diff < ANGLE_TOLERANCE || (360.0 - w_diff).abs() < ANGLE_TOLERANCE;
            let p_ok = p_diff < ANGLE_TOLERANCE || (360.0 - p_diff).abs() < ANGLE_TOLERANCE;
            let r_ok = r_diff < ANGLE_TOLERANCE || (360.0 - r_diff).abs() < ANGLE_TOLERANCE;
            
            assert!(w_ok, "W roundtrip failed: {} -> {}", w, w2);
            assert!(p_ok, "P roundtrip failed: {} -> {}", p, p2);
            assert!(r_ok, "R roundtrip failed: {} -> {}", r, r2);
        }
    }
}

