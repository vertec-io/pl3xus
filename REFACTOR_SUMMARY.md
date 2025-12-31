# UI Actions Refactor: Separated Concerns

## Problem

The execution system was returning UI action flags (can_load, can_start, can_pause, can_resume, can_stop, can_unload) from `BufferState::available_actions()`, but this created a logical conflict:

- **Execution system** doesn't track whether a program is loaded
- **Program plugin** manages program load/unload state  
- When execution system resets to `Idle` (NoSource), it would show `can_load: true` even if a program was actually loaded
- This caused the UI to show contradictory state

## Solution

Split responsibility: Each plugin owns and computes its own UI actions.

### 1. ExecutionActions (owned by Execution Plugin)

Computed by `BufferState::available_actions()` based on execution state:

```rust
pub struct ExecutionActions {
    pub can_start: bool,    // Ready, Complete, Stopped, Error
    pub can_pause: bool,    // Running, AwaitingPoints
    pub can_resume: bool,   // Paused
    pub can_stop: bool,     // Running, Paused, AwaitingPoints, Validating
}
```

**Responsibility**: Execution plugin exclusively controls what execution operations are valid based on the buffer state machine.

### 2. ProgramActions (owned by Program Plugin)

Computed by the program plugin based on its own state, in its own component:

```rust
pub struct ProgramActions {
    pub can_load: bool,     // No program loaded
    pub can_unload: bool,   // Program loaded AND execution is not active
}
```

**Responsibility**: Program plugin exclusively knows:
- Whether a program is loaded
- When it's safe to unload (observes ExecutionState.execution_actions)

## Implementation Changes

### ExecutionState Component (Execution Plugin)

Changed from individual boolean flags to just execution actions:

```rust
// Before
pub struct ExecutionState {
    pub can_load: bool,
    pub can_start: bool,
    pub can_pause: bool,
    pub can_resume: bool,
    pub can_stop: bool,
    pub can_unload: bool,
}

// After
pub struct ExecutionState {
    pub execution_actions: ExecutionActions,
}
```

### ProgramState Component (Program Plugin - NEW)

Program plugin manages its own state and actions independently:

```rust
pub struct ProgramActions {
    pub can_load: bool,
    pub can_unload: bool,
}
```

### Update Methods

Changed `ExecutionState::update_available_actions()` to `update_execution_actions()` to clarify that this updates only execution-related actions.

Program plugin has its own system to compute and update `ProgramActions` based on:
- Whether a program is loaded (program plugin's state)
- Whether execution is active (by observing ExecutionState.execution_actions)

### Files Modified

**Execution Plugin:**
1. **execution/src/components/buffer.rs**
   - Created `ExecutionActions` struct
   - Updated `BufferState::available_actions()` to return `ExecutionActions`

2. **execution/src/components/execution_state.rs**
   - Changed from individual bools to single `execution_actions: ExecutionActions` field
   - Renamed `update_available_actions()` to `update_execution_actions()`
   - Removed `set_program_actions()` (not execution's responsibility)

3. **execution/src/systems/sync.rs**
   - Updated sync system to only update `execution_actions`
   - Simplified change detection logic

4. **execution/src/handlers.rs**
   - Updated all calls from `update_available_actions()` to `update_execution_actions()`

5. **execution/src/systems/validation.rs**
   - Updated all calls from `update_available_actions()` to `update_execution_actions()`

6. **execution/src/lib.rs**
   - Updated exports: removed `ProgramActions`, kept `ExecutionActions`

**Programs Plugin:**
7. **programs/src/types.rs**
   - Created `ProgramActions` struct (owned by programs plugin)

8. **programs/src/handlers.rs**
   - Load handler: no longer tries to modify program_actions on ExecutionState
   - Removed unnecessary coupling to ExecutionState details

9. **programs/src/lib.rs**
   - Removed re-export of ProgramActions from execution

**Top-level Plugin Aggregator:**
10. **plugins/src/lib.rs**
    - Import `ExecutionActions` from execution plugin
    - Import `ProgramActions` from programs plugin
    - Removed old `UiActions` references

## Execution Flow

### Load Program
1. Program plugin loads program → creates ExecutionCoordinator + ToolpathBuffer
2. Program plugin updates ExecutionState:
   - `state = SystemState::Ready`
3. Sync system computes `execution_actions.can_start = true`
4. Program plugin maintains its own `ProgramActions: can_load = false, can_unload = true`

### Start Execution
1. Execution handler transitions BufferState to Validating
2. Sync system updates `execution_actions.can_stop = true` (can cancel validation)
3. Program plugin observes execution_actions to see if active, updates `ProgramActions.can_unload = false`

### Complete/Stop
1. Execution handler transitions BufferState to Complete/Stopped
2. Sync system updates `execution_actions.can_start = true` (can restart)
3. Program plugin observes execution_actions to see if idle, updates `ProgramActions.can_unload = true`

### Unload Program
1. Program plugin removes ExecutionCoordinator + ToolpathBuffer
2. Program plugin updates ExecutionState: `state = SystemState::NoSource`
3. Program plugin updates `ProgramActions: can_load = true, can_unload = false`

## Benefits

1. **Clear separation of concerns**: Each system manages its own actions
2. **No state conflicts**: Program state and execution state can't contradict each other
3. **Type safety**: Strong typing makes intent explicit
4. **Composability**: UI can easily combine both action sets into a complete control interface
5. **Extensibility**: Easy to add more action types in the future (e.g., StreamingActions, GeneratorActions)
