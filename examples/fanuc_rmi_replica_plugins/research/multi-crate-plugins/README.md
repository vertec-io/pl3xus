# Multi-Crate Plugin Architecture

## Problem Statement

The current `examples/fanuc_rmi_replica_plugins/` structure has several issues:

1. **Execution plugin contains application-specific code**: The `execution/` crate includes `DuetExtruder`, `FanucMotionDevice`, and their handlers - these should be in separate application crates

2. **Unclear crate boundaries**: Framework code (pl3xus-provided) and application code (user-written) are mixed

3. **Single plugins crate is monolithic**: The `plugins/` crate contains everything, making it hard to:
   - Version individual plugins
   - Test plugins in isolation
   - Reuse plugins across projects

4. **New crates sprawl at workspace root**: `execution/`, `robotics/`, `simulator/` are siblings to `app/` and `server/`, losing logical grouping

## Proposed Structure

```
examples/fanuc_rmi_replica_plugins/
├── Cargo.toml              # Top-level workspace
├── app/                    # WASM client
│   └── Cargo.toml
├── server/                 # Bevy + Axum binary
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       └── plugins.rs      # Assembles all plugins
├── plugins/                # Directory containing plugin crates
│   ├── core/               # Framework: networking, auth, logging
│   │   ├── Cargo.toml
│   │   └── src/lib.rs      # Exports CorePlugin
│   ├── robotics/           # Framework: RobotPose, FrameId, conversions
│   │   ├── Cargo.toml
│   │   └── src/lib.rs      # Types only, no plugin
│   ├── execution/          # Framework: orchestrator, buffer, traits
│   │   ├── Cargo.toml
│   │   └── src/lib.rs      # Exports ExecutionPlugin
│   ├── fanuc/              # Application: FANUC driver
│   │   ├── Cargo.toml
│   │   └── src/lib.rs      # Exports FanucPlugin
│   └── duet/               # Application: Duet extruder
│       ├── Cargo.toml
│       └── src/lib.rs      # Exports DuetPlugin
└── simulator/              # Standalone (NOT in workspace)
    └── Cargo.toml
```

## Design Decisions

### 1. Each Plugin is a Separate Crate

**Rationale:**
- Clear dependency boundaries
- Individual versioning possible
- Easier testing in isolation
- Plugins can be published to crates.io independently

### 2. Server Assembles Plugins (No Aggregator Crate)

**Rationale:**
- Server IS the application - it decides what to load
- No extra crate to maintain
- Different deployments can load different plugin sets
- Clear ownership of the "what goes together" decision

### 3. Simulator Outside Workspace

**Rationale:**
- It's a standalone HTTP server
- No Rust dependencies on the main workspace
- Can be in any language
- Simplifies workspace dependency graph

### 4. Framework vs Application Distinction

| Crate | Type | Purpose |
|-------|------|---------|
| `plugins/core` | Framework | pl3xus networking, auth, logging |
| `plugins/robotics` | Framework | Robot-agnostic types, conversions |
| `plugins/execution` | Framework | Orchestrator, buffer, device traits |
| `plugins/fanuc` | Application | FANUC-specific driver and handlers |
| `plugins/duet` | Application | Duet-specific extruder implementation |

Framework crates are reusable across projects. Application crates are specific to this project.

## Why Not Other Approaches?

### Why not nested workspaces?

Cargo doesn't support nested workspaces. `/plugins` cannot be its own workspace inside the top-level workspace.

### Why not keep single plugins crate with features?

Pattern 2 (feature-gated single crate) works but:
- Harder to see boundaries
- All code compiles together
- Can't version plugins independently
- Feature gate complexity grows

### Why not put aggregation in a separate crate?

A `plugins/all` crate would:
- Add another crate to maintain
- Create tight coupling
- Hide the assembly logic from the application

## Dependency Graph

```
server
├── plugins/core
├── plugins/robotics
├── plugins/execution
│   └── plugins/robotics
├── plugins/fanuc
│   ├── plugins/core
│   ├── plugins/robotics
│   └── plugins/execution
└── plugins/duet
    └── plugins/execution

app
├── plugins/core (default-features = false)
└── plugins/robotics (default-features = false)
```

## Migration Path

See `IMPLEMENTATION_SPEC.md` for the detailed migration plan.

## Current Status (2025-12-28)

⚠️ **The refactor is HALF-IMPLEMENTED.** See `HANDOFF.md` for:
- What's working (source of truth)
- What's dead code (needs wiring)
- Exactly what needs to be done
- Verification checklist
