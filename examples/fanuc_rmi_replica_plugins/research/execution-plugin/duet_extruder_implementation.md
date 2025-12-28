# Duet Extruder Implementation Plan

> **Goal:** Implement a coordinated FANUC robot + Duet extruder system as the first full implementation of our execution plugin architecture, with a simulator for testing.

## 1. Meteorite Analysis

### How Meteorite Does It

**Duet Communication:**
```
HTTP GET: http://{duet_ip}/rr_gcode?gcode={urlencoded_gcode}
```

**Key Components:**

1. **DuetChannel** - Async channel for sending G-code commands
   ```rust
   pub struct DuetChannel {
       pub gcode_tx: UnboundedSender<String>,
       pub poll_tx: UnboundedSender<()>,
       pub abort_tx: UnboundedSender<()>,
   }
   ```

2. **ExtruderCylinder** - Physical parameters
   ```rust
   pub struct ExtruderCylinder {
       pub position: ExtruderPosition,  // Current Y position (mm)
       pub nozzle_diameter: f32,
       pub piston_diameter: f32,
       pub reservoir_volume_used: f32,
   }
   ```

3. **ExtruderMoveEvent** - Command sent per toolpath point
   ```rust
   pub struct ExtruderMoveEvent {
       pub distance: f32,  // mm to move piston
       pub speed: f32,     // mm/s
   }
   ```

**Coordination Flow:**
```
motion_dispatch() system:
  1. Get next toolpath point
  2. Calculate robot motion → send to FANUC via RMI
  3. Calculate piston delta → send ExtruderMoveEvent
  4. ExtruderMoveEvent → DuetChannel.gcode_tx → "G1 Y{distance} F{speed*60}"
```

**Piston Calculation:**
```rust
fn calculate_piston_deltas(
    prev_pos, next_pos,
    surface_speed,           // Robot speed (mm/s)
    piston_diameter,         // Cylinder diameter
    layer_height, bead_width // Print parameters
) -> (piston_distance, piston_speed) {
    let travel_distance = distance(prev_pos, next_pos);
    let exec_time = travel_distance / surface_speed;
    
    let flow_rate = layer_height * bead_width * surface_speed;  // mm³/s
    let piston_area = π * (piston_diameter/2)²;
    let piston_speed = flow_rate / piston_area;  // mm/s
    let piston_distance = piston_speed * exec_time;  // mm
    
    (piston_distance, piston_speed)
}
```

---

## 2. Our Architecture Mapping

### How It Maps to Our Execution Plugin

| Meteorite | Our Architecture |
|-----------|------------------|
| PrinterExecutionBuffer | ToolpathBuffer |
| motion_dispatch system | Orchestrator system |
| RmiDriver | impl MotionDevice for FanucDriver |
| DuetChannel + ExtruderMoveEvent | impl AuxiliaryDevice for DuetExtruder |
| Printer entity | Entity with ExecutionCoordinator |
| Extruder as child of Printer | Extruder entity with ExecutionTarget marker |

### Entity Hierarchy

```
System
└── Printer [ExecutionCoordinator, ToolpathBuffer, BufferState]
    ├── Robot [FanucDriver, MotionDevice impl, ExecutionTarget, PrimaryMotion]
    └── Extruder [DuetExtruder, AuxiliaryDevice impl, ExecutionTarget]
        └── ExtruderConfig [piston_diameter, nozzle_diameter, etc.]
```

---

## 3. Components to Implement

### In execution_plugin (traits + orchestrator)

```rust
// Already defined in README.md:
// - ToolpathBuffer, BufferState, ExecutionCoordinator
// - MotionDevice, AuxiliaryDevice traits
// - ExecutionPoint, MotionCommand, AuxiliaryCommand
```

### In duet_extruder_plugin (new)

```rust
/// Configuration for the Duet-based extruder
#[derive(Component)]
pub struct DuetExtruderConfig {
    pub duet_ip: String,
    pub piston_diameter: f32,      // mm
    pub nozzle_diameter: f32,      // mm
    pub max_piston_travel: f32,    // mm (cylinder height)
}

/// Current state of the extruder
#[derive(Component)]
pub struct ExtruderState {
    pub piston_position: f32,      // Current Y position (mm)
    pub is_connected: bool,
}

/// Channel for async communication with Duet
#[derive(Component)]
pub struct DuetChannel {
    pub gcode_tx: UnboundedSender<String>,
}

/// Implements AuxiliaryDevice trait
pub struct DuetExtruder;

impl AuxiliaryDevice for DuetExtruder {
    fn device_type(&self) -> &str { "duet_extruder" }
    
    fn send_command(&mut self, cmd: &AuxiliaryCommand) -> Result<(), DeviceError> {
        if let AuxiliaryCommand::Extruder { distance, speed } = cmd {
            let gcode = format!("G1 Y{:.4} F{:.1}", distance, speed * 60.0);
            self.channel.gcode_tx.send(gcode)?;
        }
        Ok(())
    }
    
    fn is_ready(&self) -> bool { true }  // Duet buffers commands
}
```

