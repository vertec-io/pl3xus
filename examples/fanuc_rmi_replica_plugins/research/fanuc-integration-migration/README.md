# FANUC Integration Migration Research

**Date:** 2025-12-28
**Status:** In Progress

## Overview

This document tracks the migration of real FANUC RMI execution from `plugins/src/robot/` to the dedicated `plugins/fanuc/` crate, and integration with the new execution orchestrator architecture.

## Current State

### Working Implementation (plugins/src/robot/)

The original `plugins/src/robot/` module contains a **fully working** FANUC RMI execution implementation:

| Component | File | Status |
|-----------|------|--------|
| `RmiDriver` wrapper | `connection.rs` | ✅ Working |
| `FanucDriver` connection | `connection.rs` | ✅ Working |
| `Program` component | `program.rs` | ✅ Working |
| `ExecutionBuffer` (in-flight tracking) | `program.rs` | ✅ Working |
| `orchestrator_dispatch` system | `program.rs` | ✅ Working |
| `process_instruction_responses` | `program.rs` | ✅ Working |
| `build_motion_packet()` | `program.rs` | ✅ Working |
| Response channel handling | `connection.rs` | ✅ Working |

### New Architecture (plugins/fanuc/)

The `plugins/fanuc/` crate has been set up with placeholder implementations:

| Component | File | Status |
|-----------|------|--------|
| `FanucMotionDevice` marker | `motion.rs` | ✅ Placeholder |
| `fanuc_motion_handler_system` | `motion.rs` | 🟡 Logs only, no RMI |
| `FanucConversion` trait | `conversion.rs` | ✅ Complete |
| `robot_pose_to_fanuc_position()` | `motion.rs` | ✅ Complete |

### Execution Plugin (plugins/execution/)

The generic execution orchestrator is implemented:

| Component | File | Status |
|-----------|------|--------|
| `ExecutionCoordinator` | `components/coordinator.rs` | ✅ Complete |
| `ToolpathBuffer` | `components/buffer.rs` | ✅ Complete |
| `MotionCommandEvent` | `systems/orchestrator.rs` | ✅ Complete |
| `DeviceStatus` | `systems/orchestrator.rs` | ✅ Complete |
| `execution_orchestrator_system` | `systems/orchestrator.rs` | ✅ Complete |

## Gap Analysis

### Gap 1: FANUC Motion Handler Needs RmiDriver Integration

**Current:** `fanuc_motion_handler_system` logs motion commands but doesn't send them to the robot.

**Required:** Connect to `RmiDriver` component and send actual `SendPacket` instructions.

**Reference Implementation:** See `plugins/src/robot/program.rs`:
- `orchestrator_dispatch()` - Shows how to get RmiDriver and send packets
- `build_motion_packet()` - Shows how to build FrcLinearMotion instructions
- `process_instruction_responses()` - Shows response handling

### Gap 2: Two Parallel Execution Paths

Currently there are TWO execution paths:

1. **Old Path** (plugins/src/robot/program.rs):
   - `Program` component → `orchestrator_dispatch` → `RmiDriver.send_packet()`
   - Uses `ExecutionBuffer` for tracking in-flight instructions
   - Uses `RmiExecutionResponseChannel` for completion feedback

2. **New Path** (plugins/execution/ + plugins/fanuc/):
   - `ExecutionCoordinator` → `ToolpathBuffer` → `MotionCommandEvent`
   - `fanuc_motion_handler_system` receives events but doesn't execute
   - Uses `DeviceStatus` for completion feedback

**Decision Required:** Which path to use? Options:
- **Option A:** Enhance new path with RmiDriver integration
- **Option B:** Keep old path, deprecate new architecture
- **Option C:** Merge: use new architecture but with old execution code

### Gap 3: Response Tracking Differences

**Old System:** Uses `ExecutionBuffer` with `in_flight_by_request` and `in_flight_by_sequence` HashMaps to track instructions and handle responses.

**New System:** Uses `DeviceStatus` component with `ready_for_next` flag and `completed_count`.

The new system is simpler but may not handle the FANUC sequence tracking correctly.

## Implementation Plan

### Phase 1: Wire Up RmiDriver in New Handler

1. Add `RmiDriver` component query to `fanuc_motion_handler_system`
2. Import fanuc_rmi types for building packets
3. Build `FrcLinearMotion` instruction from event data
4. Send via `driver.send_packet()`
5. Track request_id for completion

### Phase 2: Add Response Handling

1. Add `RmiExecutionResponseChannel` component query
2. Create system to process responses
3. Map sequence_id to point_index
4. Update `DeviceStatus` on completion

### Phase 3: Integrate with ExecutionCoordinator

1. Add `FanucMotionDevice` marker when spawning robot entities
2. Set robot as execution target via `ExecutionTarget` component
3. Test end-to-end flow

## Files to Modify

1. `plugins/fanuc/src/motion.rs` - Add RMI integration
2. `plugins/fanuc/src/plugin.rs` - Add response handling system
3. `plugins/fanuc/src/lib.rs` - Re-export new components

## Dependencies

- `fanuc_rmi` crate (already a dependency)
- `RmiDriver` component from `plugins/src/robot/connection.rs`
- `bevy_tokio_tasks` for async context

## Related Research

- `research/execution-plugin/` - Execution architecture design
- `research/coordinate-abstraction/` - Quaternion/WPR conversion

