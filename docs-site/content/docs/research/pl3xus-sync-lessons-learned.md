---
title: Pl3xus Sync - Lessons Learned
---
# Pl3xus Sync - Lessons Learned

This document captures key learnings from implementing the fanuc_rmi_replica example with pl3xus_sync.

## 1. Message Routing Architecture

### Two Distinct Message Paths

Pl3xus has **two separate message routing paths**:

1. **Sync Messages** (`SyncClientMessage` / `SyncServerMessage`)
   - Used for: Subscriptions, component snapshots, component updates, mutations
   - Handled by: `pl3xus_sync` subscription/broadcast systems
   - Client API: `use_sync_component::<T>()` for receiving, `ctx.mutate()` for changes

2. **Network Messages** (RPC-style)
   - Used for: Commands, requests, custom messages
   - Handled by: `register_network_message::<T, NP>()` + `MessageReader<NetworkData<T>>`
   - Client API: `ctx.send(message)` for sending

### Key Insight: Don't Mix the Paths

The `SyncProvider` in pl3xus_client originally wrapped ALL outgoing bytes in `SyncClientMessage`. This broke RPC messages because they need to be sent as raw `NetworkPacket` with the correct type_name.

**Fix**: Check if data is already a `NetworkPacket` (has valid type_name with `::`) before wrapping.

## 2. Type Name Matching

### Schema Hash Fallback

Pl3xus matches messages by:
1. **First**: Exact `type_name` match (e.g., `fanuc_replica_types::ConnectToRobot`)
2. **Fallback**: `schema_hash` match (for cross-crate compatibility)

This allows `pl3xus_common::ControlRequest` (client) to match `pl3xus_sync::control::ControlRequest` (server) via schema hash.

### Short Names for Sync Components

Sync subscriptions use **short names** (e.g., `EntityControl`, not `pl3xus_common::EntityControl`). The client registry and server sync registry must use matching short names.

## 3. Shared Types Between Server and Client

### Problem: Bevy Dependencies

Server types often need `#[derive(Component)]` from Bevy, which clients (especially WASM) can't use due to heavy dependencies.

### Solution: Conditional Compilation in pl3xus_common

```rust
// In pl3xus_common/Cargo.toml
[features]
default = []
ecs = ["dep:bevy"]  # Enable Bevy ECS integration

// In pl3xus_common/src/lib.rs
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ecs", derive(bevy::prelude::Component))]
pub struct EntityControl {
    pub client_id: ConnectionId,
    pub last_activity: f32,
}
```

Then enable `ecs` feature only on server:
```toml
# In pl3xus_sync/Cargo.toml
runtime = ["dep:pl3xus", "dep:bevy", "pl3xus_common/ecs", ...]
```

## 4. Entity IDs

### Entity Bits Are Required for Control

The `ExclusiveControlPlugin` requires the actual `Entity.to_bits()` value:
```rust
ControlRequest::Take(entity_bits)  // NOT a placeholder!
```

### Getting Entity IDs on Client

Synced components include the entity ID. Use it from a known synced component:
```rust
let connection_state = use_sync_component::<ConnectionState>();
let entity_bits = connection_state.get().keys().next().copied();
```

## 5. ExclusiveControlPlugin Integration

### Server Setup
```rust
app.add_plugins(ExclusiveControlPlugin::default());
app.add_exclusive_control_systems::<WebSocketProvider>();
app.sync_component::<EntityControl>(None);  // Sync control state to clients
```

### Client Setup
```rust
// Register for syncing
registry.register::<EntityControl>();

// Send control requests
ctx.send(ControlRequest::Take(entity_bits));
ctx.send(ControlRequest::Release(entity_bits));

// Observe control state
let control = use_sync_component::<EntityControl>();
let has_control = control.get().values().next()
    .map(|c| c.client_id.id != 0)
    .unwrap_or(false);
```

## 6. Debugging Tips

### Enable debug_messages Feature
```toml
pl3xus = { path = "...", features = ["debug_messages"] }
```
This prints routing information for every message.

### Check Type Names
Log the expected type_name and schema_hash at startup:
```rust
info!("Registered {} with hash 0x{:016x}", 
    <MyType as Pl3xusMessage>::type_name(),
    <MyType as Pl3xusMessage>::schema_hash());
```

### Browser Console
The client logs subscription, deserialization, and message routing details to the browser console.

## 7. Common Pitfalls

| Pitfall | Symptom | Solution |
|---------|---------|----------|
| RPC wrapped in SyncClientMessage | Server doesn't receive RPC | Check NetworkPacket before wrapping |
| Type name mismatch | "Could not find registration" | Use schema_hash fallback, or same module path |
| Missing sync_component | 0 entities in subscription | Call `app.sync_component::<T>(None)` on server |
| Wrong entity ID | ControlResponse error | Get actual entity bits from synced data |
| Component serialization mismatch | Deserialization fails | Ensure identical struct fields and types |

## 8. Known Issues (TODO)

### Control Timeout Triggers Incorrect Entity Removal

**Problem**: When `EntityControl` is removed (due to timeout or explicit release), the `observe_entity_despawns<T>` system fires `EntityDespawnEvent` for ALL synced component types. This causes the client to think the entity was removed when only the control component was removed.

**Symptom**: Client shows 0 entities after control times out, even though the entity still exists.

**Root Cause**: `observe_entity_despawns` uses `RemovedComponents<T>` which fires on:
1. Component T removed from entity
2. Entity with T despawned

Current code treats both cases as entity despawn, which is wrong.

**Fix Needed**: Distinguish between component removal and entity despawn. Only send `EntityDespawnEvent` when the entity is actually despawned.

## 9. Architectural Considerations

### System vs Robot Entities

The controlled entity is typically a **System** (session/workspace), not individual robots:

```
System (controllable entity)
├── EntityControl (who has control)
├── ConnectionState (system connection status)
├── SystemConfig
├── Robot1 (child entity)
│   ├── RobotPosition
│   ├── RobotStatus
│   └── JointAngles
├── Robot2 (child entity)
│   └── ...
└── Devices (child entities)
```

### Control Flow

1. **Client takes control of System** (not robot)
2. **Authorized actions** (only for client in control):
   - Connect/disconnect robots
   - Send commands to robots
   - Modify system configuration
3. **Read-only actions** (any client):
   - Subscribe to robot state
   - View positions, status

### Authorization Pattern

RPC handlers should check control ownership before executing:

```rust
fn handle_connect_robot(
    requests: MessageReader<NetworkData<ConnectToRobot>>,
    systems: Query<&EntityControl, With<SystemMarker>>,
) {
    for request in requests.read() {
        let client_id = *request.source();

        // Check if this client has control
        let has_control = systems.iter()
            .any(|ctrl| ctrl.client_id == client_id);

        if !has_control {
            warn!("Client {:?} not authorized (no control)", client_id);
            continue;
        }

        // Proceed with connect...
    }
}
```

