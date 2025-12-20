# FANUC RMI Replica - Architecture Specification

## Purpose

This repository serves as a reference implementation for pl3xus, a real-time synchronization framework for Bevy applications. The FANUC RMI Replica application is a complete rebuild of the original FANUC RMI API web application using pl3xus:

original: https://github.com/vertec-io/Fanuc_RMI_API --> locally available at /home/apino/dev/Fanuc_RMI_API

replica: https://github.com/vertec-io/pl3xus/tree/main/examples/fanuc_rmi_replica --> locally available at /home/apino/dev/pl3xus/examples/fanuc_rmi_replica (this repo)

leptos: /home/apino/dev/leptos
leptos-use: /home/apino/dev/leptos-use

This document defines the **MANDATORY** architectural philosophy and patterns for the FANUC RMI Replica application. The goal is an **EXACT functional replica** of the original FANUC RMI API web application using the pl3xus framework (Bevy server + Leptos client).

**The original implementation is the authoritative reference.** All behavior must match the original exactly.

Fully, thoroughly, and absolutely review the pl3xus crates (pl3xus, pl3xus_sync, pl3xus_client, pl3xus_websockets, pl3xus_common, pl3xus_macros) as well as the examples (basic, fanuc, devtools-demo, control-demo) before proceeding. Also review the pl3xus documentation at /docs . Make sure you have an absolute and thorough understanding of the pl3xus framework before proceeding.

Take special care to understand this framework so that you can find anti-patterns and violations of the pl3xus framework in our curent fanuc_rmi_replica implementation. We need to fix these anti-patterns. Many of these anti-patterns are documented below, along with known issues, and a clarified architecture specification.

Before making any changes to the current code, please create a new research folder in /research/active/[topic-name] and document your analysis, proposed changes, and an implementation specification detailing a omprehensive strategy and plan for implementation.

Once you have a thorough and solid anaylsis, proposed solution, and implementation plan, please implement your solutions, document critical decisions made in a critical-decisions.md file, and then archive your research in /research/archive/[topic-name] .

Also review the other previous research attempts in /researh/active (soon to be arhived) under fanuc_replication and fanuc_rmi_complete (written by previous agents and used to achieve the current implementation). This previous research has allowed us to make good progress, but we've taken and accumulated technical debt as a result of not fully understanding the pl3xus framework, not following the core philosophy written below, and not referencing the original fanuc_rmi_api application thoroughly enough.

---

## Core Philosophy: Server-Authoritative Architecture

### The Golden Rule

> **The server is the SINGLE SOURCE OF TRUTH for ALL system state.**
> **The client is a PURE REFLECTION of server state - it does NOT own or maintain state.**

This means:

1. **ALL state lives on the server** as ECS Components
2. **The client shows server state** via `use_sync_component<T>()` hooks
3. **User actions send messages to the server** which updates state
4. **State updates automatically flow to ALL clients** via component sync

### What This Looks Like in Practice

**WRONG Pattern (Client-Owned State):**
```rust
// ❌ WRONG: Client defines and maintains its own state
#[derive(Clone)]
pub struct WorkspaceContext {
    pub program_lines: RwSignal<Vec<ProgramLine>>,      // Client-owned!
    pub executing_line: RwSignal<i32>,                   // Client-owned!
    pub loaded_program_name: RwSignal<Option<String>>,   // Client-owned!
    pub program_running: RwSignal<bool>,                 // Client-owned!
}
```

**CORRECT Pattern (Server-Owned State):**
```rust
// ✅ CORRECT: State is a synced component on the server
// In fanuc_replica_types/src/lib.rs (shared crate):
#[cfg_attr(feature = "server", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExecutionState {
    pub loaded_program_id: Option<i64>,
    pub loaded_program_name: Option<String>,
    pub running: bool,
    pub paused: bool,
    pub current_line: usize,
    pub total_lines: usize,
    pub program_lines: Vec<ProgramLineInfo>,  // Server owns the lines!
}

// In client component:
fn ProgramDisplay() -> impl IntoView {
    let execution_state = use_sync_component::<ExecutionState>();
    
    // UI is a DIRECT reflection of server state - no intermediate signals!
    view! {
        {move || {
            let states = execution_state.get();
            let state = states.values().next();
            // Render directly from server state
        }}
    }
}
```

---

## pl3xus Framework Mapping

### 1. Synced Components (`sync_component`)

**Purpose:** Server-authoritative state that all clients need to see.

**When to use:**
- Robot position, joint angles
- Connection state
- Execution state (program loaded, running, current line)
- I/O status
- Configuration state

**Server side:**
```rust
// Mark as synced during entity spawn
commands.entity(robot).insert(ExecutionState::default());
// Register the sync in the plugin
app.sync_component::<ExecutionState>();
```

