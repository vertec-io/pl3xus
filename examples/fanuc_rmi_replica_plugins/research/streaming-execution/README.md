# Research Project: Streaming Execution & Sealed Buffer Pattern

> **Status:** Design Complete, Ready for Implementation  
> **Created:** 2025-12-28  
> **Last Updated:** 2025-12-28  
> **Owner:** Architecture Team  
> **Prerequisites:** [execution-plugin](../execution-plugin/README.md) research (partially implemented)

## Executive Summary

This research project extends the execution plugin architecture to support **streaming/realtime execution** alongside static program execution. The key innovation is the **Sealed Buffer Pattern** which enables:

1. **Static programs**: Fixed toolpath, known total, progress bar with percentage
2. **Streaming execution**: Points generated dynamically, producer signals completion
3. **Unified completion logic**: Both modes use same underlying mechanism
4. **Clear state semantics**: `Stopped` vs `Complete` vs `Error` are distinct

## Problem Statement

### Current Implementation Issues

The current completion detection logic:
```rust
if toolpath_buffer.is_empty() && completed >= toolpath_buffer.expected_total() {
    *buffer_state = BufferState::Complete { total_executed: completed };
}
```

**Problems with this approach:**
1. `expected_total` is required - doesn't work for streaming where total is unknown
2. Buffer being empty means "complete" - but for streaming, it may just be waiting for more points
3. No distinction between "program finished normally" vs "user stopped execution"
4. No support for "waiting for more points" state for streaming mode

### Use Cases to Support

| Mode | Total Known? | Progress Bar | Completion Signal |
|------|-------------|--------------|-------------------|
| Static Program | Yes | "42/100 (42%)" | buffer empty + confirmed = total |
| Streaming | No | "42 processed..." | Producer signals "sealed" |
| Realtime+Sensor | No | "42 processed, generating..." | Producer signals "sealed" |

## Table of Contents

