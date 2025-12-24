---
name: pl3xus-project-structure
description: Project structure patterns for pl3xus applications. Supports two architectures - shared types crate (simpler) and plugin-based with feature gates (modular). Use when starting a new project or inferring structure from an existing project.
allowed-tools:
  - view
  - codebase-retrieval
  - save-file
  - str-replace-editor
  - launch-process
---

# pl3xus Project Structure Skill

## Purpose

This skill provides guidance on project organization for pl3xus applications. Two legitimate patterns are supported - choose based on project complexity and team preferences.

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

### Pattern 2: Plugin-Based with Feature Gates (Recommended for Complex/Modular Projects)

Best for large codebases, multi-domain applications, or when plugins need to be selectively included.

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

## Inferring Project Structure

When working in an existing project, detect the pattern:

### Check for Plugin-Based Pattern

```bash
# Look for plugins crate with features
grep -r "feature = \"ecs\"" plugins/src/ 2>/dev/null
grep "ecs\|server\|stores" plugins/Cargo.toml 2>/dev/null
```

If found: Use plugin-based patterns.

### Check for Shared Types Pattern

```bash
# Look for shared/types crate
ls shared/src/lib.rs types/src/lib.rs 2>/dev/null
```

If found: Use shared types patterns.

## When to Use Each Pattern

| Criteria | Shared Types | Plugin-Based |
|----------|--------------|--------------|
| Project size | Small-Medium | Medium-Large |
| Team size | 1-3 developers | 3+ developers |
| Domains | Single domain | Multi-domain |
| Plugin reuse | No | Yes |
| Build complexity | Simple | More complex |
| Type visibility | All types everywhere | Feature-gated |

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

## Reference Examples

- **Shared Types Pattern**: `examples/fanuc_rmi_replica/` in pl3xus
- **Plugin-Based Pattern**: meteorite codebase (external)

## Resources

- `references/shared-types-structure.md` - Detailed shared types setup
- `references/plugin-structure.md` - Detailed plugin architecture

