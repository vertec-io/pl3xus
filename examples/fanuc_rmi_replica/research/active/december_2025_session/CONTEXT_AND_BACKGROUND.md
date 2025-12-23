# December 2025 Session: Context and Background

## The pl3xus Framework

### Overview

pl3xus is a **Rust framework for building server-authoritative, real-time synchronized web applications**. It's designed for industrial robotics but is general-purpose.

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        BEVY ECS SERVER                          │
├─────────────────────────────────────────────────────────────────┤
│  Entities:                                                       │
│    System ──┐                                                    │
│             └── Robot (spawned on connect)                       │
│                   ├── ConnectionState                            │
│                   ├── RobotStatus                                │
│                   ├── RobotPosition                              │
│                   ├── IOState                                    │
│                   └── EntityControl                              │
│                                                                  │
│  Plugins:                                                        │
│    - SyncServerPlugin (component sync)                          │
│    - ExclusiveControlPlugin (authorization)                     │
│    - DriverPlugin (FANUC communication)                         │
│    - RequestsPlugin (handle queries/mutations)                  │
└──────────────────────────┬──────────────────────────────────────┘
                           │ WebSocket
                           │ (binary serialization)
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                     LEPTOS WASM CLIENT                           │
├─────────────────────────────────────────────────────────────────┤
│  Context Providers:                                              │
│    - SyncProvider (WebSocket connection, sync state)            │
│    - SystemEntityContext (entity IDs)                           │
│    - WorkspaceContext (UI-local state)                          │
│                                                                  │
│  Hooks:                                                          │
│    - use_entity_component<T>(entity_id) → (Signal<T>, exists)   │
│    - use_components<T>() → HashMap<EntityId, T>                 │
│    - use_query<R>() → QueryHandle<R>                            │
│    - use_mutation<R>(callback) → MutationHandle<R>              │
│    - use_send_targeted<M>() → fn(entity_id, M)                  │
└─────────────────────────────────────────────────────────────────┘
```

### Key Design Principles

1. **Server is source of truth**: All state lives on the server. Clients are views.
2. **Components are synced automatically**: Mark components with `#[derive(SyncComponent)]`
3. **Queries are cached and invalidated by server**: No client-side TTL/polling
4. **Entity targeting**: Messages can target specific entities with authorization
5. **Exclusive control**: Only one client can control an entity at a time

---

## The fanuc_rmi_replica Application

### Purpose

A **replica of the FANUC RMI API web interface** built on pl3xus. Controls FANUC industrial robots.

### Entity Hierarchy

```
System (entity_id: ~0, ActiveSystem marker)
  │
  └── Robot (entity_id: varies, ActiveRobot marker)
        │
        ├── ConnectionState { robot_connected: bool, ip: String, ... }
        ├── RobotStatus { servo_ready: bool, in_motion: bool, ... }
        ├── RobotPosition { joints: [f64; 6], world_coords: ... }
        ├── IOState { din: [...], dout: [...], ... }
        ├── FrameToolDataState { frames: [...], tools: [...], ... }
        └── EntityControl { controlling_client: Option<ConnectionId>, ... }
```

### Key Insight: ConnectionState Lives on Robot Entity

This was a major bug fix in this session. Many components were subscribing to `ConnectionState` on the **system entity** but it actually lives on the **robot entity**.

**Wrong**:
```rust
let (connection_state, _) = use_entity_component::<ConnectionState, _>(
    move || system_ctx.system_entity_id.get()  // ❌ Wrong entity!
);
```

**Correct**:
```rust
let (connection_state, robot_exists) = use_entity_component::<ConnectionState, _>(
    move || system_ctx.robot_entity_id.get()  // ✅ Correct entity
);
let connected = Memo::new(move |_| robot_exists.get() && connection_state.get().robot_connected);
```

---

## Communication Patterns

### 1. Synced Components (Push)

Server pushes component changes to clients automatically.

```rust
// Server: Mark component as synced
#[derive(Component, SyncComponent, Serialize, Deserialize)]
pub struct RobotPosition { pub joints: [f64; 6] }

// Client: Subscribe
let (position, exists) = use_entity_component::<RobotPosition, _>(|| robot_entity_id);
```

