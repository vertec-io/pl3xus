# Multi-Crate Plugin Architecture (Pattern 3)

## Overview

The multi-crate plugin pattern provides the most sophisticated project organization for large pl3xus applications. Each plugin is its own crate with its own `Cargo.toml`, and the server binary assembles them via a `PluginGroup`.

## Key Benefits

1. **Clear Boundaries**: Each plugin crate has explicit dependencies
2. **Independent Versioning**: Plugins can be versioned and published separately
3. **Framework/Application Separation**: Reusable framework crates vs project-specific application crates
4. **Isolated Testing**: Each plugin can be tested in isolation
5. **Selective Compilation**: Only compile what you need

## Directory Structure

```
project/
├── Cargo.toml                    # Workspace root
├── server/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       └── plugins.rs            # Plugin assembly
├── app/
│   ├── Cargo.toml
│   └── src/
├── plugins/
│   ├── core/                     # Framework
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── robotics/                 # Framework (types only)
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── execution/                # Framework
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── fanuc/                    # Application
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   └── duet/                     # Application
│       ├── Cargo.toml
│       └── src/lib.rs
└── simulator/                    # Standalone (NOT in workspace)
```

## Framework vs Application Crates

### Framework Crates (Reusable)

These provide generic functionality that can be reused across projects:

| Crate | Purpose |
|-------|---------|
| `core` | Networking, authorization, logging |
| `robotics` | Robot-agnostic types (RobotPose, FrameId, conversions) |
| `execution` | Orchestration, ToolpathBuffer, device traits |

### Application Crates (Project-Specific)

These implement specific device integrations:

| Crate | Purpose |
|-------|---------|
| `fanuc` | FANUC robot driver, motion handler |
| `duet` | Duet extruder, G-code generation |
| `abb` | ABB robot driver (hypothetical) |

## Plugin Crate Template

```toml
# plugins/fanuc/Cargo.toml
[package]
name = "myproject_fanuc"
version = "0.1.0"
edition = "2021"

[dependencies]
bevy.workspace = true
myproject_core = { path = "../core" }
myproject_robotics = { path = "../robotics" }
myproject_execution = { path = "../execution" }
fanuc_rmi = { git = "..." }

[features]
default = ["server"]
server = []  # Server-only functionality
```

```rust
// plugins/fanuc/src/lib.rs
use bevy::prelude::*;

mod driver;
mod motion_handler;
mod systems;

pub use driver::*;
pub use motion_handler::*;

pub struct FanucPlugin;

impl Plugin for FanucPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            systems::connect_system,
            systems::fanuc_motion_handler_system,
            systems::status_polling_system,
        ));
    }
}
```

## Server Assembly

```rust
// server/src/plugins.rs
use bevy::prelude::*;

pub struct AppPlugins;

impl PluginGroup for AppPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(myproject_core::CorePlugin)
            .add(myproject_execution::ExecutionPlugin)
            .add(myproject_fanuc::FanucPlugin)
            .add(myproject_duet::DuetPlugin)
    }
}
```

## When to Use This Pattern

- Large projects with 5+ developers
- Multi-device integrations (robots, peripherals, sensors)
- Clear need for framework/application separation
- Plugins may be reused across projects
- Independent versioning is important
- Complex CI/CD with per-plugin testing

## Migration from Pattern 2

1. Create `plugins/core/` crate from `plugins/src/core/`
2. Create application crates for each device
3. Move device-specific code out of framework crates
4. Create `server/src/plugins.rs` for assembly
5. Update workspace `Cargo.toml` members

