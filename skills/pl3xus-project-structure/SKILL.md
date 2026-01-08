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

### Plugin Internal Structure (STANDARD)

Each plugin crate follows this **standard internal structure**. This is the **accepted pattern** for pl3xus device/feature plugins:

```
my_plugin/src/
├── types/                  # ALWAYS AVAILABLE - all types
│   ├── mod.rs              # Re-exports all types
│   ├── config.rs           # Database/persistent config types
│   ├── device.rs           # ECS component types (conditional derives)
│   └── requests.rs         # Request/response message types
├── database/               # SERVER-ONLY - persistence
│   ├── mod.rs
│   ├── schema.rs           # Table definitions
│   └── queries.rs          # CRUD operations
├── systems/                # SERVER-ONLY - handlers
│   ├── mod.rs
│   ├── handlers.rs         # Request handlers + handler plugin
│   └── command.rs          # Device command events and systems
├── connection.rs           # SERVER-ONLY - device driver/connection
├── plugin.rs               # ECS-ONLY - Bevy plugin (registration)
└── lib.rs                  # Main exports with feature gates
```

### lib.rs Structure (STANDARD)

```rust
use cfg_if::cfg_if;

// ============================================================================
// TYPES - Always available (conditionally compiled Component/macro derives)
// ============================================================================
pub mod types;

pub use types::{
    // Config types (database storage)
    MyConnectionConfig, MyConnectionType,
    // Device component types (synced ECS)
    MyControllerConfig, MyStatus, ActiveMyDevice,
    // Request/response types
    ListMyConnections, ListMyConnectionsResponse,
    CreateMyConnection, CreateMyConnectionResponse,
    UpdateMyConnection, UpdateMyConnectionResponse,
    // ... all types exported at crate root
};

// ============================================================================
// SERVER-ONLY - Systems, handlers, database, driver
// ============================================================================
cfg_if! {
    if #[cfg(feature = "server")] {
        mod connection;
        pub mod database;
        pub mod systems;

        pub use connection::MyDriver;
        pub use database::MyDatabaseInit;
        pub use systems::{
            MyCommandEvent, MyControllerBundle,
            MyHandlerPlugin,
            // Individual systems if needed
        };
    }
}

// ============================================================================
// ECS-ONLY - Plugin (both server and tests)
// ============================================================================
cfg_if! {
    if #[cfg(feature = "ecs")] {
        mod plugin;
        pub use plugin::MyPlugin;
    }
}
```

### Conditional Derives in types/device.rs

```rust
#[cfg(feature = "ecs")]
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Component available on both client and server.
/// On server: derives Component for ECS.
/// On client: plain data type for stores.
#[cfg_attr(feature = "ecs", derive(Component))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct MyStatus {
    pub connected: bool,
    pub value: f32,
    // Server-driven UI state
    pub can_update: bool,
    pub can_reset: bool,
}
```

### Conditional Macro Derives in types/requests.rs

```rust
use pl3xus_common::RequestMessage;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use pl3xus_macros::{HasSuccess, Invalidates};

/// Request type - always available
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(Invalidates))]
#[cfg_attr(feature = "server", invalidates("ListMyConnections"))]
pub struct CreateMyConnection {
    pub name: String,
}

/// Response type - always available
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(HasSuccess))]
pub struct CreateMyConnectionResponse {
    pub success: bool,
    pub id: Option<i64>,
    pub error: Option<String>,
}

impl RequestMessage for CreateMyConnection {
    type ResponseMessage = CreateMyConnectionResponse;
}
```

### Plugin Crate Cargo.toml

```toml
# plugins/my_plugin/Cargo.toml
[package]
name = "myproject_myplugin"

[dependencies]
bevy = { workspace = true, optional = true }
serde.workspace = true
cfg-if.workspace = true
pl3xus_common.workspace = true
pl3xus_macros = { workspace = true, optional = true }
tokio = { workspace = true, optional = true }
sqlx = { workspace = true, optional = true }

[features]
default = ["ecs", "server"]
ecs = ["dep:bevy"]
server = ["ecs", "dep:pl3xus_macros", "dep:tokio", "dep:sqlx"]
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

- **Shared Types Pattern**: `examples/robot-hmi/` in pl3xus
- **Feature-Gated Pattern**: meteorite codebase (external)
- **Multi-Crate Pattern**: `examples/robot-hmi-advanced/` in pl3xus
- **Plugin Internal Structure**: `examples/meteorite/plugins/microwave/` - Standard plugin module structure

## Resources

- `references/shared-types-structure.md` - Detailed shared types setup
- `references/plugin-structure.md` - Detailed plugin architecture
- `references/multi-crate-plugins.md` - Multi-crate plugin architecture
- **`../SKILLS_REGISTRY.md`** - Complete skill registry with critical patterns (START HERE)

