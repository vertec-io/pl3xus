---
name: pl3xus-project-structure
description: Project structure patterns for pl3xus applications. Supports three architectures - shared types crate (simpler), plugin-based with feature gates (modular), and multi-crate plugins (sophisticated). Use when starting a new project or inferring structure from an existing project.
allowed-tools:
  - view
  - codebase-retrieval
  - save-file
  - str-replace-editor
  - launch-process
---

# pl3xus Project Structure Skill

## Purpose

This skill provides guidance on project organization for pl3xus applications. Three legitimate patterns are supported - choose based on project complexity and team preferences.

## Architecture Patterns

### Pattern 1: Shared Types Crate (Recommended for Simpler Projects)

Best for smaller projects, single-domain applications, or when getting started.

```
project/
├── Cargo.toml                    # Workspace root
├── shared/                       # Shared types crate
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       └── components/
├── server/                       # Bevy ECS server
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       └── plugins/
└── client/                       # Leptos WASM client
    ├── Cargo.toml
    ├── Trunk.toml
    └── src/
        ├── main.rs
        ├── app.rs
        └── pages/
```

### Pattern 2: Plugin-Based with Feature Gates (Recommended for Medium Projects)

Best for medium codebases, multi-domain applications, or when plugins need to be selectively included but individual versioning isn't required.

```
project/
├── Cargo.toml                    # Workspace root
├── plugins/                      # Single crate with feature-gated modules
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs               # build() function + feature detection
│       ├── core/                # Core plugin module
│       ├── robot_driver/        # Robot driver plugin
│       └── process_control/     # Process control plugin
├── server/                       # Minimal - imports plugins and runs
│   ├── Cargo.toml
│   └── src/
│       └── main.rs
└── app/                          # Leptos WASM client
    ├── Cargo.toml
    ├── Trunk.toml
    └── src/
        ├── main.rs
        └── hmi/
```

### Pattern 3: Multi-Crate Plugins (Recommended for Large/Sophisticated Projects)

Best for large codebases with clear separation between framework and application code, when plugins need independent versioning, or when multiple device types are integrated.

```
project/
├── Cargo.toml                    # Workspace root
├── server/                       # Binary - assembles plugins
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       └── plugins.rs           # Plugin assembly (PluginGroup)
├── app/                          # Leptos WASM client
│   ├── Cargo.toml
│   ├── Trunk.toml
│   └── src/
├── plugins/                      # Directory of plugin crates
│   ├── core/                    # Framework: networking, auth, logging
│   │   ├── Cargo.toml
│   │   └── src/lib.rs           # Exports CorePlugin
│   ├── robotics/                # Framework: robot-agnostic types
│   │   ├── Cargo.toml
│   │   └── src/lib.rs           # Types only (RobotPose, conversions)
│   ├── execution/               # Framework: orchestration, device traits
│   │   ├── Cargo.toml
│   │   └── src/lib.rs           # Exports ExecutionPlugin
│   ├── fanuc/                   # Application: FANUC robot driver
│   │   ├── Cargo.toml
│   │   └── src/lib.rs           # Exports FanucPlugin
│   └── duet/                    # Application: Duet extruder
│       ├── Cargo.toml
│       └── src/lib.rs           # Exports DuetPlugin
└── simulator/                    # Standalone (NOT in workspace)
    └── Cargo.toml
```

## Pattern 1: Shared Types Crate

### Workspace Cargo.toml

```toml
[workspace]
resolver = "2"
members = ["shared", "server", "client"]

[workspace.dependencies]
bevy = "0.17"
leptos = { version = "0.7", features = ["csr"] }
serde = { version = "1.0", features = ["derive"] }
pl3xus = { git = "..." }
pl3xus_sync = { git = "..." }
pl3xus_client = { git = "..." }
```

### Shared Crate

```toml
# shared/Cargo.toml
[package]
name = "shared"

[dependencies]
bevy.workspace = true
serde.workspace = true
pl3xus_sync.workspace = true
```

```rust
// shared/src/lib.rs
pub mod components;
pub mod requests;
pub mod mutations;
```

### Server

```toml
# server/Cargo.toml
[dependencies]
shared = { path = "../shared" }
bevy.workspace = true
pl3xus.workspace = true
pl3xus_sync.workspace = true
```

### Client

```toml
# client/Cargo.toml
[dependencies]
shared = { path = "../shared" }
leptos.workspace = true
pl3xus_client.workspace = true
```

