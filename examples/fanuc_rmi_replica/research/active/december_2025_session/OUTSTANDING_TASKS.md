# December 2025 Session: Outstanding Tasks

## ✅ Completed This Session

### Position Display Pattern
**Task ID**: `rREd5d3xi4GXWRurrVCMEx` | **Status**: ✅ COMPLETE

Already correctly using `use_entity_component` with `robot_entity_id`.

### Entity Architecture
**Task ID**: `oNZ2JJGqazKeqWgfRkazZZ` | **Status**: ✅ COMPLETE

Entity hierarchy is working: `system_entity_id` and `robot_entity_id` are properly distinguished. Components live on the correct entities. All 14+ client files updated to subscribe to correct entity.

### Imperative Request Pattern Documentation
**Task ID**: `tUbhDL8HYVFxvq1NN67MmA` | **Status**: ✅ COMPLETE

Documented in LESSONS_LEARNED.md and PL3XUS_CHEATSHEET.md:
- `use_request` for imperative triggers (I/O refresh, frame/tool loading)
- `use_query` for cached reads with server invalidation
- `GetFrameData`, `GetToolData`, `GetActiveFrameTool` are correctly using `use_request`

### Connection Flow Fix
**Task ID**: `6YmxBgMoCz6XrWeucpUKvP` | **Status**: ✅ COMPLETE

Fixed Quick Settings popup immediate close bug. Fixed all ConnectionState subscriptions.

---

## Remaining Tasks

### Priority 1: Code Quality

#### 1. Audit All use_components Usages
**Task ID**: `gkfRR9V5P6s8knRcSu7gg7` | **Status**: NOT_STARTED

Search for all `use_components::<` usages and verify they're appropriate. Most should probably be `use_entity_component` with the active robot entity.

---

### Priority 2: API Enhancements (Future Work)

These are enhancements that would improve consistency but are not blocking:

#### 2. Convert Commands to Targeted Requests
**Task ID**: `e6hLnBP8kdh4X1z9f3CHhS` | **Status**: NOT_STARTED

Commands that modify robot state could use targeted request pattern:
- `SetSpeedOverride`, `InitializeRobot`, `AbortMotion`, `ResetRobot`

**Note**: These work correctly now. This is a consistency improvement.

#### 3. Add Targeting to Program Commands
**Task ID**: `ppdDaNUKi6rZ3GcQukFtd3` | **Status**: NOT_STARTED

Program commands could target the robot entity for multi-robot support:
- `StartProgram`, `PauseProgram`, `ResumeProgram`, `StopProgram`, `LoadProgram`

#### 4. Make ConnectToRobot a Targeted Message
**Task ID**: `bn2KJewcNuSNeqi1EzZ7xh` | **Status**: NOT_STARTED

Requires robot entities to exist before connection (loaded from DB). Currently works as global message.

#### 5. Review ControlRequest Pattern
**Task ID**: `7UtUBAFmALXHuEwd2ZWuKD` | **Status**: NOT_STARTED

`ControlRequest` embeds `entity_bits` in the enum variant. Consider using `TargetedMessage` pattern for consistency.

---

### Priority 3: UX Improvements

#### 6. Program State Persistence When Navigating
**Task ID**: `36PxJz9z7UiKQUu8sf1TkL` | **Status**: NOT_STARTED

When leaving and returning to program menu, the previously open program should stay open.

---

### Priority 4: Future Enhancements

#### 7. Add Server-Side Notifications for Missing Subscriptions
**Task ID**: `fT6HPBYrDr8AjGA2XgU5hb` | **Status**: NOT_STARTED

Debug aid: warn when client subscribes to entity/component that doesn't exist.

#### 8. Add I/O Display Name Configuration
**Task ID**: `nyynUpmrDm56PCpUwAqdaR` | **Status**: NOT_STARTED

Custom display names for I/O values in robot connection settings.

#### 9. Check JogDefaultStep Units
**Task ID**: `ncHq9EzTM47rAmbQjNoHzD` | **Status**: NOT_STARTED

Verify if joint jog speed should be °/s or % by checking original Fanuc_RMI_API.

---

## ❌ Cancelled Tasks

### Test Connection Flow with Playwright
**Task ID**: `4zEnBRGL8a7e5wyndFJVUr` | **Status**: CANCELLED

Connection flow was manually verified working. Playwright testing is optional future work.

---

## Task Summary

| Category | Count | Notes |
|----------|-------|-------|
| ✅ Completed | 4 | Entity arch, position display, docs, connection fix |
| 🔧 Code Quality | 1 | Audit use_components |
| 🚀 API Enhancements | 4 | Future consistency improvements |
| 💡 UX Improvements | 1 | Program state persistence |
| 📋 Future Enhancements | 3 | Debug aids, config, verification |
| ❌ Cancelled | 1 | Playwright testing |

