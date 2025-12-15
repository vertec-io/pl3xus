# Server Setup Guide

This guide covers setting up a Bevy server with component synchronization using pl3xus_sync.

---

## Overview

A sync-enabled server consists of:

1. **Bevy App** with minimal or default plugins
2. **Pl3xusPlugin** for networking
3. **Pl3xusSyncPlugin** for component synchronization
4. **Component registrations** for types to synchronize

---

## Quick Start

### Minimal Server

```rust
use bevy::prelude::*;
use bevy::tasks::{TaskPool, TaskPoolBuilder};
use pl3xus::{Pl3xusPlugin, Pl3xusRuntime};
use pl3xus_sync::{Pl3xusSyncPlugin, AppPl3xusSyncExt};
use pl3xus_websockets::{NetworkSettings, WebSocketProvider};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use serde::{Serialize, Deserialize};

// Your component (or import from shared crate)
#[derive(Component, Serialize, Deserialize, Clone, Debug)]
struct Position {
    x: f32,
    y: f32,
}

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins(bevy::log::LogPlugin::default())
        // Networking
        .add_plugins(Pl3xusPlugin::<WebSocketProvider, TaskPool>::default())
        .insert_resource(Pl3xusRuntime(
            TaskPoolBuilder::new().num_threads(2).build()
        ))
        .insert_resource(NetworkSettings::default())
        // Sync plugin
        .add_plugins(Pl3xusSyncPlugin::<WebSocketProvider>::default())
        // Register components for sync
        .sync_component::<Position>(None)
        // Start server
        .add_systems(Startup, start_server)
        .run();
}

fn start_server(
    mut net: ResMut<pl3xus::Network<WebSocketProvider>>,
    settings: Res<NetworkSettings>,
    task_pool: Res<Pl3xusRuntime<TaskPool>>,
) {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    
    net.listen(addr, &task_pool.0, &settings)
        .expect("Failed to start server");
    
    info!("Server listening on ws://{}", addr);
}
```

---

## Dependencies

### Cargo.toml

```toml
[dependencies]
bevy = "0.17"
pl3xus = "1.1"
pl3xus_sync = { version = "0.1", features = ["runtime"] }
pl3xus_websockets = "1.1"
serde = { version = "1.0", features = ["derive"] }

# For shared types crate
my_shared_types = { path = "../shared", features = ["server"] }
```

> **Note**: The `runtime` feature is required for server-side sync functionality.

---

## Plugin Setup

### Pl3xusPlugin

Provides core networking:

```rust
app.add_plugins(Pl3xusPlugin::<WebSocketProvider, TaskPool>::default());
```

**Type parameters:**
- `WebSocketProvider` - The transport (WebSocket, TCP, etc.)
- `TaskPool` - Bevy's async task pool type

### Pl3xusRuntime

Provides the async runtime:

```rust
app.insert_resource(Pl3xusRuntime(
    TaskPoolBuilder::new()
        .num_threads(2)  // Adjust based on load
        .build()
));
```

### NetworkSettings

Configure WebSocket behavior:

```rust
app.insert_resource(NetworkSettings::default());

// Or custom settings
app.insert_resource(NetworkSettings {
    websocket_config: WebSocketConfig {
        max_message_size: Some(64 * 1024 * 1024),
        ..Default::default()
    },
    channel_capacity: 1000,
    channel_warning_threshold: 80,
});
```

### Pl3xusSyncPlugin

Adds component synchronization:

```rust
app.add_plugins(Pl3xusSyncPlugin::<WebSocketProvider>::default());
```

This plugin:
- Initializes `SyncSettings`, `SubscriptionManager`, `MutationQueue`
- Registers sync message types with pl3xus
- Adds systems for subscription handling, change detection, broadcasting

---

## Component Registration

### Basic Registration

```rust
use pl3xus_sync::AppPl3xusSyncExt;

app.sync_component::<Position>(None);
app.sync_component::<Velocity>(None);
app.sync_component::<EntityName>(None);
```

**Requirements for synced components:**
- `Component` trait (Bevy)
- `Serialize + Deserialize` (serde)
- `Send + Sync + 'static`
- `Debug` (for error messages)

