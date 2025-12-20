# Architecture Refactor Implementation Plan

## Overview

This document outlines the implementation plan for refactoring the fanuc_rmi_replica client to follow the server-authoritative architecture specified in `ARCHITECTURE_SPECIFICATION.md`.

## Core Principle

**The server is the SINGLE SOURCE OF TRUTH for ALL state. The client is a PURE REFLECTION of server state.**

## Current Violations Identified

### 1. WorkspaceContext Contains Server-Owned State (CRITICAL)

**File:** `client/src/pages/dashboard/context.rs`

The `WorkspaceContext` struct contains signals that duplicate server state:

```rust
// WRONG - These are server-owned state duplicated in client
pub program_lines: RwSignal<Vec<ProgramLine>>,      // Should come from ExecutionState
pub executing_line: RwSignal<i32>,                   // Should come from ExecutionState  
pub loaded_program_name: RwSignal<Option<String>>,   // Should come from ExecutionState
pub loaded_program_id: RwSignal<Option<i64>>,        // Should come from ExecutionState
pub program_running: RwSignal<bool>,                 // Should come from ExecutionState
pub program_paused: RwSignal<bool>,                  // Should come from ExecutionState
```

**Fix:** Remove these signals. UI components should use `use_sync_component::<ExecutionState>()` directly.

### 2. ExecutionStateHandler Anti-Pattern (CRITICAL)

**File:** `client/src/layout/top_bar.rs` (lines 905-957)

This component copies server state to client signals - a band-aid pattern that breaks multi-client sync:

```rust
// WRONG - Handler copying state
Effect::new(move |_| {
    if let Some(state) = state_map.values().next() {
        ctx.program_running.set(state.running);
        ctx.executing_line.set(state.current_line as i32);
        // ...
    }
});
```

**Fix:** Delete this component entirely. UI reads synced state directly.

### 3. Duplicate Type Definitions (MODERATE)

**File:** `client/src/pages/dashboard/context.rs`

The client defines `ProgramLine` which duplicates `ProgramLineInfo` from shared crate:

```rust
// WRONG - Duplicate type in client
#[derive(Clone, Debug, PartialEq)]
pub struct ProgramLine {
    pub x: f64, pub y: f64, pub z: f64, ...
}
```

**Fix:** Use `ProgramLineInfo` from `fanuc_replica_types` everywhere.

### 4. ProgramVisualDisplay Reads from Context (MODERATE)

**File:** `client/src/pages/dashboard/control/program_display.rs`

```rust
// WRONG - Reading from client context
let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
let lines = ctx.program_lines;
let executing = ctx.executing_line;
```

**Fix:** Use `use_sync_component::<ExecutionState>()` directly.

## Implementation Steps

### Step 1: Refactor WorkspaceContext

1. Remove server-owned signals from `WorkspaceContext`:
   - `program_lines`
   - `executing_line`
   - `loaded_program_name`
   - `loaded_program_id`
   - `program_running`
   - `program_paused`

2. Keep UI-local state:
   - `show_program_modal`
   - `show_settings_modal`
   - `show_io_modal`
   - `show_frame_tool_modal`
   - `accordion_states`
   - `dropdown_states`
   - `selected_program_id`
   - `selected_program_name`

### Step 2: Refactor ProgramVisualDisplay

1. Replace context usage with direct synced component access:
   ```rust
   let execution_state = use_sync_component::<ExecutionState>();
   ```

2. Derive program lines and current line from `ExecutionState`:
   ```rust
   let program_lines = move || {
       execution_state.get().values().next()
           .map(|s| s.program_lines.clone())
           .unwrap_or_default()
   };
   ```

### Step 3: Delete ExecutionStateHandler

Remove the entire `ExecutionStateHandler` component from `top_bar.rs`.

### Step 4: Update All Components Using Removed Signals

Search for and update all components that reference the removed signals:
- Progress bar components
- Control buttons (Start/Pause/Stop)
- Program display components
- Status indicators

### Step 5: Remove Duplicate Types

1. Remove `ProgramLine` from `context.rs`
2. Update imports to use `ProgramLineInfo` from `fanuc_replica_types`

## Correct Pattern Reference

### Reading Synced State in UI

```rust
use pl3xus_client::use_sync_component;
use fanuc_replica_types::ExecutionState;

#[component]
fn ProgramDisplay() -> impl IntoView {
    let execution_state = use_sync_component::<ExecutionState>();
    
    view! {
        {move || {
            let state_map = execution_state.get();
            if let Some(state) = state_map.values().next() {
                view! {
                    <div class="program-name">{state.loaded_program_name.clone()}</div>
                    <div class="current-line">{state.current_line}</div>
                    <div class="total-lines">{state.total_lines}</div>
                }
            } else {
                view! { <div>"No program loaded"</div> }
            }
        }}
    }
}
```

## Testing Checklist

- [ ] Program loading syncs to all connected clients
- [ ] Execution progress (current_line) updates on all clients
- [ ] Program completion is visible on all clients
- [ ] Pause/Resume state syncs to all clients
- [ ] Unload program clears state on all clients

