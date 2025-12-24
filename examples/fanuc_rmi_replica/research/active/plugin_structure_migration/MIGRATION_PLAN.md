# Detailed Migration Plan

## Domain Analysis

First, identify the domains in fanuc_rmi_replica to structure plugins:

### Identified Domains

1. **robot** - Robot entity, status, configuration, connection
2. **program** - Program management, execution state
3. **motion** - Position registers, motion commands
4. **driver** - Fanuc RMI driver communication (server-only)

## File Mapping

### shared/ → plugins/src/

| Current File | Target Location | Notes |
|--------------|-----------------|-------|
| `shared/src/components.rs` | Split by domain | Add cfg_attr |
| `shared/src/messages.rs` | Split by domain | Add cfg_attr |
| `shared/src/requests.rs` | Split by domain | |

### server/src/plugins/ → plugins/src/

| Current File | Target Location | Notes |
|--------------|-----------------|-------|
| Robot plugins | `plugins/src/robot/plugin.rs` | Wrap in cfg(feature = "ecs") |
| Program plugins | `plugins/src/program/plugin.rs` | Wrap in cfg(feature = "ecs") |
| Driver code | `plugins/src/driver/` | cfg(feature = "server") |

## plugins/Cargo.toml

```toml
[package]
name = "fanuc_rmi_replica_plugins"
version = "0.1.0"
edition = "2024"

[features]
default = ["ecs", "server"]
ecs = [
    "dep:bevy",
    "dep:pl3xus_sync",
]
server = [
    "ecs",
    "dep:tokio",
    "dep:fanuc_rmi_api",
]
stores = [
    "dep:reactive_stores",
    "dep:leptos",
]

[dependencies]
# Always available
serde = { version = "1.0", features = ["derive"] }

# ECS feature (server-side Bevy)
bevy = { workspace = true, optional = true }
pl3xus_sync = { workspace = true, optional = true }

# Server feature (server-only)
tokio = { workspace = true, optional = true }
fanuc_rmi_api = { workspace = true, optional = true }

# Stores feature (client-side)
reactive_stores = { workspace = true, optional = true }
leptos = { workspace = true, optional = true }
```

## plugins/src/lib.rs

```rust
pub mod robot;
pub mod program;
pub mod motion;

#[cfg(feature = "server")]
pub mod driver;

#[cfg(feature = "ecs")]
pub fn build() -> bevy::app::App {
    use bevy::prelude::*;
    
    let mut app = App::new();
    
    // Core plugins
    app.add_plugins(MinimalPlugins);
    
    // Domain plugins
    app.add_plugins((
        robot::RobotPlugin,
        program::ProgramPlugin,
        motion::MotionPlugin,
    ));
    
    #[cfg(feature = "server")]
    app.add_plugins(driver::DriverPlugin);
    
    app
}
```

## plugins/src/robot/mod.rs

```rust
pub mod models;

#[cfg(feature = "ecs")]
pub mod systems;

#[cfg(feature = "ecs")]
pub mod plugin;

#[cfg(feature = "ecs")]
pub use plugin::RobotPlugin;

// Re-export models for convenience
pub use models::*;
```

## server/src/main.rs (Simplified)

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    fanuc_rmi_replica_plugins::build().run();
    Ok(())
}
```

## app/Cargo.toml

```toml
[package]
name = "fanuc_rmi_replica_app"
version = "0.1.0"
edition = "2024"

[dependencies]
fanuc_rmi_replica_plugins = { path = "../plugins", default-features = false, features = ["stores"] }
leptos = { workspace = true }
pl3xus_client = { workspace = true }
# ... other client deps
```

## Validation Checklist

- [ ] `cargo build -p fanuc_rmi_replica_plugins --features ecs,server`
- [ ] `cargo build -p fanuc_rmi_replica_plugins --features stores --no-default-features`
- [ ] `cargo build -p fanuc_rmi_replica_server`
- [ ] `cargo build -p fanuc_rmi_replica_app --target wasm32-unknown-unknown`
- [ ] Run server and client together, verify all features work

