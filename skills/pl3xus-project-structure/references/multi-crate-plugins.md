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

### Directory Structure

Each plugin crate follows this internal structure:

```
plugins/device_name/
├── Cargo.toml
└── src/
    ├── lib.rs           # Module exports only (feature-gated)
    ├── plugin.rs        # Plugin impl (feature="ecs")
    ├── connection.rs    # Driver/connection (feature="server")
    ├── types/           # Types (always available)
    │   ├── mod.rs
    │   ├── config.rs
    │   ├── device.rs
    │   └── requests.rs
    ├── database/        # Database (feature="server")
    │   ├── mod.rs
    │   ├── schema.rs
    │   └── queries.rs
    ├── handlers/        # Request handlers (feature="server")
    │   ├── mod.rs
    │   └── control.rs
    └── systems/         # ECS systems (feature="server")
        ├── mod.rs
        └── command.rs
```

### Cargo.toml

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

cfg-if.workspace = true
serde.workspace = true

[features]
default = ["ecs", "server"]
ecs = []     # ECS components, plugin
server = []  # Server-only: drivers, database, handlers, systems
stores = []  # Client-side reactive stores
```

### lib.rs (Module Exports Only)

**The plugin belongs in the server feature** - `sync_component` and other pl3xus features are server-side only.

**Use `pub mod` only (no glob re-exports at lib.rs level)** for a consistent API across all plugins. This prevents name collisions and provides a predictable import pattern.

#### Feature Gating Strategy

The goal is to minimize boilerplate by structuring lib.rs to reduce feature gating in lower-level files:

1. **types/ module** - Always exported (no feature gate at lib.rs level). Internal feature gating for ECS derives, stores, etc. happens within individual type files.

2. **utils/ module** (or utility files at root level) - Same as types: always exported, internal feature gating as needed.

3. **Everything else** (plugin.rs, handlers/, systems/, database/, connection.rs) - Feature-gated behind `server` at lib.rs level. **No internal feature gating needed in these files** since they only compile when server is enabled.

```rust
// plugins/fanuc/src/lib.rs
//! FANUC Robot Plugin - lib.rs exports modules only
//!
//! # Usage
//!
//! ```rust,ignore
//! use myproject_fanuc::types::*;
//! use myproject_fanuc::database;
//! use myproject_fanuc::conversion;
//! ```

use cfg_if::cfg_if;

// ============================================================================
// ALWAYS AVAILABLE - No feature gating at lib.rs level
// ============================================================================

// Types - always available (internal feature gating for ECS derives, stores)
pub mod types;

// Conversion utilities - always available
pub mod conversion;

// Re-export vendor type for convenience
pub use fanuc_rmi::Position;

// ============================================================================
// SERVER-ONLY - Plugin, systems, handlers, database, driver
// No internal feature gating needed in these files!
// ============================================================================
cfg_if! {
    if #[cfg(feature = "server")] {
        mod connection;
        mod plugin;
        pub mod database;
        pub mod handlers;
        pub mod systems;

        pub use plugin::FanucPlugin;  // Only re-export the plugin itself
    }
}
```

#### Why No Feature Gate on types/?

Previously we used `#[cfg(any(feature = "ecs", feature = "stores"))]` on the types module. This is unnecessary because:

1. **Types are always needed** - Both server and client code need access to type definitions
2. **Internal gating is sufficient** - ECS derives like `Component` are gated within the type files themselves
3. **Simpler dependencies** - Consumers don't need to enable `ecs` or `stores` features just to access type definitions

### Consistent API Pattern

All plugins expose a consistent API that consumers can rely on:

```rust
// Option 1: Glob import from specific module
use myproject_fanuc::types::*;
let status = FanucStatus::default();

// Option 2: Qualified access via module
use myproject_fanuc::types;
let status = types::FanucStatus::default();

// Database queries accessed via namespace (prevents conflicts)
use myproject_fanuc::database;
let config = database::get_configuration(&conn, robot_id)?;

// Systems accessed via namespace
use myproject_fanuc::systems;
app.add_systems(Update, systems::fanuc_motion_handler_system);
```

