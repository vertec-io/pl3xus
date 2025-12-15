# Control Demo Example

This example demonstrates the `ExclusiveControlPlugin` for managing exclusive control of entities across multiple clients.

## Overview

The `ExclusiveControlPlugin` is an optional utility plugin in `pl3xus_sync` that provides common patterns for exclusive control transfer. It enables:

- **Exclusive control**: Only one client can control an entity at a time
- **Control requests**: Clients can request to take or release control
- **Automatic timeout**: Inactive clients automatically lose control
- **Hierarchy propagation**: Controlling a parent entity can grant control of children
- **State synchronization**: All clients see who has control of each entity

## Running the Example

### Start the Server

```bash
cargo run -p control_demo_server
```

The server will start on `ws://127.0.0.1:8083/sync` and spawn 3 robots that clients can control.

### Connect Clients

Currently, this example only includes the server. To test the control functionality, you can:

1. Use the DevTools demo client to connect and visualize the robots
2. Create a custom client using `pl3xus_client` to send control requests
3. Use a WebSocket client to manually send control messages

## How It Works

### Server Setup

The server uses the `ExclusiveControlPlugin` with the following configuration:

```rust
app.add_plugins(ExclusiveControlPlugin::new(ExclusiveControlConfig {
    timeout_seconds: Some(30.0),  // 30 second timeout
    propagate_to_children: true,   // Control parent = control children
}));

// Add the control systems for WebSocket provider
app.add_exclusive_control_systems::<WebSocketProvider>();

// Sync the EntityControl component so clients can see who has control
app.sync_component::<EntityControl>(None);
```

### Control Flow

1. **Request Control**: Client sends `ControlRequest::Take(entity_id)`
2. **Server Checks**: Server verifies entity exists and is not already controlled
3. **Grant Control**: Server adds `EntityControl` component with client ID
4. **Sync State**: `EntityControl` is synced to all clients
5. **Client Commands**: Client can now send commands (e.g., `MoveCommand`)
6. **Validate Commands**: Server checks that client has control before executing
7. **Release Control**: Client sends `ControlRequest::Release(entity_id)` or times out

### Message Types

**Control Requests** (from client to server):
```rust
pub enum ControlRequest {
    Take(u64),    // entity_id to take control of
    Release(u64), // entity_id to release control of
}
```

**Control Responses** (from server to client):
```rust
pub enum ControlResponse {
    Taken,
    Released,
    AlreadyControlled { by_client: ConnectionId },
    NotControlled,
    Error(String),
}
```

**Entity Control State** (synced to all clients):
```rust
pub struct EntityControl {
    pub client_id: ConnectionId,
    pub last_activity: f32,
}
```

## Key Features Demonstrated

### 1. Exclusive Control Semantics

Only one client can control an entity at a time. If another client tries to take control, they receive an `AlreadyControlled` response.

### 2. Automatic Timeout

If a client doesn't send any commands for 30 seconds (configurable), they automatically lose control. This prevents "stuck" control states.

### 3. Command Validation

The server validates that the client has control before executing commands:

```rust
// Check if the client has control
if control.client_id != client_id {
    warn!("Client {:?} tried to move robot but it's controlled by {:?}",
        client_id, control.client_id);
    continue;
}
```

### 4. Activity Tracking

Each command updates the `last_activity` timestamp to prevent timeout:

```rust
control.last_activity = time.elapsed_secs();
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         Server                              │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  ExclusiveControlPlugin                              │  │
│  │  - handle_control_requests (Take/Release)            │  │
│  │  - timeout_inactive_control (30s timeout)            │  │
│  │  - notify_control_changes (sync to clients)          │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Application Logic                                   │  │
│  │  - handle_move_commands (validate control)           │  │
│  │  - update_robot_status                               │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Entities                                            │  │
│  │  - Robot (position, name)                            │  │
│  │  - RobotStatus (battery, is_moving)                  │  │
│  │  - EntityControl (client_id, last_activity)          │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                            │
                            │ WebSocket
                            │
                ┌───────────┴───────────┐
                │                       │
         ┌──────▼──────┐         ┌──────▼──────┐
         │  Client A   │         │  Client B   │
         │             │         │             │
         │ - See all   │         │ - See all   │
         │   robots    │         │   robots    │
         │ - Request   │         │ - Request   │
         │   control   │         │   control   │
         │ - Send      │         │ - Send      │
         │   commands  │         │   commands  │
         └─────────────┘         └─────────────┘
```

## Next Steps

To fully test this example, you would need to:

1. Create a Leptos client that uses `pl3xus_client` to:
   - Display the list of robots
   - Show which client has control of each robot
   - Provide UI to request/release control
   - Send move commands when in control

2. Or use the DevTools demo client to visualize and interact with the robots

3. Or create a simple WebSocket client to manually test the control protocol

