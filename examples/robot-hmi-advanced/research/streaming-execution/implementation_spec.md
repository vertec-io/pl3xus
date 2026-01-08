# Implementation Specification: Streaming Execution

> **Document Purpose:** Detailed code changes for implementing the Sealed Buffer Pattern  
> **Last Updated:** 2025-12-28

## Phase Status Overview

| Phase | Name | Status | Dependencies |
|-------|------|--------|--------------|
| 1 | Add Stopped State & Notification | ✅ DONE | None |
| 2 | Sealed Buffer Pattern | 🔴 TODO | None |
| 3 | AwaitingPoints State | 🔴 TODO | Phase 2 |
| 4 | Update Completion Logic | 🔴 TODO | Phase 2, 3 |
| 5 | ExecutionProgress Type | 🔴 TODO | Phase 2 |
| 6 | UI Updates for Streaming | 🔴 TODO | Phase 5 |

---

## Phase 1: Add Stopped State & Notification

### Goal
Add `BufferState::Stopped` variant and send `ProgramNotification::Stopped` when user stops execution.

### Files to Modify

#### 1.1 `plugins/execution/src/components/buffer.rs`

**Add new variant to BufferState:**

```rust
// Add after Error variant (around line 130)
/// Execution stopped by user (not an error, not complete)
Stopped {
    /// Index at which execution was stopped
    at_index: u32,
    /// Number of points that were completed before stop
    completed_count: u32,
},
```

**Add helper method:**

```rust
// Add to impl BufferState (around line 170)
/// Check if execution was stopped by user.
pub fn is_stopped(&self) -> bool {
    matches!(self, BufferState::Stopped { .. })
}
```

#### 1.2 `plugins/fanuc/src/handlers.rs`

**Find or create StopProgram handler. Set BufferState to Stopped:**

```rust
// In handle_stop_program or similar
if let BufferState::Executing { current_index, completed_count } = &*buffer_state {
    let at_index = *current_index;
    let completed = *completed_count;
    
    *buffer_state = BufferState::Stopped {
        at_index,
        completed_count: completed,
    };
    
    // Send notification
    let notification = new_notification(ProgramNotificationKind::Stopped {
        program_name: coordinator.name.clone(),
        at_line: at_index as usize,
    });
    net.broadcast(notification);
    
    let console_msg = console_entry(
        format!("Program '{}' stopped at line {}", coordinator.name, at_index),
        ConsoleDirection::System,
        ConsoleMsgType::Info,
    );
    net.broadcast(console_msg);
}
```

#### 1.3 `plugins/execution/src/systems/lifecycle.rs`

**Update reset_on_disconnect_system to handle Stopped state:**

```rust
// In reset condition, add Stopped to the match
BufferState::Executing { .. } 
| BufferState::AwaitingPoints { .. }  // Future phase
| BufferState::Stopped { .. } => {
    *buffer_state = BufferState::Idle;
}
```

### Testing Phase 1

1. Start a program, then send StopProgram request
2. Verify `BufferState::Stopped` is set
3. Verify `ProgramNotification::Stopped` is broadcast
4. Verify console message appears
5. Verify reset_on_disconnect still works

---

## Phase 2: Sealed Buffer Pattern

### Goal
Add `sealed` field to ToolpathBuffer to distinguish static vs streaming execution.

### Files to Modify

#### 2.1 `plugins/execution/src/components/buffer.rs`

**Update ToolpathBuffer struct:**

```rust
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "ecs", derive(Component))]
pub struct ToolpathBuffer {
    /// The queue of points to execute
    points: VecDeque<ExecutionPoint>,

    /// Total number of points ever added to this buffer
    total_added: u32,

    /// Expected total for static programs (None for streaming until sealed)
    expected_total: Option<u32>,

    /// True when no more points will be added
    /// - Static programs: true immediately after creation
    /// - Streaming: becomes true when producer calls seal()
    sealed: bool,
}
```

**Update constructors:**

```rust
impl ToolpathBuffer {
    /// Create a new empty buffer (defaults to unsealed/streaming mode).
    pub fn new() -> Self {
        Self {
            points: VecDeque::new(),
            total_added: 0,
            expected_total: None,
            sealed: false,
        }
    }

    /// Create a buffer for a static program with known total.
    /// The buffer is immediately sealed.
    pub fn new_static(expected_total: u32) -> Self {
        Self {
            points: VecDeque::with_capacity(expected_total as usize),
            total_added: 0,
            expected_total: Some(expected_total),
            sealed: true,  // Static = sealed from start
        }
    }

    /// Create a buffer for streaming execution.
    /// Must call seal() when producer is done adding points.
    pub fn new_streaming() -> Self {
        Self {
            points: VecDeque::new(),
            total_added: 0,
            expected_total: None,
            sealed: false,
        }
    }
    
    // Keep existing with_expected for backwards compatibility
    // but mark as deprecated
    #[deprecated(note = "Use new_static() instead")]
    pub fn with_expected(expected_total: u32) -> Self {
        Self::new_static(expected_total)
    }
}
```

