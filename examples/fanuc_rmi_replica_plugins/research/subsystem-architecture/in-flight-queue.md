# In-Flight Command Queue (Lookahead)

## Problem Statement

The current orchestrator uses a simple `ready_for_next` boolean to control command flow:

```rust
// Current pattern - ONE command at a time
if motion_status.ready_for_next {
    let point = buffer.pop();
    motion_events.write(...);
    status.ready_for_next = false;  // Wait for response
}
// Response arrives:
status.ready_for_next = in_flight.is_empty();  // Only ready when ALL complete
```

**This doesn't work for continuous motion controllers like FANUC.**

Continuous motion (CNT) requires a queue of upcoming points for the controller to:
1. Calculate smooth blending between points
2. Maintain velocity during transitions
3. Avoid jerky stop-start motion

**Empirical requirement**: FANUC needs **minimum 5 points** in the controller's queue
for smooth continuous motion. With only 2 points, motion becomes stuttery.

## Proposed Solution: In-Flight Queue with Capacity

### Concept: `in_flight_capacity` vs `ready_for_next`

Replace the boolean `ready_for_next` with a capacity-based model:

```rust
#[derive(Component, Debug, Clone)]
pub struct DeviceStatus {
    /// True if the device is connected and operational
    pub is_connected: bool,
    
    /// Maximum number of commands that can be in-flight simultaneously
    /// For FANUC continuous motion: typically 5-10
    pub in_flight_capacity: u32,
    
    /// Current number of commands in-flight (sent but not confirmed complete)
    pub in_flight_count: u32,
    
    /// Number of motions confirmed complete (monotonically increasing)
    pub completed_count: u32,
    
    /// Error message if device is in error state
    pub error: Option<String>,
}

impl DeviceStatus {
    /// Check if device can accept another command
    pub fn ready_for_next(&self) -> bool {
        self.error.is_none() && 
        self.is_connected && 
        self.in_flight_count < self.in_flight_capacity
    }
    
    /// Check if queue is at minimum fill level for smooth motion
    pub fn queue_needs_fill(&self, min_queue_depth: u32) -> bool {
        self.ready_for_next() && self.in_flight_count < min_queue_depth
    }
}
```

### Orchestrator Changes

The orchestrator should now fill the queue up to capacity:

```rust
pub fn orchestrator_system(
    mut coordinator_query: Query<(Entity, &mut BufferState, &mut ToolpathBuffer), ...>,
    device_status_query: Query<(Entity, &DeviceStatus), With<PrimaryMotion>>,
    mut motion_events: MessageWriter<MotionCommandEvent>,
) {
    for (coordinator_entity, mut state, mut buffer) in coordinator_query.iter_mut() {
        // ... get device status ...
        
        // Send multiple points up to capacity
        // Loop while device can accept more AND buffer has points
        while motion_status.ready_for_next() {
            let Some(point) = buffer.pop() else {
                break; // Buffer empty
            };
            
            motion_events.write(MotionCommandEvent { ... });
            
            // Update in-flight count (will be updated in motion handler)
            // Note: Actual increment happens in the motion handler
        }
        
        // Update state with new current index
        // ...
    }
}
```

### Motion Handler Changes (FANUC)

```rust
pub fn fanuc_motion_handler_system(
    mut motion_events: MessageReader<MotionCommandEvent>,
    mut device_query: Query<&mut DeviceStatus, With<FanucMotionDevice>>,
    mut in_flight: ResMut<FanucInFlightInstructions>,
    // ...
) {
    for event in motion_events.read() {
        let Ok(mut status) = device_query.get_mut(event.device) else { continue };
        
        // Send motion to robot...
        match driver.0.send_packet(packet, PacketPriority::Standard) {
            Ok(request_id) => {
                in_flight.record_sent(request_id, event.device, event.point.index);
                
                // Increment in-flight count
                status.in_flight_count += 1;
            }
            Err(e) => {
                status.error = Some(format!("Failed to send motion: {}", e));
            }
        }
    }
}

pub fn fanuc_motion_response_system(
    mut in_flight: ResMut<FanucInFlightInstructions>,
    mut device_query: Query<&mut DeviceStatus, With<FanucMotionDevice>>,
    // ...
) {
    // ... on response received ...
    if let Some((entity, point_index)) = in_flight.handle_completion(seq_id) {
        if let Ok(mut status) = device_query.get_mut(entity) {
            status.completed_count += 1;
            status.in_flight_count = status.in_flight_count.saturating_sub(1);
            // No longer setting ready_for_next = false!
            // Device is ready if in_flight_count < in_flight_capacity
        }
    }
}
```

