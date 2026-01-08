# Buffer-Based Toolpath Architecture

## Vision Summary

1. **Real-time generation**: Toolpath points generated during execution based on parameters + sensor feedback
2. **External import support**: G-code, slicers output Euler angles → convert on import
3. **Buffer execution model**: Toolpath buffer that producers push to, consumer (orchestrator) pulls from
4. **Static programs as special case**: "Load full program" = push all points to buffer at once

---

## Core Concept: ToolpathBuffer

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        BUFFER-BASED EXECUTION MODEL                          │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│   PRODUCERS (push to buffer)              CONSUMER (pulls from buffer)       │
│   ════════════════════════                ════════════════════════════       │
│                                                                              │
│   ┌─────────────────────┐                                                   │
│   │  Static Program     │──┐                                                │
│   │  Loader System      │  │                                                │
│   └─────────────────────┘  │                                                │
│                            │         ┌───────────────────────┐              │
│   ┌─────────────────────┐  │         │                       │              │
│   │  G-Code Importer    │──┼────────▶│   ToolpathBuffer      │              │
│   │  (Euler → Quat)     │  │         │   (ring buffer)       │              │
│   └─────────────────────┘  │         │                       │              │
│                            │         │   [pt3][pt4][pt5]...  │──────┐       │
│   ┌─────────────────────┐  │         │    ▲              ▲   │      │       │
│   │  Real-time Path     │──┤         │  write          read  │      │       │
│   │  Generator System   │  │         │  cursor        cursor │      ▼       │
│   └─────────────────────┘  │         └───────────────────────┘  ┌───────┐   │
│                            │                                    │ Orch. │   │
│   ┌─────────────────────┐  │                                    │System │   │
│   │  Sensor Feedback    │──┘                                    └───┬───┘   │
│   │  Adjustment System  │                                           │       │
│   └─────────────────────┘                                           ▼       │
│                                                              ┌───────────┐  │
│                                                              │  Driver   │  │
│                                                              │  Layer    │  │
│                                                              └───────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Key Design Decisions

### 1. Storage Format: Quaternion (Isometry3)

**Database stores quaternion-based poses:**
```sql
CREATE TABLE toolpath_points (
    id INTEGER PRIMARY KEY,
    program_id INTEGER,
    sequence_number INTEGER,  -- Order within program
    
    -- Position (translation)
    tx REAL NOT NULL,
    ty REAL NOT NULL, 
    tz REAL NOT NULL,
    
    -- Orientation (unit quaternion)
    qw REAL NOT NULL,  -- Scalar component
    qx REAL NOT NULL,
    qy REAL NOT NULL,
    qz REAL NOT NULL,
    
    -- Motion parameters
    speed REAL NOT NULL,
    speed_type TEXT NOT NULL,  -- "mm_sec", "percent", etc.
    term_type TEXT NOT NULL,   -- "fine", "continuous"
    term_value INTEGER,        -- CNT value 0-100
    
    -- Frame reference
    frame_type TEXT NOT NULL,  -- "world", "user_frame", "tool"
    frame_number INTEGER,      -- For user_frame/tool types
    
    -- Optional: external axes
    ext1 REAL, ext2 REAL, ext3 REAL, ext4 REAL, ext5 REAL, ext6 REAL
);
```

**Rationale:**
- Quaternions are the native internal format
- External imports (G-code, Euler) convert on import
- No conversion needed during execution (performance)
- Interpolation-friendly (slerp)

### 2. Conversion on Import, Not on Execute

```
External Source          Import Layer              Database
═══════════════          ════════════              ════════

G-Code (X,Y,Z,A,B,C) ──▶ euler_to_quat() ──────▶ Isometry3
Slicer (X,Y,Z,W,P,R) ──▶ euler_to_quat() ──────▶ Isometry3
Native Generator ──────────────────────────────▶ Isometry3
```

**Why convert on import:**
- One-time cost, not per-execution
- Database is always consistent
- Execution path is simple (no conditional conversion)

### 3. Buffer Component Design

