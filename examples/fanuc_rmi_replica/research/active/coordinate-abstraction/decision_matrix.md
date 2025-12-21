# Decision Matrix: Coordinate Abstraction

## The Core Question

Should pl3xus adopt `Isometry3<f64>` (SE3) as the universal coordinate type for robot-agnostic position representation?

---

## Option A: Keep Vendor-Specific Types (Current State)

### How It Works
- `RobotPosition(dto::Position)` with x,y,z,w,p,r in fanuc_rmi_replica
- `Isometry3<f32>` in Meteorite toolpaths (but broken conversion)
- Each application defines its own position types

### Pros
| Benefit | Weight | Notes |
|---------|--------|-------|
| No conversion overhead | Medium | Direct use of vendor types |
| No new dependencies | Low | Already have fanuc_rmi::Position |
| Simpler implementation | Medium | No abstraction layer needed |

### Cons
| Drawback | Weight | Notes |
|----------|--------|-------|
| Not robot-agnostic | High | Can't reuse toolpaths for other robots |
| Euler angle issues | High | Gimbal lock, poor interpolation |
| Inconsistent representations | Medium | Different apps use different types |
| Duplicate code | Medium | Each app converts to vendor types |

### Score: 4/10 for multi-robot system

---

## Option B: Adopt Isometry3<f64> as Universal Type

### How It Works
```rust
// Stored/computed positions
pub struct RobotPose {
    pub transform: Isometry3<f64>,
    pub frame_id: FrameId,
}

// At driver layer, convert to vendor type
impl FanucDriver {
    fn to_vendor_position(&self, pose: &RobotPose) -> dto::Position {
        let (w, p, r) = quaternion_to_wpr(&pose.transform.rotation);
        dto::Position {
            x: pose.transform.translation.x,
            // ...
        }
    }
}
```

### Pros
| Benefit | Weight | Notes |
|---------|--------|-------|
| Robot-agnostic toolpaths | High | Reuse across robot brands |
| Singularity-free rotation | High | No gimbal lock |
| Proper interpolation | High | slerp for smooth motion |
| Industry alignment | High | Matches ROS, MoveIt, ABB |
| Composable transforms | Medium | Easy frame chain math |

### Cons
| Drawback | Weight | Notes |
|----------|--------|-------|
| Conversion overhead | Low | ~100ns per conversion |
| Need quaternion→Euler code | Medium | Must implement correctly |
| External axes not included | Medium | Need separate handling |
| f64 vs f32 decision | Low | Use f64 for precision |
| nalgebra dependency | Low | Already in workspace |

### Score: 8/10 for multi-robot system

---

## Option C: Hybrid Approach

### How It Works
- Store positions in Isometry3 at orchestration layer
- Convert to vendor types at driver layer
- Keep vendor-specific components for robot state feedback

```rust
// Toolpath/instruction positions (robot-agnostic)
pub struct ToolpathPoint {
    pub pose: Isometry3<f64>,
    pub frame_id: FrameId,
    pub external_axes: Option<[f64; 3]>,
}

// Robot feedback (vendor-specific, read-only)
pub struct RobotPosition(pub dto::Position);  // FANUC-specific

// Configuration (vendor-specific)
pub struct FanucArmConfig { ... }
```

### Pros
| Benefit | Weight | Notes |
|---------|--------|-------|
| Best of both worlds | High | Agnostic where possible, specific where needed |
| Minimal breaking changes | High | Keep existing feedback types |
| Clear conversion boundary | High | Driver layer handles conversion |
| External axes supported | Medium | Included in ToolpathPoint |

### Cons
| Drawback | Weight | Notes |
|----------|--------|-------|
| Two position types | Medium | Mental overhead |
| Conversion in both directions | Medium | Feedback also needs conversion |
| More complex architecture | Medium | Need clear boundaries |

### Score: 9/10 for multi-robot system

---

## Recommendation: Option C (Hybrid Approach)

### Rationale

1. **Robot-agnostic where it matters**: Toolpath generation, motion planning, and orchestration should use universal types (Isometry3)

2. **Vendor-specific where unavoidable**: Robot feedback, driver communication, and arm configuration remain vendor-specific

3. **Clear conversion boundary**: The driver layer is the natural place for conversion, matching ROS driver patterns

4. **Incremental adoption**: Can migrate gradually without breaking existing code

---

## Implementation Priority

### Phase 1: Fix Meteorite's convert_to_position()
- Implement proper quaternion → WPR (Euler ZYX) conversion
- Add unit tests with known rotations
- Validate against robot feedback

### Phase 2: Define Universal Types
- Create `RobotPose` struct with Isometry3 + frame_id
- Create `ToolpathPoint` with pose + motion params + external axes
- Add to `pl3xus_common` or new `pl3xus_robotics` crate

### Phase 3: Implement FANUC Driver Conversion
- `RobotPose → dto::Position`
- `dto::Position → RobotPose` (for feedback)
- Handle arm configuration selection

### Phase 4: Update fanuc_rmi_replica
- Use `RobotPose` in instruction storage
- Update UI to input/display quaternions as Euler angles
- Keep `RobotPosition` component for feedback

---

## Open Questions for Further Research

1. **Quaternion → Euler conversion**: Which Euler convention does FANUC use? (ZYX? ZYZ?)
2. **Arm configuration inference**: How to automatically determine front/up/left/flip from IK?
3. **Frame transform storage**: Should frame offsets also use Isometry3?
4. **Performance benchmarks**: What's the actual conversion cost in a 10ms control loop?

