---
title: MIGRATION_0.17
---
# Migration Guide: Bevy 0.17 Upgrade

This guide will help you migrate your pl3xus project from Bevy 0.16 to Bevy 0.17.

## Table of Contents

- [Overview](#overview)
- [Breaking Changes](#breaking-changes)
- [Step-by-Step Migration](#step-by-step-migration)
- [Code Examples](#code-examples)
- [Troubleshooting](#troubleshooting)

## Overview

Bevy 0.17 introduced a significant change to its event system by splitting the `Event` trait into two concepts:
- **`Event`** - For immediate, observer-based events (new behavior)
- **`Message`** - For buffered events (old `Event` behavior)

Since pl3xus uses buffered events for network communication, all event-related code must be migrated to use the `Message` API.

## Breaking Changes

### 1. Event → Message API Migration

| Bevy 0.16 (Old) | Bevy 0.17 (New) |
|-----------------|-----------------|
| `#[derive(Event)]` | `#[derive(Message)]` |
| `EventReader<T>` | `MessageReader<T>` |
| `EventWriter<T>` | `MessageWriter<T>` |
| `EventWriter::send()` | `MessageWriter::write()` |
| `App::add_event::<T>()` | `App::add_message::<T>()` |

### 2. Affected pl3xus Types

The following types now derive `Message` instead of `Event`:
- `NetworkEvent` - Connection/disconnection events
- `NetworkData<T>` - Wrapper for received network messages
- `OutboundMessage<T>` - Messages to be sent over the network

### 3. Rust Version Requirements

- **Minimum Rust version**: 1.88.0 (requires nightly until stable release)
- **Rust edition**: 2024
- **Toolchain**: Must use nightly Rust

**Why nightly?** Bevy 0.17 requires Rust 1.88.0, which hasn't been released to stable yet. This is because Bevy 0.17 uses Rust Edition 2024 and other cutting-edge features that are only available in nightly Rust.

**When can I use stable?** Once Rust 1.88.0 is released to the stable channel (expected in early 2026), you'll be able to switch back to stable Rust. At that point, you can remove the `rust-toolchain.toml` file or change it to use the stable channel.

## Step-by-Step Migration

### Step 1: Update Dependencies

Update your `Cargo.toml`:

```toml
[dependencies]
bevy = "0.17"
pl3xus = "1.1"  # Core networking library
pl3xus_websockets = "1.1"  # If using WebSockets
serde = { version = "1.0", features = ["derive"] }
```

**Note**: All pl3xus crates are versioned together (1.1.0). Always use matching versions to avoid compatibility issues.

### Step 2: Update Rust Toolchain

Create or update `rust-toolchain.toml` in your project root:

```toml
[toolchain]
channel = "nightly"
```

Then run:
```bash
rustup update
```

This will install/update nightly Rust and use it for your project.

### Step 3: Update System Parameters

Replace all `EventReader` and `EventWriter` with `MessageReader` and `MessageWriter`:

**Before (Bevy 0.16):**
```rust
fn handle_network_events(
    mut network_events: EventReader<NetworkEvent>,
) {
    for event in network_events.read() {
        // Handle event
    }
}
```

**After (Bevy 0.17):**
```rust
fn handle_network_events(
    mut network_events: MessageReader<NetworkEvent>,
) {
    for event in network_events.read() {
        // Handle event
    }
}
```

### Step 4: Update Message Sending

Replace `EventWriter::send()` with `MessageWriter::write()`:

**Before (Bevy 0.16):**
```rust
fn send_messages(
    mut outbound: EventWriter<OutboundMessage<MyMessage>>,
) {
    outbound.send(OutboundMessage::new(
        MyMessage::type_name().to_string(),
        my_message,
    ));
}
```

**After (Bevy 0.17):**
```rust
fn send_messages(
    mut outbound: MessageWriter<OutboundMessage<MyMessage>>,
) {
    outbound.write(OutboundMessage::new(
        MyMessage::type_name().to_string(),
        my_message,
    ));
}
```

### Step 5: Update Custom Message Types (If Applicable)

If you derive `Event` on your custom message types, change to `Message`:

**Before (Bevy 0.16):**
```rust
#[derive(Event, Serialize, Deserialize, Clone)]
struct MyMessage {
    data: String,
}
```

**After (Bevy 0.17):**
```rust
#[derive(Message, Serialize, Deserialize, Clone)]
struct MyMessage {
    data: String,
}
```

**Note**: Most users don't need to derive `Event`/`Message` on their network message types. Only derive `Message` if you're also using your types as Bevy messages outside of networking.

## Troubleshooting

### Issue: "Rust version 1.88.0 is not installed"

**Solution**: Bevy 0.17 requires Rust 1.88.0, which hasn't been released to stable yet. You must use nightly Rust:

1. Create `rust-toolchain.toml`:
   ```toml
   [toolchain]
   channel = "nightly"
   ```

2. Update rustup:
   ```bash
   rustup update
   ```

### Issue: "no method named `send` found for struct `MessageWriter`"

**Solution**: `MessageWriter` uses `.write()` instead of `.send()`. Replace all `.send()` calls with `.write()`.

### Issue: "the trait `Event` is not implemented for `NetworkEvent`"

**Solution**: `NetworkEvent` now implements `Message` instead of `Event`. Update your system parameters from `EventReader<NetworkEvent>` to `MessageReader<NetworkEvent>`.

### Issue: Compilation errors about `Event` trait

**Solution**: Make sure you've updated all occurrences:
- `EventReader`  `MessageReader`
- `EventWriter`  `MessageWriter`
- `#[derive(Event)]`  `#[derive(Message)]` (if applicable)

## Additional Resources

- [Bevy 0.17 Migration Guide](https://bevyengine.org/learn/migration-guides/0-16-to-0-17/)
- [pl3xus Examples](https://github.com/jamescarterbell/pl3xus/tree/master/crates/pl3xus/examples)
- [CHANGELOG.md](../../CHANGELOG.md)

## Need Help?

If you encounter issues not covered in this guide:
1. Check the [examples](https://github.com/jamescarterbell/pl3xus/tree/master/crates/pl3xus/examples) for working code
2. Open an issue on [GitHub](https://github.com/jamescarterbell/pl3xus/issues)
3. Ask on the [Bevy Discord](https://discord.gg/bevy) - look for `@SirCarter`
