# Shared Types Guide

This guide covers how to structure shared types between your Bevy server and Leptos web client for type-safe component synchronization.

---

## Overview

When using pl3xus_sync and pl3xus_client, you need types that work in both contexts:

- **Server**: Bevy `Component` trait, runs natively
- **Client**: Leptos/WASM, no Bevy dependency

The solution is a **shared types crate** with conditional compilation using Cargo feature flags.

---

## Project Structure

The recommended project structure:

```
my-app/
├── Cargo.toml              # Workspace root
├── shared/                 # Shared types crate
│   ├── Cargo.toml
│   └── src/lib.rs
├── server/                 # Bevy server
│   ├── Cargo.toml
│   └── src/main.rs
└── client/                 # Leptos web client
    ├── Cargo.toml
    └── src/main.rs
```

### Workspace Cargo.toml

```toml
[workspace]
members = ["shared", "server", "client"]
resolver = "2"
```

---

## Shared Types Crate

### Cargo.toml

```toml
[package]
name = "my_shared_types"
version = "0.1.0"
edition = "2021"

[features]
default = []
server = ["bevy"]    # Enable Bevy Component trait

[dependencies]
serde = { version = "1.0", features = ["derive"] }

# Optional: Only included when "server" feature is active
bevy = { version = "0.17", optional = true, default-features = false }
```

### src/lib.rs

```rust
use serde::{Deserialize, Serialize};

// Conditionally import Bevy for server builds
#[cfg(feature = "server")]
use bevy::prelude::*;

/// 2D position component
#[cfg_attr(feature = "server", derive(Component))]  // Only on server
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

/// Velocity component
#[cfg_attr(feature = "server", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Velocity {
    pub dx: f32,
    pub dy: f32,
}

// For entity names, use Bevy's built-in Name component:
// use bevy::prelude::Name;
// commands.spawn((Name::new("My Entity"), Position { x: 0.0, y: 0.0 }));
```

---

## Server Configuration

### Cargo.toml

```toml
[package]
name = "my_server"
version = "0.1.0"
edition = "2021"

[dependencies]
bevy = "0.17"
pl3xus = "0.17"
pl3xus_sync = { version = "0.1", features = ["runtime"] }
pl3xus_websockets = "0.17"

# Import shared types with server feature
my_shared_types = { path = "../shared", features = ["server"] }
```

### src/main.rs

```rust
use bevy::prelude::*;
use pl3xus_sync::{AppPl3xusSyncExt, Pl3xusSyncPlugin};
use pl3xus_websockets::WebSocketProvider;

// Import shared types - they include Component trait
use my_shared_types::{Position, Velocity};

fn main() {
    let mut app = App::new();

    // ... other plugins ...

    app.add_plugins(Pl3xusSyncPlugin::<WebSocketProvider>::default());

    // Register components for synchronization
    app.sync_component::<Position>(None);
    app.sync_component::<Velocity>(None);
    // Use Bevy's Name component for entity labels

    app.run();
}
```

---

## Client Configuration

### Cargo.toml

```toml
[package]
name = "my_client"
version = "0.1.0"
edition = "2021"

[dependencies]
leptos = "0.8"
pl3xus_client = { version = "0.1", features = ["devtools"] }

# Import shared types WITHOUT server feature (no Bevy)
my_shared_types = { path = "../shared" }
```

### src/main.rs

```rust
use leptos::prelude::*;
use pl3xus_client::{
    SyncProvider, use_sync_component, ClientTypeRegistry,
};
use std::sync::Arc;

// Import shared types - plain structs, no Component trait
use my_shared_types::{Position, Velocity};

#[component]
pub fn App() -> impl IntoView {
    // Register the same types as the server
    let registry = Arc::new(
        ClientTypeRegistry::builder()
            .register::<Position>()
            .register::<Velocity>()
            .with_devtools_support()
            .build()
    );

    view! {
        <SyncProvider url="ws://localhost:8082" registry=registry.clone()>
            <EntityList />
        </SyncProvider>
    }
}

#[component]
fn EntityList() -> impl IntoView {
    let positions = use_sync_component::<Position>();

    view! {
        <ul>
            <For
                each=move || positions.get().into_iter()
                key=|(id, _)| *id
                children=move |(id, pos)| {
                    view! {
                        <li>{format!("Entity {}: ({:.1}, {:.1})", id, pos.x, pos.y)}</li>
                    }
                }
            />
        </ul>
    }
}

fn main() {
    leptos::mount::mount_to_body(App);
}
```

