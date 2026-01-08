# Open Questions

## Resolved Decisions

### 1. Where should message types live?
**RESOLVED**: Each plugin owns its message types with re-exports through `plugins/src/lib.rs`
- Simple verbs: `Start`, `Pause`, `Resume`, `Stop` → execution plugin
- Simple verbs: `Load`, `Unload` → programs plugin
- Re-export all through the main plugins crate for convenience

### 2. How to handle validation timeout?
**RESOLVED**: 30 second timeout + user-cancellable via Stop

```rust
BufferState::Validating { started_at: Instant }
// After 30 seconds, auto-transition to Error
// User can cancel at any time via Stop
```

The stop button is always available during Validating state.

### 3. Should subsystems register dynamically or statically?
**RESOLVED**: Dynamic registration

Each plugin inserts a `SubsystemEntry` into the `Subsystems` component on the System entity.
The System entity doesn't know or care what subsystems exist - it just checks the entries.

```rust
// Each plugin's startup system:
fn register_fanuc_subsystem(
    mut subsystems: Query<&mut Subsystems, With<ActiveSystem>>,
) {
    if let Ok(mut subs) = subsystems.single_mut() {
        subs.entries.push(SubsystemEntry {
            name: SUBSYSTEM_FANUC.to_string(),
            readiness: SubsystemReadiness::NotReady,
        });
    }
}
```

### 4. Where does ExecutionState and program info live?
**RESOLVED**: Split into two synced components

**ExecutionState (execution plugin, synced)**:
- `state: SystemState` (NoProgram, Idle, Validating, Running, Paused, Completed, Error)
- `current_index: usize` (where we are in execution)
- Available actions: `can_start`, `can_pause`, `can_resume`, `can_stop`, `can_load`, `can_unload`

**LoadedProgramInfo (programs plugin, synced)**:
- `program_id: Option<i64>`
- `program_name: Option<String>`
- `total_lines: usize`
- `program_lines: Vec<ProgramLineInfo>` (for UI display)

**LoadedProgram (programs plugin, NOT synced)**:
- Internal execution data, sequences, points
- Only server uses this

### 5. How to handle the motion feedback loop?
**RESOLVED**: Keep current approach for now

- Fanuc plugin directly updates BufferState.completed_count
- Consider events later if needed

### 6. How to coordinate multiple subsystems during execution?
**RESOLVED**: Subsystem readiness is for startup validation only

During execution, the existing event-based coordination (ExecutionCoordinator) remains unchanged.
Subsystems don't coordinate during execution - that's a different system.

### 7. Client-side implications
**RESOLVED**: Update all imports at once (no gradual migration with type aliases)

- Client imports `SystemState` from execution plugin
- Client imports `LoadedProgramInfo` from programs plugin
- No backwards-compatibility aliases needed

### 8. What does "Validating" look like in UI?
**RESOLVED**: Spinner with expandable details showing subsystem status

### 9. Should validation be visible or instant?
**RESOLVED**: No artificial delay - fast is good

If all subsystems report Ready within 100ms, user sees nearly instant transition.

### 10. How should Resume handle re-validation?
**RESOLVED**: Resume goes through Validating state to re-check subsystems

When resuming from pause, the system should validate all subsystems before
continuing execution. This ensures:
- Robot is still connected
- No emergency stop was triggered
- All safety conditions are still met

The paused-at index is preserved using a `ValidatingForResume { resume_from_index }`
variant of BufferState. See `state-machine.md` for implementation details.

### 11. How to handle multiple in-flight motion commands for continuous motion?
**RESOLVED**: Use capacity-based in-flight queue instead of boolean ready_for_next

FANUC continuous motion (CNT) requires multiple points in the controller's queue
for smooth motion blending. The orchestrator should fill the queue up to a
configured capacity (typically 5-10 points for CNT motion).

```rust
pub struct DeviceStatus {
    pub in_flight_capacity: u32,  // Max commands in-flight (e.g., 8)
    pub in_flight_count: u32,     // Current in-flight count
    // ...
}

impl DeviceStatus {
    pub fn ready_for_next(&self) -> bool {
        self.in_flight_count < self.in_flight_capacity
    }
}
```

The orchestrator loops while `ready_for_next()` is true, sending multiple points
per tick. See `in-flight-queue.md` for full implementation details.

