# Execution State Machine

Note: ExecutionState IS ProgramState. They are unified in the execution plugin.
There is no separate "program state" - a loaded program is just a LoadedProgram
component on the System entity, and execution state tracks everything.

## State Diagram

```
                    ┌──────────────┐
                    │   NoProgram  │◄──────────────────────────┐
                    └──────┬───────┘                           │
                           │ LoadProgram                       │ UnloadProgram
                           ▼                                   │
                    ┌──────────────┐                           │
         ┌─────────►│     Idle     │───────────────────────────┤
         │          └──────┬───────┘                           │
         │                 │ StartProgram                      │
         │                 ▼                                   │
         │          ┌──────────────┐                           │
         │          │  Validating  │◄──────┐                   │
         │          │ (30s timeout)│       │                   │
         │          └──────┬───────┘       │                   │
         │                 │               │ ResumeProgram     │
         │     ┌───────────┼───────────┐   │                   │
         │     │           │           │   │                   │
         │     ▼           ▼           ▼   │                   │
         │  ┌─────┐  ┌──────────┐  ┌───────┴───┐               │
         │  │Error│  │Executing │  │  Paused   │               │
         │  └──┬──┘  └────┬─────┘  └─────┬─────┘               │
         │     │          │              │                     │
         │     │          │ Complete     │ StopProgram         │
         │     │          ▼              │                     │
         │     │   ┌──────────────┐      │                     │
         │     │   │  Completed   │      │                     │
         │     │   └──────┬───────┘      │                     │
         │     │          │              │                     │
         └─────┴──────────┴──────────────┘
                     (all return to Idle or can Unload)
```

User can cancel validation at any time via StopProgram (which is always available
during Validating state).

## Transition Rules

### NoProgram → Idle
- **Trigger**: `LoadProgram` handler (programs plugin)
- **Action**: Insert `LoadedProgram` component on System entity
- **Subsystem**: Programs plugin sets `Ready`

### Idle → Validating
- **Trigger**: `StartProgram` handler (execution plugin)
- **Action**: Set `BufferState::Validating`
- **Effect**: All subsystems start readiness checks

### Validating → Executing
- **Trigger**: Execution plugin's validation system
- **Condition**: `Subsystems::all_ready() == true`
- **Action**: 
  - Set `BufferState::Executing`
  - Start sending points to motion system

### Validating → Error
- **Trigger**: Execution plugin's validation system
- **Condition**: `Subsystems::first_error().is_some()`
- **Action**: Set `BufferState::Error { message }`

### Executing → Paused
- **Trigger**: `PauseProgram` handler (execution plugin)
- **Action**: Set `BufferState::Paused { paused_at_index }`

### Paused → Validating (Resume Validation)
- **Trigger**: `ResumeProgram` handler (execution plugin)
- **Action**: Set `BufferState::Validating` (preserve current index)
- **Rationale**: Re-validates all subsystems before resuming to ensure:
  - Robot is still connected
  - Emergency stop hasn't been triggered
  - No errors occurred during pause
  - All safety conditions are still met
- **Index Preservation**: The `paused_at_index` is stored and used when
  transitioning from Validating → Executing after resume

**Why not Paused → Executing directly?**

While pause was active, conditions may have changed:
1. Robot may have been disconnected
2. Estop may have been triggered
3. Program source may have been modified
4. Safety zones may have changed

Re-validating before resuming ensures we don't send motion commands
to a system that's no longer ready to receive them.

### Executing → Completed
- **Trigger**: Motion feedback indicates all points executed
- **Action**: Set `BufferState::Complete { total_executed }`

### Any → Idle (via Stop)
- **Trigger**: `StopProgram` handler (execution plugin)
- **Action**: 
  - Set `BufferState::Stopped` then transition to `Idle`
  - Clear motion buffers
  - Keep program loaded

### Any → NoProgram (via Unload)
- **Trigger**: `UnloadProgram` handler (programs plugin)
- **Condition**: Not in Executing or Validating state
- **Action**: 
  - Remove `LoadedProgram` component
  - Set `BufferState::Idle` (or NoProgram state)

## Resume Validation Index Preservation

When resuming from pause, we need to preserve the execution index while
going through validation. This requires storing the resume index separately:

### Option 1: Validating Variant with Resume Index

```rust
pub enum BufferState {
    // ...
    Validating,
    ValidatingForResume { resume_from_index: u32 },
    // ...
}
```

The validation coordinator checks which variant we're in:
- `Validating` → on success, start from index 0
- `ValidatingForResume { resume_from_index }` → on success, start from `resume_from_index`

