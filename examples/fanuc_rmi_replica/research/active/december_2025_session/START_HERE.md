# December 2025 Session: START HERE

## Quick Context

This document provides orientation for an AI agent continuing work on the **pl3xus framework** - a Bevy ECS server + Leptos WASM client state synchronization framework for industrial robotics applications.

**Date**: December 23, 2025

## What is pl3xus?

pl3xus is a framework for building **server-authoritative, real-time synchronized** web applications. Key characteristics:

- **Server**: Bevy ECS (Entity Component System) with plugins
- **Client**: Leptos (reactive Rust compiled to WASM)
- **Transport**: WebSocket with binary serialization
- **Sync**: Components are automatically synchronized from server → clients
- **Example Application**: `fanuc_rmi_replica` - a robot control interface

## Repository Structure

```
/home/apino/dev/pl3xus/
├── crates/
│   ├── pl3xus/                    # Main crate (re-exports)
│   ├── pl3xus_client/             # Client-side hooks and context
│   ├── pl3xus_common/             # Shared types (messages, serialization)
│   ├── pl3xus_sync/               # Server-side sync, control, registration
│   └── pl3xus_driver/             # FANUC robot driver
├── examples/
│   └── fanuc_rmi_replica/         # Primary example application
│       ├── client/                # Leptos WASM client
│       ├── server/                # Bevy ECS server
│       └── shared/fanuc_replica_types/  # Shared message/component types
└── docs/
    └── research/                  # API research documents
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

## What Was Accomplished This Session

See `ACCOMPLISHMENTS.md` for detailed list. Key highlights:

1. **Implemented TanStack Query-inspired API** (`use_query`, `use_mutation`, `use_query_keyed`)
2. **Server-side query invalidation** (server pushes invalidation, client auto-refetches)
3. **Fixed message batching bug** (multiple messages in one WebSocket frame)
4. **Fixed entity targeting** (`ConnectionState` lives on robot entity, not system)
5. **Implemented `use_request` pattern** for `ConnectToRobot` with proper state handling

## Outstanding Tasks

See `OUTSTANDING_TASKS.md` for detailed list. Priority items:

1. **Position display fix** - Uses `use_components` instead of `use_entity_component`
2. **Convert commands to targeted requests** - `SetSpeedOverride`, `InitializeRobot`, etc.
3. **Program state persistence** - Remember open program when navigating away
4. **Server-side notifications** - Send warnings when subscriptions fail

## Next Steps

See `NEXT_STEPS.md` for recommended approach.

## Key Concepts to Remember

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
When subscribing to robot components, always handle the case where robot doesn't exist:
```rust
let (connection_state, robot_exists) = use_entity_component::<ConnectionState, _>(
    move || system_ctx.robot_entity_id.get()
);
let robot_connected = Memo::new(move |_| 
    robot_exists.get() && connection_state.get().robot_connected
);
```