---

## 4. Duet Simulator

### Purpose
Test the full pipeline without real hardware.

### Implementation

```rust
// In examples/duet_simulator/

use axum::{routing::get, Router, extract::Query};
use serde::Deserialize;

#[derive(Deserialize)]
struct GcodeQuery {
    gcode: String,
}

#[derive(Default)]
struct SimulatorState {
    position: AtomicF32,        // Current Y position
    commands_received: AtomicU64,
}

async fn handle_gcode(
    Query(query): Query<GcodeQuery>,
    State(state): State<Arc<SimulatorState>>,
) -> impl IntoResponse {
    let gcode = urlencoding::decode(&query.gcode).unwrap();
    println!("[Duet Sim] Received: {}", gcode);
    
    // Parse G1 Y{pos} F{speed} commands
    if let Some(caps) = GCODE_REGEX.captures(&gcode) {
        let y_pos: f32 = caps["y"].parse().unwrap();
        state.position.store(y_pos, Ordering::SeqCst);
    }
    
    state.commands_received.fetch_add(1, Ordering::SeqCst);
    
    // Duet returns empty JSON on success
    Json(json!({}))
}

async fn handle_model(State(state): State<Arc<SimulatorState>>) -> impl IntoResponse {
    // Return object model with current position
    Json(json!({
        "result": {
            "move": {
                "axes": [{ "machinePosition": state.position.load(Ordering::SeqCst) }]
            }
        }
    }))
}

#[tokio::main]
async fn main() {
    let state = Arc::new(SimulatorState::default());
    
    let app = Router::new()
        .route("/rr_gcode", get(handle_gcode))
        .route("/rr_model", get(handle_model))
        .with_state(state);
    
    println!("Duet Simulator running on http://127.0.0.1:8080");
    axum::Server::bind(&"0.0.0.0:8080".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
```

---

## 5. Implementation Phases

### Phase 0: Create Crate Structure

**execution_plugin crate:**
```
plugins/
├── execution_plugin/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── plugin.rs
│       ├── components/
│       │   ├── mod.rs
│       │   ├── buffer.rs          # ToolpathBuffer, BufferState
│       │   ├── coordinator.rs     # ExecutionCoordinator, ExecutionTarget
│       │   └── execution_point.rs # ExecutionPoint, MotionCommand, AuxCommand
│       ├── traits/
│       │   ├── mod.rs
│       │   ├── motion_device.rs   # MotionDevice trait
│       │   ├── auxiliary_device.rs # AuxiliaryDevice trait
│       │   └── path_producer.rs   # PathProducer trait
│       └── systems/
│           ├── mod.rs
│           └── orchestrator.rs    # Main execution loop
```

**duet_extruder_plugin crate:**
```
plugins/
├── duet_extruder_plugin/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── plugin.rs
│       ├── components.rs          # DuetExtruderConfig, ExtruderState, DuetChannel
│       ├── driver.rs              # AuxiliaryDevice impl
│       └── systems.rs             # connect, disconnect, poll
```

**duet_simulator binary:**
```
examples/
├── duet_simulator/
│   ├── Cargo.toml
│   └── src/
│       └── main.rs
```

### Phase 1: Core Types & Traits

1. Define `ExecutionPoint` with motion + aux commands
2. Define `MotionDevice` trait
3. Define `AuxiliaryDevice` trait
4. Define marker components: `ExecutionCoordinator`, `ExecutionTarget`, `PrimaryMotion`

### Phase 2: Buffer Components

1. Implement `ToolpathBuffer` with VecDeque
2. Implement `BufferState` with state machine
3. Add push/pop/peek/clear operations

### Phase 3: Orchestrator System

1. Query coordinators in `Executing` state
2. Check if primary device is ready
3. Pop from buffer, send to devices
4. Update state on completion

### Phase 4: Duet Extruder Plugin

1. Implement `DuetExtruderConfig` component
2. Implement HTTP client with retry
3. Implement `AuxiliaryDevice` for Duet
4. Connect/disconnect systems

### Phase 5: FANUC Integration

1. Implement `MotionDevice` for FANUC driver
2. Use quaternion → WPR conversion at send time
3. Register as `PrimaryMotion` on coordinator

### Phase 6: Duet Simulator

1. Create simple HTTP server (axum)
2. Implement `/rr_gcode` endpoint
3. Implement `/rr_model` endpoint for position polling
4. Log received commands for verification

### Phase 7: End-to-End Test