### Option 2: Separate Resource

```rust
#[derive(Resource, Default)]
pub struct ResumeContext {
    /// If Some, we're resuming from this index after validation
    pub resume_from_index: Option<u32>,
}
```

The resume handler sets this resource, and the validation coordinator
reads it when transitioning to Executing.

### Recommended: Option 1

Option 1 is cleaner because:
- The state is self-contained
- No separate resource to manage
- Easier to reason about state transitions

### Implementation in Handler

```rust
pub fn handle_resume(
    mut buffer_state: Query<&mut BufferState, With<ActiveSystem>>,
    mut subsystems: Query<&mut Subsystems, With<ActiveSystem>>,
    // ...
) {
    let paused_at = match *buffer_state {
        BufferState::Paused { paused_at_index } => paused_at_index,
        _ => return, // Not paused
    };

    // Reset subsystems for re-validation
    subsystems.reset_all();

    // Transition to ValidatingForResume, preserving the index
    *buffer_state = BufferState::ValidatingForResume {
        resume_from_index: paused_at
    };
}
```

### Validation Coordinator Update

```rust
fn coordinate_validation(
    mut buffer_state: Query<&mut BufferState, With<ActiveSystem>>,
    subsystems: Query<&Subsystems, With<ActiveSystem>>,
) {
    // ...

    if subs.all_ready() {
        let start_index = match *state {
            BufferState::Validating => 0,
            BufferState::ValidatingForResume { resume_from_index } => resume_from_index,
            _ => return,
        };

        *state = BufferState::Executing {
            current_index: start_index,
            completed_count: start_index, // Points before resume are "complete"
        };
    }
}
```

## Subsystem Validation Behaviors

### Programs Plugin
```rust
fn validate_program_subsystem(
    buffer_state: Query<&BufferState, With<ActiveSystem>>,
    loaded_program: Query<&LoadedProgram, With<ActiveSystem>>,
    mut subsystems: Query<&mut Subsystems, With<ActiveSystem>>,
) {
    let Ok(state) = buffer_state.single() else { return };
    if !matches!(state, BufferState::Validating) { return }
    
    let mut subs = subsystems.single_mut().unwrap();
    
    if loaded_program.single().is_ok() {
        subs.set_readiness(SUBSYSTEM_PROGRAMS, SubsystemReadiness::Ready);
    } else {
        subs.set_readiness(SUBSYSTEM_PROGRAMS, 
            SubsystemReadiness::Error("No program loaded".into()));
    }
}
```

### Fanuc Plugin
```rust
fn validate_fanuc_subsystem(
    buffer_state: Query<&BufferState, With<ActiveSystem>>,
    robot: Query<&FanucConnectionStatus, With<FanucRobot>>,
    mut subsystems: Query<&mut Subsystems, With<ActiveSystem>>,
) {
    let Ok(state) = buffer_state.single() else { return };
    if !matches!(state, BufferState::Validating) { return }
    
    let mut subs = subsystems.single_mut().unwrap();
    
    if let Ok(status) = robot.single() {
        if status.connected {
            subs.set_readiness(SUBSYSTEM_FANUC, SubsystemReadiness::Ready);
        } else {
            subs.set_readiness(SUBSYSTEM_FANUC,
                SubsystemReadiness::Error("Robot not connected".into()));
        }
    } else {
        subs.set_readiness(SUBSYSTEM_FANUC, SubsystemReadiness::NotReady);
    }
}
```

### Execution Plugin (Coordinator)
```rust
fn coordinate_validation(
    mut buffer_state: Query<&mut BufferState, With<ActiveSystem>>,
    subsystems: Query<&Subsystems, With<ActiveSystem>>,
) {
    let Ok(mut state) = buffer_state.single_mut() else { return };

    // Only process during Validating state
    let BufferState::Validating { started_at } = *state else { return };

    let Ok(subs) = subsystems.single() else { return };

    // Check for timeout (30 seconds)
    if started_at.elapsed() > VALIDATION_TIMEOUT {
        let not_ready: Vec<_> = subs.not_ready();
        *state = BufferState::Error {
            message: format!("Validation timeout. Not ready: {:?}", not_ready)
        };
        return;
    }

    // Check for errors from any subsystem
    if let Some(error) = subs.first_error() {
        *state = BufferState::Error { message: error.to_string() };
        return;
    }

    // All subsystems ready? Transition to Executing
    if subs.all_ready() {
        *state = BufferState::Executing {
            current_index: 0,
            completed_count: 0
        };
    }
    // Otherwise stay in Validating - user can cancel via StopProgram
}
```

