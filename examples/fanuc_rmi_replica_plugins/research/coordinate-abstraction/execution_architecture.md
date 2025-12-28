# Execution Architecture: Deep Dive

## The Core Question

**Where does execution state live in a multi-robot, multi-device system?**

This question has implications for:
- Plugin dependencies
- Component placement in entity hierarchy
- Coordination between devices
- Sensor feedback integration
- Scalability to complex work cells

---

## Use Cases to Consider

### Single Robot Scenarios
1. **Robot + Extruder** (3D printing): Robot motion synchronized with extrusion rate
2. **Robot + Welder**: Robot motion + wire feed + voltage/current
3. **Robot + End-Effector Sensor**: Force feedback affects path

### Multi-Robot Scenarios
4. **Two Robots, Coordinated**: Both follow synchronized toolpath (e.g., one holds part, one welds)
5. **Two Robots, Independent**: Each runs its own program
6. **Robot + Positioner**: Coordinated motion with external axis

### Sensor/IO Hierarchy
7. **Robot-Level Sensors**: Force sensor on end effector (affects that robot only)
8. **System-Level Sensors**: Camera watching work cell (affects all robots)
9. **Mixed**: Both robot-local and system-wide sensors

---

## What IS a "Toolpath Point" Really?

In a simple case: `robot position + speed`

In reality, it's a **synchronized command set**:

```rust
/// A single execution step for all coordinated devices
pub struct ExecutionPoint {
    /// Unique sequence number
    pub sequence: usize,
    
    /// Motion command for the primary motion device
    pub motion: Option<MotionCommand>,
    
    /// Commands for auxiliary devices (extruder, welder, IO, etc.)
    pub auxiliaries: Vec<AuxiliaryCommand>,
    
    /// How to synchronize motion with auxiliaries
    pub sync_mode: SyncMode,
    
    /// Timing constraints
    pub timing: Option<TimingConstraint>,
}

pub struct MotionCommand {
    pub pose: RobotPose,
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
    /// Start all commands simultaneously
    Simultaneous,
    /// Motion first, then auxiliaries at point
    MotionThenAux,
    /// Auxiliaries track motion progress (e.g., extrusion rate ~ speed)
    ProportionalToMotion,
}
```

---

## Component Placement Options

### Option A: Robot Entity Only

```
System
└── Robot (ToolpathBuffer, Orchestrator)
    ├── Extruder
    └── Sensor
```

**Pros:**
- Clean ownership
- Each robot is independent
- Simple mental model

**Cons:**
- How do we coordinate multi-robot?
- System-level sensors have no natural home

### Option B: System Entity Only

```
System (ToolpathBuffer, Orchestrator)
├── Robot1
├── Robot2
└── Sensors
```

**Pros:**
- Single source of truth
- Natural for coordinated motion
- System sensors are easy

**Cons:**
- How do we run independent programs on different robots?
- Doesn't scale to many independent cells

### Option C: ExecutionContext as Flexible Grouping

```
System
├── ExecutionContext1 (ToolpathBuffer) ← Component, not entity
│   ├── targets: [Robot1, Extruder1]
│   └── sensors: [EndEffectorSensor]
│
├── ExecutionContext2 (ToolpathBuffer)
│   ├── targets: [Robot2]
│   └── sensors: [Robot2Sensor]
│
└── CellSensor (feeds into... which context?)
```

**Key Insight:** `ExecutionContext` is a **component that can attach to ANY entity**.

---

## The ECS Answer: Hierarchical Flexibility

Use the entity hierarchy itself to express the relationships:

```rust
/// Marker component: "This entity coordinates execution for its children"
#[derive(Component)]
pub struct ExecutionCoordinator {
    /// Buffer for this coordination scope
    pub buffer: ToolpathBuffer,
    
    /// Devices that receive commands from this coordinator
    pub devices: Vec<Entity>,
    
    /// Sensors that provide feedback to this coordinator
    pub feedback_sources: Vec<Entity>,
}
```

### Example Hierarchies

**Single Robot (Simple):**
```
System
└── Robot [ExecutionCoordinator] ← buffer lives here
    ├── Extruder [AuxiliaryDevice]
    └── ForceSensor [FeedbackSource]
```

**Multi-Robot Independent:**
```
System
├── Robot1 [ExecutionCoordinator] ← has its own buffer
│   └── Sensor1 [FeedbackSource]
└── Robot2 [ExecutionCoordinator] ← has its own buffer
    └── Sensor2 [FeedbackSource]
```