**Client side:**
```rust
let state = use_sync_component::<ExecutionState>();
// `state` is a ReadSignal<HashMap<Entity, ExecutionState>>
// Automatically updates when server changes!
```

### 2. Request/Response (`RequestMessage`)

**Purpose:** Client asks server to do something and gets a response.

**When to use:**
- Database queries (ListPrograms, GetProgram)
- Operations that need confirmation (LoadProgram, CreateProgram)
- Getting data that isn't continuously synced

**Shared types:**
```rust
#[derive(Serialize, Deserialize)]
pub struct LoadProgram { pub program_id: i64 }

#[derive(Serialize, Deserialize)]
pub struct LoadProgramResponse {
    pub success: bool,
    pub error: Option<String>,
}

impl RequestMessage for LoadProgram {
    type ResponseMessage = LoadProgramResponse;
}
```

**Client side:**
```rust
let load = use_request::<LoadProgram>();
let onclick = move |_| {
    spawn_local(async move {
        let response = load.call(LoadProgram { program_id: 42 }).await;
        // Handle response
    });
};
```

### 3. Fire-and-Forget Messages

**Purpose:** Client tells server to do something, no response needed.

**When to use:**
- Jog commands
- Speed override changes
- Control actions (initialize, reset, abort)

**Client side:**
```rust
let send = use_send_message::<JogCommand>();
let jog = move |_| {
    send.send(JogCommand { axis: JogAxis::X, distance: 1.0, speed: 10.0 });
};
```

### 4. Control Request/Grant System

**Purpose:** Only one client can control the robot at a time.

**Flow:**
1. Client sends `ControlRequest::RequestControl`
2. Server grants control (or denies if another client has it)
3. Only the client with control can send robot commands
4. Control state is a synced component that ALL clients can see

---

## Type Ownership

### Types MUST be in `fanuc_replica_types` (Shared Crate)

**ALL data structures used by both server and client MUST be defined in the shared crate:**

- `ProgramLineInfo` - NOT `ProgramLine` in the client
- `CommandStatus` - NOT a client-only enum
- `ConsoleDirection`, `ConsoleMsgType` - NOT client-only types

**Current Violation Example:**
```rust
// ❌ WRONG: Client defines its own ProgramLine type
// In client/src/pages/dashboard/context.rs:
pub struct ProgramLine {
    pub line_number: usize,
    pub x: f64, pub y: f64, pub z: f64,
    // ...
}
```

**Should be:**
```rust
// ✅ CORRECT: Use the shared type everywhere
use fanuc_replica_types::ProgramLineInfo;
```

---

## Current Architecture Violations (Must Fix)

### 1. Client-Side WorkspaceContext Holds State

**Problem:** `WorkspaceContext` in `client/src/pages/dashboard/context.rs` contains:
- `program_lines: RwSignal<Vec<ProgramLine>>` - Should be from synced `ExecutionState`
- `executing_line: RwSignal<i32>` - Should be from synced `ExecutionState`
- `loaded_program_name` - Should be from synced `ExecutionState`
- `program_running`, `program_paused` - Should be from synced `ExecutionState`

**Fix:** Remove these from WorkspaceContext. Components should use `use_sync_component::<ExecutionState>()` directly.

### 2. ExecutionStateHandler Is a Band-Aid

**Problem:** `ExecutionStateHandler` copies server state to client signals:
```rust
// This is a symptom of wrong architecture!
ctx.program_running.set(state.running);
ctx.executing_line.set(state.current_line as i32);
```

**Fix:** Don't copy state. Use the synced component directly in UI components.

### 3. Duplicate Type Definitions

**Problem:** Types defined in client that exist in shared crate:
- `ProgramLine` in context.rs vs `ProgramLineInfo` in fanuc_replica_types
- `CommandStatus` in context.rs vs similar types in shared crate
- `MessageDirection`, `MessageType` in context.rs

**Fix:** Delete client types, use shared types everywhere.

### 4. Program Loading Not Persisting to All Clients

**Problem:** When one client loads a program, other clients don't see it.

**Root Cause:** The `LoadProgram` handler updates `ProgramExecutor` resource but doesn't properly sync to `ExecutionState` component, or the component sync isn't working.

**Fix:** Ensure `ExecutionState` component is updated on the robot entity, not just the resource.

---

## Correct Implementation Patterns

### Pattern 1: Program Loading Flow

**Correct Flow:**
1. Client with control clicks "Load" on program X
2. Client sends `LoadProgram { program_id: X }` request
3. Server:
   - Reads program from database
   - Updates `ExecutionState` component on robot entity with:
     - `loaded_program_id = Some(X)`
     - `loaded_program_name = Some("name")`
     - `program_lines = [...]` (from database)
     - `running = false, paused = false`
