# pl3xus_sync

Bincode-based ECS component synchronization for Bevy servers.

[![Crates.io](https://img.shields.io/crates/v/pl3xus_sync.svg)](https://crates.io/crates/pl3xus_sync)
[![Documentation](https://docs.rs/pl3xus_sync/badge.svg)](https://docs.rs/pl3xus_sync)
[![License](https://img.shields.io/crates/l/pl3xus_sync.svg)](https://github.com/vertec-io/pl3xus/blob/main/LICENSE)

---

## Overview

pl3xus_sync is a server-side Bevy plugin that automatically synchronizes ECS components to connected clients using bincode serialization. It's designed for high-performance, high-throughput applications like robotics control, industrial automation, and networked applications.

### Key Features

- Automatic synchronization of components to subscribed clients
- Fast binary serialization using bincode
- Opt-in per component registration
- Client mutation support with authorization
- Message conflation and rate limiting for performance
- Configurable update rates and authorization policies

---

## Quick Start

### Installation

```toml
[dependencies]
bevy = "0.17"
pl3xus = "1.1"
pl3xus_sync = "0.1"
pl3xus_websockets = "1.1"
serde = { version = "1.0", features = ["derive"] }
```

### Shared Crate Pattern (Recommended)

The recommended approach is to create a shared crate for types used by both server and client:

**`shared_types/Cargo.toml`**:
```toml
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

// Component trait is only derived when building with "server" feature
#[cfg_attr(feature = "server", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

#[cfg_attr(feature = "server", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Velocity {
    pub x: f32,
    pub y: f32,
}
```

This pattern enables:
- Bincode serialization without reflection
- Conditional `Component` derivation with the `server` feature
- WASM-compatible client builds without Bevy dependency
- Type safety between server and client

### Basic Usage

```rust
use bevy::prelude::*;
use bevy::tasks::TaskPoolBuilder;
use pl3xus::{Pl3xusPlugin, Pl3xusRuntime, NetworkSettings, AppNetworkMessage};
use pl3xus_sync::{Pl3xusSyncPlugin, AppPl3xusSyncExt};
use pl3xus_websockets::WebSocketProvider;
use shared_types::{Position, Velocity};

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins);

    // Add pl3xus networking
    app.add_plugins(Pl3xusPlugin::<WebSocketProvider, bevy::tasks::TaskPool>::default());
    app.insert_resource(Pl3xusRuntime(
        TaskPoolBuilder::new().num_threads(2).build()
    ));
    app.insert_resource(NetworkSettings::default());

    // Add pl3xus_sync plugin
    app.add_plugins(Pl3xusSyncPlugin::<WebSocketProvider>::default());

    // Register components for synchronization
    app.sync_component::<Position>(None);
    app.sync_component::<Velocity>(None);

    app.add_systems(Startup, setup);
    app.add_systems(Update, move_entities);

    app.run();
}

fn setup(mut commands: Commands) {
    // Start listening for connections
    commands.listen("127.0.0.1:8082");

    // Spawn entities - they'll be automatically synchronized
    commands.spawn((
        Position { x: 0.0, y: 0.0 },
        Velocity { x: 1.0, y: 0.5 },
    ));
}

fn move_entities(
    time: Res<Time>,
    mut query: Query<(&mut Position, &Velocity)>,
) {
    for (mut pos, vel) in &mut query {
        pos.x += vel.x * time.delta_secs();
        pos.y += vel.y * time.delta_secs();
    }
}
```

Components are now automatically synchronized to connected clients.

---

## Configuration

### Global Settings

```rust
use pl3xus_sync::SyncSettings;

app.insert_resource(SyncSettings {
    // Limit updates to 30 Hz (30 updates per second)
    max_update_rate_hz: Some(30.0),
    
    // Enable message conflation (only send latest update)
    enable_message_conflation: true,
});
```

### Mutation Authorization

Control which mutations clients can perform:

```rust
use pl3xus_sync::{MutationAuthorizer, MutationAuthContext, MutationStatus};

struct MyAuthorizer;

impl MutationAuthorizer for MyAuthorizer {
    fn authorize(&self, ctx: &MutationAuthContext) -> MutationStatus {
        // Implement your authorization logic
        if ctx.component_type == "Position" {
            MutationStatus::Accepted
        } else {
            MutationStatus::Rejected("Not authorized".to_string())
        }
    }
}

app.insert_resource(MutationAuthorizerResource(Box::new(MyAuthorizer)));
```

---

## Documentation

- **[Getting Started Guide](../../docs/getting-started/pl3xus-sync.md)** - Step-by-step tutorial
- **[API Documentation](https://docs.rs/pl3xus_sync)** - Complete API reference
- **[Architecture](../../docs/architecture/sync-architecture.md)** - How it works internally
- **[Examples](./examples/)** - Working code examples

---

## Examples

See the `examples/` directory for complete working examples:

- **`basic_sync_server.rs`** - Minimal getting started example
- **`devtools-demo-server.rs`** - Server for DevTools demo
- **`fanuc_server.rs`** - Robotics control example

Run an example:
```bash
cargo run -p pl3xus_sync --example basic_sync_server --features runtime
```

---

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.

---

**Part of the [pl3xus](https://github.com/vertec-io/pl3xus) ecosystem**