1. [Architecture Design](#1-architecture-design)
2. [Sealed Buffer Pattern](#2-sealed-buffer-pattern)
3. [State Machine Design](#3-state-machine-design)
4. [Component Changes](#4-component-changes)
5. [Notification System](#5-notification-system)
6. [Progress Display](#6-progress-display)
7. [Implementation Phases](#7-implementation-phases)
8. [Current State Analysis](#8-current-state-analysis)
9. [Handoff Information](#9-handoff-information)

---

## Quick Links

- [Architecture Diagrams](./diagrams.md)
- [Implementation Specification](./implementation_spec.md)
- [Current State & Status](./current_state.md)

---

## 1. Architecture Design

### Core Insight: Who Decides Completion?

| Execution Mode | Completion Decision |
|----------------|---------------------|
| **Static** | System detects: buffer empty + all confirmed |
| **Streaming** | Producer signals: "I'm done generating" |

This leads to the **Sealed Buffer Pattern**: A buffer is "sealed" when no more points will be added.

### Conversion Boundaries

```
Static:    LoadProgram → [seal immediately] → Buffer → Orchestrator → Complete
Streaming: Generator → Buffer → [running...] → seal() → Draining → Complete
```

### Producer Types

1. **StaticLoader**: Loads all points from DB, seals immediately
2. **StreamingImporter**: Imports from external source, seals when source ends
3. **RealtimeGenerator**: Generates from sensor feedback, seals when algorithm decides

---

## 2. Sealed Buffer Pattern

### Concept

The `sealed` flag indicates whether more points may be added to the buffer:

| State | `sealed` | `expected_total` | Meaning |
|-------|----------|------------------|---------|
| Static program loaded | `true` | `Some(N)` | All N points known upfront |
| Streaming in progress | `false` | `None` | More points may come |
| Streaming sealed | `true` | `Some(total_added)` | Producer finished, draining |

### Buffer Lifecycle

```
                    Static Program
                    ══════════════
    new_static(100) ─► sealed=true, expected=Some(100)
                          │
                          ▼ (execute all points)
                          │
                      Complete


                    Streaming Execution
                    ═══════════════════
    new_streaming() ─► sealed=false, expected=None
          │
          ▼ (receive points dynamically)
          │
       seal() ─► sealed=true, expected=Some(total_added)
          │
          ▼ (drain remaining points)
          │
      Complete
```

### Completion Logic

```rust
fn is_execution_complete(&self, completed_count: u32) -> bool {
    self.sealed                        // Must be sealed (no more points coming)
    && self.points.is_empty()          // Buffer fully drained
    && completed_count >= self.total_added  // All confirmations received
}
```

---

## 3. State Machine Design

### Enhanced BufferState

```rust
pub enum BufferState {
    /// No execution active
    Idle,

    /// Buffering points before execution starts
    Buffering { min_threshold: u32 },

    /// Ready to start execution
    Ready,

    /// Actively executing
    Executing {
        current_index: u32,
        completed_count: u32,
    },

    /// Buffer empty, waiting for more points (streaming only)
    AwaitingPoints {
        completed_count: u32,
    },

    /// Execution paused by user
    Paused {
        paused_at_index: u32,
    },

    /// Waiting for external condition
    WaitingForFeedback {
        reason: WaitReason,
    },

    /// Execution completed successfully
    Complete {
        total_executed: u32,
    },

    /// Execution stopped by user (not an error, not complete)
    Stopped {
        at_index: u32,
        completed_count: u32,
    },

    /// Execution encountered an error
    Error {
        message: String,
    },
}
```

### State Transitions

See [diagrams.md](./diagrams.md) for visual state machine.

Key transitions:
- `Executing` → `AwaitingPoints`: Buffer empty, not sealed
- `AwaitingPoints` → `Executing`: New points added
- `Executing`/`AwaitingPoints` → `Stopped`: User stop command
- `Executing` → `Complete`: Buffer empty, sealed, all confirmed

---

## 4. Component Changes

### ToolpathBuffer (Enhanced)

```rust
pub struct ToolpathBuffer {
    /// The queue of points to execute
    points: VecDeque<ExecutionPoint>,

    /// Total number of points ever added to this buffer
    total_added: u32,

    /// Expected total for static programs (None for streaming until sealed)
    expected_total: Option<u32>,

    /// True when no more points will be added
    /// - Static programs: true immediately after loading
    /// - Streaming: becomes true when producer calls seal()
    sealed: bool,
}

impl ToolpathBuffer {
    /// Create buffer for static program with known total
    pub fn new_static(expected: u32) -> Self {
        Self {
            points: VecDeque::with_capacity(expected as usize),
            total_added: 0,
            expected_total: Some(expected),
            sealed: true,  // All points known upfront
        }
    }

    /// Create buffer for streaming execution
    pub fn new_streaming() -> Self {
        Self {
            points: VecDeque::new(),
            total_added: 0,
            expected_total: None,
            sealed: false,
        }
    }

    /// Seal the buffer - no more points will be added
    pub fn seal(&mut self) {
        self.sealed = true;
        self.expected_total = Some(self.total_added);
    }

    /// Check if buffer is sealed
    pub fn is_sealed(&self) -> bool {
        self.sealed
    }

    /// Check if execution is logically complete
    pub fn is_execution_complete(&self, completed_count: u32) -> bool {
        self.sealed
        && self.points.is_empty()
        && completed_count >= self.total_added
    }

    /// Get progress information for display
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

### ExecutionProgress (New Type)

```rust
/// Progress information for UI display
pub enum ExecutionProgress {
    /// Known total - can show percentage and progress bar
    Determinate {
        completed: u32,
        total: u32,
        percent: f32,
    },
    /// Unknown total - show count only
    Indeterminate {
        completed: u32,
    },
}
```

---

## 5. Notification System

### ProgramNotification Types

```rust
pub enum ProgramNotificationKind {
    /// Program completed successfully (all points executed)
    Completed {
        program_name: String,
        total_instructions: usize,
    },
    /// Program was stopped by user
    Stopped {
        program_name: String,
        at_line: usize,
    },
    /// Program encountered an error
    Error {
        program_name: String,
        at_line: usize,
        error_message: String,
    },
}
```

### When Notifications are Sent

| Transition | Notification | Console Message |
|------------|--------------|-----------------|
| → `Complete` | `Completed` | "Program 'X' completed (N instructions)" |
| → `Stopped` | `Stopped` | "Program 'X' stopped at line N" |
| → `Error` | `Error` | "Program 'X' error at line N: message" |

---

## 6. Progress Display

### Static Program

```
┌─────────────────────────────────────────┐
│ Program: MyToolpath                     │
│ ████████████░░░░░░░░░░░░░░░░ 42/100 42% │
│ Status: Executing                       │
└─────────────────────────────────────────┘
```

### Streaming (Generating)

```
┌─────────────────────────────────────────┐
│ Program: RealtimePath                   │
│ ◐ 42 points processed, generating...   │
│ Status: Executing                       │
└─────────────────────────────────────────┘
```

### Streaming (Awaiting Points)

```
┌─────────────────────────────────────────┐
│ Program: RealtimePath                   │
│ ⏳ 42 points processed                  │
│ Status: Waiting for motion data...      │
└─────────────────────────────────────────┘
```

### Streaming (Draining - after seal)

```
┌─────────────────────────────────────────┐
│ Program: RealtimePath                   │
│ ████████████████░░░░░░░░░░░░ 42/50 84%  │
│ Status: Completing...                   │
└─────────────────────────────────────────┘
```

---

## 7. Implementation Phases

### Phase Overview

| Phase | Description | Status | Estimated Effort |
|-------|-------------|--------|------------------|
| 1 | Add `Stopped` state and notification | ✅ **DONE** | 1 hour |
| 2 | Add `sealed` pattern to ToolpathBuffer | **TODO** | 2 hours |
| 3 | Add `AwaitingPoints` state | **TODO** | 1 hour |
| 4 | Update completion logic | **TODO** | 1 hour |
| 5 | Add `ExecutionProgress` type | **TODO** | 1 hour |
| 6 | Update UI for streaming mode | **TODO** | 2 hours |

See [implementation_spec.md](./implementation_spec.md) for detailed specifications.

---

## 8. Current State Analysis

### What Has Been Implemented (as of 2025-12-28)

1. ✅ **ExecutionPlugin created** with basic structure
2. ✅ **BufferState enum** with core states (Idle, Executing, Complete, Error, etc.)
3. ✅ **ToolpathBuffer** with basic push/pop/expected_total
4. ✅ **Orchestrator system** consuming buffer and emitting events
5. ✅ **FANUC motion chain** (handler → sent → response → sync systems)
6. ✅ **Completion notification** (ProgramNotification::Completed)
7. ✅ **Error notification** (ProgramNotification::Error)
8. ✅ **Stopped notification** (ProgramNotification::Stopped) - **Implemented 2025-12-28**
9. ❌ **Sealed buffer pattern** - Not yet implemented
10. ❌ **AwaitingPoints state** - Not yet implemented
11. ❌ **Streaming execution mode** - Not yet implemented

### Current Code Locations

| Component | File | Notes |
|-----------|------|-------|
| BufferState | `plugins/execution/src/components/buffer.rs` | ✅ Has Stopped, needs AwaitingPoints |
| ToolpathBuffer | `plugins/execution/src/components/buffer.rs` | Needs sealed pattern |
| Orchestrator | `plugins/execution/src/systems/orchestrator.rs` | Good, no changes needed |
| Completion Logic | `plugins/fanuc/src/motion.rs:sync_device_status_to_buffer_state` | Needs sealed check |
| Notifications | `plugins/fanuc/src/motion.rs` | ✅ Complete, Error done; Stopped in handlers.rs |
| Stop Handler | `plugins/fanuc/src/handlers.rs` | ✅ Sets Stopped state and sends notification |

---

## 9. Handoff Information

### For Another Agent Picking Up This Work

**Goal:** Implement streaming execution support via the Sealed Buffer Pattern.

**Entry Points:**
1. Read this README for architecture understanding
2. Read [implementation_spec.md](./implementation_spec.md) for detailed code changes
3. Check [current_state.md](./current_state.md) for what's been done

**Key Files to Modify:**
1. `plugins/execution/src/components/buffer.rs` - Add sealed pattern
2. `plugins/fanuc/src/motion.rs` - Update completion logic
3. `plugins/fanuc/src/handlers.rs` - Add StopProgram handler

**Testing Strategy:**
1. Static program: Should work exactly as before
2. Add a test that calls `seal()` after adding points dynamically
3. Verify Stopped notification when user stops program

**Dependencies:**
- No new crate dependencies
- Uses existing `pl3xus_websockets` for notifications

---

## Appendix A: Related Research

- [execution-plugin](../execution-plugin/README.md) - Original execution architecture
- [coordinate-abstraction](../coordinate-abstraction/) - Quaternion storage design

---

## Appendix B: Open Questions

1. **Timeout for AwaitingPoints?** Should we have a configurable timeout for how long to wait for new points before treating it as an error?

2. **Resume after Stopped?** Can a stopped program be resumed, or must it be reloaded? Current design treats Stopped as terminal.

3. **Streaming from network?** For realtime generation, points may come from a network source (e.g., ROS node). How do we handle network latency and buffering?

---

*End of Research Document*