```rust
/// Toolpath buffer attached to robot entity.
/// Producers push points, orchestrator consumes.
#[derive(Component)]
pub struct ToolpathBuffer {
    /// Ring buffer of pending points
    points: VecDeque<ToolpathPoint>,
    
    /// Maximum buffer size (backpressure)
    capacity: usize,
    
    /// Points consumed so far (for progress tracking)
    consumed_count: usize,
    
    /// Total points expected (None = streaming/unknown)
    expected_total: Option<usize>,
    
    /// Current execution state
    state: BufferState,
}

pub enum BufferState {
    /// Waiting for points to be pushed
    Empty,
    /// Has points, ready to execute
    Ready,
    /// Actively executing
    Executing,
    /// Paused (can resume)
    Paused,
    /// All expected points consumed
    Complete,
    /// Error occurred
    Error(String),
}
```

---

## Execution Modes

### Mode 1: Static Program (Current Use Case)
```rust
// On "Run Program" command:
fn load_static_program(program_id: i64, buffer: &mut ToolpathBuffer) {
    let points = db.load_all_points(program_id);
    buffer.expected_total = Some(points.len());
    for point in points {
        buffer.push(point);
    }
    buffer.state = BufferState::Ready;
}
```

### Mode 2: Streaming from External Source
```rust
// G-code streaming system
fn gcode_streaming_system(
    mut gcode_reader: ResMut<GCodeReader>,
    mut query: Query<&mut ToolpathBuffer>,
) {
    for mut buffer in query.iter_mut() {
        // Push points as they're parsed, respecting backpressure
        while buffer.has_capacity() {
            if let Some(line) = gcode_reader.next_line() {
                let point = parse_gcode_to_toolpath_point(line);
                buffer.push(point);
            } else {
                break;
            }
        }
    }
}
```

### Mode 3: Real-time Generation with Sensor Feedback
```rust
// Adaptive path generation system
fn adaptive_path_system(
    sensor_data: Res<SensorFeedback>,
    path_params: Res<PathParameters>,
    mut query: Query<(&mut ToolpathBuffer, &RobotPosition)>,
) {
    for (mut buffer, current_pos) in query.iter_mut() {
        if buffer.needs_more_points() {
            // Generate next point based on current state + sensor feedback
            let next_point = generate_adaptive_point(
                current_pos,
                &sensor_data,
                &path_params,
            );
            buffer.push(next_point);
        }
    }
}
```

---

## Orchestrator System

The orchestrator is the **consumer** of the buffer:

```rust
/// Orchestrator system - consumes buffer, sends to driver
fn orchestrator_system(
    time: Res<Time>,
    mut query: Query<(
        &mut ToolpathBuffer,
        &mut OrchestrationState,
        &RobotDriver,
    )>,
) {
    for (mut buffer, mut state, driver) in query.iter_mut() {
        // Check if robot is ready for next point
        if !state.ready_for_next() {
            continue;
        }

        // Pop next point from buffer
        if let Some(point) = buffer.pop() {
            // Convert to vendor format and send
            match driver.send_motion(&point) {
                Ok(()) => {
                    state.mark_sent(point.sequence);
                }
                Err(e) => {
                    buffer.state = BufferState::Error(e.to_string());
                }
            }
        } else if buffer.is_complete() {
            state.execution_complete = true;
        }
        // else: buffer empty but not complete - waiting for producer
    }
}
```

---

## Import Pipeline

### G-Code Import Example

```rust
pub struct GCodeImporter;

impl GCodeImporter {
    /// Import G-code file into database as quaternion-based toolpath
    pub fn import(
        &self,
        gcode_content: &str,
        program_id: i64,
        db: &Database,
    ) -> Result<usize, ImportError> {
        let mut sequence = 0;

        for line in gcode_content.lines() {
            if let Some(motion) = self.parse_motion_line(line)? {
                // Convert Euler to quaternion at import time
                let pose = RobotPose::from_xyz_wpr(
                    motion.x, motion.y, motion.z,
                    motion.a, motion.b, motion.c,  // Euler angles
                    FrameId::World,
                );

                let point = ToolpathPoint {
                    pose,
                    speed: motion.feedrate,
                    termination: TerminationType::Continuous(50),
                    external_axes: None,
                };

                // Store as quaternion in database
                db.insert_toolpath_point(program_id, sequence, &point)?;
                sequence += 1;
            }
        }

        Ok(sequence)
    }
}
```

