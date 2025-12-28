# Multi-Crate Plugin Refactor - Handoff Document

**Date:** 2025-12-28
**Status:** In Progress - Half-Implemented
**Goal:** Refactor to Pattern 3 (Multi-Crate Plugins) per `/home/apino/dev/pl3xus/skills/pl3xus-project-structure/`

---

## ⚠️ CRITICAL: DO NOT LOSE FUNCTIONALITY

**THE MOST IMPORTANT THING:** We must NOT lose ANY functionality that currently exists in the working code.

### Rules for This Refactor

1. **COPY, don't rewrite** - Take the working code and copy it to the new structure. Do not attempt to "improve" or rewrite the logic.

2. **Double and triple check** - Before deleting any old code, verify:
   - Every system that was registered is still registered
   - Every handler that existed still exists
   - Every sync_component call is preserved
   - Every database table/query is preserved

3. **The working code is the source of truth** - If there's any doubt, refer to `plugins/src/robot/` and `plugins/src/core/` for the correct implementation.

4. **Test incrementally** - After wiring up each module, run `cargo check` and verify the dead code warnings decrease.

5. **Don't delete old code until verified** - The old `plugins/src/` directory should only be removed AFTER the new plugins are fully working.

### Success Criteria (non-exhaustive)

The refactor is ONLY complete when:
- [ ] All functionality from `plugins/src/robot/` works in `plugins/fanuc/`
- [ ] All functionality from `plugins/src/core/` works in `plugins/core/`
- [ ] Server starts and all WebSocket handlers respond
- [ ] Database operations work (CRUD for robots, programs, configs)
- [ ] Robot connection/disconnection works
- [ ] Jogging works
- [ ] Program execution works
- [ ] Zero dead code warnings in the new crates
- [ ] Configuration changes and active configuration management works

---

## Executive Summary

The refactoring effort to Pattern 3 (Multi-Crate Plugins) is **partially complete but NOT functional**. New plugin crates have been created with code copied/adapted, but the code is **not wired up** and shows as dead code (89 warnings). The **working system** is still in `plugins/src/core/` and `plugins/src/robot/`.

## Current State

### What's Working (Source of Truth)

```
plugins/
├── src/
│   ├── lib.rs              # ✅ WORKING - Uses CorePlugin + RobotPlugin
│   ├── core/               # ✅ WORKING - Full implementation
│   │   ├── database.rs     # Contains FANUC-specific schema (needs extraction)
│   │   ├── plugin.rs
│   │   └── systems.rs
│   └── robot/              # ✅ WORKING - Full FANUC functionality
│       ├── connection.rs
│       ├── handlers.rs
│       ├── jogging.rs
│       ├── polling.rs
│       ├── program.rs
│       ├── sync.rs
│       └── types.rs
```

**These files are used by the server and the app compiles/runs successfully.**

### What's Half-Implemented (Dead Code)

```
plugins/
├── core/                   # ⚠️ NEW CRATE - Skeleton only
│   └── src/
│       ├── database.rs     # DatabaseResource + DatabaseInit trait (good)
│       ├── plugin.rs       # CorePlugin (minimal)
│       └── types.rs        # ActiveSystem only
├── fanuc/                  # ⚠️ NEW CRATE - Code copied but not wired
│   └── src/
│       ├── connection.rs   # 89 warnings - dead code
│       ├── handlers.rs     # All functions unused
│       ├── jogging.rs
│       ├── polling.rs
│       ├── program.rs
│       ├── sync.rs
│       ├── types.rs
│       ├── database/       # Extension trait + impl (good design)
│       │   ├── schema.rs
│       │   ├── queries.rs
│       │   └── impl_queries.rs
│       └── plugin.rs       # FanucPlugin exists but empty
├── execution/              # ⚠️ NEW CRATE - Has orchestrator
│   └── src/
├── robotics/               # ⚠️ NEW CRATE - Has RobotPose types
│   └── src/
└── duet/                   # ⚠️ NEW CRATE - Minimal
    └── src/
```

**These crates compile but are NOT USED. All systems are dead code.**

## The Problem

The previous attempt:
1. ✅ Created the new crate structures
2. ✅ Copied code into the new crates
3. ✅ Set up proper import paths
4. ❌ **Did NOT wire up the plugins** - FanucPlugin::build() is empty
5. ❌ **Did NOT integrate with plugins/src/lib.rs** - Still uses old code
6. ❌ **Did NOT extract FANUC schema from core** - Core still has FANUC tables

## What Needs To Be Done

### Phase 1: Wire Up FanucPlugin (Critical)

The `plugins/fanuc/src/plugin.rs` needs to register all systems:

```rust
impl Plugin for FanucPlugin {
    fn build(&self, app: &mut App) {
        // 1. Initialize FANUC schema (call FanucDatabaseInit)
        // 2. Register systems from handlers.rs
        // 3. Register systems from connection.rs
        // 4. Register systems from polling.rs
        // 5. Register systems from program.rs
        // 6. Register systems from sync.rs
        // 7. Register systems from jogging.rs
    }
}
```

**Reference:** Look at `plugins/src/robot/plugin.rs` for the working implementation.

### Phase 2: Update Core Plugin

1. **Extract** FANUC-specific tables from `plugins/core/src/database.rs`
2. Move schema to `plugins/fanuc/src/database/schema.rs` (already partially done)
3. Core should only create `DatabaseResource` and call `DatabaseInitRegistry`

### Phase 3: Update plugins/src/lib.rs

Change from:
```rust
app.add_plugins(CorePlugin);  // Old - in plugins/src/core/
app.add_plugins(RobotPlugin); // Old - in plugins/src/robot/
```

