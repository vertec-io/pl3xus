# Plugin Structure Migration Research

**Status**: Active Research
**Created**: 2025-12-24
**Goal**: Migrate fanuc_rmi_replica from shared-types structure to plugin-based structure

---

## Objective

Create a copy of fanuc_rmi_replica and convert it to use the plugin-based project structure (as documented in `pl3xus-project-structure` skill). This is a structural refactoring only - no functionality changes.

## Current Structure (Shared Types)

```
fanuc_rmi_replica/
├── Cargo.toml              # Workspace
├── server/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       └── plugins/        # Bevy plugins
├── client/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       └── pages/
└── shared/
    ├── Cargo.toml
    └── src/
        ├── lib.rs
        ├── components.rs
        ├── messages.rs
        └── requests.rs
```

## Target Structure (Plugin-Based)

```
fanuc_rmi_replica_plugins/   # Or new directory name
├── Cargo.toml               # Workspace
├── plugins/
│   ├── Cargo.toml           # Features: ecs, server, stores
│   └── src/
│       ├── lib.rs           # build() function, plugin exports
│       ├── robot/
│       │   ├── mod.rs
│       │   ├── models.rs    # Types with cfg_attr for Component derive
│       │   ├── systems.rs   # cfg(feature = "ecs")
│       │   └── plugin.rs    # cfg(feature = "ecs")
│       ├── program/
│       │   └── ...
│       └── ...
├── server/
│   ├── Cargo.toml           # Imports plugins with ecs + server features
│   └── src/
│       └── main.rs          # Minimal: imports plugins, calls build().run()
└── app/
    ├── Cargo.toml           # Imports plugins with stores feature only
    └── src/
        ├── main.rs
        └── hmi/
```

## Migration Steps

### Phase 1: Setup
- [ ] Create new directory `fanuc_rmi_replica_plugins/`
- [ ] Set up workspace Cargo.toml
- [ ] Create plugins/, server/, app/ crate structure

### Phase 2: Migrate Types to Plugins
- [ ] Move `shared/src/components.rs` types to domain modules in plugins/
- [ ] Add `#[cfg_attr(feature = "ecs", derive(Component))]` to types
- [ ] Move `shared/src/messages.rs` to appropriate domain modules
- [ ] Move `shared/src/requests.rs` to appropriate domain modules
- [ ] Set up feature gates in plugins/Cargo.toml

### Phase 3: Migrate Server
- [ ] Move server plugins to plugins/src/{domain}/plugin.rs
- [ ] Move server systems to plugins/src/{domain}/systems.rs
- [ ] Wrap ECS code with `#[cfg(feature = "ecs")]`
- [ ] Create plugins/src/lib.rs with build() function
- [ ] Simplify server/src/main.rs to just call build().run()

### Phase 4: Migrate Client
- [ ] Rename client/ to app/
- [ ] Update app/Cargo.toml to use plugins with stores feature
- [ ] Verify client code works with plugins dependency

### Phase 5: Validation
- [ ] Both server and client compile
- [ ] All functionality works identically
- [ ] Document any lessons learned

## Key Transformations

### Before (shared/src/components.rs)
```rust
use bevy::prelude::*;

#[derive(Component, Clone, Debug, Serialize, Deserialize)]
pub struct RobotStatus {
    pub connected: bool,
    pub program_loaded: Option<String>,
}
```

### After (plugins/src/robot/models.rs)
```rust
#[cfg_attr(feature = "ecs", derive(bevy::prelude::Component))]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RobotStatus {
    pub connected: bool,
    pub program_loaded: Option<String>,
}
```

## Reference

- [Plugin Structure Reference](../../../../../research/active/pl3xus-skills/pl3xus-project-structure/references/plugin-structure.md)
- [Meteorite Codebase](/home/apino/dev/meteorite) - Reference implementation

## Notes

- This is a structural migration only - no logic changes
- Feature flags: `ecs` (server), `server` (server-only deps), `stores` (client)
- The server should become minimal - just imports and runs plugins

