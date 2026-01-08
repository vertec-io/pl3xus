# Implementation Plan

Key decisions made (see open-questions.md and program-execution-relationship.md):

## Architectural Decisions

1. **Buffer-Centric Execution**: Execution is about the buffer, not the program.
   Programs (and future streams/generators) are "sources" that feed the buffer.

2. **Programs as Static Loader**: The programs plugin is a StaticLoader that:
   - Loads program from database
   - Converts to ExecutionPoints
   - Pushes ALL points to ToolpathBuffer
   - Seals the buffer (all points known)
   - Updates ExecutionState and BufferDisplayData

3. **UI = Buffer View**: The "program table" in the UI IS the execution buffer table.
   - `BufferDisplayData` contains the lines to display
   - `ExecutionState.current_index` determines the highlighted row
   - Works for static programs, streams, and generators

4. **Simple verb handler names**: Start, Stop, Pause, Resume, Load, Unload
5. **30 second validation timeout** + user-cancellable via Stop
6. **Dynamic subsystem registration**
7. **No backwards-compatibility aliases** - update everything at once

## Phase 1: Add Subsystem Infrastructure to Execution Plugin

### 1.1 Create Subsystem Types
- [ ] Create `execution/src/components/subsystems.rs`
- [ ] Define `Subsystems`, `SubsystemEntry`, `SubsystemReadiness`
- [ ] Add helper methods: `all_ready()`, `first_error()`, `not_ready()`, `set_readiness()`
- [ ] Export from `execution/src/lib.rs`

### 1.2 Update BufferState
- [ ] Add `Validating { started_at: Instant }` variant to `BufferState`
- [ ] Add `VALIDATION_TIMEOUT` constant (30 seconds)
- [ ] Add `is_validation_timed_out()` helper method
- [ ] Update `to_ui_state()` to map `Validating` → `SystemState::Validating`
- [ ] Update `available_actions()` for `Validating` state (can_stop = true)

### 1.3 Create ExecutionState Component (Synced)
- [ ] Create `execution/src/components/execution_state.rs`
- [ ] Define `ExecutionState` with:
  - `state: SystemState`
  - `source_type: SourceType` (None, StaticProgram, Stream, Generator)
  - `source_name: Option<String>`
  - `current_index: usize` (for UI highlighting)
  - `total_points: Option<usize>`
  - `points_executed: usize`
  - `can_*` action flags
- [ ] Define `SystemState` enum (NoSource, Ready, Validating, Running, Paused, AwaitingPoints, Completed, Stopped, Error)
- [ ] Define `SourceType` enum (None, StaticProgram, Stream, Generator)
- [ ] Add Store derive for client sync
- [ ] Export from `execution/src/lib.rs`

### 1.4 Create BufferDisplayData Component (Synced)
- [ ] Create `execution/src/components/buffer_display.rs`
- [ ] Define `BufferDisplayData` with `lines: Vec<BufferLineDisplay>`
- [ ] Define `BufferLineDisplay` with: index, line_type, description, sequence_name, source_line
- [ ] Add Store derive for client sync
- [ ] Export from `execution/src/lib.rs`

### 1.5 Create Validation Coordinator System
- [ ] Create `execution/src/systems/validation.rs`
- [ ] Implement `coordinate_validation` system with timeout logic
- [ ] Register system in ExecutionPlugin

### 1.6 Create BufferState → ExecutionState Sync System
- [ ] Create `execution/src/systems/sync_execution_state.rs`
- [ ] Sync internal BufferState to client-facing ExecutionState
- [ ] Update current_index, points_executed on changes
- [ ] Register system in ExecutionPlugin

## Phase 2: Move Execution Handlers to Execution Plugin

### 2.1 Create Handler Infrastructure
- [ ] Create `execution/src/handlers.rs`
- [ ] Define message types in `execution/src/types.rs`:
  - `Start`, `StartResponse`
  - `Pause`, `PauseResponse`
  - `Resume`, `ResumeResponse`
  - `Stop`, `StopResponse`

### 2.2 Implement Handlers
- [ ] Implement `handle_start` (→ Validating)
- [ ] Implement `handle_pause` (→ Paused)
- [ ] Implement `handle_resume` (→ ValidatingForResume, preserves index)
- [ ] Implement `handle_stop` (→ Stopped, clear motion buffer)

### 2.3 Update BufferState for Resume Validation
- [ ] Add `ValidatingForResume { resume_from_index: u32 }` variant to BufferState
- [ ] Update validation coordinator to handle both Validating and ValidatingForResume
- [ ] When ValidatingForResume succeeds, start execution from `resume_from_index`

### 2.3 Register Handlers
- [ ] Add handler registration in ExecutionPlugin
- [ ] Update `plugins/src/lib.rs` exports

## Phase 3: Implement Programs Plugin as Static Loader

**Key Change**: Programs plugin is a "Static Loader" that feeds the execution buffer.

### 3.1 Create LoadedProgram Component (Internal)
- [ ] Create `programs/src/components/loaded_program.rs`
- [ ] Define `LoadedProgram` struct (internal tracking only)
- [ ] Used to track what program is loaded for Unload

### 3.2 Create Program → Buffer Conversion
- [ ] Create `programs/src/conversion.rs`
- [ ] Implement `ProgramDetail::to_execution_points()` → Vec<ExecutionPoint>
- [ ] Implement `ProgramDetail::to_display_lines()` → Vec<BufferLineDisplay>

