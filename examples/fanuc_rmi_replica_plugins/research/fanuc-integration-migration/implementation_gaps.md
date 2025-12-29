# Implementation Gaps

## ✅ Gap 1: FANUC Motion Handler (COMPLETED)

**Location:** `plugins/fanuc/src/motion.rs`

**Status:** IMPLEMENTED

The `fanuc_motion_handler_system` now:
1. Queries `RmiDriver` component from `FanucRobot` entities
2. Uses `TokioTasksRuntime` for async context
3. Builds `FrcLinearMotion` packets from `MotionCommandEvent`
4. Sends via `driver.0.send_packet(packet, PacketPriority::Standard)`
5. Tracks request_id in `FanucInFlightInstructions` resource

---

## ✅ Gap 2: Response Handling System (COMPLETED)

**Location:** `plugins/fanuc/src/motion.rs`

**Status:** IMPLEMENTED

Two new systems added:
- `fanuc_sent_instruction_system` - Maps request_id to sequence_id
- `fanuc_motion_response_system` - Processes instruction responses and updates DeviceStatus

---

## ✅ Gap 3: In-Flight Tracking Resource (COMPLETED)

**Location:** `plugins/fanuc/src/motion.rs`

**Status:** IMPLEMENTED

```rust
#[derive(Resource, Default)]
pub struct FanucInFlightInstructions {
    /// Map request_id (from send_packet) -> (entity, point_index)
    pub by_request: HashMap<u64, (Entity, usize)>,
    /// Map sequence_id (from SentInstructionInfo) -> (entity, point_index)
    pub by_sequence: HashMap<u32, (Entity, usize)>,
}
```

---

## Gap 4: Duet HTTP Client

**Location:** `plugins/duet/src/handler.rs`

**Current State:** Logs G-code but doesn't send HTTP request.

**Required:** Add reqwest HTTP client to send actual commands.

---

## Gap 5: Robot Entity Markers

**Location:** Robot spawning code

**Current State:** Robots are spawned with `FanucRobot` marker but not `FanucMotionDevice`.

**Required:** Add `FanucMotionDevice` marker when spawning robot entities for motion handling.

---

## Gap 6: ExecutionTarget Integration

**Location:** System entity setup

**Current State:** `ExecutionTarget` and `PrimaryMotion` markers not added.

**Required:** When setting up execution:
1. Add `ExecutionTarget { target: robot_entity }` to coordinator
2. Add `PrimaryMotion` marker to robot entity
3. Set robot as child of coordinator

---

## Priority Order

1. **[HIGH]** Gap 1: FANUC Motion Handler RMI Integration
2. **[HIGH]** Gap 2: Response Handling System  
3. **[MEDIUM]** Gap 3: In-Flight Tracking Resource
4. **[MEDIUM]** Gap 5: Robot Entity Markers
5. **[LOW]** Gap 4: Duet HTTP Client
6. **[LOW]** Gap 6: ExecutionTarget Integration

## Dependencies Graph

```
Gap 1 (Motion Handler)
   └── Gap 3 (In-Flight Tracking)
       └── Gap 2 (Response Handling)
           └── Gap 5 (Entity Markers)
               └── Gap 6 (ExecutionTarget)
```

Gap 4 (Duet) is independent and can be done in parallel.

