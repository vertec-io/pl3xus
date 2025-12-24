# Fanuc RMI Replica - Completion Handoff

**START HERE** - Master handoff document for completing all outstanding tasks.

**Last Updated**: December 24, 2025

## Quick Context

The **fanuc_rmi_replica** is a reference implementation demonstrating the pl3xus framework (Bevy ECS server + Leptos WASM client). It replicates the original Fanuc_RMI_API web application functionality.

### Framework Core Principles
1. **Server-authoritative** - Server is the source of truth
2. **Eliminate boilerplate** - Convenience over configuration
3. **Best-in-class DX** - TanStack Query-inspired patterns

### Key Technical Patterns
- `use_entity_component` - Subscribe to a specific entity's component
- `use_mut_component` - Read/write component with Apply/Cancel
- `use_query` / `use_query_keyed` - Cached queries with server invalidation
- `use_mutation` / `use_mutation_targeted` - Fire-and-forget mutations
- `app.request::<T, WS>().register()` - Server request registration
- `respond_and_invalidate` - Respond + auto-invalidate in one line

## Repository Structure

```
/home/apino/dev/pl3xus/
├── crates/
│   ├── pl3xus/                    # Main crate (re-exports)
│   ├── pl3xus_client/             # Client-side hooks and context
│   ├── pl3xus_common/             # Shared types (messages, serialization)
│   ├── pl3xus_sync/               # Server-side sync, control, registration
│   └── pl3xus_macros/             # Derive macros (Invalidates, HasSuccess)
├── examples/
│   └── fanuc_rmi_replica/         # Primary example application
│       ├── client/                # Leptos WASM client
│       ├── server/                # Bevy ECS server
│       └── research/active/       # Research documents
└── examples/shared/
    └── fanuc_replica_types/       # Shared message/component types
```

## Key Files to Understand

### Framework (pl3xus)

| File | Purpose |
|------|---------|
| `crates/pl3xus_client/src/hooks.rs` | Client hooks: `use_query`, `use_mutation`, `use_entity_component`, `use_request` |
| `crates/pl3xus_sync/src/control.rs` | `ExclusiveControlPlugin`, entity control, authorization |
| `crates/pl3xus_sync/src/registration.rs` | Message/request registration builders |
| `crates/pl3xus_sync/src/messages.rs` | Sync message types |

### Example Application (fanuc_rmi_replica)

| File | Purpose |
|------|---------|
| `examples/fanuc_rmi_replica/server/src/main.rs` | Server setup and plugin registration |
| `examples/fanuc_rmi_replica/server/src/plugins/` | Server plugins (connection, requests, sync, etc.) |
| `examples/fanuc_rmi_replica/client/src/app.rs` | Client setup and component registration |
| `examples/fanuc_rmi_replica/client/src/layout/top_bar.rs` | Connection UI, quick settings |
| `examples/fanuc_rmi_replica/client/src/pages/dashboard/context.rs` | `SystemEntityContext` for entity IDs |

## Related Research Documents

| Document | Location | Purpose |
|----------|----------|---------|
| Query API Research | `docs/research/QUERY_API_RESEARCH.md` | TanStack Query-inspired API design |
| Targeted Authorization | `research/active/targeted_requests_authorization/` | Entity-targeted message authorization |
| Messages vs Requests | `research/active/messages_vs_requests/` | Taxonomy of communication patterns |
| Architecture Spec | `ARCHITECTURE_SPECIFICATION.md` | Server-authoritative design principles |

## How to Run

```bash
# Terminal 1: Start the FANUC simulator
cd /home/apino/dev/Fanuc_RMI_API && cargo run -p sim -- --realtime

# Terminal 2: Start the server
cd examples/fanuc_rmi_replica/server && cargo run

# Terminal 3: Build and serve the client
cd examples/fanuc_rmi_replica/client && trunk serve
```

## Outstanding Tasks

### Priority 1: Code Quality (No Blockers) - ~20 min