**Add new methods:**

```rust
impl ToolpathBuffer {
    // ... existing methods ...

    /// Push a point to the back of the buffer.
    /// Also increments total_added counter.
    pub fn push(&mut self, point: ExecutionPoint) {
        self.points.push_back(point);
        self.total_added += 1;
    }

    /// Seal the buffer - no more points will be added.
    /// Sets expected_total to current total_added.
    pub fn seal(&mut self) {
        self.sealed = true;
        if self.expected_total.is_none() {
            self.expected_total = Some(self.total_added);
        }
    }

    /// Check if buffer is sealed (no more points will be added).
    pub fn is_sealed(&self) -> bool {
        self.sealed
    }

    /// Get total number of points ever added to this buffer.
    pub fn total_added(&self) -> u32 {
        self.total_added
    }

    /// Check if execution is logically complete.
    /// Returns true only if: sealed AND empty AND all confirmed.
    pub fn is_execution_complete(&self, completed_count: u32) -> bool {
        self.sealed
            && self.points.is_empty()
            && completed_count >= self.total_added
    }

    /// Get the expected total (Some for static/sealed, None for streaming).
    pub fn expected_total(&self) -> Option<u32> {
        self.expected_total
    }

    /// Legacy method for backwards compatibility.
    /// Returns expected_total or 0 if None.
    #[deprecated(note = "Use expected_total() which returns Option")]
    pub fn expected_total_or_zero(&self) -> u32 {
        self.expected_total.unwrap_or(0)
    }
}
```

#### 2.2 Update LoadProgram Handler

**In the handler that loads a static program:**

```rust
// When loading a program from database
let point_count = instructions.len() as u32;
let mut buffer = ToolpathBuffer::new_static(point_count);

for instruction in instructions {
    buffer.push(/* convert to ExecutionPoint */);
}
// Buffer is already sealed since we used new_static()
```

### Testing Phase 2

1. Load a static program, verify `is_sealed() == true`
2. Create streaming buffer, verify `is_sealed() == false`
3. Add points to streaming buffer, call `seal()`, verify `is_sealed() == true`
4. Verify `total_added` increments correctly
5. Verify `expected_total()` returns `Some` after seal

---

## Phase 3: AwaitingPoints State

### Goal
Add `BufferState::AwaitingPoints` for when streaming buffer is empty but not sealed.

### Files to Modify

#### 3.1 `plugins/execution/src/components/buffer.rs`

**Add new variant to BufferState:**

```rust
// Add after Executing variant
/// Buffer empty but expecting more points (streaming only).
/// The producer has not yet signaled completion via seal().
AwaitingPoints {
    /// Number of points completed before buffer emptied
    completed_count: u32,
},
```

**Add helper method:**

```rust
/// Check if waiting for more points (streaming mode).
pub fn is_awaiting_points(&self) -> bool {
    matches!(self, BufferState::AwaitingPoints { .. })
}
```

#### 3.2 `plugins/fanuc/src/motion.rs`

**Update sync_device_status_to_buffer_state to handle AwaitingPoints:**

```rust
// In the completion check section
if toolpath_buffer.is_empty() {
    if toolpath_buffer.is_sealed() {
        // Sealed and empty = complete
        if completed >= toolpath_buffer.total_added() {
            *buffer_state = BufferState::Complete { total_executed: completed };
            // ... send notification ...
        }
    } else {
        // Not sealed, empty = waiting for more points
        *buffer_state = BufferState::AwaitingPoints { completed_count: completed };
        info!("⏳ BufferState -> AwaitingPoints (waiting for more points)");
    }
}
```

**Also add transition from AwaitingPoints back to Executing:**

This should happen in the orchestrator or a new system when points are added.

```rust
// New system or in orchestrator
pub fn resume_from_awaiting_system(
    mut query: Query<(&mut BufferState, &ToolpathBuffer)>,
) {
    for (mut state, buffer) in query.iter_mut() {
        if let BufferState::AwaitingPoints { completed_count } = *state {
            if !buffer.is_empty() {
                // Points were added, resume execution
                *state = BufferState::Executing {
                    current_index: completed_count,
                    completed_count,
                };
                info!("▶️ BufferState -> Executing (points received)");
            }
        }
    }
}
```

