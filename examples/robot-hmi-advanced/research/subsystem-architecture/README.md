# Subsystem Architecture Research

## Goal

Design and implement a decoupled plugin architecture where:
1. **Execution plugin** owns:
   - `ExecutionState` (synced): state, source_type, source_name, current_index, can_* actions
   - `BufferDisplayData` (synced): lines for UI table display
   - `ToolpathBuffer` (internal): sealed buffer pattern for static and streaming
   - `Subsystems`: validation coordination
2. **Programs plugin** is a **Static Loader** that:
   - Loads programs from database
   - Converts to ExecutionPoints and BufferLineDisplay
   - Pushes ALL points to ToolpathBuffer and seals
   - Updates ExecutionState and BufferDisplayData
   - Owns `LoadedProgram` (internal): for tracking/unload
3. **Lower-level plugins** (fanuc, duet, etc.) register as subsystems and report readiness
4. **No cross-dependencies** between lower-level plugins
5. Lower-level plugins CAN import from core and execution plugins

## Key Architectural Insight

**Buffer-Centric Execution**: Execution is about the buffer, not the program.
Programs (and future streams/generators) are "sources" that feed the buffer.

**UI = Buffer View**: The "program table" in the UI IS the execution buffer table.
- `BufferDisplayData` contains the lines to display
- `ExecutionState.current_index` determines the highlighted row
- Works for static programs, streams, and generators

## Current Problems

### 1. Tangled Dependencies
```
fanuc_plugin
├── imports from: execution, programs, core
├── exports: execution handlers (start/stop/pause/resume)
├── exports: program handlers (load/unload) 
└── has: ProgramExecutionState (duplicates execution state)

programs_plugin  
├── imports from: core
├── exports: program CRUD handlers
└── missing: load/unload to System entity

execution_plugin
├── imports from: robotics
├── exports: BufferState, ToolpathBuffer, etc.
└── missing: execution handlers, UI state types
```

### 2. Execution System Does Too Much Checking
The execution handlers currently check:
- Is a program loaded?
- Is the robot connected?
- Is the buffer in the right state?
- Are there pending errors?

Each of these should be the responsibility of the respective subsystem.

### 3. Program State at Wrong Level
Programs are currently associated with robot entities, but they should be at the System level since a program coordinates multiple devices.

## Proposed Architecture

### Core Types (execution plugin)

```rust
/// Subsystem registration - added to System entity
#[derive(Component)]
pub struct Subsystems {
    pub entries: Vec<SubsystemEntry>,
}

#[derive(Clone)]
pub struct SubsystemEntry {
    pub name: &'static str,
    pub readiness: SubsystemReadiness,
}

#[derive(Clone, Default)]
pub enum SubsystemReadiness {
    #[default]
    NotReady,
    Ready,
    Error(String),
}

/// Extended BufferState with validation phase
pub enum BufferState {
    Idle,
    // NEW: Validation phase before execution
    Validating,
    // ... existing states
    Executing { ... },
    Paused { ... },
    // etc.
}
```

### Dependency Graph (Target)

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                              System Entity                                    │
│                                                                              │
│  SYNCED TO CLIENTS:                         INTERNAL (server only):          │
│  ┌────────────────┐  ┌───────────────────┐  ┌────────────────┐              │
│  │ ExecutionState │  │ LoadedProgramInfo │  │ LoadedProgram  │              │
│  │ (execution)    │  │ (programs)        │  │ (programs)     │              │
│  └────────────────┘  └───────────────────┘  └────────────────┘              │
│                                                                              │
│  ┌─────────────┐  ┌─────────────┐                                           │
│  │ BufferState │  │ Subsystems  │                                           │
│  │ (execution) │  │ (execution) │                                           │
│  └─────────────┘  └─────────────┘                                           │
└──────────────────────────────────────────────────────────────────────────────┘
                          ▲
                          │ imports
        ┌─────────────────┼─────────────────┐
        │                 │                 │
        ▼                 ▼                 ▼
   ┌─────────┐      ┌──────────┐      ┌─────────┐
   │ programs│      │  fanuc   │      │  duet   │
   │ plugin  │      │  plugin  │      │ plugin  │
   └─────────┘      └──────────┘      └─────────┘
        │                 │                 │
        └─────────────────┴─────────────────┘
                    NO cross-imports
