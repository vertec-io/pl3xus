# Implementation Specifications

## Overview

This document provides detailed implementation specifications for each TODO item, including code patterns, dependencies, and test criteria.

---

## TODO-001: ReadDin Implementation

### Specification

**Handler**: `handle_read_din`  
**Request Type**: `TargetedRequest<ReadDin>` (no authorization required - read-only)  
**Response Type**: `DinValueResponse`

### Current Signature
```rust
pub fn handle_read_din(
    mut requests: MessageReader<Request<TargetedRequest<ReadDin>>>,
)
```

### Required Changes

**Add System Parameters**:
```rust
pub fn handle_read_din(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut requests: MessageReader<Request<TargetedRequest<ReadDin>>>,
    robots: Query<(Option<&RmiDriver>, &RobotConnectionState), With<FanucRobot>>,
)
```

### Implementation Pattern

Follow the pattern from `handle_get_frame_data` (lines 829-984):

1. Enter Tokio runtime context
2. Parse target entity from request
3. Find connected robot with driver
4. Spawn background task
5. Subscribe to response channel
6. Send FrcReadDIN command
7. Wait for response with timeout
8. Convert u8 (0/1) to bool
9. Send response to client

### Code Template

```rust
pub fn handle_read_din(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut requests: MessageReader<Request<TargetedRequest<ReadDin>>>,
    robots: Query<(Option<&RmiDriver>, &RobotConnectionState), With<FanucRobot>>,
) {
    use fanuc_rmi::packets::{SendPacket, Command, ResponsePacket, CommandResponse};
    use fanuc_rmi::commands::FrcReadDIN;
    use std::time::Duration;

    let _guard = tokio_runtime.runtime().enter();

    for request in requests.read() {
        let targeted = request.get_request();
        let port_number = targeted.request.port_number;
        info!("📋 Handling ReadDin for port {} on target {}", port_number, targeted.target_id);

        // Find connected robot with driver
        let robot_info = robots.iter()
            .find(|(driver, state)| **state == RobotConnectionState::Connected && driver.is_some());

        if let Some((Some(driver), _)) = robot_info {
            let driver = driver.0.clone();
            let request = request.clone();

            tokio_runtime.spawn_background_task(move |_ctx| async move {
                let packet = SendPacket::Command(Command::FrcReadDIN(FrcReadDIN {
                    port_number,
                }));

                let mut response_rx = driver.response_tx.subscribe();

                if let Err(e) = driver.send_packet(packet, PacketPriority::Standard) {
                    bevy::log::error!("Failed to send FrcReadDIN: {}", e);
                    let response = DinValueResponse {
                        port_number,
                        port_value: false,
                    };
                    let _ = request.respond(response);
                    return;
                }

                let result = tokio::time::timeout(Duration::from_secs(5), async {
                    while let Ok(response) = response_rx.recv().await {
                        if let ResponsePacket::CommandResponse(CommandResponse::FrcReadDIN(resp)) = response {
                            return Some(resp);
                        }
                    }
                    None
                }).await;

                match result {
                    Ok(Some(resp)) => {
                        if resp.error_id != 0 {
                            bevy::log::error!("Robot error reading DIN[{}]: error_id={}", port_number, resp.error_id);
                        } else {
                            bevy::log::info!("✅ Read DIN[{}] = {}", port_number, resp.port_value);
                        }
                        let response = DinValueResponse {
                            port_number,
                            port_value: resp.port_value != 0,
                        };
                        let _ = request.respond(response);
                    }
                    Ok(None) => {
                        bevy::log::error!("No response received for DIN[{}]", port_number);
                        let response = DinValueResponse {
                            port_number,
                            port_value: false,
                        };
                        let _ = request.respond(response);
                    }
                    Err(_) => {
                        bevy::log::error!("Timeout waiting for DIN[{}] response", port_number);
                        let response = DinValueResponse {
                            port_number,
                            port_value: false,
                        };
                        let _ = request.respond(response);
                    }
                }
            });
        } else {
            warn!("ReadDin: No connected robot");
            let response = DinValueResponse {
                port_number,
                port_value: false,
            };
            if let Err(e) = request.clone().respond(response) {
                error!("Failed to send response: {:?}", e);
            }
        }
    }
}
```

### Test Criteria

