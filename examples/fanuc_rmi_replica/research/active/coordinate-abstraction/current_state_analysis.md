# Current State Analysis

## Position Flow in fanuc_rmi_replica

### 1. Data Entry (Client)
```
User Input → UI Components → Request Messages → Server
```

The client's `Composer` and `CommandInput` components accept:
- Position: x, y, z (text inputs → f64)
- Orientation: w, p, r (Euler angles in degrees)
- Speed, termination type, etc.

### 2. Storage (Database via fanuc_api)
```sql
-- program_instructions table
x, y, z REAL  -- position in mm
w, p, r REAL  -- orientation in degrees (Euler ZYX)
uframe, utool INTEGER  -- frame/tool selection
speed_type, speed_value, term_type, term_value  -- motion params
```

### 3. Execution (Server)
```rust
// In program.rs: instruction_to_packet()
fn instruction_to_packet(instruction: &ProgramInstruction, defaults: &ExecutionDefaults) -> SendPacket {
    let position = dto::Position {
        x: instruction.x,
        y: instruction.y,
        z: instruction.z,
        w: instruction.w,
        p: instruction.p,
        r: instruction.r,
        ext1: 0.0, ext2: 0.0, ext3: 0.0,  // ⚠️ External axes not stored
    };

    let configuration = Configuration {
        u_tool_number: instruction.utool.unwrap_or(1) as i8,
        u_frame_number: instruction.uframe.unwrap_or(1) as i8,
        front: 1, up: 1, left: 0, flip: 0,  // ⚠️ Hardcoded!
        turn4: 0, turn5: 0, turn6: 0,        // ⚠️ Hardcoded!
    };

    FrcLinearMotion::new(line_number, configuration, position, speed_type, speed, term_type, term_value)
}
```

### 4. Robot Communication
```
Server → fanuc_rmi driver → TCP/IP → FANUC Controller
```

---

## Position Flow in Meteorite

### 1. Toolpath Generation
```rust
// Geometry generates Isometry3 points
let points: Vec<Isometry3<f32>> = geometry.generate_points();

// Stored as ToolpathPoints component
commands.entity(toolpath_entity).insert(ToolpathPoints {
    points,
    current_index: 0,
    points_completed: 0,
});
```

### 2. Robot Configuration (Stored on Entity)
```rust
// FanucConfig loaded from .env at startup
pub struct FanucConfig(pub Configuration);  // uframe, utool, front, up, left, flip, turn
```

### 3. Execution
```rust
// In toolpath_execution.rs
fn send_toolpath_point(
    fanuc_config: &FanucConfig,  // From entity
    rmi_driver: &RmiDriver,
    point: &NextMoveInfo,        // Contains Isometry3
    // ...
) {
    let motion = FrcLinearMotion::new(
        0,
        fanuc_config.0.clone(),            // Configuration from entity
        convert_to_position(&point.point), // Isometry3 → Position
        SpeedType::MilliSeconds,
        point.move_time as f64 * 1000.0,
        point.term_type.clone(),
        100,
    );
}
```

### 4. The Critical Gap: convert_to_position()
```rust
pub fn convert_to_position(iso: &Isometry3<f32>) -> Position {
    Position {
        x: iso.translation.x as f64,
        y: iso.translation.y as f64,
        z: iso.translation.z as f64,
        w: 0.0,   // ❌ IGNORES quaternion rotation!
        p: 0.0,   // ❌ 
        r: 180.0, // ❌ Hardcoded for downward-facing tool
        ext1: 0.0,
        ext2: 0.0,
        ext3: 0.0,
    }
}
```

**This is the critical bug**: The quaternion rotation stored in Isometry3 is completely discarded. The conversion assumes a fixed tool orientation.

---

## Problems Identified

### 1. Incomplete Quaternion→Euler Conversion
Meteorite stores rotation as quaternion but doesn't convert it when sending to FANUC. This works only because:
- The toolpath geometry always uses the same orientation
- The hardcoded `r=180.0` matches the expected tool-down orientation

### 2. Hardcoded Arm Configuration
Both systems hardcode arm configuration (front/up/left/flip/turn):
- fanuc_rmi_replica: Hardcoded in `instruction_to_packet()`
- Meteorite: Loaded from .env once at startup

In reality, arm configuration should be determined by:
- Current robot pose (which IK solution is closest)
- Collision avoidance requirements
- Process constraints

### 3. No Frame Transform Chain
Neither system models the frame hierarchy:
```
World → UFrame[n] → Robot Base → ... → Flange → UTool[m] → TCP
```

This means:
- Can't visualize position in different frames
- Can't compose transforms for multi-robot systems
- Can't properly handle frame changes mid-program

### 4. External Axes Ignored
Both systems set ext1/ext2/ext3 = 0.0:
- Can't control linear tracks
- Can't control positioners
- Can't coordinate multi-robot cells

---

## What a Proper Abstraction Would Look Like

```rust
/// Robot-agnostic pose representation
pub struct WorldPose {
    pub transform: Isometry3<f64>,  // SE3 in world frame
    pub external_axes: Option<Vec<f64>>,
}

/// Frame relationship
pub struct FrameTransform {
    pub from_frame: FrameId,
    pub to_frame: FrameId,
    pub transform: Isometry3<f64>,
}

/// Robot-specific configuration
pub trait RobotConfiguration {
    fn arm_config(&self) -> &dyn std::any::Any;
    fn active_frames(&self) -> (FrameId, FrameId);  // (uframe, utool)
}

/// Conversion layer (robot-specific driver)
pub trait RobotDriver {
    fn world_to_robot(&self, pose: &WorldPose, config: &dyn RobotConfiguration) -> Box<dyn RobotPosition>;
    fn robot_to_world(&self, pos: &dyn RobotPosition, config: &dyn RobotConfiguration) -> WorldPose;
    fn send_motion(&self, pos: &dyn RobotPosition, motion_params: MotionParams) -> Result<(), Error>;
}
```

This separation enables:
1. Robot-agnostic toolpath storage
2. Proper frame transform handling
3. Driver-specific conversions
4. Multi-robot coordination

