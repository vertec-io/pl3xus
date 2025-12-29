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

### Paused → Validating
- **Trigger**: `ResumeProgram` handler (execution plugin)
- **Action**: Set `BufferState::Validating`
- **Note**: Re-validates all subsystems before resuming

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

