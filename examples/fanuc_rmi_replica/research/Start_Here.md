# Fanuc RMI Replica - Start Here

## Project Goal

Create an **exact replica** of the Fanuc_RMI_API web application using the pl3xus framework.
The original application is located at `/home/apino/dev/Fanuc_RMI_API/`.

## Current Status: ~85% Complete

The application is mostly functional with:
- ✅ Real-time robot state sync working (position, joint angles, robot status)
- ✅ Database integration for robot connections and programs
- ✅ Dashboard with Control and Configuration tabs
- ✅ Settings page with robot management
- ✅ Programs page with CRUD operations
- ✅ Toast notification system
- ✅ I/O panel with 6 tabs (DIN/DOUT/AIN/AOUT/GIN/GOUT)
- ✅ Jog controls (joint + cartesian with W/P/R rotation)
- ⚠️ Control system has reactive graph panic when processing ControlResponse

## Critical Issue Being Debugged

**Symptom:** Client panics with `reactive_graph... RuntimeError: unreachable` when clicking "REQUEST CONTROL" button.

**Root Cause Identified:** The `handle_incoming_message` function in `crates/pl3xus_client/src/context.rs` is called from inside an Effect and performs reactive reads/writes that cause the reactive graph to panic.

**Latest Fix Applied:**
Changed `handle_incoming_message` to use `get_untracked()` for reading and `update_untracked()` + `notify()` for writing (lines 268-281 in context.rs).

**Status:** Fix just applied, needs testing.

## Architecture Overview

```
pl3xus Framework
├── crates/
│   ├── pl3xus/           # Bevy server-side sync framework
│   ├── pl3xus_client/    # Leptos client-side sync framework  
│   ├── pl3xus_common/    # Shared types (NetworkPacket, ControlRequest/Response)
│   └── pl3xus_sync/      # Entity sync + ExclusiveControlPlugin
│
└── examples/fanuc_rmi_replica/
    ├── server/           # Bevy app with FANUC driver
    ├── client/           # Leptos WASM app
    └── types/            # Shared types between server/client
```

## Key Technical Concepts

### Sync Components
- Server syncs components to clients using `SyncComponent` trait
- Client receives via `use_sync_component::<T>()` hook returning `ReadSignal<HashMap<u64, T>>`
- Entity bits key: `4294967295` (0xFFFFFFFF) is the robot entity

### Exclusive Control System
- `ExclusiveControlPlugin` handles control requests
- `ControlRequest::Take/Release` messages from client
- `ControlResponse::Granted/Denied/Released` back to client
- `EntityControl` component tracks which client_id has control

### Request/Response Pattern
- `use_request::<R>()` hook for database operations
- Server handles with `RequestHandlerPlugin`
- Examples: ListRobotConnections, CreateProgram, GetFrameData

## Running the Application

```bash
# Terminal 1: Start FANUC simulator
cd /home/apino/dev/Fanuc_RMI_API && ./target/release/sim

# Terminal 2: Start server
cd /home/apino/dev/pl3xus && ./target/release/fanuc_replica_server

# Terminal 3: Start client
cd /home/apino/dev/pl3xus/examples/fanuc_rmi_replica/client && trunk serve --port 8084

# Open browser: http://127.0.0.1:8084/
```

## Files to Study First

1. `crates/pl3xus_client/src/context.rs` - Client sync context (current bug location)
2. `crates/pl3xus_client/src/provider.rs` - WebSocket message handler
3. `examples/fanuc_rmi_replica/client/src/layout/top_bar.rs` - Control button handler
4. `examples/fanuc_rmi_replica/server/src/main.rs` - Server setup
5. `crates/pl3xus_sync/src/control.rs` - ExclusiveControlPlugin

## Reference Documentation

See other files in this research folder:
- `Task_Status.md` - All 127 tasks with status
- `Current_Issue.md` - Detailed description of current bug
- `Related_Repos.md` - Reference repositories and patterns
- `LESSONS_LEARNED.md` - Patterns to avoid in Leptos