### 3.3 Implement Load Handler (Static Loader Pattern)
- [ ] Create `programs/src/handlers.rs`
- [ ] Define message types: `Load`, `LoadResponse`
- [ ] On Load:
  1. Load program from database
  2. Convert to ExecutionPoints
  3. Push ALL points to ToolpathBuffer (from execution plugin)
  4. Seal the buffer
  5. Update BufferDisplayData (from execution plugin)
  6. Update ExecutionState.source_type = StaticProgram
  7. Store internal LoadedProgram for tracking

### 3.4 Implement Unload Handler
- [ ] Define message types: `Unload`, `UnloadResponse`
- [ ] On Unload:
  1. Remove LoadedProgram component
  2. Clear ToolpathBuffer
  3. Clear BufferDisplayData
  4. Reset ExecutionState to NoSource

### 3.5 Register Subsystem
- [ ] Add programs subsystem registration on startup
- [ ] Create `validate_program_subsystem` system (checks LoadedProgram exists)
- [ ] Register in ProgramsPlugin

## Phase 4: Update Fanuc Plugin

### 4.1 Remove Moved Handlers
- [ ] Remove execution handlers (handle_start_program, etc.) from `fanuc/src/handlers.rs`
- [ ] Remove program handlers (handle_load_program, etc.) from `fanuc/src/handlers.rs`
- [ ] Remove related message types from `fanuc/src/types.rs`
- [ ] Remove `ExecutionState`/`ProgramExecutionState` from fanuc

### 4.2 Add Subsystem Registration
- [ ] Register fanuc subsystem on plugin startup
- [ ] Create `validate_fanuc_subsystem` system
- [ ] Check robot connection status

### 4.3 Update Motion System
- [ ] Update `fanuc_motion_dispatch_system` to check BufferState
- [ ] Only dispatch motion when `BufferState::Executing`

### 4.4 Implement In-Flight Queue for Continuous Motion
See `in-flight-queue.md` for detailed design.

**DeviceStatus Changes:**
- [ ] Add `in_flight_capacity: u32` field (default 8 for FANUC CNT motion)
- [ ] Add `in_flight_count: u32` field
- [ ] Add `ready_for_next()` method checking `in_flight_count < in_flight_capacity`
- [ ] Remove boolean `ready_for_next` field
- [ ] Update all DeviceStatus creation sites

**Orchestrator Changes:**
- [ ] Change single-point dispatch to loop while `ready_for_next()`
- [ ] Add per-tick burst limit (e.g., max 5 points per tick)

**FANUC Motion Handler Changes:**
- [ ] Increment `in_flight_count` on send
- [ ] Decrement `in_flight_count` on response
- [ ] Remove `ready_for_next = false` after send
- [ ] Remove `ready_for_next = in_flight.is_empty()` on response

**Edge Case Handling:**
- [ ] On Stop, clear in_flight tracking and reset in_flight_count to 0
- [ ] On Pause, track in_flight_count at pause time for potential rollback
- [ ] On Error response, set error and potentially abort remaining in-flight

## Phase 5: Update UI Types and Sync

### 5.1 Update Sync Systems
- [ ] Create sync system for `ExecutionState` (execution plugin)
- [ ] Create sync system for `BufferDisplayData` (execution plugin)
- [ ] Ensure both are added to System entity on spawn

### 5.2 Update Client App
- [ ] Update imports to use `SystemState` from execution plugin
- [ ] Update imports to use `BufferDisplayData` from execution plugin
- [ ] Update handler calls: `StartProgram` → `Start`, etc.
- [ ] Add UI for `Validating` state (spinner with expandable details)
- [ ] Update button visibility logic based on can_* actions

### 5.3 Update "Program Table" to Buffer View
The "program table" is now the execution buffer table:
- [ ] Render `BufferDisplayData.lines` as table rows
- [ ] Use `ExecutionState.current_index` to highlight current row
- [ ] Gray out rows where `index < points_executed`
- [ ] Show source_name and source_type from ExecutionState

This change enables future streaming/generator support:
- New points appearing in the table automatically as they're added to buffer
- Same UI works for all source types

## Phase 6: Integration Testing

### 6.1 Test Execution Flow
- [ ] Test Load → Start → Executing flow
- [ ] Test Pause → Resume flow
- [ ] Test Stop during execution
- [ ] Test Stop during validation (cancel)
- [ ] Test validation failures (no program, robot disconnected)

### 6.2 Test Error Handling
- [ ] Test subsystem error propagation
- [ ] Test validation timeout (30 seconds)
- [ ] Test recovery from errors

### 6.3 Test Buffer-Centric Model
- [ ] Verify BufferDisplayData populates on Load
- [ ] Verify current_index advances during execution
- [ ] Verify UI highlights correct row
- [ ] Verify source_type and source_name display correctly

## Estimated Effort

| Phase | Effort | Risk |
|-------|--------|------|
| Phase 1: Subsystem Infrastructure | 4-5 hours | Low |
| Phase 2: Execution Handlers | 3-4 hours | Medium |
| Phase 3: Programs as Static Loader | 4-5 hours | Medium |
| Phase 4: Fanuc Updates | 2-3 hours | Medium |
| Phase 5: UI Updates | 4-5 hours | Low |
| Phase 6: Testing | 3-4 hours | Low |
| **Total** | **20-26 hours** | |

## Related Research References

When implementing, also reference:

- **`../streaming-execution/README.md`**: Sealed Buffer Pattern, completion logic
- **`../streaming-execution/implementation_spec.md`**: Detailed ToolpathBuffer changes
- **`./program-execution-relationship.md`**: Design rationale for buffer-centric model
- **`./types.md`**: Exact component definitions to implement
- **`./handler-ownership.md`**: Handler implementations with full code examples