### With Custom Configuration

```rust
use pl3xus_sync::{AppPl3xusSyncExt, ComponentSyncConfig};

app.sync_component::<HighFrequencyData>(Some(ComponentSyncConfig {
    max_updates_per_frame: Some(10),  // Limit updates per frame
}));
```

---

## SyncSettings

Global settings for the sync system:

```rust
use pl3xus_sync::SyncSettings;

app.insert_resource(SyncSettings {
    // Maximum updates per second to clients
    max_update_rate_hz: Some(30.0),  // Default: 30 Hz
    
    // Enable message conflation (latest-wins for same entity+component)
    enable_message_conflation: true,  // Default: true
});
```

### Update Rate Recommendations

| Use Case | Rate | Notes |
|----------|------|-------|
| Real-time visualization | 30-60 Hz | Good balance |
| Industrial monitoring | 10-30 Hz | Efficient |
| Low-bandwidth | 5-10 Hz | Mobile/slow connections |
| Maximum throughput | None | For LAN/testing |

```rust
// Unlimited updates (every frame)
app.insert_resource(SyncSettings {
    max_update_rate_hz: None,
    enable_message_conflation: false,
});
```

---

## Message Conflation

Conflation prevents network flooding by keeping only the latest update:

```
Without Conflation:
  Frame 1: Position(1,1) → Send
  Frame 2: Position(2,2) → Send
  Frame 3: Position(3,3) → Send
  = 3 messages

With Conflation (30 Hz, 60 FPS game):
  Frame 1: Position(1,1) → Queue
  Frame 2: Position(2,2) → Replace
  Timer fires → Send Position(2,2)
  = 1 message
```

**Non-conflatable items** are always sent in order:
- Entity removals
- Component removals

---

## Change Detection

Pl3xusSync automatically detects component changes using Bevy's `Changed<T>` query:

```rust
// Internal system (for understanding)
fn observe_component_changes<T: Component + Serialize>(
    query: Query<(Entity, &T), Changed<T>>,
    mut writer: MessageWriter<ComponentChangeEvent>,
) {
    for (entity, component) in query.iter() {
        // Serialize and emit change event
    }
}
```

**No manual change tracking needed** - just modify components normally:

```rust
fn move_entities(mut query: Query<&mut Position>) {
    for mut pos in query.iter_mut() {
        pos.x += 1.0;  // Automatically detected and synced
    }
}
```

---

## Entity Lifecycle

### Spawning Entities

Entities with synced components are automatically tracked:

```rust
fn spawn_robot(mut commands: Commands) {
    commands.spawn((
        Position { x: 0.0, y: 0.0 },
        Velocity { dx: 1.0, dy: 0.0 },
        EntityName { name: "Robot A".into() },
    ));
    // Subscribers to Position, Velocity, EntityName will receive this entity
}
```

### Despawning Entities

Clients receive `EntityRemove` when entities are despawned:

```rust
fn cleanup_robot(mut commands: Commands, entity: Entity) {
    commands.entity(entity).despawn();
    // All subscribers receive EntityRemove notification
}
```

### Removing Components

Clients receive `ComponentRemove` when components are removed:

```rust
fn remove_velocity(mut commands: Commands, entity: Entity) {
    commands.entity(entity).remove::<Velocity>();
    // Velocity subscribers receive ComponentRemove for this entity
}
```

---

## Mutation Authorization

By default, clients can mutate any synced component. Add authorization for production:

```rust
use pl3xus_sync::{MutationAuthorizerResource, MutationStatus};

// Option 1: Server-only (no client mutations)
app.insert_resource(MutationAuthorizerResource::server_only());

// Option 2: Custom authorization
app.insert_resource(MutationAuthorizerResource::from_fn(|world, mutation| {
    // Your authorization logic
    MutationStatus::Ok
}));
```

See [Mutations Guide](./mutations.md) for detailed authorization patterns.

---

## Exclusive Control (Optional)

For scenarios where only one client should control an entity:

```rust
use pl3xus_sync::control::{ExclusiveControlPlugin, AppExclusiveControlExt};

app.add_plugins(ExclusiveControlPlugin::default());

// Register control for specific component types
app.register_control::<Robot, WebSocketProvider>();
```

