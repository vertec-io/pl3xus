# Hierarchical Entity Architecture

## Executive Summary

This document outlines the architectural vision for a scalable, hierarchical entity system where:
- A **System/Apparatus entity** serves as the hierarchy root
- **Robot entities** are spawned as children of the System on connection
- **EntityControl** is managed at the System level with hierarchical propagation
- **Components are filtered by entity** on the client using `where` clauses
- **Executors are entity components**, not resources, enabling multi-system scalability

---

## 1. Current Architecture (Problems)

### 1.1 Entity Spawning
- Robot entity is spawned on `ConnectToRobot` request
- No entity exists before connection → **clients cannot request control**
- All synced components are attached to the single robot entity

### 1.2 Control Model
- `EntityControl` is per-entity, but no entity exists to control before connection
- No hierarchical propagation of control
- Client checks `robot_entity_bits()` which fails if no robot connected

### 1.3 Executor as Resource
- `ProgramExecutor` is a Bevy Resource (singleton)
- Cannot scale to multiple systems/apparatuses
- Tightly coupled to "the one robot"

### 1.4 Client Component Subscription
- `use_sync_component::<T>()` returns `HashMap<u64, T>` (all entities)
- No filtering by entity hierarchy
- No concept of "currently selected robot" in context

---

## 2. Proposed Hierarchical Architecture

### 2.1 Entity Hierarchy

```
System/Apparatus (Root)
├── EntityControl (granted to controlling client)
├── ProgramExecutor (component, not resource)
├── SystemSettings (execution defaults, etc.)
│
├── Robot_1 (child, spawned on connection)
│   ├── RobotConnectionState
│   ├── RobotConnectionDetails
│   ├── RmiDriver (when connected)
│   ├── RobotPosition
│   ├── JointAngles
│   ├── RobotStatus
│   ├── IoStatus
│   ├── ConnectionState (synced)
│   ├── JogSettingsState (synced)
│   └── ...other robot-specific components
│
├── Robot_2 (another child, if multi-robot)
│   └── ...
│
└── (Future: PLC connections, other controllers)
```

### 2.2 Control Flow

1. **Server Startup**: System entity is spawned with `EntityControl` component
2. **Client Connects**: Sees System entity via synced `EntityControl`
3. **Request Control**: Client sends `ControlRequest::Take(system_entity_bits)`
4. **Hierarchical Propagation**: pl3xus propagates control to all children
5. **Connection Request**: Only controller can issue `ConnectToRobot`
6. **Robot Spawned**: As child of System, inherits control from parent

### 2.3 Connection Lifecycle

```
[Client requests ConnectToRobot]
         │
         ▼
[Server validates control at System level]
         │
         ▼
[Spawn Robot entity as child of System]
  - Insert RobotConnectionDetails from DB/message
  - Insert ConnectionState { robot_connecting: true }
  - Set RobotConnectionState::Connecting
         │
         ▼
[handle_connecting_state system]
  - Async connect to FANUC controller
  - On success: insert RmiDriver, set Connected
  - On failure: set Disconnected with error
         │
         ▼
[Robot is now Connected]
  - Polling systems read RmiDriver
  - UI sees ConnectionState { robot_connected: true }
```

### 2.4 Disconnection Lifecycle

```
[Client requests DisconnectRobot(robot_entity_id)]
         │
         ▼
[Server validates control]
         │
         ▼
[Set RobotConnectionState::Disconnecting]
         │
         ▼
[handle_disconnecting_state system]
  - Call driver.disconnect()
  - Wait for acknowledgment
  - Remove RmiDriver, RmiResponseChannel
  - Set RobotConnectionState::Disconnected
         │
         ▼
[Optional: Despawn robot entity or keep for reconnect]
```

---

## 3. Component Placement

### 3.1 System Entity Components
| Component | Purpose |
|-----------|---------|
| `SystemMarker` | Marker struct for queries |
| `EntityControl` | Who controls the apparatus |
| `ProgramExecutor` | Execution state (was resource) |
| `SystemSettings` | Global settings |

### 3.2 Robot Entity Components
| Component | Purpose |
|-----------|---------|
| `FanucRobot` | Marker struct |
| `RobotConnectionState` | State machine (enum) |
| `RobotConnectionDetails` | addr, port, name -- potentially a wrapper on Fanuc_RMI_API's FanucDriverConfig |
| `RmiDriver` | Arc<FanucDriver> (when connected) |
| `RobotPosition` | Cartesian position (synced) |
| `JointAngles` | Joint values (synced) |
| `RobotStatus` | Alarms, mode, etc. (synced) |
| `IoStatus` | Digital/analog I/O (synced) |
| `ConnectionState` | UI-facing connection info (synced) |
| `JogSettingsState` | Jog defaults (synced) |
| `ExecutionState` | Current execution progress (synced) |

---

## 4. Client-Side Architecture

### 4.1 Context Structure

