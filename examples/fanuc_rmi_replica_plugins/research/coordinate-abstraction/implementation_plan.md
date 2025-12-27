# Implementation Plan: Coordinate Abstraction

## Overview

This plan implements the **Hybrid Approach** (Option C from decision_matrix.md):
- Universal `Isometry3<f64>` for toolpaths/orchestration
- Vendor-specific types for robot feedback/drivers
- Clear conversion boundary at driver layer

---

## Phase 1: Notify Meteorite's developers of the convert_to_position() (IMMEDIATE)

### Goal
Make a file that documents the existing `plugins/src/toolpath/utils.rs` ignoring of the quaternion rotation and provide a recommendation to actually use the quaternion rotation.

### Suggestions to Include
1. [ ] Update `convert_to_position()` to extract Euler angles from quaternion
2. [ ] Implement `position_to_isometry()` for the reverse direction
3. [ ] Add unit tests with known rotations
4. [ ] Validate against real robot position feedback


---

## Phase 2: Create Core Types Crate

### Goal
Define robot-agnostic types in a shared crate.

### New Crate: `crates/pl3xus_robotics`

```rust
// crates/pl3xus_robotics/src/lib.rs

pub mod pose;
pub mod frame;
pub mod conversion;

// Re-export key types
pub use pose::{RobotPose, ToolpathPoint};
pub use frame::{FrameId, FrameTransform};
pub use conversion::{EulerConvention, ToEuler, FromEuler};
```

### Core Types

```rust
// pose.rs
use nalgebra::Isometry3;

/// Robot-agnostic pose in a named frame
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RobotPose {
    pub transform: Isometry3<f64>,
    pub frame_id: FrameId,
}

/// Toolpath point with motion parameters
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolpathPoint {
    pub pose: RobotPose,
    pub speed: f64,
    pub term_type: TerminationType,
    pub external_axes: Option<[f64; 6]>,
}
```

### Estimated Effort: 4-8 hours

---

## Phase 3: FANUC Driver Conversion Layer

### Goal
Implement bidirectional conversion between universal and FANUC-specific types.

### New Module: in fanuc_rmi_replica

```rust
// conversion.rs
use pl3xus_robotics::{RobotPose, FrameId};
use fanuc_rmi::{Position, Configuration};

pub trait FanucConversion {
    fn to_fanuc_position(&self) -> Position;
    fn from_fanuc_position(pos: &Position, frame: FrameId) -> Self;
}

impl FanucConversion for RobotPose {
    fn to_fanuc_position(&self) -> Position {
        let (w, p, r) = quaternion_to_wpr_degrees(&self.transform.rotation);
        Position {
            x: self.transform.translation.x,
            y: self.transform.translation.y,
            z: self.transform.translation.z,
            w, p, r,
            ext1: 0.0, ext2: 0.0, ext3: 0.0,
        }
    }
    
    fn from_fanuc_position(pos: &Position, frame: FrameId) -> Self {
        RobotPose {
            transform: position_to_isometry(pos),
            frame_id: frame,
        }
    }
}
```

### Estimated Effort: 4-6 hours

---

## Phase 4: Update fanuc_rmi_replica

### Goal
Use universal types internally while maintaining current API.

### Changes

1. **Instruction Storage** (optional - can keep current format)
   - Store as Isometry3 internally
   - Convert on display/edit

2. **Execution Pipeline**
   ```rust
   // program.rs
   fn instruction_to_packet(instruction: &ProgramInstruction, ...) -> SendPacket {
       let pose = RobotPose {
           transform: instruction_to_isometry(instruction),
           frame_id: FrameId::UserFrame(instruction.uframe),
       };
       
       let position = pose.to_fanuc_position();
       // ... build motion packet
   }
   ```

3. **Robot Feedback** (keep vendor-specific)
   ```rust
   // RobotPosition remains as dto::Position wrapper for sync
   pub struct RobotPosition(pub dto::Position);
   ```

### Estimated Effort: 8-16 hours

---

## Phase 5: Frame Transform System (Future)

### Goal
Implement a frame tree similar to ROS TF2.

### Concept
```rust
pub struct FrameTree {
    transforms: HashMap<(FrameId, FrameId), Isometry3<f64>>,
}

impl FrameTree {
    pub fn lookup(&self, from: FrameId, to: FrameId) -> Option<Isometry3<f64>> {
        // Find path through tree and compose transforms
    }
    
    pub fn transform_pose(&self, pose: &RobotPose, to_frame: FrameId) -> RobotPose {
        let transform = self.lookup(pose.frame_id, to_frame)?;
        RobotPose {
            transform: transform * pose.transform,
            frame_id: to_frame,
        }
    }
}
```

### Estimated Effort: 16-32 hours

---

## Testing Strategy

### Unit Tests
- Quaternion ↔ Euler round-trip accuracy
- Identity transform conversions
- Gimbal lock edge cases
- 90°/180°/270° rotations

### Integration Tests
- Send converted position to robot, read back, compare
- Execute toolpath with varied orientations
- Compare with teach pendant readings

### Validation
- Record robot positions via teach pendant
- Compare our conversion output
- Document any discrepancies

---

## Migration Strategy

1. **Phase 1-2**: Non-breaking, new code only
2. **Phase 3**: Add conversion functions, keep old code working
3. **Phase 4**: Gradually update consumers to use new types
4. **Phase 5**: Future enhancement

Each phase should be a separate PR with tests.