- [ ] Compiles without errors
- [ ] Returns actual value from connected robot
- [ ] Returns false when no robot connected
- [ ] Handles timeout gracefully
- [ ] Handles robot errors gracefully
- [ ] Logs appropriate messages

---

## TODO-002: ReadDinBatch Implementation

### Specification

**Handler**: `handle_read_din_batch`  
**Request Type**: `TargetedRequest<ReadDinBatch>`  
**Response Type**: `DinBatchResponse`

### Strategy

Since Fanuc RMI doesn't have a native batch read command, we have two options:

**Option A**: Sequential reads (simpler, slower)
- Read each port one at a time
- Wait for each response before next request

**Option B**: Concurrent reads (faster, more complex)
- Send all requests concurrently
- Collect responses as they arrive
- Wait for all with timeout

**Recommendation**: Start with Option A for simplicity, optimize to Option B if needed.

### Implementation Notes

- Reuse the ReadDin pattern for each port
- Collect results in a Vec<(u16, bool)>
- Handle partial failures (some ports succeed, some fail)
- Consider overall timeout for batch operation

---

## TODO-003: ReadAin Implementation

### Specification

**Handler**: `handle_read_ain`  
**Request Type**: `TargetedRequest<ReadAin>`  
**Response Type**: `AinValueResponse`

### Implementation

Nearly identical to ReadDin, but:
- Use `FrcReadAIN` command instead of `FrcReadDIN`
- Response value is `f64` instead of `u8`
- No bool conversion needed

### Code Differences from ReadDin

```rust
// Command
let packet = SendPacket::Command(Command::FrcReadAIN(FrcReadAIN {
    port_number,
}));

// Response matching
if let ResponsePacket::CommandResponse(CommandResponse::FrcReadAIN(resp)) = response {
    return Some(resp);
}

// Response construction
let response = AinValueResponse {
    port_number,
    port_value: resp.port_value,  // Already f64
};
```

---

## TODO-004: ReadGin Implementation

### Specification

**Handler**: `handle_read_gin`  
**Request Type**: `TargetedRequest<ReadGin>`  
**Response Type**: `GinValueResponse`

### Implementation

Nearly identical to ReadDin, but:
- Use `FrcReadGIN` command instead of `FrcReadDIN`
- Response value is `u32` instead of `u8`
- No bool conversion needed

---

## TODO-005: UpdateJogSettings Implementation

### Specification

**Handler**: `handle_update_jog_settings`  
**Request Type**: `UpdateJogSettings` (non-targeted, global)  
**Response Type**: `UpdateJogSettingsResponse`

### Current Signature
```rust
fn handle_update_jog_settings(
    mut requests: MessageReader<Request<UpdateJogSettings>>,
    db: Option<Res<DatabaseResource>>,
)
```

### Required Changes

Need to determine which robot connection to update. Options:

**Option A**: Add `robot_connection_id` to request
**Option B**: Update currently active robot connection
**Option C**: Update all robot connections

**Recommendation**: Option B - update the currently active robot connection.

### Additional System Parameters Needed

```rust
fn handle_update_jog_settings(
    mut requests: MessageReader<Request<UpdateJogSettings>>,
    db: Option<Res<DatabaseResource>>,
    robot_query: Query<&ConnectionState, With<FanucRobot>>,
)
```

### Database Query Function

Add to `database/queries.rs`:

```rust
pub fn update_jog_settings(
    conn: &Connection,
    robot_connection_id: i64,
    settings: &UpdateJogSettings,
) -> anyhow::Result<()> {
    conn.execute(
        "UPDATE robot_connections SET
            default_cartesian_jog_speed = ?,
            default_cartesian_jog_step = ?,
            default_joint_jog_speed = ?,
            default_joint_jog_step = ?,
            default_rotation_jog_speed = ?,
            default_rotation_jog_step = ?
         WHERE id = ?",
        params![
            settings.cartesian_jog_speed,
            settings.cartesian_jog_step,
            settings.joint_jog_speed,
            settings.joint_jog_step,
            settings.rotation_jog_speed,
            settings.rotation_jog_step,
            robot_connection_id,
        ],
    )?;
    Ok(())
}
```

### Implementation Pattern

1. Get active robot connection ID from ConnectionState
2. Call database update function
3. Return success/error response

---

## Summary

All implementations follow established patterns in the codebase. The I/O read operations are nearly identical, differing only in command type and response value type.