4. pl3xus automatically syncs `ExecutionState` to ALL clients
5. All clients see the loaded program in their UI

**All clients should see:** The same loaded program, same lines, same state.

### Pattern 2: Program Execution Flow

**Correct Flow:**
1. Client with control clicks "▶ Run"
2. Client sends `StartProgram { program_id: X }` request
3. Server:
   - Validates program is loaded
   - Sets `ExecutionState.running = true`
   - Begins sending instructions to robot
   - Updates `ExecutionState.current_line` as instructions complete
4. pl3xus syncs to ALL clients every frame
5. All clients see:
   - Progress bar updating
   - Current line highlighted
   - Pause/Stop buttons visible

### Pattern 3: Program Completion

**When program finishes:**
1. Server detects all instructions complete
2. Server updates `ExecutionState`:
   - `running = false`
   - `current_line = total_lines`
3. Server broadcasts a toast notification (or uses synced component for notifications)
4. All clients see:
   - "Program finished" toast
   - Buttons switch back to "Run" and "Unload"
   - Progress bar shows 100%

---

## Outstanding Issues to Fix

### Critical Issues

1. **Progress bar not visible** - UI not reading from synced `ExecutionState`
2. **Program doesn't persist** - `ExecutionState` not being properly updated/synced
3. **No "Program finished" toast** - Completion notification not implemented
4. **Buttons don't reset** - UI not reacting to `running = false`
5. **Position updates may be wrong** - Check `RobotPosition` sync

### Architecture Issues

1. **Client owns state via WorkspaceContext** - Remove state signals
2. **Duplicate types in client** - Use shared crate types
3. **ExecutionStateHandler pattern is wrong** - Don't copy state

---

## How to Fix: Step-by-Step Refactor Plan

### Step 1: Audit ExecutionState Sync

1. Verify `ExecutionState` is registered with `sync_component::<ExecutionState>()`
2. Verify the robot entity has `ExecutionState` inserted
3. Add debug logging when `ExecutionState` changes
4. Verify client receives updates via browser console

### Step 2: Fix Program Display Component

**Current (Wrong):**
```rust
fn ProgramDisplay() -> impl IntoView {
    let ctx = use_context::<WorkspaceContext>().unwrap();
    let lines = ctx.program_lines.get();  // Reading from client state!
}
```

**Fixed:**
```rust
fn ProgramDisplay() -> impl IntoView {
    let execution = use_sync_component::<ExecutionState>();

    view! {
        {move || {
            let states = execution.get();
            if let Some(state) = states.values().next() {
                // Read directly from synced server state
                let lines = &state.program_lines;
                // Render lines...
            }
        }}
    }
}
```

### Step 3: Remove WorkspaceContext State Signals

Delete these signals from `WorkspaceContext`:
- `program_lines`
- `executing_line`
- `loaded_program_name`
- `loaded_program_id`
- `program_running`
- `program_paused`

Keep only UI-local state that doesn't need to be shared:
- `show_composer` (modal visibility)
- `expanded_frames`, `expanded_tools` (accordion state)
- `selected_command_id` (dropdown selection)

### Step 4: Delete ExecutionStateHandler

It's a symptom of wrong architecture. Once components use synced state directly, this handler is unnecessary.

### Step 5: Move Types to Shared Crate

1. Delete `ProgramLine` from client, use `ProgramLineInfo`
2. Delete `CommandStatus` from client
3. Delete `MessageDirection`, `MessageType` from client, use shared versions

### Step 6: Verify Multi-Client Sync

1. Open two browser windows
2. In client A, take control and load a program
3. Client B should immediately see the program loaded
4. In client A, start execution
5. Both clients should see progress bar and current line

---

## Reference: Original Implementation

The original FANUC RMI API web application at `/home/apino/dev/Fanuc_RMI_API/` is the authoritative reference.

Key directories:
- `web_server/` - Axum server with WebSocket handlers
- `web_app/` - Leptos client
- `web_common/` - Shared types

**Important:** The original uses WebSocket JSON messages for sync. pl3xus provides this automatically through `sync_component`. The patterns should be equivalent - server owns state, clients reflect it.

---

## Testing Checklist

Before considering any feature complete, verify:

- [ ] State is defined in server ECS component
- [ ] Component is registered with `sync_component`
- [ ] Client uses `use_sync_component` to read state
- [ ] Multiple clients see same state
- [ ] State persists when navigating away and back
- [ ] Toast notifications appear at correct times
- [ ] Buttons reflect current state
- [ ] No client-owned duplicate state

---

## Summary

**DO:**
- Define state as server ECS components
- Register components with `sync_component`
- Use `use_sync_component` in client UI
- Define shared types in `fanuc_replica_types`
- Send messages/requests to modify state