**Multi-Robot Coordinated:**
```
System [ExecutionCoordinator] ← single buffer coordinates both
├── Robot1 [MotionDevice]
├── Robot2 [MotionDevice]
└── CellCamera [FeedbackSource]
```

**Hybrid (most complex):**
```
System [ExecutionCoordinator] ← high-level phase coordination
├── Robot1 [ExecutionCoordinator] ← detailed motion for robot1
│   └── EndEffector1 [FeedbackSource]
├── Robot2 [ExecutionCoordinator] ← detailed motion for robot2
│   └── EndEffector2 [FeedbackSource]
└── CellCamera [FeedbackSource] ← feeds System coordinator
```

---

## ToolpathBuffer Revised Design

```rust
#[derive(Component)]
pub struct ToolpathBuffer {
    /// The buffer of points to execute
    points: VecDeque<ExecutionPoint>,
    
    /// Index of point currently being executed (in-flight)
    current_index: Option<usize>,
    
    /// Count of fully completed points
    completed_count: usize,
    
    /// Expected total (None = streaming/unknown)
    expected_total: Option<usize>,
    
    /// Maximum buffer capacity (for backpressure)
    capacity: usize,
    
    /// Execution state
    state: ExecutionState,
    
    /// Associated motion devices (Entities with MotionDevice component)
    motion_devices: Vec<Entity>,
    
    /// Associated auxiliary devices
    auxiliary_devices: Vec<Entity>,
    
    /// Feedback sources that can modify upcoming points
    feedback_sources: Vec<Entity>,
}

pub enum ExecutionState {
    Idle,
    Buffering,      // Receiving points, not executing yet
    Ready,          // Has points, ready to start
    Executing,      // Actively sending commands
    Paused,         // User-initiated pause
    WaitingForFeedback, // Waiting for sensor/condition
    Complete,
    Error(ExecutionError),
}
```

---

## Plugin Architecture

### The Problem: Avoiding Spaghetti

If we're not careful:
- `execution_plugin` depends on `fanuc_plugin` depends on `core` depends on `execution_plugin`
- Every plugin knows about every other plugin
- Adding a new device type requires modifying 5 plugins

### The Solution: Trait-Based Abstraction

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          EXECUTION PLUGIN                                    │
│                     (knows about abstractions only)                          │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│   Defines traits:                  Contains:                                 │
│   - MotionDevice                   - ToolpathBuffer component                │
│   - AuxiliaryDevice                - ExecutionCoordinator component          │
│   - FeedbackSource                 - Orchestrator system                     │
│   - PathProducer                   - Buffer management systems               │
│                                                                              │
│   Does NOT know about:                                                       │
│   - FANUC, ABB, UR specifics                                                │
│   - Extruder details                                                        │
│   - Sensor protocols                                                        │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ Defines traits that...
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                          DEVICE PLUGINS                                      │
│                     (implement the traits)                                   │
├─────────────┬─────────────┬─────────────┬─────────────┬─────────────────────┤
│ fanuc_rmi   │ abb_egm     │ extruder    │ plc_io      │ sensor_feedback     │
│             │             │             │             │                     │
│ impl Motion │ impl Motion │ impl Aux    │ impl Aux    │ impl Feedback       │
│ Device for  │ Device for  │ Device for  │ Device for  │ Source for          │
│ FanucRobot  │ AbbRobot    │ Extruder    │ PlcDevice   │ Sensor              │
└─────────────┴─────────────┴─────────────┴─────────────┴─────────────────────┘
```

### Trait Definitions (in execution_plugin)

```rust
/// A device that can execute motion commands
pub trait MotionDevice: Send + Sync {
    /// Send a motion command to the device
    fn send_motion(&self, cmd: &MotionCommand) -> Result<(), DeviceError>;

    /// Check if device is ready for next command
    fn ready_for_next(&self) -> bool;

    /// Get current position (for feedback)
    fn current_pose(&self) -> Option<RobotPose>;

    /// Convert from internal format to device-specific (vendor-specific)
    fn prepare_motion(&self, cmd: &MotionCommand) -> Box<dyn Any>;
}

/// A device that receives auxiliary commands synchronized with motion
pub trait AuxiliaryDevice: Send + Sync {
    /// Device type identifier
    fn device_type(&self) -> &str;

    /// Send an auxiliary command
    fn send_command(&self, cmd: &AuxiliaryCommand) -> Result<(), DeviceError>;

    /// Check device status
    fn is_ready(&self) -> bool;
}

/// A source of feedback that can influence execution
pub trait FeedbackSource: Send + Sync {
    /// Read current feedback value
    fn read(&self) -> FeedbackValue;

