# Plugin-Based Project Structure Reference

## Overview

The plugin-based pattern organizes code into feature-gated modules within a single `plugins` crate. This enables modular, reusable plugins with types shared between server and client.

## Directory Structure

```
project/
├── Cargo.toml                    # Workspace root
├── plugins/                      # All domain logic and types
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs               # build() + feature exports
│       ├── core/                # Core plugin (networking, auth)
│       │   ├── mod.rs
│       │   ├── plugin.rs
│       │   ├── models/
│       │   └── systems/
│       ├── robot_driver/        # Domain plugin
│       │   ├── mod.rs
│       │   ├── plugin.rs
│       │   ├── models/
│       │   ├── systems/
│       │   └── bundles/
│       └── process_control/     # Another domain plugin
├── server/                       # Thin server binary
│   ├── Cargo.toml
│   └── src/
│       └── main.rs
└── app/                          # Leptos WASM client
    ├── Cargo.toml
    ├── Trunk.toml
    └── src/
```

## Feature Strategy

### plugins/Cargo.toml

```toml
[package]
name = "plugins"
version = "0.1.0"
edition = "2024"

[dependencies]
# Core - always needed for types
bevy = { version = "0.17", default-features = false, features = ["multi_threaded", "bevy_state"] }
serde = { version = "1.0", features = ["derive"] }
cfg-if = "1.0"

# Networking (ECS feature)
pl3xus_sync = { workspace = true, optional = true }

# Server-only dependencies
tokio = { workspace = true, optional = true }
robot_driver_lib = { workspace = true, optional = true }

# Client-only reactive stores
reactive_stores = { workspace = true, optional = true }

[features]
default = ["ecs", "server"]

# ECS enables Bevy Components and systems
ecs = ["dep:pl3xus_sync"]

# Server enables hardware drivers, async runtime
server = [
    "dep:tokio",
    "dep:robot_driver_lib",
    "bevy/multi_threaded",
]

# Client reactive stores
stores = ["dep:reactive_stores"]

# Development with dynamic linking
ecs-dev = ["ecs", "bevy/dynamic_linking"]
```

## Plugin Module Pattern

### mod.rs Structure

```rust
// plugins/src/robot_driver/mod.rs

// Models are always available (types)
pub mod models;
pub use models::*;

// ECS code only when feature enabled
use cfg_if::cfg_if;
cfg_if! {
    if #[cfg(feature = "ecs")] {
        pub mod register;    // Component/request registration
        pub mod systems;     // Bevy systems
        pub mod bundles;     // Entity bundles
        pub mod plugin;      // Plugin struct
        pub use plugin::*;
    }
}
```

### Model with Conditional Component Derive

```rust
// plugins/src/robot_driver/models/status.rs
use serde::{Deserialize, Serialize};

/// Robot status - works on both server and client
#[cfg_attr(feature = "ecs", derive(bevy::prelude::Component))]
#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct RobotStatus {
    pub id: String,
    pub connected: bool,
    pub servo_on: bool,
    pub position: [f64; 6],
    pub errors: Vec<String>,
}
```

### Plugin Implementation

```rust
// plugins/src/robot_driver/plugin.rs
use bevy::prelude::*;
use super::systems::*;
use super::register::register_robot_requests;

pub struct RobotDriverPlugin;

impl Plugin for RobotDriverPlugin {
    fn build(&self, app: &mut App) {
        register_robot_requests(app);
        
        app.add_systems(Update, (
            poll_robot_status,
            handle_motion_commands,
        ));
    }
}
```

## Server Main

```rust
// server/src/main.rs
use plugins::build;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    initialize_tracing();
    
    let mut app = build()?;
    app.run();
    
    Ok(())
}
```

## Client App

```rust
// app/src/main.rs
use leptos::prelude::*;
use plugins::robot_driver::RobotStatus;  // Type from plugins

fn main() {
    mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    // Register types
    provide_context(create_type_registry());
    
    view! { <RobotPanel/> }
}

fn create_type_registry() -> TypeRegistry {
    let mut registry = TypeRegistry::new();
    registry.register::<RobotStatus>();
    registry
}
```

## Key Benefits

1. **Single Source of Truth** - Types defined once in plugins
2. **Modular** - Feature-gate entire domains
3. **Reusable** - Plugins can be shared across projects
4. **Clean Separation** - Server logic never reaches client
5. **Minimal Server** - Server just imports and runs plugins