This adds:
- Control request/release handling
- Exclusive control semantics
- Optional timeout for inactive clients
- Hierarchy propagation (parent control → child control)

---

## System Ordering

Pl3xusSync uses system sets for ordering:

```rust
use pl3xus_sync::Pl3xusSyncSystems;

// Your systems that modify synced components should run before Observe
app.add_systems(
    Update,
    move_entities.before(Pl3xusSyncSystems::Observe)
);
```

**System set order:**
1. `Inbound` - Handle incoming subscription/mutation requests
2. `Observe` - Detect component changes, process snapshots
3. `Outbound` - Send updates to clients, flush conflation queue

---

## Complete Example

```rust
use bevy::prelude::*;
use bevy::tasks::{TaskPool, TaskPoolBuilder};
use pl3xus::{Pl3xusPlugin, Pl3xusRuntime, Network};
use pl3xus_sync::{
    Pl3xusSyncPlugin, AppPl3xusSyncExt, SyncSettings,
    MutationAuthorizerResource,
};
use pl3xus_websockets::{NetworkSettings, WebSocketProvider};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

// Import from shared crate
use my_shared_types::{Position, Velocity, EntityName};

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins(bevy::log::LogPlugin::default())
        // Networking
        .add_plugins(Pl3xusPlugin::<WebSocketProvider, TaskPool>::default())
        .insert_resource(Pl3xusRuntime(
            TaskPoolBuilder::new().num_threads(2).build()
        ))
        .insert_resource(NetworkSettings::default())
        // Sync settings
        .insert_resource(SyncSettings {
            max_update_rate_hz: Some(30.0),
            enable_message_conflation: true,
        })
        // Sync plugin
        .add_plugins(Pl3xusSyncPlugin::<WebSocketProvider>::default())
        // Authorization (read-only clients)
        .insert_resource(MutationAuthorizerResource::server_only())
        // Register components
        .sync_component::<Position>(None)
        .sync_component::<Velocity>(None)
        .sync_component::<EntityName>(None)
        // Systems
        .add_systems(Startup, (start_server, spawn_entities))
        .add_systems(Update, update_positions)
        .run();
}

fn start_server(
    mut net: ResMut<Network<WebSocketProvider>>,
    settings: Res<NetworkSettings>,
    task_pool: Res<Pl3xusRuntime<TaskPool>>,
) {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 8080);
    net.listen(addr, &task_pool.0, &settings).unwrap();
    info!("Server listening on ws://{}", addr);
}

fn spawn_entities(mut commands: Commands) {
    for i in 0..10 {
        commands.spawn((
            Position { x: i as f32 * 10.0, y: 0.0 },
            Velocity { dx: 1.0, dy: 0.0 },
            EntityName { name: format!("Entity {}", i) },
        ));
    }
}

fn update_positions(mut query: Query<(&mut Position, &Velocity)>) {
    for (mut pos, vel) in query.iter_mut() {
        pos.x += vel.dx;
        pos.y += vel.dy;
    }
}
```

---

## Troubleshooting

### No Updates Received

**Check:**
1. Component is registered with `sync_component::<T>()`
2. Client subscribed to the correct type name
3. Components are actually changing (use `Changed<T>`)

### High Bandwidth Usage

**Solutions:**
- Enable conflation: `enable_message_conflation: true`
- Lower update rate: `max_update_rate_hz: Some(10.0)`
- Register only necessary components

### Mutations Failing

**Check:**
- Server has no `MutationAuthorizerResource::server_only()`
- Custom authorizer returns `MutationStatus::Ok`
- See [Mutations Guide](./mutations.md)

---

## Related Documentation

- [Shared Types](./shared-types.md) - Setting up shared component types
- [Subscriptions](./subscriptions.md) - How subscriptions work
- [Mutations](./mutations.md) - Client-side component editing
- [WebSocket Patterns](./websocket-patterns.md) - Connection handling
- [API Reference](https://docs.rs/pl3xus_sync) - Full API documentation

---

**Last Updated**: 2025-12-07
**pl3xus_sync Version**: 0.1
```