This pattern prevents name collisions (e.g., `database::get_configuration` won't conflict with `leptos::prelude::get_configuration`).

### Sub-module mod.rs (Glob Re-exports OK)

Within sub-modules, glob re-exports are fine since they're contained within the namespace:

```rust
// types/mod.rs - glob exports OK here
mod config;
mod device;
mod requests;

pub use config::*;
pub use device::*;
pub use requests::*;
```

```rust
// database/mod.rs - glob exports OK here
mod schema;
mod queries;

pub use schema::*;
pub use queries::*;
```

### plugin.rs (Separate File)

**IMPORTANT**: The plugin implementation lives in its own `plugin.rs` file, NOT inline in `lib.rs`.

**NO INTERNAL FEATURE GATING**: Since `plugin.rs` is already feature-gated behind `server` in `lib.rs`, you don't need `#[cfg(feature = "server")]` inside this file. Just write normal code.

```rust
// plugins/fanuc/src/plugin.rs
//! Bevy plugin registration for FANUC robot functionality.

use bevy::prelude::*;
use pl3xus_sync::{ComponentSyncConfig, AppPl3xusSyncExt};

use crate::types::*;
use crate::handlers::FanucHandlerPlugin;
use crate::systems::{fanuc_motion_handler_system, FanucCommandEvent};
use crate::database::FanucDatabaseInit;
use myproject_core::database::DatabaseInitRegistry;

pub struct FanucPlugin;

impl Plugin for FanucPlugin {
    fn build(&self, app: &mut App) {
        // Sync components
        app.sync_component::<FanucStatus>(Some(ComponentSyncConfig::read_only()));
        app.sync_component::<FanucConfig>(Some(ComponentSyncConfig::read_only()));

        // Register database initializer via registry
        if let Some(mut registry) = app.world_mut().get_resource_mut::<DatabaseInitRegistry>() {
            registry.register(FanucDatabaseInit);
        }

        // Add handler plugin
        app.add_plugins(FanucHandlerPlugin);

        // Register events
        app.add_message::<FanucCommandEvent>();

        // Add systems
        app.add_systems(Update, fanuc_motion_handler_system);

        info!("🤖 FanucPlugin initialized");
    }
}
```

### Key Points

1. **lib.rs exports modules, plugin.rs implements**: lib.rs uses `pub mod` only; plugin.rs contains the Plugin impl
2. **No glob re-exports at lib.rs level**: Use `pub mod types;` not `pub use types::*;` for consistent, collision-free API
3. **Glob re-exports OK inside sub-modules**: Within `types/mod.rs`, `database/mod.rs` etc., glob exports are fine
4. **Always have a types directory**: Put all types in `src/types/` for consistency across plugins
5. **Database initialization via registry**: Use `DatabaseInitRegistry` resource, not a separate `init_*_database()` function
6. **Sync components in plugin**: Register synced components in `plugin.rs` build method
7. **No redundant feature gates**: Since plugin.rs is already behind `#[cfg(feature = "server")]` in lib.rs, don't add internal feature gates

### Feature Flag Semantics

| Feature | Purpose | Contents |
|---------|---------|----------|
| `ecs` | Bevy ECS integration | Types with `#[derive(Component)]`, `#[derive(Resource)]`, Plugin struct |
| `server` | Server-only logic | Drivers, database, handlers, systems |
| `stores` | Client reactive stores | Leptos/reactive store implementations |

**Important**: The `ecs` feature provides types with ECS derives that can be used by:
- Server applications (Bevy server)
- Bevy client applications (native Bevy client, not web)

The `server` feature is for logic that should ONLY run on the server (hardware drivers, database access, request handlers, ECS systems).

```rust
// In types/device.rs - ECS derives gated by "ecs" feature
#[cfg_attr(feature = "ecs", derive(bevy::prelude::Component))]
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct RobotStatus {
    pub connected: bool,
    pub position: [f64; 6],
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

