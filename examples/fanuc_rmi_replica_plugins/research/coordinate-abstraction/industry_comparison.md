# Industry Coordinate Representation Comparison

## Overview

This document compares coordinate representation approaches across robotics frameworks and vendor SDKs to inform our abstraction design.

---

## 1. ROS / ROS 2 (Industry Standard)

### Pose Representation
```
geometry_msgs/Pose:
  position:
    x: float64
    y: float64
    z: float64
  orientation:  # Quaternion!
    x: float64
    y: float64
    z: float64
    w: float64
```

### Key Decisions
- **Quaternions over Euler angles** - Avoids gimbal lock, proper interpolation
- **Separation of concerns**: Pose (where) vs Transform (relationship between frames)
- **TF2 system**: Manages frame transformations as a tree
- **URDF**: Robot description with coordinate frames for each link

### Frame Management (TF2)
- World → Base → Link1 → Link2 → ... → Tool
- Each frame has a named relationship
- Transforms can be looked up between any two frames
- Time-stamped transforms for sensor fusion

---

## 2. MoveIt (Motion Planning Framework)

### Position Representation
- Uses `geometry_msgs/PoseStamped` (Pose + frame_id + timestamp)
- Internally uses Eigen's `Isometry3d` for transforms
- Robot-agnostic: Works with any robot that has URDF

### Planning Interface
```cpp
// Target as pose
moveit::core::RobotState& current_state = *move_group.getCurrentState();
geometry_msgs::Pose target_pose;
target_pose.orientation.w = 1.0;
target_pose.position.x = 0.28;
// ...
move_group.setPoseTarget(target_pose);
```

### Key Insight
MoveIt separates:
- **Kinematics**: Robot-specific (configured via URDF/SRDF)
- **Poses**: Robot-agnostic (SE3/Isometry3d)
- **Planning**: Robot-agnostic algorithms
- **Execution**: Robot-specific drivers

---

## 3. Vendor SDKs

### FANUC (via RMI/PCDK)
```
Position:
  x, y, z (mm) - Cartesian position
  w, p, r (degrees) - Euler angles (ZYX convention)
  ext1, ext2, ext3 - External axes

Configuration:
  UFrame, UTool - Active frame/tool numbers
  Front/Up/Left/Flip - Arm pose flags
  Turn4/5/6 - Joint turn indicators
```

### ABB (RAPID)
```
robtarget:
  trans (pos) - x, y, z
  rot (orient) - quaternion (q1, q2, q3, q4)
  robconf - cf1, cf4, cf6, cfx (arm configuration)
  extax - external axes
```

### KUKA (KRL)
```
E6POS:
  X, Y, Z (mm)
  A, B, C (degrees) - Euler angles (ZYZ)
  S, T - Status and Turn bits
  E1-E6 - External axes
```

### Universal Robots (URScript)
```
pose = p[x, y, z, rx, ry, rz]
  # Position in mm, rotation as rotation vector (axis-angle)
```

### Key Observation
- **Position**: All vendors use x, y, z for translation
- **Rotation**: Varies significantly:
  - FANUC: Euler WPR (ZYX)
  - ABB: Quaternion
  - KUKA: Euler ABC (ZYZ)
  - UR: Rotation vector (axis-angle)
- **Configuration**: Vendor-specific arm pose flags
- **External axes**: All support, but representation varies

---

## 4. Academic / Research Standards

### SE(3) - Special Euclidean Group
The mathematical group of rigid body transformations in 3D:
- 3 DOF translation
- 3 DOF rotation
- Represented as 4×4 homogeneous matrix or Isometry3

### Representations
| Type | Pros | Cons |
|------|------|------|
| 4×4 Matrix | Universal, composable | 16 floats, redundant |
| Isometry3 | Efficient (7 floats), singularity-free | Quaternion normalization needed |
| Euler Angles | Human-readable, compact (6 floats) | Gimbal lock, interpolation issues |
| Axis-Angle | Compact rotation (4 floats) | Singularity at 0/360° |

### Best Practice Consensus
1. **Store and compute in quaternions** (no gimbal lock)
2. **Display/input as Euler angles** (human-readable)
3. **Interpolate with slerp** (smooth motion)
4. **Separate position from configuration** (robot-agnostic where possible)

---

## 5. Implications for pl3xus

### What Meteorite Gets Right
1. Uses `Isometry3<f32>` for toolpath points (singularity-free)
2. Separates configuration (FanucConfig) from position
3. Uses slerp for rotation interpolation

### What Needs Work
1. `convert_to_position()` drops quaternion info (hardcodes w=0, p=0, r=180)
2. No frame transform chain (like TF2)
3. Configuration hardcoded at execution time

### Recommended Approach
```rust
// Universal position type (robot-agnostic)
pub struct RobotPose {
    pub position: Isometry3<f32>,  // SE3 transform
    pub frame_id: String,          // "world", "base", "tool", etc.
}

// Robot-specific configuration (FANUC-specific)
pub struct FanucArmConfig {
    pub uframe: i8,
    pub utool: i8,
    pub front: i8,
    pub up: i8,
    pub left: i8,
    pub flip: i8,
    pub turn: [i8; 3],
    pub external_axes: Option<[f32; 3]>,
}

// Conversion trait for robot-specific types
trait ToVendorPosition {
    fn to_fanuc(&self, config: &FanucArmConfig) -> fanuc_rmi::Position;
    // fn to_abb(&self, config: &AbbArmConfig) -> abb_rapid::RobTarget;
    // fn to_kuka(&self, config: &KukaArmConfig) -> kuka_krl::E6Pos;
}
```

---

## Research Questions Still Open

1. Should frame transforms (UFrame/UTool offsets) be Isometry3 as well?
2. How to handle external axes (7+ DOF) in the abstraction?
3. Where should quaternion→Euler conversion happen (client, server, driver)?
4. Performance cost of nalgebra Isometry3 vs vendor-native types?

