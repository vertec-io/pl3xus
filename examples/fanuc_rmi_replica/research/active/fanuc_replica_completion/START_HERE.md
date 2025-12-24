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

### Priority 1: Code Quality - ✅ COMPLETE

| Task | Status | Notes |
|------|--------|-------|
| Unused variable warnings | ✅ DONE | Fixed/removed unused variables |
| Dead code cleanup | ✅ DONE | Removed obsolete `execution.rs` (legacy `ProgramExecutor` replaced by orchestrator pattern) |

### Priority 2: API Consistency (Multi-Robot Support) - ✅ COMPLETE

| Task | Status | Notes |
|------|--------|-------|
| Convert robot commands to targeted | ✅ DONE | `SetSpeedOverride`, `InitializeRobot`, `AbortMotion`, `ResetRobot` use `AuthorizedRequest<T>` |
| Add targeting to program commands | ✅ DONE | `StartProgram`, `PauseProgram`, `ResumeProgram`, `StopProgram`, `LoadProgram`, `UnloadProgram` |
| Review `ConnectToRobot` | ✅ REVIEWED | Works correctly with manual control check. Pre-spawning from DB deferred as future enhancement |
| Review `ControlRequest` pattern | ✅ REVIEWED | Works correctly with embedded entity_bits. Simpler than TargetedMessage wrapper |

### Priority 3: UX Improvements - ✅ COMPLETE

| Task | Status | Description |
|------|--------|-------------|
| Program state persistence | ✅ DONE | Program stays open when navigating away and back |

### Priority 4: Future Enhancements - ✅ COMPLETE

| Task | Status | Notes |
|------|--------|-------|
| Check JogDefaultStep units | ✅ DONE | Fixed labels and defaults (see below) |
| I/O display name configuration | ✅ DONE | Full UI in settings.rs (see below) |
| Server-side missing subscription warnings | ✅ DONE | Implemented in pl3xus_sync (see below) |

#### JogDefaultStep Units (Completed)
- Fixed `robot_wizard.rs`: Changed joint jog speed label from `"Speed (%)"` to `"Speed (°/s)"`
- Updated defaults: joint_jog_speed=10.0 °/s, joint_jog_step=1.0° (was 0.1 and 0.25)
- Updated database schema defaults with unit comments
- Updated `JogSettingsState::default()` with unit comments

#### I/O Display Name Configuration (Completed)
- **Client UI** (`settings.rs` - RobotSettingsPanel):
  - Added "I/O Display Names" section with "Configure" button
  - New `IoConfigModal` component with tabbed interface for all 6 I/O types (DIN, DOUT, AIN, AOUT, GIN, GOUT)
  - `IoConfigRow` component for each port with display name input + visibility toggle
  - Uses `use_query_keyed::<GetIoConfig, _>()` to load config when robot changes
  - Uses `use_mutation::<UpdateIoConfig>()` to save changes
- **Server** (`requests.rs`):
  - `handle_update_io_config` updates `IoConfigState` component after database write
  - Changes are automatically synced to all subscribed clients
- **Connection flow** (`connection.rs`):
  - IoConfigState is loaded from database when connecting via saved connection
  - Uses `db.get_io_config(conn_id)` instead of `IoConfigState::default()`

#### Server-side Missing Subscription Warnings (Completed)
- **Location**: `crates/pl3xus_sync/src/systems.rs` in `process_snapshot_queue()`
- **Warnings logged**:
  - When client subscribes to unregistered component type: `"component type 'X' is not registered for sync"`
  - When client subscribes to specific entity that doesn't exist or lacks the component: `"entity {:?} does not exist or does not have component 'X'"`
- **Note**: Wildcard entity subscriptions (entity=None) don't warn on empty results since that's normal (no entities with that component exist yet)

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