```rust
/// Global application context
struct AppContext {
    /// The root System/Apparatus entity ID
    system_entity_id: RwSignal<Option<u64>>,

    /// Currently selected robot entity ID (for multi-robot support)
    selected_robot_id: RwSignal<Option<u64>>,

    /// Whether this client has control (derived from EntityControl)
    has_control: Memo<bool>,
}
```

### 4.2 Component Filtering

Instead of reading all entities:
```rust
// Current: Gets ALL entities with RobotPosition
let positions = use_sync_component::<RobotPosition>();
// Returns HashMap<u64, RobotPosition> - must filter manually
```

Use entity-filtered subscription:
```rust
// Proposed: Get component for specific entity
let robot_id = app_context.selected_robot_id;
let position = use_sync_component_where::<RobotPosition>(
    move || robot_id.get()
);
// Returns Signal<Option<RobotPosition>> for that entity only
```

### 4.3 Control Request Flow

```rust
fn ControlButton() -> impl IntoView {
    let app_ctx = use_app_context();
    let ctx = use_sync_context();

    // Always use the System entity for control requests
    let system_id = app_ctx.system_entity_id;

    let on_click = move |_| {
        if let Some(id) = system_id.get() {
            ctx.send(ControlRequest::Take(id));
        }
    };

    // ...
}
```

### 4.4 Robot Selection

For multi-robot support:
```rust
fn RobotSelector() -> impl IntoView {
    let app_ctx = use_app_context();
    let robots = use_sync_component::<ConnectionState>(); // All robots

    view! {
        <For
            each=move || robots.get().into_iter()
            key=|(id, _)| *id
            children=move |(id, state)| {
                view! {
                    <button on:click=move |_| app_ctx.selected_robot_id.set(Some(id))>
                        {state.robot_name.clone()}
                    </button>
                }
            }
        />
    }
}
```

---

## 5. Executor as Component

### 5.1 Current (Resource)

```rust
// In main.rs or plugin
app.insert_resource(ProgramExecutor::default());

// In systems
fn execute_program(mut executor: ResMut<ProgramExecutor>, ...) {
    // Single global executor
}
```

### 5.2 Proposed (Component on System Entity)

```rust
// At startup
commands.spawn((
    SystemMarker,
    Name::new("Apparatus"),
    EntityControl::default(),
    ProgramExecutor::default(), // Component, not resource
));

// In systems
fn execute_program(
    mut systems: Query<&mut ProgramExecutor, With<SystemMarker>>,
    robots: Query<&RmiDriver, With<FanucRobot>>,
) {
    for mut executor in systems.iter_mut() {
        // Each system has its own executor
        // Can query child robots via Bevy's Parent/Children
    }
}
```

### 5.3 Benefits

1. **Multi-System Support**: Multiple apparatuses can run independently
2. **Clear Ownership**: Executor state is tied to its system
3. **Hierarchical Queries**: Can query `Children` to find system's robots
4. **Sync-Ready**: Executor state can be synced per-system

---

## 6. State Machine Design

### 6.1 Robot Connection State Machine

```
                    ┌─────────────────────────────────────┐
                    │                                     │
                    ▼                                     │
    [NotSpawned] ──► [Disconnected] ──► [Connecting] ──► [Connected]
                         ▲                                  │
                         │                                  │
                         └────── [Disconnecting] ◄──────────┘
```

### 6.2 State Transitions

| From | Event | To | Handler |
|------|-------|-----|---------|
| NotSpawned | ConnectToRobot | Connecting | `handle_connect_requests` |
| Disconnected | ConnectToRobot | Connecting | `handle_connect_requests` |
| Connecting | Driver connected | Connected | `handle_connecting_state` |
| Connecting | Connection failed | Disconnected | `handle_connecting_state` |
| Connected | DisconnectRobot | Disconnecting | `handle_disconnect_requests` |
| Disconnecting | Cleanup complete | Disconnected | `handle_disconnecting_state` |

### 6.3 System Handlers

```rust
// Handle Connecting → Connected/Disconnected
fn handle_connecting_state(
    robots: Query<(Entity, &RobotConnectionDetails),
                  (With<FanucRobot>, Changed<RobotConnectionState>)>,
    // ...
) { }

// Handle Disconnecting → Disconnected
fn handle_disconnecting_state(
    robots: Query<(Entity, &RmiDriver),
                  (With<FanucRobot>, Added<RobotConnectionState>)>,
    // Filter for Disconnecting state
) { }
```

---

## 7. Migration Path

### Phase 1: System Entity
- [x] Spawn System entity at startup (in progress, needs correction)
- [ ] Move EntityControl to System entity
- [ ] Update client to request control of System entity

### Phase 2: Hierarchical Robots
- [ ] Spawn robots as children of System (`commands.spawn(...).set_parent(system_entity)`)
- [ ] Update queries to use `Parent`/`Children` where appropriate
- [ ] Add `selected_robot_id` to client context

