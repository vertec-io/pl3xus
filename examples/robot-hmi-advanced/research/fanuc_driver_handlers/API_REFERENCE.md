# Fanuc RMI API Reference

## Overview

This document provides a reference for the Fanuc RMI API commands needed to implement the TODO handlers.

**Source**: `/home/apino/dev/Fanuc_RMI_API/fanuc_rmi/`

---

## Digital Input Commands

### FrcReadDIN

**Purpose**: Read a single digital input port value

**Command Structure**:
```rust
use fanuc_rmi::commands::FrcReadDIN;

let command = FrcReadDIN {
    port_number: u16,  // Port number to read (1-based)
};
```

**Response Structure**:
```rust
use fanuc_rmi::commands::FrcReadDINResponse;

struct FrcReadDINResponse {
    error_id: u32,      // 0 = success, non-zero = error
    port_number: u16,   // Echo of requested port
    port_value: u8,     // 0 = false, 1 = true
}
```

**Usage Pattern**:
```rust
use fanuc_rmi::packets::{SendPacket, Command, ResponsePacket, CommandResponse};

let packet = SendPacket::Command(Command::FrcReadDIN(FrcReadDIN {
    port_number: 5,
}));

// Subscribe to responses before sending
let mut response_rx = driver.response_tx.subscribe();

driver.send_packet(packet, PacketPriority::Standard)?;

// Wait for response
while let Ok(response) = response_rx.recv().await {
    if let ResponsePacket::CommandResponse(CommandResponse::FrcReadDIN(resp)) = response {
        if resp.error_id == 0 {
            let value = resp.port_value != 0; // Convert to bool
            // Use value...
        }
        break;
    }
}
```

**Notes**:
- Port numbers are typically 1-based
- Response `port_value` is u8: 0 or 1
- Convert to bool for application use

---

## Analog Input Commands

### FrcReadAIN

**Purpose**: Read a single analog input port value

**Command Structure**:
```rust
use fanuc_rmi::commands::FrcReadAIN;

let command = FrcReadAIN {
    port_number: u16,  // Port number to read (1-based)
};
```

**Response Structure**:
```rust
use fanuc_rmi::commands::FrcReadAINResponse;

struct FrcReadAINResponse {
    error_id: u32,      // 0 = success, non-zero = error
    port_number: u16,   // Echo of requested port
    port_value: f64,    // Analog value (range depends on robot config)
}
```

**Usage Pattern**:
```rust
let packet = SendPacket::Command(Command::FrcReadAIN(FrcReadAIN {
    port_number: 3,
}));

let mut response_rx = driver.response_tx.subscribe();
driver.send_packet(packet, PacketPriority::Standard)?;

while let Ok(response) = response_rx.recv().await {
    if let ResponsePacket::CommandResponse(CommandResponse::FrcReadAIN(resp)) = response {
        if resp.error_id == 0 {
            let value = resp.port_value;
            // Use value...
        }
        break;
    }
}
```

**Notes**:
- Analog values are f64
- Range depends on robot configuration (typically 0-4095 for 12-bit ADC or scaled values)

---

## Group Input Commands

### FrcReadGIN

**Purpose**: Read a group input port value (multiple bits as single value)

**Command Structure**:
```rust
use fanuc_rmi::commands::FrcReadGIN;

let command = FrcReadGIN {
    port_number: u16,  // Group port number to read (1-based)
};
```

**Response Structure**:
```rust
use fanuc_rmi::commands::FrcReadGINResponse;

struct FrcReadGINResponse {
    error_id: u32,      // 0 = success, non-zero = error
    port_number: u16,   // Echo of requested port
    port_value: u32,    // Group value (typically 0-255 for 8-bit groups)
}
```

**Usage Pattern**:
```rust
let packet = SendPacket::Command(Command::FrcReadGIN(FrcReadGIN {
    port_number: 1,
}));

let mut response_rx = driver.response_tx.subscribe();
driver.send_packet(packet, PacketPriority::Standard)?;

while let Ok(response) = response_rx.recv().await {
    if let ResponsePacket::CommandResponse(CommandResponse::FrcReadGIN(resp)) = response {
        if resp.error_id == 0 {
            let value = resp.port_value;
            // Use value...
        }
        break;
    }
}
```

**Notes**:
- Group I/O allows reading multiple bits as a single integer value
- Typical range is 0-255 for 8-bit groups, but can be larger
- Useful for reading multiple related signals efficiently

---

## Common Patterns

### Async Read Pattern with Timeout

All I/O read operations should follow this pattern:

```rust
use std::time::Duration;

tokio_runtime.spawn_background_task(move |mut ctx| async move {
    let packet = SendPacket::Command(Command::FrcReadXXX(/* ... */));
    
    // Subscribe BEFORE sending to avoid race condition
    let mut response_rx = driver.response_tx.subscribe();
    
    if let Err(e) = driver.send_packet(packet, PacketPriority::Standard) {
        bevy::log::error!("Failed to send command: {}", e);
        // Send error response to client
        return;
    }
    
    // Wait for response with timeout
    let result = tokio::time::timeout(Duration::from_secs(5), async {
        while let Ok(response) = response_rx.recv().await {
            if let ResponsePacket::CommandResponse(CommandResponse::FrcReadXXX(resp)) = response {
                return Some(resp);
            }
        }
        None
    }).await;
    
    match result {
        Ok(Some(resp)) => {
            if resp.error_id != 0 {
                bevy::log::error!("Robot error: {}", resp.error_id);
                // Send error response
            } else {
                bevy::log::info!("✅ Read successful");
                // Send success response with value
            }
        }
        Ok(None) => {
            bevy::log::error!("No response received");
            // Send error response
        }
        Err(_) => {
            bevy::log::error!("Timeout waiting for response");
            // Send error response
        }
    }
});
```

### Key Points

1. **Always subscribe before sending**: Prevents race conditions
2. **Use timeouts**: Prevents hanging on lost responses
3. **Check error_id**: Robot may return error codes
4. **Log appropriately**: Use bevy::log in async tasks
5. **Handle all cases**: Success, error, timeout, no response

---

## Error Handling

### Error ID Codes

When `error_id != 0`, the robot encountered an error:

- Check robot documentation for specific error codes
- Common errors:
  - Invalid port number
  - Port not configured
  - Communication error
  - Robot in error state

### Timeout Handling

- Default timeout: 5 seconds
- Adjust based on robot response time
- Consider network latency

### Response Validation

Always validate:
- `error_id == 0` for success
- `port_number` matches request
- Value is within expected range