### Testing Phase 3

1. Create streaming buffer, add 5 points, start execution
2. Execute all 5 points (buffer empties)
3. Verify state is `AwaitingPoints` (not Complete)
4. Add 3 more points
5. Verify state transitions to `Executing`
6. Call `seal()`, verify state goes to `Complete` when done

---

## Phase 4: Update Completion Logic

### Goal
Use `is_execution_complete()` instead of manual checks.

### Files to Modify

#### 4.1 `plugins/fanuc/src/motion.rs`

**Replace completion check:**

```rust
// BEFORE (current code)
if toolpath_buffer.is_empty() && new_completed >= toolpath_buffer.expected_total() {
    *buffer_state = BufferState::Complete { ... };
}

// AFTER (new code)
if toolpath_buffer.is_execution_complete(new_completed) {
    *buffer_state = BufferState::Complete { total_executed: new_completed };
    // ... send notification ...
} else if toolpath_buffer.is_empty() && !toolpath_buffer.is_sealed() {
    *buffer_state = BufferState::AwaitingPoints { completed_count: new_completed };
}
```

### Testing Phase 4

1. Run existing static program tests - should still pass
2. Run streaming tests from Phase 3 - should work correctly
3. Verify no regressions in completion detection

---

## Phase 5: ExecutionProgress Type

### Goal
Add typed progress reporting for UI.

### Files to Modify

#### 5.1 `plugins/execution/src/components/buffer.rs` or new file

**Add new type:**

```rust
/// Progress information for UI display.
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionProgress {
    /// Known total - can show percentage and progress bar
    Determinate {
        completed: u32,
        total: u32,
        percent: f32,
    },
    /// Unknown total - show count only (streaming mode)
    Indeterminate {
        completed: u32,
    },
}

impl ToolpathBuffer {
    /// Get progress information for display.
    pub fn progress(&self, completed_count: u32) -> ExecutionProgress {
        match self.expected_total {
            Some(total) => ExecutionProgress::Determinate {
                completed: completed_count,
                total,
                percent: if total > 0 {
                    (completed_count as f32 / total as f32) * 100.0
                } else {
                    0.0
                },
            },
            None => ExecutionProgress::Indeterminate {
                completed: completed_count,
            },
        }
    }
}
```

### Testing Phase 5

1. Static program: `progress()` returns `Determinate` with correct percentage
2. Streaming (unsealed): `progress()` returns `Indeterminate`
3. Streaming (sealed): `progress()` returns `Determinate`

---

## Phase 6: UI Updates for Streaming

### Goal
Update frontend to display appropriate progress for both modes.

### Files to Modify

This phase involves frontend changes (likely in `frontend/` directory).

**Key UI Logic:**

```typescript
// Pseudocode for UI
function renderProgress(state: BufferState, buffer: ToolpathBuffer) {
  if (state.type === 'AwaitingPoints') {
    return <Spinner label="Waiting for motion data..." />;
  }

  const progress = buffer.progress(state.completed_count);

  if (progress.type === 'Determinate') {
    return <ProgressBar
      value={progress.completed}
      max={progress.total}
      label={`${progress.completed}/${progress.total} (${progress.percent.toFixed(0)}%)`}
    />;
  } else {
    return <PulsingBar label={`${progress.completed} processed...`} />;
  }
}
```

---

## Backwards Compatibility

### Breaking Changes

1. `ToolpathBuffer::expected_total()` now returns `Option<u32>` instead of `u32`
   - Add deprecated `expected_total_or_zero()` for transition

2. `ToolpathBuffer::with_expected()` deprecated in favor of `new_static()`

### Migration Path

1. Replace `with_expected(n)` → `new_static(n)`
2. Replace `expected_total()` comparisons with `is_execution_complete()`
3. Handle `AwaitingPoints` state in any code that checks BufferState

---

## Test Matrix

| Scenario | Expected Behavior |
|----------|-------------------|
| Static program completes | `Complete` notification sent |
| Static program stopped | `Stopped` notification sent |
| Static program errors | `Error` notification sent |
| Streaming buffer empties (not sealed) | `AwaitingPoints` state |
| Streaming buffer receives points | Resume `Executing` |
| Streaming sealed + empty + confirmed | `Complete` notification |
| Streaming stopped | `Stopped` notification |
| Disconnect while executing | Reset to `Idle` |
| Disconnect while awaiting | Reset to `Idle` |

---

*End of Implementation Specification*
```