### Phase 3: Executor Migration
- [ ] Convert ProgramExecutor from Resource to Component
- [ ] Attach to System entity
- [ ] Update all systems to query `&mut ProgramExecutor` on System entity

### Phase 4: Client Filtering
- [ ] Implement `use_sync_component_where` or equivalent
- [ ] Update all component reads to filter by entity
- [ ] Remove HashMap iteration patterns

---

## 8. Example: Full Connection Flow

```
1. Server starts
   └── Spawns System entity with EntityControl, ProgramExecutor

2. Client A connects via WebSocket
   └── Receives synced EntityControl (no controller yet)

3. Client A clicks "Request Control"
   └── Sends ControlRequest::Take(system_entity_id)
   └── Server grants control, broadcasts EntityControl update
   └── Client A sees has_control = true

4. Client A selects robot from database list
   └── Sends ConnectToRobot { connection_id: 1 }

5. Server receives ConnectToRobot
   └── Validates: Client A has control of System ✓
   └── Loads robot config from database
   └── Spawns Robot entity as child of System
   └── Sets RobotConnectionState::Connecting

6. handle_connecting_state runs
   └── Async connects to FANUC controller
   └── On success: inserts RmiDriver, sets Connected
   └── Updates ConnectionState { robot_connected: true }

7. Client A sees robot connected
   └── Synced ConnectionState shows connected
   └── Position, status, I/O start updating

8. Client B connects
   └── Sees EntityControl (Client A has control)
   └── Sees Robot entity with all synced components
   └── Can observe but cannot control
```

---

## 9. Key Principles

1. **System Entity is the Control Root**: All control requests target the System
2. **Robots are Children**: Spawned on connection, despawned or reset on disconnect
3. **Executor is Per-System**: Component on System, not global resource
4. **Filter by Entity on Client**: Use `where` clauses, store entity IDs in context
5. **State Machines are Component-Based**: Use `RobotConnectionState` enum component
6. **Hierarchical Control Propagation**: pl3xus handles child control automatically

---

## 10. Design Decisions (Resolved)

### 10.1 Robot Entity Lifecycle ✅ DECIDED
- **On disconnect**: Keep entity in Disconnected state, add `DisconnectedSince(Instant)` component
- **Cleanup**: System monitors for robots disconnected > 10 minutes, despawns them
- **On reconnect**: Remove `DisconnectedSince` component when state changes from Disconnected

```rust
#[derive(Component)]
pub struct DisconnectedSince(pub std::time::Instant);
```

### 10.2 Multi-Robot Support ✅ DECIDED
- **Multiple robots can be connected simultaneously**
- **Only one robot is "active"** via `ActiveRobot` marker component
- Active robot receives orchestrator commands
- **Cannot change active robot while program is running**
- UI shows all connected robots, allows setting active status

```rust
#[derive(Component)]
pub struct ActiveRobot;
```

### 10.3 Execution Architecture ✅ DECIDED
- **ProgramOrchestrator**: System-level component managing program flow
- **RobotExecutionState**: Per-robot synced component showing what robot is doing
- **OrchestratorStatus**: System-level synced component for UI
- See `ORCHESTRATOR_TECHNICAL_SPEC.md` for full details

### 10.4 pl3xus Hierarchical Control ✅ CONFIRMED
- **YES**: pl3xus supports `propagate_to_children: true` in `ExclusiveControlConfig`
- **YES**: `has_control_hierarchical<C, F>()` checks control up the Parent chain
- Uses Bevy's `Parent`/`Children` components for hierarchy
- **No changes needed** - framework already supports this

### 10.5 Client Entity Filtering ✅ CONFIRMED
- **YES**: `use_sync_component_where<T, F>(filter)` exists in pl3xus_client
- Filter is a predicate: `Fn(&T) -> bool`
- Returns filtered `HashMap<u64, T>` as `ReadSignal`
- For single-entity lookup, filter by entity bits:
  ```rust
  let robot_id: u64 = 12345;
  let position = use_sync_component_where::<RobotPosition, _>(
      move |_| true  // Could filter by component properties
  );
  // Or filter the HashMap: position.get().get(&robot_id)
  ```

---

## 11. Summary

This architecture provides:

| Feature | Benefit |
|---------|---------|
| Hierarchical entities | Clear ownership, scalable |
| System-level control | One control point for entire apparatus |
| Robots as children | Spawned on demand, inherit control |
| Executor as component | Per-system, not singleton |
| Entity-filtered sync | Efficient client rendering |
| State machine for connections | Clear lifecycle, proper cleanup |

The key insight is that **control is apparatus-level, not robot-level**. Clients request control of the System, which automatically grants control over all child entities (robots, future PLCs, etc.).

This architecture supports future expansion to:
- Multiple systems/apparatuses
- Multiple robots per system
- Additional device types (PLCs, vision systems, etc.)
- Independent execution per system

