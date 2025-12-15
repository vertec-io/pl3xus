---
title: Getting Started with pl3xus_sync
---
# Getting Started with pl3xus_sync

`pl3xus_sync` is a server-side Bevy plugin that automatically synchronizes ECS components to connected clients using bincode serialization.

**Time**: 30-45 minutes  
**Difficulty**: Intermediate  
**Prerequisites**: Basic Bevy knowledge, pl3xus setup

---

## Overview

`pl3xus_sync` provides:

- **Automatic component synchronization** to subscribed clients
- **Fast binary serialization** using bincode
- **Opt-in per component** registration
- **Client mutation support** with authorization
- **Configurable update rates** and conflation

---

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
bevy = "0.17"
pl3xus = "1.1"
pl3xus_sync = { version = "0.1", features = ["runtime"] }
pl3xus_websockets = "1.1"
serde = { version = "1.0", features = ["derive"] }
```

---

## Quick Start

### Step 1: Create Shared Types

Create a shared crate for types used by both server and client:

**`shared_types/Cargo.toml`**:

```toml
[package]
name = "shared_types"
version = "0.1.0"
edition = "2021"

[features]
server = ["dep:bevy"]

[dependencies]
serde = { version = "1.0", features = ["derive"] }
bevy = { version = "0.17", optional = true }
```

**`shared_types/src/lib.rs`**:

```rust
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use bevy::prelude::*;

/// Position component - synchronized to clients
#[cfg_attr(feature = "server", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

/// Status flags - synchronized to clients
#[cfg_attr(feature = "server", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct StatusFlags {
    pub label: String,
    pub enabled: bool,
}
```

This pattern enables:
- Bincode serialization without reflection
- Server gets `Component` trait via feature flag
- Clients can use types without Bevy dependency

### Step 2: Set Up the Server

**Server `Cargo.toml`**:

```toml
[dependencies]
bevy = "0.17"
pl3xus = "1.1"
pl3xus_sync = { version = "0.1", features = ["runtime"] }
pl3xus_websockets = "1.1"
shared_types = { path = "../shared_types", features = ["server"] }
```

**Server `main.rs`**:

```rust
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use bevy::prelude::*;
use bevy::tasks::{TaskPool, TaskPoolBuilder};
use pl3xus::{Pl3xusPlugin, Pl3xusRuntime, Network};
use pl3xus_sync::{Pl3xusSyncPlugin, AppPl3xusSyncExt};
use pl3xus_websockets::{WebSocketProvider, NetworkSettings};
use shared_types::{Position, StatusFlags};

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins(bevy::log::LogPlugin::default())
        // Add pl3xus networking
        .add_plugins(Pl3xusPlugin::<WebSocketProvider, TaskPool>::default())
        .insert_resource(Pl3xusRuntime(TaskPoolBuilder::new().num_threads(2).build()))
        .insert_resource(NetworkSettings::default())
        // Add sync plugin
        .add_plugins(Pl3xusSyncPlugin::<WebSocketProvider>::default())
        // Register components for synchronization
        .sync_component::<Position>(None)
        .sync_component::<StatusFlags>(None)
        // Add systems
        .add_systems(Startup, (setup_world, setup_networking))
        .add_systems(Update, update_positions)
        .run();
}
```

### Step 3: Start the Network Listener

```rust
fn setup_networking(
    net: Res<Network<WebSocketProvider>>,
    settings: Res<NetworkSettings>,
    task_pool: Res<Pl3xusRuntime<TaskPool>>,
) {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8082);

    match net.listen(addr, &task_pool.0, &settings) {
        Ok(_) => info!("Sync server listening on {addr}"),
        Err(err) => {
            error!("Could not start listening: {err}");
            panic!("Failed to bind listener");
        }
    }
}
```

### Step 4: Spawn and Update Entities

```rust
fn setup_world(mut commands: Commands) {
    // Spawn entities with synchronized components
    commands.spawn((
        Position { x: 0.0, y: 0.0 },
        StatusFlags { label: "Entity A".to_string(), enabled: true },
    ));
    
    commands.spawn((
        Position { x: 10.0, y: 5.0 },
        StatusFlags { label: "Entity B".to_string(), enabled: false },
    ));
}

fn update_positions(time: Res<Time>, mut query: Query<&mut Position>) {
    for mut pos in &mut query {
        // Changes are automatically detected and synced
        pos.x += time.delta_secs();
    }
}
```

Your server is now synchronizing `Position` and `StatusFlags` components to any connected clients!

---

## Configuration

### Sync Settings

Configure global sync behavior:

```rust
use pl3xus_sync::SyncSettings;

app.insert_resource(SyncSettings {
    max_update_rate_hz: Some(30.0),      // Limit to 30 updates/second
    enable_message_conflation: true,      // Only send latest value
});
```

### Per-Component Configuration

```rust
use pl3xus_sync::ComponentSyncConfig;

app.sync_component::<Position>(Some(ComponentSyncConfig {
    // Component-specific settings
}));
```

---

## How It Works

1. **Registration**: `app.sync_component::<T>()` registers the component type
2. **Change Detection**: Bevy's change detection tracks modifications
3. **Subscription**: Clients send subscription requests for component types
4. **Synchronization**: Changes are serialized and sent to subscribed clients
5. **Conflation**: Multiple updates to same entity+component are conflated
6. **Rate Limiting**: Updates throttled to `max_update_rate_hz`

---

## Next Steps

- [Client Integration](../../client/index.md) - Build a Leptos client to display synced data
- [Mutation Authorization](../core/guides/mutations.md) - Control client mutations
- [Exclusive Control](../core/guides/server-setup.md) - Implement control transfer patterns