    /// Check if feedback suggests path modification
    fn needs_adjustment(&self) -> Option<PathAdjustment>;
}

/// A system that produces toolpath points
pub trait PathProducer: Send + Sync {
    /// Produce next points (may return multiple for look-ahead)
    fn produce(&mut self, context: &ProducerContext) -> Vec<ExecutionPoint>;

    /// Check if more points are available
    fn has_more(&self) -> bool;

    /// Total expected points (None = streaming/unknown)
    fn expected_total(&self) -> Option<usize>;
}
```

### Plugin Dependency Graph

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                               APPLICATION                                    │
│          (composes plugins, configures entity hierarchy)                     │
└─────────────────────────────────────────────────────────────────────────────┘
                │
                │ uses
                ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                          execution_plugin                                    │
│           Depends on: core (only)                                           │
│           Defines: MotionDevice, AuxiliaryDevice, FeedbackSource traits     │
└─────────────────────────────────────────────────────────────────────────────┘
                │
                │ defines traits implemented by...
                ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  fanuc_plugin    │  abb_plugin     │  extruder_plugin  │  sensor_plugin     │
│                  │                 │                   │                    │
│  Depends on:     │  Depends on:    │  Depends on:      │  Depends on:       │
│  - core          │  - core         │  - core           │  - core            │
│  - execution     │  - execution    │  - execution      │  - execution       │
│                  │                 │                   │                    │
│  Does NOT        │  Does NOT       │  Does NOT         │  Does NOT          │
│  depend on       │  depend on      │  depend on        │  depend on         │
│  other devices   │  other devices  │  other devices    │  other devices     │
└─────────────────────────────────────────────────────────────────────────────┘
                │
                │ all depend on...
                ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                               core                                           │
│           Base ECS, events, common types, coordinate types                  │
│           NO execution logic, NO device specifics                           │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Key Rules

1. **core** knows nothing about execution or devices
2. **execution_plugin** knows nothing about specific devices
3. **device_plugins** know nothing about each other
4. **application** wires everything together

---

## The Orchestrator Question

**Should orchestrator be in execution_plugin or separate?**

### Arguments for execution_plugin:
- Orchestrator IS execution - it's the core consumer of the buffer
- Tight coupling with ToolpathBuffer is natural
- Simpler plugin structure

### Arguments for separate:
- Different orchestration strategies might exist
- Could swap orchestrators without touching execution_plugin
- Separation of concerns

### Recommendation: Keep in execution_plugin (for now)

The orchestrator is the "brain" of execution. Separating it creates artificial complexity. If we later need multiple orchestration strategies, we can make it a trait with multiple implementations within the same plugin.

```rust
// In execution_plugin
pub trait Orchestrator: Send + Sync {
    fn tick(
        &mut self,
        buffer: &mut ToolpathBuffer,
        motion_devices: &[&dyn MotionDevice],
        aux_devices: &[&dyn AuxiliaryDevice],
        feedback: &[&dyn FeedbackSource],
    ) -> OrchestratorResult;
}

// Default implementation
pub struct StandardOrchestrator {
    // ...
}

impl Orchestrator for StandardOrchestrator {
    fn tick(&mut self, ...) { ... }
}
```

---

## Summary: Recommended Architecture

### Component Placement
- `ToolpathBuffer` attaches to **any entity that coordinates execution**
- Single robot → attach to Robot entity
- Multi-robot coordinated → attach to System entity
- Multi-robot independent → attach to each Robot entity separately

### Plugin Structure
```
core/                    # Base types, ECS, no execution logic
execution_plugin/        # Buffer, Orchestrator, device traits
├── components/
│   ├── toolpath_buffer.rs
│   ├── execution_coordinator.rs
│   └── execution_point.rs
├── traits/
│   ├── motion_device.rs
│   ├── auxiliary_device.rs
│   ├── feedback_source.rs
│   └── path_producer.rs
├── systems/
│   ├── orchestrator.rs
│   ├── buffer_management.rs
│   └── feedback_integration.rs
└── lib.rs

fanuc_plugin/            # Implements MotionDevice for FANUC
extruder_plugin/         # Implements AuxiliaryDevice for extruders
sensor_plugin/           # Implements FeedbackSource for sensors
```

### Data Flow
```
PathProducers → ToolpathBuffer → Orchestrator → Devices
                     ↑                             │
                     └────── FeedbackSources ──────┘
```

### Key Insight
**The ECS hierarchy IS the configuration.** We don't hard-code where components live - we let the entity relationships express the coordination structure.
```

