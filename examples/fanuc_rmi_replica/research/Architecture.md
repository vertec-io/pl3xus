# Architecture Deep Dive

## System Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           FANUC RMI Replica                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────────┐    WebSocket     ┌──────────────────────────────────┐ │
│  │   Leptos Client  │ ◄──────────────► │       Bevy Server                │ │
│  │   (WASM)         │    :8083/sync    │                                  │ │
│  │                  │                  │  ┌────────────────────────────┐  │ │
│  │  pl3xus_client   │                  │  │  FANUC Driver (async)      │  │ │
│  │                  │                  │  │  - fanuc_rmi crate         │  │ │
│  │  - use_sync_*    │                  │  │  - TCP to :16001           │  │ │
│  │  - use_request   │                  │  └────────────────────────────┘  │ │
│  │  - SyncContext   │                  │                                  │ │
│  └──────────────────┘                  │  pl3xus + pl3xus_sync            │ │
│                                        │  - SyncServerPlugin              │ │
│                                        │  - ExclusiveControlPlugin        │ │
│                                        └──────────────────────────────────┘ │
│                                                           │                  │
│                                                           │ TCP              │
│                                                           ▼                  │
│                                        ┌──────────────────────────────────┐ │
│                                        │   FANUC Simulator (sim)          │ │
│                                        │   - Listens on :16001            │ │
│                                        │   - Simulates robot controller   │ │
│                                        └──────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Data Flow

### 1. Real-time State Sync (Server → Client)

```
[Bevy ECS]                    [WebSocket]                [Leptos Client]
    │                              │                           │
    │ RobotPosition component      │                           │
    │ changes on robot entity      │                           │
    ├─────────────────────────────►│                           │
    │ SyncServerPlugin detects     │ NetworkPacket             │
    │ component change, serializes │ {type:"RobotPosition",    │
    │ and broadcasts               │  data: [...]}            │
    │                              ├──────────────────────────►│
    │                              │                           │ SyncProvider receives
    │                              │                           │ routes to SyncContext
    │                              │                           │ updates incoming_messages
    │                              │                           │
    │                              │                           │ use_sync_component<T>()
    │                              │                           │ deserializes and returns
    │                              │                           │ ReadSignal<HashMap<u64,T>>
    │                              │                           │
    │                              │                           │ UI re-renders with new data
```

### 2. Control Request Flow (Client → Server → Client)

```
[Client clicks REQUEST CONTROL]
         │
         ▼
┌────────────────────────────────┐
│ SyncContext::send_control_request(Take) │
│ - entity_bits = 0xFFFFFFFF     │
│ - client_id = ctx.client_id    │
└────────────────────────────────┘
         │
         ▼ ControlRequest message
         │
┌────────────────────────────────────────────┐
│ Server: ExclusiveControlPlugin             │
│ - Receives ControlRequest::Take            │
│ - Checks if entity already controlled      │
│ - If free: add EntityControl component     │
│            send ControlResponse::Granted   │
│ - If taken: send ControlResponse::Denied   │
└────────────────────────────────────────────┘
         │
         ▼ ControlResponse message
         │
┌────────────────────────────────────────────┐
│ Client: handle_incoming_message            │
│ - Routes to "ControlResponse" signal       │
│ - ControlResponseHandler effect triggers   │
│ - Shows toast: "Control granted"           │
│ - Button updates to "IN CONTROL"           │
└────────────────────────────────────────────┘
```

### 3. Request/Response Pattern (Database Operations)

```
[Client wants list of robots]
         │
         ▼
┌────────────────────────────────┐
│ use_request::<ListRobotConnections>() │
│ returns (data: RwSignal, fetch: Fn)   │
└────────────────────────────────┘
         │ fetch() called
         ▼ RequestInternal message with request data
         │
┌────────────────────────────────────────────┐
│ Server: RequestHandlerPlugin               │
│ - Deserializes request                     │
│ - Queries database                         │
│ - Serializes response                      │
│ - Sends ResponseInternal message           │
└────────────────────────────────────────────┘
         │
         ▼ ResponseInternal message
         │
┌────────────────────────────────┐
│ Client: SyncProvider           │
│ - Matches response to request  │
│ - Deserializes and updates     │
│   the data signal              │
└────────────────────────────────┘
```

## Key Structs

### Server Side

```rust
// Entity with robot state
struct RobotEntity {
    RobotPosition,      // x, y, z, w, p, r
    JointAngles,        // j1..j6
    RobotStatus,        // connected, e_stopped, etc.
    ConnectionState,    // ip, port, connected
    ExecutionState,     // Idle/Running/Paused
    EntityControl,      // client_id, last_activity (optional)
}

// Driver connection
struct FanucDriver {
    stream: TcpStream,
    // methods: read_position, jog, etc.
}
```

### Client Side

```rust
// Provided by SyncProvider
struct SyncContext {
    ws: WebSocket,
    incoming_messages: RwSignal<HashMap<String, RwSignal<Vec<u8>>>>,
    client_id: StoredValue<Option<u64>>,
    // methods: send, subscribe_component, etc.
}
```

## File Locations

| Component | Server | Client |
|-----------|--------|--------|
| Entry point | `server/src/main.rs` | `client/src/main.rs` |
| App setup | - | `client/src/app.rs` |
| Layout | - | `client/src/layout/` |
| Pages | - | `client/src/pages/` |
| Components | - | `client/src/components/` |
| Plugins | `server/src/plugins/` | - |
| Database | `server/src/database.rs` | - |
| Driver | `server/src/driver/` | - |
| Shared types | `types/src/lib.rs` | `types/src/lib.rs` |

