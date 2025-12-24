# Shared Types Project Structure Reference

## Overview

The shared types pattern uses a dedicated crate for types shared between server and client. This is simpler to understand and works well for smaller projects.

## Directory Structure

```
project/
├── Cargo.toml                    # Workspace root
├── shared/                       # Shared types crate (or "types/")
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── components/           # Synced components
│       │   ├── mod.rs
│       │   ├── robot.rs
│       │   └── process.rs
│       ├── requests/             # Request/response types
│       │   ├── mod.rs
│       │   └── robot_requests.rs
│       └── mutations/            # Mutation types
│           ├── mod.rs
│           └── robot_mutations.rs
├── server/                       # Bevy ECS server
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── plugins/
│       │   ├── mod.rs
│       │   ├── robot.rs
│       │   └── process.rs
│       └── handlers/
│           ├── mod.rs
│           └── robot_handlers.rs
└── client/                       # Leptos WASM client
    ├── Cargo.toml
    ├── Trunk.toml
    └── src/
        ├── main.rs
        ├── app.rs
        ├── pages/
        │   ├── mod.rs
        │   └── dashboard.rs
        └── components/
            ├── mod.rs
            └── robot_panel.rs
```

## Workspace Configuration

### Root Cargo.toml

```toml
[workspace]
resolver = "2"
members = ["shared", "server", "client"]

[workspace.dependencies]
# Core
bevy = "0.17"
leptos = { version = "0.7", features = ["csr"] }
serde = { version = "1.0", features = ["derive"] }

# pl3xus framework
pl3xus = { git = "https://github.com/vertec-io/pl3xus" }
pl3xus_sync = { git = "https://github.com/vertec-io/pl3xus" }
pl3xus_client = { git = "https://github.com/vertec-io/pl3xus" }
pl3xus_websockets = { git = "https://github.com/vertec-io/pl3xus" }

# Common
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
```

## Shared Crate

### shared/Cargo.toml

```toml
[package]
name = "shared"
version = "0.1.0"
edition = "2024"

[dependencies]
bevy.workspace = true
serde.workspace = true
pl3xus_sync.workspace = true
```

### shared/src/lib.rs

```rust
pub mod components;
pub mod requests;
pub mod mutations;

// Re-export for convenience
pub use components::*;
pub use requests::*;
pub use mutations::*;
```

### shared/src/components/robot.rs

```rust
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Component, Clone, Debug, Serialize, Deserialize, Default)]
pub struct RobotStatus {
    pub connected: bool,
    pub servo_on: bool,
    pub position: [f64; 6],
}

#[derive(Component, Clone, Debug, Serialize, Deserialize, Default)]
pub struct RobotConfig {
    pub ip_address: String,
    pub port: u16,
    pub speed_override: u8,
}
```

## Server Crate

### server/Cargo.toml

```toml
[package]
name = "server"
version = "0.1.0"
edition = "2024"

[dependencies]
shared = { path = "../shared" }
bevy.workspace = true
pl3xus.workspace = true
pl3xus_sync.workspace = true
pl3xus_websockets.workspace = true
tokio.workspace = true
tracing.workspace = true
```

### server/src/main.rs

```rust
use bevy::prelude::*;
use pl3xus::prelude::*;
use pl3xus_websockets::WebSocketProvider;

mod plugins;
use plugins::RobotPlugin;

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins(Pl3xusServerPlugins::<WebSocketProvider>::default())
        .add_plugins(RobotPlugin)
        .run();
}
```

## Client Crate

### client/Cargo.toml

```toml
[package]
name = "client"
version = "0.1.0"
edition = "2024"

[dependencies]
shared = { path = "../shared" }
leptos.workspace = true
pl3xus_client.workspace = true
```

### client/src/main.rs

```rust
use leptos::prelude::*;
use pl3xus_client::prelude::*;
use shared::*;

fn main() {
    mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    let registry = create_type_registry();
    provide_context(registry);
    
    view! { <Dashboard/> }
}

fn create_type_registry() -> TypeRegistry {
    let mut registry = TypeRegistry::new();
    registry.register::<RobotStatus>();
    registry.register::<RobotConfig>();
    registry
}
```

## Key Benefits

1. **Simple** - Easy to understand structure
2. **Clear Boundaries** - Shared, server, client are obvious
3. **No Feature Complexity** - No conditional compilation
4. **IDE Friendly** - Better autocomplete/refactoring
5. **Quick Setup** - Less boilerplate to get started