**DON'T:**
- Create client-side state signals for server data
- Define duplicate types in client
- Use "handler" components to copy server state to client signals
- Assume state will persist without being on server

**The server is king. The client is a mirror.**

---

## Appendix: Known Broken Behaviors (Detailed)

This section documents specific broken behaviors that must be fixed. Compare each against the original implementation.

### A1. Program Loading

**Expected Behavior (Original):**
- Click "Load" on a program
- Program lines appear in the dashboard control panel
- Program name shows in the header
- "Run" and "Unload" buttons become visible
- If another client loads a different program, all clients see the new program

**Current Broken Behavior:**
- Program may load but not display correctly
- Navigating away and back loses the loaded program
- Other clients don't see the loaded program
- No visual confirmation of loading

### A2. Program Execution Progress

**Expected Behavior (Original):**
- Click "Run" to start execution
- Progress bar appears showing completion percentage
- Current executing line is highlighted in the program list
- Line number updates as each instruction completes
- "Pause" and "Stop" buttons replace "Run" button

**Current Broken Behavior:**
- Progress bar not visible
- No line highlighting
- Status updates not reaching UI
- Buttons don't change during execution

### A3. Program Completion

**Expected Behavior (Original):**
- When all instructions complete, "Program finished!" toast appears
- Progress bar shows 100%
- Status changes to "Complete" or "Idle"
- Buttons switch back to "Run" and "Unload"

**Current Broken Behavior:**
- No completion toast
- Buttons may not reset
- State may not update to stopped

### A4. Robot Position Display

**Expected Behavior (Original):**
- Position displays update continuously (30+ fps)
- X, Y, Z, W, P, R values change during motion
- Joint angles display shows J1-J6 values
- Values are accurate to 2-3 decimal places

**Current Broken Behavior:**
- Position updates may be incorrect
- Values may not update during motion
- Display may show stale values

### A5. Toast Notifications

**Expected Behavior (Original):**
- "Connected to Robot" on successful connection
- "Disconnected from Robot" on disconnect
- "Program loaded: [name]" on load
- "Program finished!" on completion
- Error messages when operations fail

**Current Broken Behavior:**
- Some toasts missing
- Completion toast not implemented
- Duplicate toasts may appear

### A6. Control Handoff

**Expected Behavior (Original):**
- Only client with control can execute commands
- Control can be requested and granted
- All clients see who has control
- Control can be released

**Current Status:**
- Control system implemented but may have edge cases

---

## Appendix: pl3xus API Quick Reference

### Server-Side (Bevy)

```rust
// Register a synced component type
app.sync_component::<MyComponent>();

// Insert synced component on entity
commands.entity(entity).insert(MyComponent { ... });

// Modify synced component (triggers automatic sync)
query.single_mut().value = new_value;

// Handle network message
app.add_systems(Update, handle_message::<MyMessage>(my_handler));

// Handle request/response
app.add_systems(Update, handle_request::<MyRequest, MyResponse>(my_handler));
```

### Client-Side (Leptos)

```rust
// Subscribe to synced component
let state = use_sync_component::<MyComponent>();
// Returns ReadSignal<HashMap<Entity, MyComponent>>

// Send fire-and-forget message
let send = use_send_message::<MyMessage>();
send.send(MyMessage { ... });

// Send request, get response
let request = use_request::<MyRequest>();
spawn_local(async move {
    let response = request.call(MyRequest { ... }).await;
    // response is Result<MyResponse, ...>
});

// Check control status
let has_control = use_has_control();
// Returns Memo<bool>
```

---

## Appendix: Files to Audit/Modify

### Server Files

| File | Issue | Action |
|------|-------|--------|
| `server/src/plugins/connection.rs` | ExecutionState insertion | Verify robot entity has ExecutionState |
| `server/src/plugins/execution.rs` | State sync | Ensure ExecutionState component updates |
| `server/src/plugins/requests.rs` | LoadProgram handler | Verify program_lines sync to ExecutionState |
| `server/src/main.rs` | sync_component registration | Verify ExecutionState registered |

### Client Files

| File | Issue | Action |
|------|-------|--------|
| `client/src/pages/dashboard/context.rs` | Duplicate state | Remove server-owned signals |
| `client/src/pages/dashboard/control/program_display.rs` | Reading from context | Use use_sync_component |
| `client/src/layout/top_bar.rs` | ExecutionStateHandler | Remove after refactor |
| `client/src/layout/mod.rs` | WorkspaceContext provider | Simplify or remove |

### Shared Files

| File | Issue | Action |
|------|-------|--------|
| `fanuc_replica_types/src/lib.rs` | ExecutionState fields | Verify all needed fields present |

---

**END OF SPECIFICATION**

