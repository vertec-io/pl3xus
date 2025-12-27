# Quaternion to Euler Conversion for FANUC

## FANUC Euler Convention

FANUC uses **W-P-R** (Euler ZYX) convention:
- **W** (Yaw): Rotation around Z axis (first)
- **P** (Pitch): Rotation around Y' axis (second)  
- **R** (Roll): Rotation around X'' axis (third)

This is an **intrinsic rotation** sequence: each rotation is around the current (rotated) axis.

Equivalent to **extrinsic XYZ**: R around fixed X, then P around fixed Y, then W around fixed Z.

---

## The Math

Given quaternion `q = (qx, qy, qz, qw)` where `qw` is the scalar component:

### Conversion to ZYX Euler (W, P, R)

```rust
use nalgebra::{Quaternion, UnitQuaternion};

pub fn quaternion_to_wpr(q: &UnitQuaternion<f64>) -> (f64, f64, f64) {
    let q = q.quaternion();
    let (qx, qy, qz, qw) = (q.i, q.j, q.k, q.w);

    // Roll (X) - R
    let sinr_cosp = 2.0 * (qw * qx + qy * qz);
    let cosr_cosp = 1.0 - 2.0 * (qx * qx + qy * qy);
    let r = sinr_cosp.atan2(cosr_cosp);

    // Pitch (Y) - P
    let sinp = 2.0 * (qw * qy - qz * qx);
    let p = if sinp.abs() >= 1.0 {
        std::f64::consts::FRAC_PI_2.copysign(sinp)  // Gimbal lock
    } else {
        sinp.asin()
    };

    // Yaw (Z) - W
    let siny_cosp = 2.0 * (qw * qz + qx * qy);
    let cosy_cosp = 1.0 - 2.0 * (qy * qy + qz * qz);
    let w = siny_cosp.atan2(cosy_cosp);

    // Convert to degrees
    (w.to_degrees(), p.to_degrees(), r.to_degrees())
}
```

### Conversion from WPR to Quaternion

```rust
pub fn wpr_to_quaternion(w_deg: f64, p_deg: f64, r_deg: f64) -> UnitQuaternion<f64> {
    let w = w_deg.to_radians();  // Yaw (Z)
    let p = p_deg.to_radians();  // Pitch (Y)
    let r = r_deg.to_radians();  // Roll (X)

    // Half angles
    let (cw, sw) = (w / 2.0).cos_sin();
    let (cp, sp) = (p / 2.0).cos_sin();
    let (cr, sr) = (r / 2.0).cos_sin();

    // ZYX order (intrinsic) = XYZ order (extrinsic)
    let qw = cw * cp * cr + sw * sp * sr;
    let qx = cw * cp * sr - sw * sp * cr;
    let qy = cw * sp * cr + sw * cp * sr;
    let qz = sw * cp * cr - cw * sp * sr;

    UnitQuaternion::from_quaternion(Quaternion::new(qw, qx, qy, qz))
}
```

---

## Gimbal Lock Warning

When **P = ±90°** (pitch straight up/down), the system loses one degree of freedom:
- W and R become indistinguishable
- The quaternion is still valid, but Euler angles are ambiguous
- Convention: Set R=0 and encode all rotation in W

```rust
if sinp.abs() >= 0.9999 {
    // Gimbal lock: P = ±90°
    let r = 0.0;
    let w = 2.0 * qw.atan2(qx);  // All yaw in W
    return (w.to_degrees(), p.to_degrees(), r);
}
```

---

## Validation Test Cases

| Description | W (°) | P (°) | R (°) | Expected Quaternion |
|-------------|-------|-------|-------|---------------------|
| Identity | 0 | 0 | 0 | (0, 0, 0, 1) |
| Yaw 90° | 90 | 0 | 0 | (0, 0, 0.707, 0.707) |
| Pitch 90° | 0 | 90 | 0 | (0, 0.707, 0, 0.707) |
| Roll 90° | 0 | 0 | 90 | (0.707, 0, 0, 0.707) |
| Tool down | 0 | 0 | 180 | (1, 0, 0, 0) |
| Compound | 45 | 30 | 15 | ... (compute) |

---

## Using nalgebra's Built-in Functions

nalgebra provides Euler angle functions:

```rust
use nalgebra::{UnitQuaternion, Rotation3};

// From Euler ZYX (intrinsic) = XYZ (extrinsic)
let rotation = Rotation3::from_euler_angles(r_rad, p_rad, w_rad);  // Roll, Pitch, Yaw
let quaternion = UnitQuaternion::from_rotation_matrix(&rotation);

// To Euler ZYX
let (roll, pitch, yaw) = rotation.euler_angles();  // Returns (R, P, W) in radians
```

⚠️ **Careful**: nalgebra's `from_euler_angles` order is `(roll, pitch, yaw)` which is `(X, Y, Z)`.

---

## Fixed convert_to_position()

```rust
use nalgebra::{Isometry3, UnitQuaternion, Rotation3};
use fanuc_rmi::Position;

pub fn convert_to_position(iso: &Isometry3<f64>) -> Position {
    // Extract rotation as Euler angles (ZYX intrinsic = XYZ extrinsic)
    let rotation_matrix = iso.rotation.to_rotation_matrix();
    let (r_rad, p_rad, w_rad) = rotation_matrix.euler_angles();

    Position {
        x: iso.translation.x,
        y: iso.translation.y,
        z: iso.translation.z,
        w: w_rad.to_degrees(),  // Yaw (Z)
        p: p_rad.to_degrees(),  // Pitch (Y)
        r: r_rad.to_degrees(),  // Roll (X)
        ext1: 0.0,
        ext2: 0.0,
        ext3: 0.0,
    }
}

pub fn position_to_isometry(pos: &Position) -> Isometry3<f64> {
    let rotation = Rotation3::from_euler_angles(
        pos.r.to_radians(),  // Roll (X)
        pos.p.to_radians(),  // Pitch (Y)
        pos.w.to_radians(),  // Yaw (Z)
    );
    
    Isometry3::from_parts(
        nalgebra::Translation3::new(pos.x, pos.y, pos.z),
        UnitQuaternion::from_rotation_matrix(&rotation),
    )
}
```

---

## TODO

- [ ] Verify FANUC's exact Euler convention (ZYX vs ZYZ) with real robot tests
- [ ] Add gimbal lock handling
- [ ] Add unit tests comparing our conversion with fanuc_rmi expected values
- [ ] Benchmark conversion performance

