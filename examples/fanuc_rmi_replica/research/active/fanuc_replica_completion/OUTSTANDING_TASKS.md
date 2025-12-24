# Fanuc RMI Replica - Outstanding Tasks

**Last Updated**: December 24, 2025

---

## ✅ Completed (December 2025 Sessions)

### Framework Improvements
- ✅ **Automatic Query Invalidation (Phase 3)** - `respond_and_invalidate` pattern
- ✅ **Component Mutation Handlers** - `use_mut_component` implemented
- ✅ **API Migration** - All `listen_for_request_message` → new API
- ✅ **Entity Architecture** - System/Robot entity hierarchy working

### Code Quality
- ✅ **Position Display** - Uses correct `use_entity_component` pattern
- ✅ **Connection Flow** - Fixed Quick Settings popup bug
- ✅ **Audit use_components** - Verified existing usages are appropriate (finding entity IDs)

---

## Remaining Tasks

### Priority 1: Code Quality (~20 min)

#### 1. Fix Compiler Warnings
**Status**: NOT_STARTED

Fix these warnings for clean compilation:

| File | Line | Warning |
|------|------|---------|
| `client/src/pages/dashboard/control/program_display.rs` | 56 | Unused `system_entity_bits` |
| `server/src/plugins/execution.rs` | 74-80 | Dead code: `LoadedProgramData` fields |
| `server/src/plugins/program.rs` | 144 | Dead code: `Program::all_completed()` |
| `server/src/plugins/program.rs` | 181 | Dead code: `ExecutionBuffer::available_slots()` |

---

### Priority 2: API Consistency for Multi-Robot (~4 hr)

These enhance consistency for multi-robot scenarios but **work correctly now**.

#### 2. Convert Commands to Targeted Requests
**Status**: NOT_STARTED | **Effort**: 1 hr

Commands in `quick_commands.rs` that modify robot state:
- `SetSpeedOverride`, `InitializeRobot`, `AbortMotion`, `ResetRobot`

**Implementation**: Use `TargetedRequestMessage` trait and `use_mutation_targeted` on client.

#### 3. Add Targeting to Program Commands
**Status**: NOT_STARTED | **Effort**: 1 hr

Program commands for multi-robot support:
- `StartProgram`, `PauseProgram`, `ResumeProgram`, `StopProgram`, `LoadProgram`

#### 4. Make ConnectToRobot Targeted
**Status**: NOT_STARTED | **Effort**: 1 hr

Requires robot entities to exist before connection (loaded from DB).
Currently works as global message.

#### 5. Review ControlRequest Pattern
**Status**: NOT_STARTED | **Effort**: 30 min

`ControlRequest` embeds `entity_bits` in enum variant. Consider `TargetedMessage` pattern.

---

### Priority 3: UX Improvements

#### 6. Program State Persistence
**Status**: ✅ COMPLETE

Verified working - program stays open when navigating away and back.

---

### Priority 4: Future Enhancements (~6 hr)

#### 7. Server-Side Missing Subscription Warnings
**Status**: NOT_STARTED | **Effort**: 2 hr

Debug aid: warn when client subscribes to entity/component that doesn't exist.

**Implementation**: Check entity/component existence in sync system, send warning message.

#### 8. I/O Display Name Configuration
**Status**: NOT_STARTED | **Effort**: 3 hr

Custom display names for I/O values in robot connection settings.

**Implementation**: Add `display_names: HashMap<u8, String>` to `IoConfigState`.

#### 9. Check JogDefaultStep Units
**Status**: NOT_STARTED | **Effort**: 30 min

Verify if joint jog speed should be °/s or % by checking original Fanuc_RMI_API.

---

## Task Summary

| Priority | Tasks | Effort | Status |
|----------|-------|--------|--------|
| P1: Code Quality | 1 | 20 min | NOT_STARTED |
| P2: API Consistency | 4 | 4 hr | NOT_STARTED |
| P3: UX Improvements | 1 | - | ✅ COMPLETE |
| P4: Future Enhancements | 3 | 6 hr | NOT_STARTED |
| **TOTAL REMAINING** | **8** | **~10 hr** | |

