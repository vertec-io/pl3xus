# TODO Inventory - Complete List

## Overview

This document provides a complete inventory of all TODO items found in `handlers.rs` that require implementation.

**Last Updated**: 2025-12-31  
**Total Items**: 5  
**Status**: All items documented

---

## TODO-001: ReadDin - Read Digital Input

**Location**: `handlers.rs:1430-1440`

**Current Code**:
```rust
pub fn handle_read_din(
    mut requests: MessageReader<Request<TargetedRequest<ReadDin>>>,
) {
    for request in requests.read() {
        let targeted = request.get_request();
        let port_number = targeted.request.port_number;
        info!("📋 Handling ReadDin for port {} on target {}", port_number, targeted.target_id);

        // TODO: Read from connected robot driver when available
        // For now, return mock value (false)
        let response = DinValueResponse {
            port_number,
            port_value: false,
        };
        // ...
    }
}
```

**Issue**: Returns hardcoded `false` instead of reading actual value from robot

**Required Implementation**:
- Use Fanuc RMI `FrcReadDIN` command
- Parse target entity to get robot driver
- Send command and wait for response
- Return actual port value from robot

**Dependencies**:
- `RmiDriver` component on target entity
- `TokioTasksRuntime` resource
- Fanuc RMI API: `FrcReadDIN` command and `FrcReadDINResponse`

**Priority**: High (Core I/O functionality)

---

## TODO-002: ReadDinBatch - Read Multiple Digital Inputs

**Location**: `handlers.rs:1453-1465`

**Current Code**:
```rust
pub fn handle_read_din_batch(
    mut requests: MessageReader<Request<TargetedRequest<ReadDinBatch>>>,
) {
    for request in requests.read() {
        let targeted = request.get_request();
        let port_numbers = &targeted.request.port_numbers;
        info!("📋 Handling ReadDinBatch for {} ports on target {}", port_numbers.len(), targeted.target_id);

        // TODO: Read from connected robot driver when available
        // For now, return mock values (all false)
        let values: Vec<(u16, bool)> = port_numbers.iter()
            .map(|&port| (port, false))
            .collect();
        // ...
    }
}
```

**Issue**: Returns hardcoded `false` for all ports instead of reading actual values

**Required Implementation**:
- Iterate through requested port numbers
- Send `FrcReadDIN` command for each port
- Collect all responses
- Return actual port values from robot

**Alternative Approach**:
- If Fanuc RMI supports batch reads, use that
- Otherwise, send individual commands concurrently

**Dependencies**:
- Same as TODO-001
- May need concurrent request handling

**Priority**: High (Core I/O functionality)

---

## TODO-003: ReadAin - Read Analog Input

**Location**: `handlers.rs:1621-1632`

**Current Code**:
```rust
pub fn handle_read_ain(
    mut requests: MessageReader<Request<TargetedRequest<ReadAin>>>,
) {
    for request in requests.read() {
        let targeted = request.get_request();
        let port_number = targeted.request.port_number;
        info!("📋 Handling ReadAin for port {} on target {}", port_number, targeted.target_id);

        // TODO: Read from connected robot driver when available
        // For now, return mock value (0.0)
        let response = AinValueResponse {
            port_number,
            port_value: 0.0,
        };
        // ...
    }
}
```

**Issue**: Returns hardcoded `0.0` instead of reading actual analog value

**Required Implementation**:
- Use Fanuc RMI `FrcReadAIN` command
- Parse target entity to get robot driver
- Send command and wait for response
- Return actual analog value from robot

**Dependencies**:
- `RmiDriver` component on target entity
- `TokioTasksRuntime` resource
- Fanuc RMI API: `FrcReadAIN` command and `FrcReadAINResponse`

**Priority**: High (Core I/O functionality)

---

## TODO-004: ReadGin - Read Group Input

**Location**: `handlers.rs:1694-1705`

**Current Code**:
```rust
pub fn handle_read_gin(
    mut requests: MessageReader<Request<TargetedRequest<ReadGin>>>,
) {
    for request in requests.read() {
        let targeted = request.get_request();
        let port_number = targeted.request.port_number;
        info!("📋 Handling ReadGin for port {} on target {}", port_number, targeted.target_id);

        // TODO: Read from connected robot driver when available
        // For now, return mock value (0)
        let response = GinValueResponse {
            port_number,
            port_value: 0,
        };
        // ...
    }
}
```

**Issue**: Returns hardcoded `0` instead of reading actual group input value

**Required Implementation**:
- Use Fanuc RMI `FrcReadGIN` command
- Parse target entity to get robot driver
- Send command and wait for response
- Return actual group value from robot

**Dependencies**:
- `RmiDriver` component on target entity
- `TokioTasksRuntime` resource
- Fanuc RMI API: `FrcReadGIN` command and `FrcReadGINResponse`

**Priority**: High (Core I/O functionality)

---

## TODO-005: UpdateJogSettings - Save Jog Settings to Database

**Location**: `handlers.rs:1949-1952`

**Current Code**:
```rust
fn handle_update_jog_settings(
    mut requests: MessageReader<Request<UpdateJogSettings>>,
    db: Option<Res<DatabaseResource>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling UpdateJogSettings: cartesian_speed={}", inner.cartesian_jog_speed);

        // TODO: Save to database when available
        let _ = db;
        let response = UpdateJogSettingsResponse { success: true, error: None };
        // ...
    }
}
```

**Issue**: Does not persist jog settings to database

**Required Implementation**:
- Add database query function to save jog settings
- Jog settings are stored per robot connection in `robot_connections` table
- Need to identify which robot connection to update
- Update the appropriate columns in the database

**Database Schema** (from `schema.rs`):
```sql
CREATE TABLE robot_connections (
    -- ... other fields ...
    default_cartesian_jog_speed REAL DEFAULT 10.0,
    default_cartesian_jog_step REAL DEFAULT 1.0,
    default_joint_jog_speed REAL DEFAULT 10.0,
    default_joint_jog_step REAL DEFAULT 1.0,
    default_rotation_jog_speed REAL DEFAULT 5.0,
    default_rotation_jog_step REAL DEFAULT 1.0,
    -- ...
)
```

**Dependencies**:
- Database connection via `DatabaseResource`
- Need to determine which robot_connection_id to update
- May need to add query function to `database/queries.rs`

**Priority**: Medium (Configuration persistence)

---

## Summary Statistics

| Category | Count |
|----------|-------|
| I/O Read Operations | 4 |
| Database Operations | 1 |
| **Total** | **5** |

| Priority | Count |
|----------|-------|
| High | 4 |
| Medium | 1 |

## Implementation Order Recommendation

1. **TODO-001**: ReadDin (simplest I/O read, establishes pattern)
2. **TODO-003**: ReadAin (similar to ReadDin, different data type)
3. **TODO-004**: ReadGin (similar to ReadDin, different data type)
4. **TODO-002**: ReadDinBatch (builds on ReadDin, adds batching)
5. **TODO-005**: UpdateJogSettings (database operation, different pattern)