| Task | File | Fix |
|------|------|-----|
| ⚠️ Unused variable warning | `client/src/pages/dashboard/control/program_display.rs:56` | Prefix with `_` or remove |
| ⚠️ Dead code: `LoadedProgramData` fields | `server/src/plugins/execution.rs` | Mark `#[allow(dead_code)]` or implement |
| ⚠️ Dead code: `Program::all_completed()` | `server/src/plugins/program.rs:144` | Mark or remove |
| ⚠️ Dead code: `ExecutionBuffer::available_slots()` | `server/src/plugins/program.rs:181` | Mark or remove |

### Priority 2: API Consistency (Multi-Robot Support) - ~4 hr

These enhance consistency for multi-robot scenarios but work correctly now.

| Task | Effort | Description |
|------|--------|-------------|
| Convert commands to targeted requests | 1 hr | `SetSpeedOverride`, `InitializeRobot`, `AbortMotion`, `ResetRobot` in `quick_commands.rs` |
| Add targeting to program commands | 1 hr | `StartProgram`, `PauseProgram`, `ResumeProgram`, `StopProgram`, `LoadProgram` |
| Make `ConnectToRobot` targeted | 1 hr | Requires robot entities to exist before connection (from DB) |
| Review `ControlRequest` pattern | 30 min | `ControlRequest` embeds `entity_bits` - consider `TargetedMessage` pattern |

### Priority 3: UX Improvements - ✅ COMPLETE

| Task | Status | Description |
|------|--------|-------------|
| Program state persistence | ✅ DONE | Program stays open when navigating away and back |

### Priority 4: Future Enhancements - ~6 hr

| Task | Effort | Description |
|------|--------|-------------|
| Server-side missing subscription warnings | 2 hr | Debug aid: warn when client subscribes to non-existent entity/component |
| I/O display name configuration | 3 hr | Custom display names in robot connection settings |
| Check JogDefaultStep units | 30 min | Verify if joint jog speed should be °/s or % by checking original Fanuc_RMI_API |

---

## Completed Items ✅

### Automatic Query Invalidation (Phase 3) - COMPLETE
- `HasSuccess` trait + `#[derive(HasSuccess)]` macro
- `RequestInvalidateExt` extension trait with `respond_and_invalidate()`
- All 11 mutation handlers migrated to one-line invalidation

### Component Mutation Handlers - COMPLETE
- `use_mut_component` hook implemented in pl3xus_client
- `JogSettingsState` fully migrated (reference implementation)

### API Migration - COMPLETE
- All `listen_for_request_message` → `app.request::<T, WS>().register()`
- All 28+ requests using new batch registration API

### Entity Architecture - COMPLETE
- `system_entity_id` and `robot_entity_id` properly distinguished
- All 14+ client files subscribe to correct entity

---

## Key Concepts

### Entity Hierarchy
```
System (ActiveSystem) ← EntityControl lives here
  └── Robot (ActiveRobot) ← ConnectionState, RobotStatus, etc. live here
```

### Hook Usage Patterns
- **Real-time state**: `use_entity_component::<ConnectionState, _>(|| robot_entity_id.get())`
- **Cached queries**: `use_query::<ListPrograms>()` or `use_query_keyed::<GetProgram, _>(|| Some(...))`
- **Mutations**: `use_mutation::<CreateProgram>(callback)`
- **Fire-and-forget**: `use_request::<GetFrameData>()` (for imperative triggers)

### The `robot_exists` Pattern
```rust
let (connection_state, robot_exists) = use_entity_component::<ConnectionState, _>(
    move || system_ctx.robot_entity_id.get()
);
let robot_connected = Memo::new(move |_|
    robot_exists.get() && connection_state.get().robot_connected
);
```

---

## Next Agent Instructions

1. **Read this document first** - Understand the framework and patterns
2. **Start with Priority 1** - Quick wins, ~20 min total
3. **Move to Priority 2** if multi-robot consistency is desired
4. **Test after each change** - `cargo check --package fanuc_replica_server --package fanuc_replica_client`
5. **Mark tasks complete** as you go
6. **Update this document** with any new findings
```