---

## Advanced: Additional Feature Flags

### Reactive Stores Support

For fine-grained reactivity with Leptos stores:

```toml
# shared/Cargo.toml
[features]
default = []
server = ["bevy"]
stores = ["reactive_stores"]

[dependencies]
reactive_stores = { version = "0.1", optional = true }
```

```rust
// shared/src/lib.rs
#[cfg(feature = "stores")]
use reactive_stores::Store;

#[cfg_attr(feature = "server", derive(Component))]
#[cfg_attr(feature = "stores", derive(Store))]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}
```

### JSON Schema Generation

For documentation or client code generation:

```toml
[features]
json-schema = ["schemars"]

[dependencies]
schemars = { version = "0.8", optional = true }
```

```rust
#[cfg(feature = "json-schema")]
use schemars::JsonSchema;

#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[derive(Serialize, Deserialize, Clone)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}
```

---

## Type Registration Best Practices

### 1. Register Types in the Same Order

While not strictly required, it's good practice to register in consistent order:

```rust
// Server
app.sync_component::<Position>(None);
app.sync_component::<Velocity>(None);

// Client - same order
let registry = ClientTypeRegistry::builder()
    .register::<Position>()
    .register::<Velocity>()
    .build();
```

### 2. Use a Registration Function

Create a shared registration function to ensure consistency:

```rust
// shared/src/lib.rs
#[cfg(feature = "server")]
pub fn register_sync_components(app: &mut bevy::prelude::App) {
    use pl3xus_sync::AppPl3xusSyncExt;
    app.sync_component::<Position>(None);
    app.sync_component::<Velocity>(None);
}

// Can also add a client-side helper if desired
```

### 3. Handle Version Mismatches

If server and client are at different versions, unregistered types appear as raw bytes. Handle gracefully:

```rust
// Client logs a warning for unknown component types
// DevTools shows "[unknown type: SomeNewComponent]"
```

---

## Entity Hierarchies

For parent/child relationships, use **Bevy's built-in hierarchy system** instead of custom components:

```rust
use bevy::prelude::*;

// Spawn a parent entity
let parent = commands.spawn((
    Name::new("System"),  // Bevy's built-in Name component
    SystemConfig::default(),
)).id();

// Spawn children using ChildOf (Bevy 0.17+)
commands.spawn((
    Name::new("Robot A"),
    ChildOf(parent),  // Creates parent-child relationship
    RobotConfig::default(),
));

// Or use the fluent API
commands.entity(parent).with_children(|builder| {
    builder.spawn((
        Name::new("Robot B"),
        RobotConfig::default(),
    ));
});
```

Bevy automatically maintains the `Children` component on parent entities. DevTools detects `ChildOf` relationships for hierarchy visualization.

> **Note**: Use Bevy's `Name` component for display names and `ChildOf`/`Children` for hierarchies - these are built-in and don't need custom definitions.

---

## Complete Example

See the `examples/shared/` directory for working examples:

- **`basic_types/`**: Simple Position, Velocity components
- **`demo_types/`**: DemoCounter, DemoFlag with hierarchy
- **`control_demo_types/`**: Robot, RobotStatus for control demo

```bash
# View the basic types example
cat examples/shared/basic_types/src/lib.rs
```

---

## Troubleshooting

### Type Name Mismatch

**Symptom**: Server sends data but client shows empty.

**Cause**: Type names differ between server and client.

**Fix**: Ensure the type is defined in the shared crate, not separately.

### Missing Bevy Component Trait

**Symptom**: Compiler error on server - "the trait `Component` is not implemented"

**Fix**: Enable the `server` feature in the server's dependency:
```toml
my_shared_types = { path = "../shared", features = ["server"] }
```

### WASM Build Fails

**Symptom**: Client build fails with Bevy-related errors.

**Fix**: Ensure client does NOT enable the `server` feature:
```toml
my_shared_types = { path = "../shared" }  # No features
```

---

## Related Documentation

- [Mutations](./mutations.md) - Client-side component editing
- [DevTools](./devtools.md) - Inspecting synchronized data
- [Getting Started: pl3xus_sync](../../sync/index.md) - Server setup
- [Getting Started: pl3xus_client](../../client/index.md) - Client setup

---

**Last Updated**: 2025-12-07
**pl3xus_sync Version**: 0.1
```