## Pattern 2: Plugin-Based Architecture

### Feature Strategy

Types are defined in plugins with conditional derives:

```rust
// plugins/src/robot_driver/models/status.rs
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "ecs", derive(bevy::prelude::Component))]
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct RobotStatus {
    pub id: String,
    pub connected: bool,
    pub position: [f64; 6],
}
```

### Plugin Module Structure

```rust
// plugins/src/robot_driver/mod.rs
pub mod models;
pub use models::*;

use cfg_if::cfg_if;
cfg_if! {
    if #[cfg(feature = "ecs")] {
        pub mod systems;
        pub mod bundles;
        pub mod plugin;
        pub use plugin::*;
    }
}
```

### plugins/Cargo.toml

```toml
[package]
name = "plugins"

[dependencies]
bevy = { version = "0.17", default-features = false, features = ["multi_threaded"] }
serde.workspace = true
cfg-if.workspace = true
pl3xus_sync = { workspace = true, optional = true }
tokio = { workspace = true, optional = true }

[features]
default = ["ecs", "server"]
ecs = ["dep:pl3xus_sync"]
server = ["dep:tokio", "ecs"]
stores = []  # Client-side reactive stores
```

### plugins/src/lib.rs

```rust
pub mod core;
pub mod robot_driver;
pub mod process_control;

#[cfg(feature = "ecs")]
use bevy::prelude::*;

#[cfg(feature = "ecs")]
pub fn build() -> Result<App, Box<dyn std::error::Error>> {
    use bevy::state::app::StatesPlugin;
    use core::CorePlugins;

    let mut app = App::new();
    app.add_plugins(CorePlugins);
    app.add_plugins(StatesPlugin);

    #[cfg(feature = "server")]
    {
        use robot_driver::RobotDriverPlugin;
        use process_control::ProcessControlPlugin;
        app.add_plugins((RobotDriverPlugin, ProcessControlPlugin));
    }

    Ok(app)
}
```

### Server (Minimal)

```rust
// server/src/main.rs
use plugins::build;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = build()?;
    app.run();
    Ok(())
}
```

```toml
# server/Cargo.toml
[dependencies]
plugins = { path = "../plugins", features = ["ecs", "server"] }
```

### Client Usage

```toml
# app/Cargo.toml
[dependencies]
plugins = { path = "../plugins", default-features = false, features = ["stores"] }
leptos.workspace = true
pl3xus_client.workspace = true
```

```rust
// app/src/hmi/robot_status.rs
use leptos::prelude::*;
use plugins::robot_driver::RobotStatus;  // Types from plugin

#[component]
pub fn RobotStatusPanel() -> impl IntoView {
    let status = use_entity_component::<RobotStatus>(robot_id);
    // ...
}
```

## Pattern 3: Multi-Crate Plugin Architecture

### Key Concepts

**Framework vs Application Separation:**
- Framework crates (`core`, `robotics`, `execution`) are reusable across projects
- Application crates (`fanuc`, `duet`) are project-specific device implementations

**Server Assembles Plugins:**
- No aggregator crate needed
- Server decides which plugins to load
- Different deployments can load different plugin sets

### Workspace Cargo.toml

```toml
[workspace]
resolver = "2"
members = [
    "app",
    "server",
    "plugins/core",
    "plugins/robotics",
    "plugins/execution",
    "plugins/fanuc",
    "plugins/duet",
]
# Note: simulator/ is NOT a member - standalone binary

[workspace.dependencies]
bevy = { version = "0.17", default-features = false, features = ["multi_threaded"] }
serde = { version = "1.0", features = ["derive"] }
pl3xus = { git = "..." }
```

### Plugin Crate Example (plugins/fanuc)

```toml
# plugins/fanuc/Cargo.toml
[package]
name = "myproject_fanuc"

[dependencies]
bevy.workspace = true
myproject_core = { path = "../core" }
myproject_robotics = { path = "../robotics" }
myproject_execution = { path = "../execution" }
fanuc_rmi = { git = "..." }

[features]
default = ["server"]
server = []
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

### Server Plugin Assembly

```rust
// server/src/plugins.rs
use bevy::prelude::*;
use myproject_core::CorePlugin;
use myproject_execution::ExecutionPlugin;
use myproject_fanuc::FanucPlugin;
use myproject_duet::DuetPlugin;

pub struct AppPlugins;

impl PluginGroup for AppPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(CorePlugin)
            .add(ExecutionPlugin)
            .add(FanucPlugin)
            .add(DuetPlugin)
    }
}
```

```rust
// server/src/main.rs
mod plugins;