### Slicer Import (CSV/JSON)

Same pattern - convert on import, store as quaternion.

---

## Database Migration

### New Schema

```sql
-- Programs table (metadata)
CREATE TABLE programs (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    source_type TEXT NOT NULL,  -- "native", "gcode", "slicer", "csv"
    source_file TEXT,           -- Original filename if imported
    point_count INTEGER,
    created_at TEXT,
    updated_at TEXT
);

-- Toolpath points table (quaternion storage)
CREATE TABLE toolpath_points (
    id INTEGER PRIMARY KEY,
    program_id INTEGER NOT NULL REFERENCES programs(id),
    sequence_number INTEGER NOT NULL,

    -- Isometry3 representation
    tx REAL NOT NULL, ty REAL NOT NULL, tz REAL NOT NULL,
    qw REAL NOT NULL, qx REAL NOT NULL, qy REAL NOT NULL, qz REAL NOT NULL,

    -- Motion parameters
    speed REAL NOT NULL,
    speed_type TEXT NOT NULL DEFAULT 'mm_sec',
    term_type TEXT NOT NULL DEFAULT 'continuous',
    term_value INTEGER DEFAULT 50,

    -- Frame reference
    frame_type TEXT NOT NULL DEFAULT 'world',
    frame_number INTEGER DEFAULT 0,

    -- External axes (nullable)
    ext1 REAL, ext2 REAL, ext3 REAL, ext4 REAL, ext5 REAL, ext6 REAL,

    UNIQUE(program_id, sequence_number)
);

CREATE INDEX idx_toolpath_program ON toolpath_points(program_id, sequence_number);
```

---

## Summary: Data Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           COMPLETE DATA FLOW                                 │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  EXTERNAL SOURCES                    NATIVE GENERATION                       │
│  ════════════════                    ═════════════════                       │
│                                                                              │
│  ┌──────────┐  ┌──────────┐         ┌──────────────────┐                    │
│  │ G-Code   │  │ Slicer   │         │ Path Generator   │                    │
│  │ (Euler)  │  │ (Euler)  │         │ (Quaternion)     │                    │
│  └────┬─────┘  └────┬─────┘         └────────┬─────────┘                    │
│       │             │                        │                               │
│       ▼             ▼                        │                               │
│  ┌─────────────────────────┐                 │                               │
│  │   IMPORT LAYER          │                 │                               │
│  │   euler_to_quaternion() │                 │                               │
│  └───────────┬─────────────┘                 │                               │
│              │                               │                               │
│              ▼                               ▼                               │
│  ┌───────────────────────────────────────────────────────┐                  │
│  │                    DATABASE                            │                  │
│  │              (Quaternion Storage)                      │                  │
│  │   toolpath_points: tx,ty,tz,qw,qx,qy,qz,...           │                  │
│  └───────────────────────────┬───────────────────────────┘                  │
│                              │                                               │
│                              ▼                                               │
│  ┌───────────────────────────────────────────────────────┐                  │
│  │                 TOOLPATH BUFFER                        │                  │
│  │   VecDeque<ToolpathPoint>                             │                  │
│  │   (Runtime execution queue)                           │                  │
│  └───────────────────────────┬───────────────────────────┘                  │
│                              │                                               │
│                              ▼                                               │
│  ┌───────────────────────────────────────────────────────┐                  │
│  │                 ORCHESTRATOR                           │                  │
│  │   Consumes buffer, manages timing, handles state      │                  │
│  └───────────────────────────┬───────────────────────────┘                  │
│                              │                                               │
│  ════════════════════ CONVERSION BOUNDARY ═══════════════                   │
│                              │                                               │
│                              ▼                                               │
│  ┌───────────────────────────────────────────────────────┐                  │
│  │                 DRIVER LAYER                           │                  │
│  │   quaternion_to_vendor_format()                        │                  │
│  │   FanucDriver: to_wpr()                               │                  │
│  │   ABBDriver: to_quaternion() (native!)                │                  │
│  │   URDriver: to_axis_angle()                           │                  │
│  └───────────────────────────────────────────────────────┘                  │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```
```

