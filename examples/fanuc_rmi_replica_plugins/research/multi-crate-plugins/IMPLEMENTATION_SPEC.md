# Implementation Specification: Multi-Crate Plugin Refactoring

## Overview

Refactor `examples/fanuc_rmi_replica_plugins/` from the current structure to multi-crate plugins.

## Current State

```
examples/fanuc_rmi_replica_plugins/
├── Cargo.toml              # Workspace
├── app/                    # WASM client
├── server/                 # Server binary
├── plugins/                # Single crate (monolithic)
│   └── src/
│       ├── lib.rs
│       ├── core/           # Framework: networking, auth
│       └── fanuc_driver/   # Application: FANUC driver
├── execution/              # Mixed framework + application code
├── robotics/               # Framework: RobotPose, conversions
└── simulator/              # Duet simulator
```

## Target State

```
examples/fanuc_rmi_replica_plugins/
├── Cargo.toml              # Workspace (updated members)
├── app/
├── server/
│   └── src/plugins.rs      # NEW: assembles plugins
├── plugins/
│   ├── core/               # Moved from plugins/src/core/
│   ├── robotics/           # Moved from ./robotics/
│   ├── execution/          # Moved from ./execution/ (cleaned)
│   ├── fanuc/              # Moved from plugins/src/fanuc_driver/
│   └── duet/               # Extracted from execution/
└── simulator/              # Removed from workspace
```

## Phase 1: Create New Plugin Crate Structures

### Step 1.1: Create plugins/core crate

1. Create `plugins/core/Cargo.toml`
2. Move `plugins/src/core/` → `plugins/core/src/`
3. Create `plugins/core/src/lib.rs` with `CorePlugin`
4. Update imports

### Step 1.2: Create plugins/robotics crate

1. Create `plugins/robotics/Cargo.toml`
2. Move `./robotics/src/` → `plugins/robotics/src/`
3. Update package name to `fanuc_replica_robotics` (keep same)
4. Remove old `./robotics/` directory

### Step 1.3: Create plugins/execution crate

1. Create `plugins/execution/Cargo.toml`
2. Move `./execution/src/` → `plugins/execution/src/`
3. **Remove**: `systems/duet_handler.rs`, `systems/fanuc_handler.rs`
4. **Remove**: `devices/duet_extruder.rs`
5. Keep only: components, traits, orchestrator, plugin
6. Update package name to `fanuc_replica_execution` (keep same)
7. Remove old `./execution/` directory

### Step 1.4: Create plugins/fanuc crate

1. Create `plugins/fanuc/Cargo.toml`
2. Move `plugins/src/fanuc_driver/` → `plugins/fanuc/src/`
3. Add `fanuc_motion_handler_system` from old execution
4. Create `plugins/fanuc/src/lib.rs` with `FanucPlugin`

### Step 1.5: Create plugins/duet crate

1. Create `plugins/duet/Cargo.toml`
2. Move `execution/src/devices/duet_extruder.rs` → `plugins/duet/src/`
3. Move `execution/src/systems/duet_handler.rs` → `plugins/duet/src/`
4. Create `plugins/duet/src/lib.rs` with `DuetPlugin`

## Phase 2: Update Workspace Configuration

### Step 2.1: Update root Cargo.toml

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
# Note: simulator is NOT included
```

### Step 2.2: Remove old directories

- Remove `./robotics/` (moved to plugins/)
- Remove `./execution/` (moved to plugins/)
- Remove `./plugins/src/` (split into separate crates)

## Phase 3: Update Server

### Step 3.1: Create server/src/plugins.rs

```rust
use bevy::prelude::*;
use fanuc_replica_core::CorePlugin;
use fanuc_replica_execution::ExecutionPlugin;
use fanuc_replica_fanuc::FanucPlugin;
use fanuc_replica_duet::DuetPlugin;

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

### Step 3.2: Update server/src/main.rs

```rust
mod plugins;
use plugins::AppPlugins;

fn main() {
    App::new()
        .add_plugins(AppPlugins)
        .run();
}
```

### Step 3.3: Update server/Cargo.toml

Add dependencies on all plugin crates.

## Phase 4: Update App (Client)

### Step 4.1: Update app dependencies

```toml
fanuc_replica_core = { path = "../plugins/core", default-features = false }
fanuc_replica_robotics = { path = "../plugins/robotics", default-features = false }
```

## Phase 5: Cleanup and Verification

### Step 5.1: Remove old plugins/src directory

After all code is moved, delete:
- `plugins/src/` (old monolithic structure)
- `plugins/Cargo.toml` (old single-crate config)

### Step 5.2: Run cargo check

```bash
cargo check --workspace
```

### Step 5.3: Run tests

```bash
cargo test --workspace
```

## Rollback Plan

If issues arise:
1. Revert all changes via git
2. The old structure still works

## Verification Checklist

- [ ] `cargo check --workspace` passes
- [ ] `cargo test --workspace` passes
- [ ] Server starts and connects to FANUC simulator
- [ ] App builds with trunk
- [ ] No duplicate code between crates
- [ ] Each plugin has a working `Plugin::build()`

