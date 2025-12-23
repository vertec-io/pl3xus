# Fanuc RMI Replica - Start Here

## Project Goal

Create an **exact replica** of the Fanuc_RMI_API web application using the pl3xus framework.
The original application is located at `/home/apino/dev/Fanuc_RMI_API/`.

## 🔥 LATEST SESSION: December 2025

**For the most up-to-date context, read: [`active/december_2025_session/START_HERE.md`](./active/december_2025_session/START_HERE.md)**

The December 2025 session focused on:
1. **TanStack Query-inspired API** (`use_query`, `use_mutation`, `use_query_keyed`)
2. **Server-side query invalidation** (server pushes, client auto-refetches)
3. **Fixed entity targeting bugs** (`ConnectionState` lives on robot entity)
4. **Migrated all client code** to new patterns

## Current Status: ~80% Complete

### What's Working
- ✅ Real-time robot state sync (position, joint angles, robot status)
- ✅ Database integration (full CRUD)
- ✅ Connection management (connect, disconnect, save connections)
- ✅ Query/mutation API with proper error handling
- ✅ Server-side query invalidation
- ✅ Exclusive control system with authorization
- ✅ Quick commands (Initialize, Reset, Abort)
- ✅ Program list and details
- ✅ Configuration management
- ✅ Toast notification system

### What Needs Work
- ⚠️ Position display uses wrong pattern (Priority 1)
- ⚠️ Some commands not yet entity-targeted
- ⚠️ Program state doesn't persist when navigating
- ⚠️ I/O panel needs display name configuration
- ⚠️ Pop-out functionality missing

See [`active/december_2024_session/OUTSTANDING_TASKS.md`](./active/december_2024_session/OUTSTANDING_TASKS.md) for full list.

## Architecture Overview

```
pl3xus Framework
├── crates/
│   ├── pl3xus/           # Main crate (re-exports)
│   ├── pl3xus_client/    # Client hooks and context
│   ├── pl3xus_common/    # Shared types
│   ├── pl3xus_sync/      # Server sync + control
│   └── pl3xus_driver/    # FANUC driver
│
└── examples/fanuc_rmi_replica/
    ├── server/           # Bevy ECS server
    ├── client/           # Leptos WASM client
    └── shared/           # Shared types (fanuc_replica_types)
```

## Key Technical Concepts

### Entity Hierarchy
```
System (ActiveSystem) ← EntityControl lives here
  └── Robot (ActiveRobot) ← ConnectionState, RobotStatus, RobotPosition live here
```

### Client Hooks
- `use_entity_component<T>(entity_id)` - Subscribe to specific entity's component
- `use_components<T>()` - Get all components of type (HashMap)
- `use_query<R>()` - Cached query with server-side invalidation
- `use_mutation<R>(callback)` - Fire-and-forget with response handler
- `use_send_targeted<M>()` - Send entity-targeted message

### Authorization
- `ExclusiveControlPlugin` handles exclusive entity control
- Messages registered with `.with_entity_policy(ExclusiveControlPolicy)` require control

## Running the Application

```bash
# Terminal 1: Start FANUC simulator (optional)
cd /path/to/fanuc_rmi_api && python -m http.server

# Terminal 2: Start server
cd examples/fanuc_rmi_replica && cargo run -p fanuc_replica_server

# Terminal 3: Start client
cd examples/fanuc_rmi_replica/client && trunk serve

# Open browser: http://localhost:8080/
```

## Files to Study First

1. `crates/pl3xus_client/src/hooks.rs` - Client hooks (use_query, use_mutation, etc.)
2. `crates/pl3xus_sync/src/control.rs` - ExclusiveControlPlugin
3. `examples/fanuc_rmi_replica/client/src/pages/dashboard/context.rs` - SystemEntityContext
4. `examples/fanuc_rmi_replica/server/src/plugins/` - Server plugins

## Research Documents

### Active Research
- **[`active/december_2024_session/`](./active/december_2024_session/)** - Most recent session (START HERE)
- **[`active/targeted_requests_authorization/`](./active/targeted_requests_authorization/)** - Authorization API
- **[`active/messages_vs_requests/`](./active/messages_vs_requests/)** - Communication patterns

### Historical Reference
- `Known_Issues.md` - Historical issues (many now fixed)
- `Architecture.md` - System architecture
- `LESSONS_LEARNED.md` - Gotchas and solutions