1. Start Duet simulator
2. Start FANUC simulator (via replay_client)
3. Create test toolpath
4. Execute and verify both devices receive commands

---

## 6. AuxiliaryCommand Structure

```rust
/// Commands that can be sent to auxiliary devices
#[derive(Debug, Clone)]
pub enum AuxiliaryCommand {
    /// Extruder move command
    Extruder {
        distance: f32,     // mm to move piston (relative)
        speed: f32,        // mm/s
    },

    /// Digital output command
    DigitalOutput {
        channel: u8,
        state: bool,
    },

    /// Analog output command
    AnalogOutput {
        channel: u8,
        value: f32,        // 0.0 - 1.0 normalized
    },

    /// Generic G-code pass-through
    Gcode(String),

    /// No operation (skip this device)
    None,
}
```

---

## 7. ExecutionPoint with Extrusion Data

```rust
/// A point in the toolpath with all device commands
#[derive(Debug, Clone)]
pub struct ExecutionPoint {
    /// Unique index in the toolpath
    pub index: u32,

    /// Target pose for motion device (quaternion-based)
    pub target_pose: RobotPose,

    /// Motion parameters
    pub motion_config: MotionConfig,

    /// Commands for auxiliary devices, keyed by device_type
    pub aux_commands: HashMap<String, AuxiliaryCommand>,

    /// Metadata for this point
    pub metadata: PointMetadata,
}

#[derive(Debug, Clone)]
pub struct MotionConfig {
    pub speed: f32,              // mm/s
    pub motion_type: MotionType, // Linear, Joint, Circular
    pub blend_radius: f32,       // mm (0 = stop at point)
}

#[derive(Debug, Clone)]
pub struct PointMetadata {
    pub layer_height: Option<f32>,
    pub bead_width: Option<f32>,
    pub is_travel: bool,         // True if no extrusion
    pub comment: Option<String>,
}
```

---

## 8. Calculating Extrusion at Import Time

When importing a toolpath (e.g., from G-code or database), we pre-calculate the extruder command:

```rust
fn import_toolpath_point(
    prev_point: &ExecutionPoint,
    current_point: &mut ExecutionPoint,
    extruder_config: &ExtruderConfig,
) {
    let travel_distance = prev_point.target_pose.translation
        .distance(&current_point.target_pose.translation);

    let exec_time = travel_distance / current_point.motion_config.speed;

    let layer_height = current_point.metadata.layer_height.unwrap_or(0.0);
    let bead_width = current_point.metadata.bead_width.unwrap_or(0.0);

    // Skip if travel move
    if current_point.metadata.is_travel || layer_height == 0.0 {
        current_point.aux_commands.insert(
            "duet_extruder".into(),
            AuxiliaryCommand::None,
        );
        return;
    }

    let flow_rate = layer_height * bead_width * current_point.motion_config.speed;
    let piston_area = std::f32::consts::PI * (extruder_config.piston_diameter / 2.0).powi(2);
    let piston_speed = flow_rate / piston_area;
    let piston_distance = piston_speed * exec_time;

    current_point.aux_commands.insert(
        "duet_extruder".into(),
        AuxiliaryCommand::Extruder {
            distance: piston_distance,
            speed: piston_speed,
        },
    );
}
```

---

## 9. Framework vs Application Boundary

### Part of pl3xus Framework (generic)

| Component | Reason |
|-----------|--------|
| `execution_plugin` | Universal orchestration pattern |
| `MotionDevice` trait | Any robot can implement |
| `AuxiliaryDevice` trait | Any peripheral can implement |
| `ToolpathBuffer` | Generic execution buffer |
| `ExecutionPoint` | Universal point structure |
| `BufferState` | Universal state machine |

### Part of Application (Meteorite-specific or example)

| Component | Reason |
|-----------|--------|
| `duet_extruder_plugin` | Specific to Duet-based extruders |
| `DuetExtruderConfig` | Application-specific config |
| `calculate_piston_deltas` | Application-specific formula |
| Refill logic | Meteorite-specific |
| Microwave control | Meteorite-specific |

### Boundary Rule
> The execution_plugin provides the **orchestration pattern**. Device plugins implement the **device-specific protocols**. Application code handles **domain-specific calculations** (extrusion math, heating, etc.)

---

## 10. Success Criteria

1. ✅ Duet simulator receives G-code commands in correct order
2. ✅ FANUC simulator receives motion commands with correct WPR
3. ✅ Commands are synchronized (same point index triggers both)
4. ✅ State machine transitions correctly: Idle → Buffering → Ready → Executing → Complete
5. ✅ Can pause/resume execution
6. ✅ Error in one device is propagated correctly

---

## 11. Next Steps

Begin implementation with Phase 0: Create the crate structure.