fn main() {
    App::new()
        .add_plugins(plugins::AppPlugins)
        .run();
}
```

```toml
# server/Cargo.toml
[dependencies]
bevy.workspace = true
myproject_core = { path = "../plugins/core" }
myproject_execution = { path = "../plugins/execution" }
myproject_fanuc = { path = "../plugins/fanuc" }
myproject_duet = { path = "../plugins/duet" }
```

### Client Usage

```toml
# app/Cargo.toml
[dependencies]
# Only import types crates, not server plugins
myproject_core = { path = "../plugins/core", default-features = false }
myproject_robotics = { path = "../plugins/robotics", default-features = false }
leptos.workspace = true
pl3xus_client.workspace = true
```

### Dependency Graph

```
server
├── plugins/core
├── plugins/execution
│   └── plugins/robotics
├── plugins/fanuc
│   ├── plugins/core
│   ├── plugins/robotics
│   └── plugins/execution
└── plugins/duet
    └── plugins/execution

app (WASM)
├── plugins/core (default-features = false)
└── plugins/robotics (default-features = false)
```

## Inferring Project Structure

When working in an existing project, detect the pattern:

### Check for Multi-Crate Plugins (Pattern 3)

```bash
# Look for multiple Cargo.toml files in plugins/ directory
ls plugins/*/Cargo.toml 2>/dev/null
# Look for server/src/plugins.rs
ls server/src/plugins.rs 2>/dev/null
```

If found: Use multi-crate plugin patterns.

### Check for Feature-Gated Plugins (Pattern 2)

```bash
# Look for single plugins crate with features
grep -r "feature = \"ecs\"" plugins/src/ 2>/dev/null
grep "ecs\|server\|stores" plugins/Cargo.toml 2>/dev/null
```

If found: Use feature-gated plugin patterns.

### Check for Shared Types (Pattern 1)

```bash
# Look for shared/types crate
ls shared/src/lib.rs types/src/lib.rs 2>/dev/null
```

If found: Use shared types patterns.

## When to Use Each Pattern

| Criteria | Shared Types | Feature-Gated | Multi-Crate |
|----------|--------------|---------------|-------------|
| Project size | Small | Medium | Large |
| Team size | 1-3 devs | 3-5 devs | 5+ devs |
| Domains | Single | Multi | Multi + devices |
| Plugin reuse | No | Limited | Full |
| Independent versioning | No | No | Yes |
| Build complexity | Simple | Medium | Complex |
| Type visibility | All everywhere | Feature-gated | Per-crate |
| Framework/App separation | No | Partial | Full |

## Anti-Patterns

### ❌ Mixing Patterns

Don't create both a `shared/` crate AND feature-gated types in `plugins/`:

```
# BAD - confusing, duplicate types
project/
├── shared/           # ❌ Types here...
├── plugins/          # ❌ ...AND here with features
```

### ❌ Server Logic in Plugins Without Feature Gates

```rust
// ❌ BAD - will fail on client (WASM)
use tokio::net::TcpStream;  // No feature gate!

// ✅ GOOD
#[cfg(feature = "server")]
use tokio::net::TcpStream;
```

### ❌ Client Importing Server Features

```toml
# ❌ BAD - will fail WASM build
plugins = { path = "../plugins", features = ["ecs", "server"] }

# ✅ GOOD - client-only features
plugins = { path = "../plugins", default-features = false, features = ["stores"] }
```

### ❌ Application Code in Framework Crates (Pattern 3)

```
# BAD - execution plugin contains device-specific code
plugins/
├── execution/
│   └── src/
│       ├── duet_handler.rs    # ❌ Device-specific
│       └── fanuc_handler.rs   # ❌ Device-specific

# GOOD - device code in separate crates
plugins/
├── execution/                  # Framework only
├── duet/                       # Device-specific
└── fanuc/                      # Device-specific
```

## Reference Examples

- **Shared Types Pattern**: `examples/fanuc_rmi_replica/` in pl3xus
- **Feature-Gated Pattern**: meteorite codebase (external)
- **Multi-Crate Pattern**: `examples/fanuc_rmi_replica_plugins/` in pl3xus

## Resources

- `references/shared-types-structure.md` - Detailed shared types setup
- `references/plugin-structure.md` - Detailed plugin architecture
- `references/multi-crate-plugins.md` - Multi-crate plugin architecture