To:
```rust
app.add_plugins(fanuc_replica_core::CorePlugin);
app.add_plugins(fanuc_replica_fanuc::FanucPlugin);
app.add_plugins(fanuc_replica_execution::ExecutionPlugin);
app.add_plugins(fanuc_replica_duet::DuetPlugin);
```

### Phase 4: Cleanup

After verification:
1. Remove `plugins/src/core/` (old)
2. Remove `plugins/src/robot/` (old)
3. Update `plugins/src/lib.rs` to re-export from new crates

## Key Architecture Decisions

### Database Extension Trait Pattern

The database extension pattern is **correct and should be kept**:

```rust
// In plugins/fanuc/src/database/queries.rs
pub trait FanucDatabaseExt {
    fn list_robot_connections(&self) -> anyhow::Result<Vec<RobotConnection>>;
    // ... all FANUC-specific queries
}

// In plugins/fanuc/src/database/impl_queries.rs
impl FanucDatabaseExt for DatabaseResource {
    fn list_robot_connections(&self) -> anyhow::Result<Vec<RobotConnection>> {
        // Implementation
    }
}
```

This allows:
- Core provides `DatabaseResource`
- Each plugin extends it with their own methods
- Clean separation of concerns

### DatabaseInit Trait Pattern

The schema registration pattern is **correct**:

```rust
// In plugins/core/src/database.rs
pub trait DatabaseInit: Send + Sync {
    fn name(&self) -> &'static str;
    fn init_schema(&self, conn: &Connection) -> anyhow::Result<()>;
    fn run_migrations(&self, conn: &Connection) -> anyhow::Result<()> { Ok(()) }
    fn seed_data(&self, conn: &Connection) -> anyhow::Result<()> { Ok(()) }
}

// In plugins/fanuc/src/database/schema.rs
pub struct FanucDatabaseInit;
impl DatabaseInit for FanucDatabaseInit { ... }
```

## Critical Code Comparison

### Working Plugin (plugins/src/robot/plugin.rs)

```rust
impl Plugin for RobotPlugin {
    fn build(&self, app: &mut App) {
        // SYNCED COMPONENTS - All registered properly
        app.sync_component::<ActiveRobot>(Some(ComponentSyncConfig::read_only()));
        app.sync_component::<RobotPosition>(...);
        app.sync_component::<JointAngles>(...);
        app.sync_component::<RobotStatus>(...);
        app.sync_component::<IoStatus>(...);
        app.sync_component::<ExecutionState>(...);
        app.sync_component::<ConnectionState>(...);
        app.sync_component::<FrameToolDataState>(...);
        app.sync_component::<IoConfigState>(...);
        app.sync_component::<ActiveConfigState>(None);

        app.sync_component_builder::<JogSettingsState>()
            .with_handler::<WebSocketProvider, _, _>(jogging::handle_jog_settings_mutation)
            .targeted()
            .with_default_entity_policy()
            .build();

        // SUB-PLUGINS - All 5 registered
        app.add_plugins((
            RobotConnectionPlugin,  // Connection state machine
            RobotSyncPlugin,        // Driver polling and jogging
            RequestHandlerPlugin,   // Database request handlers
            RobotPollingPlugin,     // Periodic position/status polling
            ProgramPlugin,          // Orchestrator-based program execution
        ));
    }
}
```

### Dead Code Plugin (plugins/fanuc/src/plugin.rs)

```rust
impl Plugin for FanucPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "server")]
        {
            // ONLY ONE SYSTEM REGISTERED!
            app.add_systems(Update, fanuc_motion_handler_system);
            info!("FANUC plugin loaded");
        }
        // ❌ No synced components
        // ❌ No sub-plugins (connection, handlers, polling, program, sync)
        // ❌ No database initialization
    }
}
```

**The entire system registration is MISSING from the new crate.**

## Files Reference

### Working Code (Copy FROM these)

| File | Purpose |
|------|---------|
| `plugins/src/robot/handlers.rs` | All WebSocket request handlers |
| `plugins/src/robot/connection.rs` | RMI connection management |
| `plugins/src/robot/plugin.rs` | System registration pattern |
| `plugins/src/robot/types.rs` | All type definitions |
| `plugins/src/core/database.rs` | Schema + queries (extract FANUC tables) |

### New Crates (Wire UP these)

| Crate | Files Needing Work |
|-------|-------------------|
| `plugins/fanuc` | `plugin.rs` - needs system registration |
| `plugins/core` | `plugin.rs` - needs to call DatabaseInitRegistry |
| `plugins/execution` | Integration with FANUC motion handler |

## Verification Checklist

When complete, verify:

- [ ] `cargo check --workspace` - No errors
- [ ] All 89 dead code warnings in fanuc/ are GONE
- [ ] Server starts with new plugins
- [ ] `plugins/src/robot/` can be deleted without breaking anything
- [ ] `plugins/src/core/` can be deleted without breaking anything
- [ ] WebSocket handlers respond correctly
- [ ] Database operations work (create robot, list connections, etc.)

## Commands

```bash
# Check compilation
cd examples/fanuc_rmi_replica_plugins
cargo check --package fanuc_replica_fanuc
cargo check --workspace

# Run server
cargo run

# See warnings count
cargo check --package fanuc_replica_fanuc 2>&1 | grep "warning:" | wc -l
```

## Skill Reference

Full architecture documentation:
- `/home/apino/dev/pl3xus/skills/pl3xus-project-structure/SKILL.md` - Pattern 3 definition
- `/home/apino/dev/pl3xus/skills/pl3xus-project-structure/references/multi-crate-plugins.md` - Detailed guide

