# Research Project: Execution Plugin Architecture

> **Status:** Design Complete, Ready for Implementation  
> **Created:** 2025-12-27  
> **Last Updated:** 2025-12-27  
> **Owner:** Architecture Team

## Executive Summary

This research project defines the architecture for a **buffer-based toolpath execution system** that:

1. Stores toolpaths in **quaternion format** (Isometry3) for mathematical consistency
2. Uses a **ToolpathBuffer component** for flexible execution (static, streaming, real-time)
3. Leverages **ECS hierarchy as configuration** for multi-robot/multi-device coordination
4. Implements **trait-based device abstraction** to avoid plugin interdependencies

## Table of Contents

1. [Problem Statement](#1-problem-statement)
2. [Architectural Decisions](#2-architectural-decisions)
3. [Component Design](#3-component-design)
4. [Plugin Architecture](#4-plugin-architecture)
5. [Data Flow](#5-data-flow)
6. [Database Schema](#6-database-schema)
7. [Implementation Phases](#7-implementation-phases)
8. [Diagrams](#8-diagrams)

---

## 1. Problem Statement

### Current State Issues

1. **FANUC-specific throughout**: The orchestrator uses `Instruction` with `x,y,z,w,p,r` (FANUC Euler) directly
2. **No abstraction layer**: Adding ABB, UR, or other robots requires rewriting core logic
3. **Static execution only**: Load all points → execute sequentially (no streaming/real-time)
4. **No multi-device coordination**: Robot, extruder, sensors can't synchronize

### Vision

Build a robotics platform that supports:
- **Multiple robot vendors** (FANUC, ABB, UR, KUKA, etc.)
- **Real-time toolpath generation** with sensor feedback
- **Multi-robot coordination** (independent or synchronized)
- **Auxiliary devices** (extruders, welders, PLCs, sensors)

---

## 2. Architectural Decisions

### Decision 1: Internal Coordinate Representation

| Option | Storage | Pros | Cons |
|--------|---------|------|------|
| Euler (vendor-specific) | x,y,z,w,p,r | Familiar to operators | Gimbal lock, vendor-specific conventions |
| **Quaternion (Isometry3)** | tx,ty,tz,qw,qx,qy,qz | Math-safe, interpolation-friendly | Less intuitive for humans |

**Decision:** Use **quaternion (Isometry3)** internally. Convert on import and at driver layer.

### Decision 2: Conversion Boundaries

```
External → [Import Layer: euler_to_quat] → Database → Buffer → Orchestrator → [Driver: quat_to_vendor] → Robot
```

**Decision:** Convert at **import time** and **driver output time** only.

### Decision 3: Component Placement

**Decision:** `ExecutionCoordinator` and `ToolpathBuffer` are **components that can attach to any entity** in the hierarchy. The ECS hierarchy IS the configuration.

- Single robot → attach to Robot entity
- Multi-robot coordinated → attach to System entity
- Multi-robot independent → attach to each Robot entity

### Decision 4: Device References

**Decision:** Use **ECS relationship components** instead of `Vec<Entity>` fields:

```rust
// Instead of:
struct ToolpathBuffer {
    motion_devices: Vec<Entity>,  // ❌ Anti-pattern
}

// Use:
#[derive(Component)]
struct ExecutionTarget;  // Marker for devices receiving commands

#[derive(Component)]
struct FeedbackProvider;  // Marker for sensors providing feedback
```

### Decision 5: Orchestrator Location

**Decision:** Orchestrator lives in `execution_plugin`. It's the core consumer of the buffer—separating it adds complexity without benefit.

---

## 3. Component Design

### ExecutionCoordinator (Marker Component)

```rust
/// Marks an entity as an execution coordinator.
/// The coordinator manages execution for its children (or referenced entities).
#[derive(Component, Default)]
pub struct ExecutionCoordinator;
```

### ToolpathBuffer Component

```rust
#[derive(Component)]
pub struct ToolpathBuffer {
    /// Ring buffer of pending execution points
    points: VecDeque<ExecutionPoint>,
    
    /// Maximum buffer capacity (for backpressure)
    capacity: usize,
}
```

### BufferState Component (Separate for ECS queries)

```rust
#[derive(Component)]
pub struct BufferState {
    /// Index of point currently being executed (in-flight to robot)
    pub current_index: Option<usize>,
    
    /// Count of fully completed points
    pub completed_count: usize,
    
    /// Expected total points (None = streaming/unknown)
    pub expected_total: Option<usize>,
    
    /// Current execution state
    pub state: ExecutionState,
}

pub enum ExecutionState {
    Idle,
    Buffering,          // Receiving points, not executing yet
    Ready,              // Has points, ready to start
    Executing,          // Actively sending commands
    Paused,             // User-initiated pause
    WaitingForFeedback, // Blocked on sensor/condition
    Complete,
    Error(String),
}
```

### ExecutionPoint (The Synchronized Command)

```rust
/// A single execution step for all coordinated devices
pub struct ExecutionPoint {
    /// Unique sequence number
    pub sequence: usize,
    
    /// Motion command (robot pose + speed + termination)
    pub motion: Option<MotionCommand>,
    
    /// Auxiliary commands (extruder, welder, IO, etc.)
    pub auxiliaries: Vec<AuxiliaryCommand>,
    
    /// How to synchronize motion with auxiliaries
    pub sync_mode: SyncMode,
}

pub struct MotionCommand {
    pub pose: RobotPose,           // From pl3xus_robotics crate
    pub speed: Speed,
    pub termination: TerminationType,
}

pub enum AuxiliaryCommand {
    Extruder { flow_rate: f64, temperature: f64 },
    Welder { wire_feed: f64, voltage: f64, current: f64 },
    DigitalOutput { port: u8, value: bool },
    AnalogOutput { port: u8, value: f64 },
    Wait { condition: WaitCondition },
}

pub enum SyncMode {
    Simultaneous,         // Start all commands at same time
    MotionThenAux,        // Motion first, auxiliaries at point
    ProportionalToMotion, // Aux rate tracks motion progress
}
```

### Device Marker Components

```rust
/// Marks entity as a target that receives execution commands
#[derive(Component)]
pub struct ExecutionTarget;

/// Marks entity as a source of feedback that can modify execution
#[derive(Component)]
pub struct FeedbackProvider;

/// Marks entity as the primary motion device
#[derive(Component)]
pub struct PrimaryMotion;
```

---

## 4. Plugin Architecture

### Layer Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                               APPLICATION                                    │
│          (composes plugins, configures entity hierarchy)                     │
└─────────────────────────────────────────────────────────────────────────────┘
                │
                ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                          execution_plugin                                    │
│           Depends on: core (only)                                           │
│           Contains: ToolpathBuffer, BufferState, ExecutionCoordinator       │
│           Defines: MotionDevice, AuxiliaryDevice, FeedbackSource traits     │
└─────────────────────────────────────────────────────────────────────────────┘
                │
                │ defines traits implemented by...
                ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  fanuc_plugin    │  abb_plugin     │  extruder_plugin  │  sensor_plugin     │
│                  │                 │                   │                    │
│  impl Motion     │  impl Motion    │  impl Auxiliary   │  impl Feedback     │
│  Device for      │  Device for     │  Device for       │  Source for        │
│  FanucRobot      │  AbbRobot       │  Extruder         │  Sensor            │
│                  │                 │                   │                    │
│  Depends on:     │  Depends on:    │  Depends on:      │  Depends on:       │
│  core, execution │  core, execution│  core, execution  │  core, execution   │
└─────────────────────────────────────────────────────────────────────────────┘
                │
                ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                               core                                           │
│           Base ECS, events, RobotPose, coordinate types                     │
│           NO execution logic, NO device specifics                           │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Key Rules

1. **core** knows nothing about execution or devices
2. **execution_plugin** knows nothing about specific devices (only traits)
3. **device_plugins** know nothing about each other
4. **application** wires everything together via entity hierarchy

### Trait Definitions (in execution_plugin)

```rust
/// A device that can execute motion commands
pub trait MotionDevice: Send + Sync + 'static {
    /// Send a motion command to the device
    fn send_motion(&mut self, cmd: &MotionCommand) -> Result<(), DeviceError>;

    /// Check if device is ready for next command
    fn ready_for_next(&self) -> bool;

    /// Get current position (for feedback/display)
    fn current_pose(&self) -> Option<RobotPose>;
}

/// A device that receives auxiliary commands synchronized with motion
pub trait AuxiliaryDevice: Send + Sync + 'static {
    /// Device type identifier
    fn device_type(&self) -> &str;

    /// Send an auxiliary command
    fn send_command(&mut self, cmd: &AuxiliaryCommand) -> Result<(), DeviceError>;

    /// Check device status
    fn is_ready(&self) -> bool;
}

/// A source of feedback that can influence execution
pub trait FeedbackSource: Send + Sync + 'static {
    /// Read current feedback value
    fn read(&self) -> FeedbackValue;

    /// Check if feedback suggests path modification needed
    fn needs_adjustment(&self) -> Option<PathAdjustment>;
}

/// A system that produces toolpath points
pub trait PathProducer: Send + Sync + 'static {
    /// Produce next batch of points
    fn produce(&mut self, context: &ProducerContext) -> Vec<ExecutionPoint>;

    /// Check if more points are available
    fn has_more(&self) -> bool;

    /// Total expected points (None = streaming/unknown)
    fn expected_total(&self) -> Option<usize>;
}
```

---

## 5. Data Flow

### Complete Data Flow Diagram

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
│  │              PATH PRODUCERS (ECS Systems)              │                  │
│  │   StaticLoader | StreamingImporter | RealtimeGenerator │                  │
│  └───────────────────────────┬───────────────────────────┘                  │
│                              │                                               │
│                              ▼                                               │
│  ┌───────────────────────────────────────────────────────┐                  │
│  │                 TOOLPATH BUFFER                        │                  │
│  │   VecDeque<ExecutionPoint>                            │                  │
│  │   (Runtime execution queue)                           │                  │
│  └───────────────────────────┬───────────────────────────┘                  │
│                              │                                               │
│                              ▼                                               │
│  ┌───────────────────────────────────────────────────────┐                  │
│  │                 ORCHESTRATOR SYSTEM                    │                  │
│  │   Consumes buffer, manages timing, handles state      │                  │
│  └───────────────────────────┬───────────────────────────┘                  │
│                              │                                               │
│  ════════════════════ CONVERSION BOUNDARY ═══════════════                   │
│                              │                                               │
│                              ▼                                               │
│  ┌───────────────────────────────────────────────────────┐                  │
│  │                 DRIVER LAYER                           │                  │
│  │   quaternion_to_vendor_format()                        │                  │
│  │   FanucDriver: quat→WPR                               │                  │
│  │   ABBDriver: quat→quat (native!)                      │                  │
│  │   URDriver: quat→axis-angle                           │                  │
│  └───────────────────────────────────────────────────────┘                  │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Feedback Loop

```
                    ┌──────────────────────────┐
                    │    FeedbackProvider      │
                    │    (Sensor Entities)     │
                    └────────────┬─────────────┘
                                 │
                    ┌────────────▼─────────────┐
                    │   Feedback System        │
                    │   reads sensor data      │
                    └────────────┬─────────────┘
                                 │
            ┌────────────────────┼────────────────────┐
            ▼                    ▼                    ▼
   ┌────────────────┐   ┌────────────────┐   ┌────────────────┐
   │ Modify upcoming│   │ Adjust current │   │ Pause/abort    │
   │ buffer points  │   │ speed/params   │   │ execution      │
   └────────────────┘   └────────────────┘   └────────────────┘
```

---

## 6. Database Schema

### New Quaternion-Based Schema

```sql
-- Programs table (metadata)
CREATE TABLE programs (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    source_type TEXT NOT NULL,  -- "native", "gcode", "slicer", "csv"
    source_file TEXT,           -- Original filename if imported
    point_count INTEGER,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);

-- Toolpath points table (quaternion storage)
CREATE TABLE toolpath_points (
    id INTEGER PRIMARY KEY,
    program_id INTEGER NOT NULL REFERENCES programs(id) ON DELETE CASCADE,
    sequence_number INTEGER NOT NULL,

    -- Isometry3 representation (position + orientation)
    tx REAL NOT NULL,  -- Translation X
    ty REAL NOT NULL,  -- Translation Y
    tz REAL NOT NULL,  -- Translation Z
    qw REAL NOT NULL,  -- Quaternion W (scalar)
    qx REAL NOT NULL,  -- Quaternion X
    qy REAL NOT NULL,  -- Quaternion Y
    qz REAL NOT NULL,  -- Quaternion Z

    -- Motion parameters
    speed REAL NOT NULL,
    speed_type TEXT NOT NULL DEFAULT 'mm_sec',  -- "mm_sec", "percent"
    term_type TEXT NOT NULL DEFAULT 'continuous',  -- "fine", "continuous"
    term_value INTEGER DEFAULT 50,  -- CNT value 0-100

    -- Frame reference (for future use)
    frame_type TEXT NOT NULL DEFAULT 'world',  -- "world", "user_frame", "tool"
    frame_number INTEGER DEFAULT 0,

    -- External axes (nullable, for extended systems)
    ext1 REAL, ext2 REAL, ext3 REAL, ext4 REAL, ext5 REAL, ext6 REAL,

    -- Auxiliary commands (JSON blob for flexibility)
    auxiliary_commands TEXT,  -- JSON: [{"type": "extruder", "flow_rate": 1.5}, ...]

    UNIQUE(program_id, sequence_number)
);

-- Index for efficient sequential access
CREATE INDEX idx_toolpath_program_seq ON toolpath_points(program_id, sequence_number);
```

### Migration Strategy

Since we're in development, we can drop and recreate:

```sql
DROP TABLE IF EXISTS instructions;
DROP TABLE IF EXISTS programs;

-- Create new schema (above)
```

---

## 7. Implementation Phases

### Phase 0: Create execution_plugin Crate Structure

**Goal:** Establish the plugin with core traits and types.

**Deliverables:**
- [ ] New crate: `crates/execution_plugin/`
- [ ] Trait definitions: `MotionDevice`, `AuxiliaryDevice`, `FeedbackSource`, `PathProducer`
- [ ] Component definitions: `ExecutionCoordinator`, `ToolpathBuffer`, `BufferState`
- [ ] Type definitions: `ExecutionPoint`, `MotionCommand`, `AuxiliaryCommand`
- [ ] Plugin struct implementing Bevy `Plugin` trait

**Files to create:**
```
crates/execution_plugin/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Plugin definition, exports
│   ├── components/
│   │   ├── mod.rs
│   │   ├── execution_coordinator.rs
│   │   ├── toolpath_buffer.rs
│   │   └── buffer_state.rs
│   ├── traits/
│   │   ├── mod.rs
│   │   ├── motion_device.rs
│   │   ├── auxiliary_device.rs
│   │   ├── feedback_source.rs
│   │   └── path_producer.rs
│   ├── types/
│   │   ├── mod.rs
│   │   ├── execution_point.rs
│   │   ├── motion_command.rs
│   │   └── auxiliary_command.rs
│   └── systems/
│       ├── mod.rs
│       └── placeholder.rs  # To be filled in later phases
```

### Phase 1: Database Schema Migration

**Goal:** Create new quaternion-based database schema.

**Deliverables:**
- [ ] Drop old `instructions` table
- [ ] Create new `programs` table
- [ ] Create new `toolpath_points` table with quaternion storage
- [ ] Update database initialization code

### Phase 2: ToolpathBuffer Implementation

**Goal:** Implement the buffer component with full functionality.

**Deliverables:**
- [ ] `ToolpathBuffer` with `VecDeque<ExecutionPoint>`
- [ ] `BufferState` with execution tracking
- [ ] Push/pop/peek operations
- [ ] Backpressure handling (capacity limits)
- [ ] Unit tests for buffer operations

### Phase 3: Import Layer

**Goal:** Create importers for external toolpath formats.

**Deliverables:**
- [ ] `euler_to_quaternion()` conversion utility
- [ ] G-Code parser (basic linear moves)
- [ ] CSV/slicer importer
- [ ] Static program loader (from database)
- [ ] All importers produce `ExecutionPoint`s

### Phase 4: Orchestrator System

**Goal:** Implement the core execution consumer.

**Deliverables:**
- [ ] Orchestrator ECS system
- [ ] Consume from `ToolpathBuffer`
- [ ] Query for `ExecutionTarget` entities
- [ ] Call `MotionDevice::send_motion()` on targets
- [ ] Handle `BufferState` transitions
- [ ] Timing and synchronization logic

### Phase 5: FANUC Driver Integration

**Goal:** Implement `MotionDevice` trait for FANUC.

**Deliverables:**
- [ ] `impl MotionDevice for FanucRobot`
- [ ] `quaternion_to_wpr()` conversion at driver layer
- [ ] Integration with existing `fanuc_rmi` driver
- [ ] Update `fanuc_plugin` to depend on `execution_plugin`

### Phase 6: End-to-End Testing

**Goal:** Validate complete pipeline works.

**Deliverables:**
- [ ] Import test toolpath (G-code or CSV)
- [ ] Verify quaternion storage in database
- [ ] Load into buffer
- [ ] Execute via orchestrator
- [ ] Verify correct WPR sent to FANUC robot

---

## 8. Diagrams

### Entity Hierarchy Examples

#### Single Robot Configuration
```
System
└── Robot [ExecutionCoordinator, ToolpathBuffer, BufferState]
    ├── FanucDriver [MotionDevice impl, ExecutionTarget]
    ├── Extruder [AuxiliaryDevice impl, ExecutionTarget]
    └── ForceSensor [FeedbackSource impl, FeedbackProvider]
```

#### Multi-Robot Independent Configuration
```
System
├── Robot1 [ExecutionCoordinator, ToolpathBuffer, BufferState]
│   ├── FanucDriver [MotionDevice impl, ExecutionTarget]
│   └── Sensor1 [FeedbackSource impl, FeedbackProvider]
└── Robot2 [ExecutionCoordinator, ToolpathBuffer, BufferState]
    ├── ABBDriver [MotionDevice impl, ExecutionTarget]
    └── Sensor2 [FeedbackSource impl, FeedbackProvider]
```

#### Multi-Robot Coordinated Configuration
```
System [ExecutionCoordinator, ToolpathBuffer, BufferState]
├── Robot1 [MotionDevice impl, ExecutionTarget, PrimaryMotion]
├── Robot2 [MotionDevice impl, ExecutionTarget]
├── Positioner [AuxiliaryDevice impl, ExecutionTarget]
└── CellCamera [FeedbackSource impl, FeedbackProvider]
```

#### Hybrid Configuration (Multi-Level Coordination)
```
System [ExecutionCoordinator]  ← High-level phase coordination
├── Cell1 [ExecutionCoordinator, ToolpathBuffer, BufferState]
│   ├── Robot1 [MotionDevice impl, ExecutionTarget]
│   └── EndEffector1 [FeedbackSource impl, FeedbackProvider]
├── Cell2 [ExecutionCoordinator, ToolpathBuffer, BufferState]
│   ├── Robot2 [MotionDevice impl, ExecutionTarget]
│   └── EndEffector2 [FeedbackSource impl, FeedbackProvider]
└── CellCamera [FeedbackSource impl, FeedbackProvider]  ← Feeds System-level
```

---

## Appendix A: Coordinate Conversion Reference

### Euler (W, P, R) to Quaternion

```rust
use nalgebra::{UnitQuaternion, Vector3};

/// Convert FANUC W, P, R (degrees) to quaternion
/// FANUC convention: Z-Y-X intrinsic (W=Yaw, P=Pitch, R=Roll)
pub fn wpr_to_quaternion(w: f64, p: f64, r: f64) -> UnitQuaternion<f64> {
    let w_rad = w.to_radians();
    let p_rad = p.to_radians();
    let r_rad = r.to_radians();

    UnitQuaternion::from_euler_angles(r_rad, p_rad, w_rad)
}
```

### Quaternion to Euler (W, P, R)

```rust
/// Convert quaternion to FANUC W, P, R (degrees)
pub fn quaternion_to_wpr(q: &UnitQuaternion<f64>) -> (f64, f64, f64) {
    let (roll, pitch, yaw) = q.euler_angles();

    let w = yaw.to_degrees();
    let p = pitch.to_degrees();
    let r = roll.to_degrees();

    (w, p, r)
}
```

---

## Appendix B: Related Research

See the `coordinate-abstraction/` folder for earlier research:
- `current_state_analysis.md` - Analysis of existing codebase
- `industry_comparison.md` - How ABB, UR, KUKA handle coordinates
- `quaternion_to_euler.md` - Mathematical details of conversion
- `buffer_architecture.md` - Early buffer design exploration
- `execution_architecture.md` - Plugin architecture exploration

---

## Appendix C: Open Questions

1. **External axis handling:** How do we represent coordinated external axes in `ExecutionPoint`? Current design has `ext1-ext6` in database, but needs trait support.

2. **Look-ahead:** How many points should the orchestrator look ahead? This affects timing calculations and cornering behavior.

3. **Error recovery:** When a device errors, should we:
   - Pause and wait for user intervention?
   - Retry the current point?
   - Skip and continue?

4. **Multi-device synchronization:** For `SyncMode::ProportionalToMotion`, how do we track motion progress to sync auxiliary devices?

---

*End of Research Document*