### 2. Queries (Pull with Caching)

Client requests data, server responds, client caches until server invalidates.

```rust
// Client
let programs = use_query::<ListPrograms>();
// Access: programs.data(), programs.is_loading(), programs.error()

// Server invalidation (triggers auto-refetch on all clients)
invalidate_queries::<ListPrograms>(&mut sync_state);
```

### 3. Mutations (Fire-and-Forget with Response)

Client sends command, server responds with success/error.

```rust
// Client
let create = use_mutation::<CreateProgram>(move |result| {
    match result {
        Ok(r) if r.success => toast.success("Created!"),
        Ok(r) => toast.error(r.error.unwrap_or_default()),
        Err(e) => toast.error(format!("{e}")),
    }
});
create.send(CreateProgram { name: "test".into() });
```

### 4. Targeted Messages (Entity-Specific Commands)

Messages sent to a specific entity with authorization check.

```rust
// Client
let send_jog = use_send_targeted::<JogCommand>();
send_jog(robot_entity_id, JogCommand { direction: ... });

// Server registration (with exclusive control authorization)
app.message::<JogCommand>()
    .targeted()
    .with_entity_policy(ExclusiveControlPolicy)
    .register();
```

---

## Session History Summary

1. Started with request/response patterns
2. Added entity-targeted messages with authorization
3. Implemented TanStack Query-inspired hooks
4. Added server-side query invalidation
5. Fixed message batching bug
6. Fixed entity targeting (ConnectionState location)
7. Migrated all client code to new API

---

## Important Type Definitions

### SharedTypes (fanuc_replica_types)

```rust
// Connection request (use_request pattern)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConnectToRobot {
    pub connection_id: i64,  // Database ID of saved connection
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConnectToRobotResponse {
    pub success: bool,
    pub error: Option<String>,
    pub entity_id: Option<u64>,  // Robot entity ID on success
}

// Targeted message (fire-and-forget)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JogCommand {
    pub direction: JogDirection,
    pub speed: f64,
}
```

### Client Context Types

```rust
/// Provided by DesktopLayout, accessed via use_system_entity()
#[derive(Clone, Copy)]
pub struct SystemEntityContext {
    pub system_entity_id: Memo<Option<u64>>,  // Always Some after init
    pub robot_entity_id: Memo<Option<u64>>,   // Some when robot connected
}
```

---

## Known Gotchas

### 1. Effect Runs on Mount

Leptos Effects run immediately on mount with initial values. Guard against stale data:

```rust
Effect::new(move |_| {
    // Guard: only process when we expect a response
    if waiting_for_response.get().is_none() {
        return;
    }
    // ... process response
});
```

### 2. robot_exists vs robot_connected

- `robot_exists`: Robot entity is spawned (connection attempt started)
- `robot_connected`: Robot is actually connected to physical robot

Always check both when determining if robot is ready:
```rust
let ready = robot_exists.get() && connection_state.get().robot_connected;
```

### 3. Query Type Names Must Match

Server invalidation uses type names. The `short_type_name()` helper extracts just the type name:

```rust
// Server sends: "GetProgram"
// Client matches against std::any::type_name::<GetProgram>() → "fanuc_replica_types::GetProgram"
// Fixed by short_type_name() extracting just "GetProgram"
```

### 4. Number Inputs in UI

Never use `<input type="number">` - they're difficult with decimals/negatives. Use text inputs with validation instead.

---

## Related Codebases

### meteorite

Located at `/home/apino/dev/meteorite`. Uses an older version of pl3xus (called "eventwork"). Reference for patterns but don't modify.

### Fanuc_RMI_API

The original web application being replicated. Reference for UI/UX and feature parity.

---

## Build Commands

```bash
# Check compilation
cargo check -p fanuc_replica_client
cargo check -p fanuc_replica_server

# Build and run server
cargo run -p fanuc_replica_server

# Build and serve client (with hot reload)
cd examples/fanuc_rmi_replica/client && trunk serve

# Full build
cargo build --release -p fanuc_replica_server
cd examples/fanuc_rmi_replica/client && trunk build --release
```

