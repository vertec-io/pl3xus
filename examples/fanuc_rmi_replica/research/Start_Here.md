# Fanuc RMI Replica - Start Here

## Project Goal

Create an **exact replica** of the Fanuc_RMI_API web application using the pl3xus framework.
The original application is located at `/home/apino/dev/Fanuc_RMI_API/`.

## ⚠️ CRITICAL: Read Known_Issues.md First

**The previous agent's assessment of "~85% complete" was OVERLY OPTIMISTIC.**

There are **16 major unresolved issues** documented in `Known_Issues.md`. Many core features are broken or incomplete:
- Program loading/editing is completely broken
- Robot connection editing has infinite loops
- Joint jogging sends no data to robot
- Control system doesn't indicate who has control
- Quick command buttons do nothing
- Configuration doesn't load properly

**READ `Known_Issues.md` IMMEDIATELY** - it contains the user's actual assessment of what's broken.

## Current Status: ~60% Complete (Realistic Assessment)

What's actually working:
- ✅ Real-time robot state sync (position, joint angles, robot status)
- ✅ Database integration (basic CRUD)
- ✅ UI layout roughly matches original
- ✅ Toast notification system (wrong position)

What's broken or incomplete:
- ❌ Program loading/editing completely broken (Issue #3, #11)
- ❌ Robot connection editing broken with infinite loops (Issue #12)
- ❌ Control system incomplete - no feedback, no disconnect release (Issue #15)
- ❌ Joint jogging non-functional (Issue #14)
- ❌ Quick command buttons do nothing (Issue #13)
- ❌ Configuration tab has multiple issues (Issue #6, #10)
- ❌ I/O panel not exact replica (Issue #2)
- ❌ Number inputs used instead of text inputs (Issue #1)
- ❌ Pop-out functionality missing (Issue #8)
- ❌ Command composer doesn't run commands (Issue #5)

## Critical Issue Being Debugged

**Symptom:** Client panics with `reactive_graph... RuntimeError: unreachable` when clicking "REQUEST CONTROL" button.

**Root Cause Identified:** The `handle_incoming_message` function in `crates/pl3xus_client/src/context.rs` is called from inside an Effect and performs reactive reads/writes that cause the reactive graph to panic.

**Latest Fix Applied:**
Changed `handle_incoming_message` to use `get_untracked()` for reading and `update_untracked()` + `notify()` for writing (lines 268-281 in context.rs).

**Status:** Fix just applied, needs testing. Even if this works, Issue #15 has additional control problems.

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
- **`Known_Issues.md`** - ⚠️ 16 CRITICAL ISSUES - READ FIRST
- `Task_Status.md` - All 127 tasks with status
- `Current_Issue.md` - Detailed description of reactive graph bug
- `Related_Repos.md` - Reference repositories and patterns
- `Feature_Comparison.md` - Original vs replica feature matrix
- `Architecture.md` - System architecture and data flows
- `LESSONS_LEARNED.md` - Patterns to avoid in Leptos

## Priority Order for Fixes

1. **CRITICAL** - Program loading/editing (Issues #3, #11) - Core functionality broken
2. **CRITICAL** - Robot connection editing (Issue #12) - Infinite loops, data not saving
3. **CRITICAL** - Control system (Issue #15) - No feedback, no disconnect release
4. **HIGH** - Joint jogging (Issue #14) - Sends no data
5. **HIGH** - Quick commands (Issue #13) - Buttons do nothing
6. **HIGH** - Configuration (Issues #6, #10) - Doesn't load, server overrides input
7. **HIGH** - Number inputs (Issue #1) - App-wide, replace with text inputs
8. **HIGH** - I/O panel (Issue #2) - Not exact replica
9. **MEDIUM** - Command composer (Issue #5) - Doesn't run commands
10. **MEDIUM** - Sidebar (Issue #7) - Missing Uframe/Utool
11. **MEDIUM** - Pop-out (Issue #8) - Missing functionality
12. **MEDIUM** - Console (Issue #4) - Missing message types
13. **LOW** - Toast position (Issue #16) - Wrong location