```

### Handler Ownership

| Handler | Current Location | Target Location | Notes |
|---------|-----------------|-----------------|-------|
| `Load` | fanuc (as LoadProgram) | programs | Loads program to System entity, updates LoadedProgramInfo |
| `Unload` | fanuc (as UnloadProgram) | programs | Unloads program, clears LoadedProgramInfo |
| `Start` | fanuc (as StartProgram) | execution | Triggers Validating → Executing |
| `Pause` | fanuc (as PauseProgram) | execution | Paused state |
| `Resume` | fanuc (as ResumeProgram) | execution | Validating → Executing |
| `Stop` | fanuc (as StopProgram) | execution | Stop and reset (also cancels validation) |

### Validation Flow

```
User clicks "Start"
        │
        ▼
BufferState::Validating
        │
        ▼
Each subsystem runs validation:
  - programs_plugin: Is program loaded? → Ready/NotReady
  - fanuc_plugin: Is robot connected? → Ready/Error
  - duet_plugin: Is extruder ready? → Ready/NotReady
        │
        ▼
execution_plugin checks Subsystems:
  - All Ready? → BufferState::Executing
  - Any Error? → BufferState::Error
  - Any NotReady? → Stay in Validating (with timeout?)
```

## Related Research

This research builds on and connects to:

### Streaming Execution Research (`../streaming-execution/`)
The streaming-execution research defines the **Sealed Buffer Pattern** which is foundational:
- **Sealed Buffer**: `ToolpathBuffer` has `sealed: bool` to indicate no more points will be added
- **Producer Types**: StaticLoader, StreamingImporter, RealtimeGenerator
- **Completion Logic**: `sealed && buffer.is_empty() && all_confirmed`

**Key Connection**: This subsystem architecture implements "Programs as StaticLoader" from that research.
The programs plugin is the first producer type; future streaming/generator plugins will follow the same pattern.

### Execution Plugin Research (`../execution-plugin/`)
Original execution architecture with BufferState, ToolpathBuffer, orchestrator systems.

### In-Flight Queue Design (`in-flight-queue.md`)
For FANUC continuous motion (CNT), we need multiple points queued in the controller
for smooth blending. This document describes the capacity-based in-flight queue
that replaces the simple boolean `ready_for_next` pattern.

---

## Handoff Information

### For Another Agent Implementing This

**Goal**: Implement the subsystem architecture with programs as a static loader feeding the execution buffer.

**Start Here**:
1. Read this README for architecture overview
2. Read `implementation-plan.md` for detailed phase-by-phase tasks
3. Read `types.md` for exact component definitions
4. Read `handler-ownership.md` for handler implementations
5. Reference `program-execution-relationship.md` for design rationale
6. Read `state-machine.md` for state transitions including resume validation
7. Read `in-flight-queue.md` for continuous motion queue design

**Key Files to Create/Modify**:

| Phase | Files | Purpose |
|-------|-------|---------|
| 1 | `execution/src/components/subsystems.rs` | Subsystem registration |
| 1 | `execution/src/components/execution_state.rs` | Synced state with source_type |
| 1 | `execution/src/components/buffer_display.rs` | BufferDisplayData for UI |
| 1 | `execution/src/systems/validation.rs` | Validation coordinator |
| 1 | `execution/src/systems/sync_execution_state.rs` | BufferState → ExecutionState sync |
| 2 | `execution/src/handlers.rs` | Start, Pause, Resume, Stop |
| 3 | `programs/src/handlers.rs` | Load (static loader), Unload |
| 3 | `programs/src/conversion.rs` | Program → ExecutionPoints/DisplayLines |
| 4 | `fanuc/src/handlers.rs` | Remove moved handlers, add subsystem |

**Critical Design Decisions** (do not deviate):
1. **Buffer-Centric**: Execution is about the buffer, not the program
2. **Static Loader Pattern**: Programs plugin pushes ALL points to buffer and seals
3. **UI = Buffer View**: BufferDisplayData shows lines, current_index highlights row
4. **Simple Verbs**: Start, Stop, Pause, Resume, Load, Unload (not StartProgram, etc.)

**Testing Strategy**:
1. Unit tests for each new component
2. Integration test: Load → Start → Executing → Complete flow
3. Test validation: subsystem errors, timeout, cancellation
4. Test UI sync: verify current_index updates, highlighting works

**Estimated Effort**: 20-26 hours total (see implementation-plan.md)

---

## Implementation Plan

See `implementation-plan.md` for detailed steps.