## Configuration

Different motion types require different queue depths:

| Motion Type | Recommended Queue Depth | Notes |
|-------------|------------------------|-------|
| Fine (FINE) | 1 | Stop at each point, no blending |
| CNT 100 | 5-10 | Full continuous motion |
| CNT 50 | 3-5 | Moderate blending |
| Joint motion | 3-5 | Less blending sensitivity |

Consider making this configurable per-system or per-motion-type.

## Edge Cases

### 1. Pause During In-Flight Commands

When paused, the robot will complete already-sent commands before stopping.
The orchestrator should:
- Stop sending new commands immediately
- Track which commands are still in-flight
- Wait for in-flight commands to complete OR be aborted

```rust
// In pause handler
*buffer_state = BufferState::Paused {
    paused_at_index: current_idx,
    in_flight_at_pause: status.in_flight_count,  // NEW: track in-flight
};
```

### 2. Stop During In-Flight Commands

On stop, send `FrcAbort` to cancel all pending commands.
Clear the `in_flight` tracking since those commands won't complete normally.

```rust
// In stop handler / reaction system
in_flight.clear();
status.in_flight_count = 0;
```

### 3. Error Response for In-Flight Command

If one command fails, should we:
- **Abort all**: Stop immediately, report error
- **Skip and continue**: Log error, continue with next commands

Recommendation: **Abort all** - motion errors should stop execution.

### 4. Buffer Underrun During Execution

If buffer empties while in-flight commands exist:
- For **sealed buffers**: Wait for in-flight to complete → Complete
- For **streaming**: Transition to AwaitingPoints (but keep in-flight going)

```rust
// In orchestrator
if buffer.is_empty() {
    if buffer.is_sealed() {
        // No more points coming, wait for in-flight to drain
        if status.in_flight_count == 0 {
            // All done
        }
    } else {
        // Streaming mode - still in-flight commands running
        // but need more points from producer
    }
}
```

## Implementation Checklist

### DeviceStatus Changes
- [ ] Add `in_flight_capacity: u32` field
- [ ] Add `in_flight_count: u32` field
- [ ] Add `ready_for_next()` method
- [ ] Update all creation sites to initialize capacity (default 5 for FANUC)
- [ ] Remove boolean `ready_for_next` field

### Orchestrator Changes
- [ ] Change single-point dispatch to loop while `ready_for_next()`
- [ ] Consider burst limit (don't send 100 points in one tick)

### FANUC Motion Handler Changes
- [ ] Increment `in_flight_count` on send
- [ ] Decrement `in_flight_count` on response
- [ ] Remove `ready_for_next = false` after send
- [ ] Remove `ready_for_next = in_flight.is_empty()` on response

### BufferState Changes
- [ ] Consider adding `in_flight_at_pause` to Paused variant
- [ ] Update completion detection to account for in-flight

## Alternative: Minimum Queue Depth Signal

Another approach is a "queue low" signal rather than capacity:

```rust
impl DeviceStatus {
    /// Returns true if the device's internal queue is below the minimum depth
    /// and can accept more commands. The orchestrator should fill to min_depth.
    pub fn queue_low(&self) -> bool {
        self.in_flight_count < self.min_queue_depth
    }
}
```

The orchestrator would then fill whenever `queue_low()` is true, rather than
filling to full capacity. This is simpler and prevents over-buffering.

## Recommendation

Use the **capacity-based model** with a sensible default:

1. `in_flight_capacity = 8` for FANUC CNT motion (provides headroom)
2. Orchestrator sends up to capacity each tick
3. Consider a per-tick burst limit of 5 to avoid overwhelming a single tick

This approach:
- Maintains the existing feedback loop structure
- Allows smooth continuous motion
- Is backwards-compatible (capacity=1 gives old behavior)
- Works for different motion types by adjusting capacity

