---
title: Bevy Pl3xus 0.10.0 Release Notes
---
# Bevy Pl3xus 0.10.0 Release Notes

## 🎉 Major Feature: Automatic Message Registration

We're excited to announce Bevy Pl3xus 0.10.0, featuring **automatic message registration** - a major ergonomic improvement that eliminates boilerplate and makes networking in Bevy easier than ever!

## What's New

### Zero Boilerplate Networking

You no longer need to implement the `NetworkMessage` trait for every type. Just derive `Serialize` and `Deserialize`, and you're ready to go!

**Before (0.9):**
```rust
use pl3xus::NetworkMessage;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
struct PlayerPosition {
    x: f32,
    y: f32,
}

// Had to implement this for every type
impl NetworkMessage for PlayerPosition {
    const NAME: &'static str = "app:PlayerPosition";
}

app.listen_for_message::<PlayerPosition, TcpProvider>();
net.send_message(conn_id, msg)?;
```

**After (0.10):**
```rust
use serde::{Serialize, Deserialize};

// That's it! No trait implementation needed
#[derive(Serialize, Deserialize, Clone)]
struct PlayerPosition {
    x: f32,
    y: f32,
}

app.register_network_message::<PlayerPosition, TcpProvider>();
net.send(conn_id, msg)?;
```

### Key Benefits

- ✨ **Zero Boilerplate** - No trait implementation required
- 🔧 **External Crate Support** - Use types from any crate as network messages
- 🚀 **Faster Development** - Less code to write and maintain
- ⚡ **No Performance Cost** - Type names are cached, zero runtime overhead
- ✅ **Fully Backward Compatible** - All existing code continues to work

### New API Methods

#### `register_network_message<T>()`
Register any serializable type as a network message with automatic naming:
```rust
app.register_network_message::<ChatMessage, TcpProvider>();
```

#### `send<T>()`
Simplified send method that works with any message type:
```rust
net.send(connection_id, ChatMessage { ... })?;
```

#### `broadcast<T>()`
Updated broadcast method (same signature, now works with automatic messages):
```rust
net.broadcast(StateUpdate { ... });
```

## Migration Guide

### For New Projects

Use the new automatic API - it's simpler and more flexible:

```rust
use bevy::prelude::*;
use pl3xus::{AppNetworkMessage, Pl3xusPlugin, Pl3xusRuntime};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
struct MyMessage {
    data: String,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(Pl3xusPlugin::<TcpProvider, TaskPool>::default())
        .insert_resource(Pl3xusRuntime(...))
        .register_network_message::<MyMessage, TcpProvider>()
        .run();
}
```

### For Existing Projects

**No changes required!** Your existing code will continue to work exactly as before. The old API is deprecated but fully functional.

To remove deprecation warnings, update your code:

| Old (Deprecated) | New (Recommended) |
|-----------------|-------------------|
| `listen_for_message::<T, P>()` | `register_network_message::<T, P>()` |
| `send_message(id, msg)` | `send(id, msg)` |

### When to Use Each API

**Use `register_network_message()` (Automatic) When:**
- ✅ You want the simplest API
- ✅ You're working with types from external crates
- ✅ You don't need explicit control over message names
- ✅ You're starting a new project

**Use `listen_for_message()` (Explicit) When:**
- ✅ You need explicit message names for versioning (e.g., `"auth:v2:Login"`)
- ✅ You're maintaining existing code
- ✅ You need cross-language protocol compatibility
- ✅ You want to decouple message names from Rust type names

## Examples

### Complete Working Example

Check out the new `automatic_messages` example:

```bash
# Run the server
cargo run --package pl3xus --example automatic_messages -- server

# Run the client (in another terminal)
cargo run --package pl3xus --example automatic_messages -- client
```

This example demonstrates:
- Automatic message registration
- Multiple message types
- Send and broadcast functionality
- Connection handling

### Quick Example

```rust
use bevy::prelude::*;
use pl3xus::{AppNetworkMessage, Network, NetworkData};
use serde::{Serialize, Deserialize};

// Define your messages - just derive Serialize and Deserialize
#[derive(Serialize, Deserialize, Clone)]
struct ChatMessage {
    user: String,
    message: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct PlayerPosition {
    x: f32,
    y: f32,
}

fn setup(mut app: App) {
    // Register messages with one line each
    app.register_network_message::<ChatMessage, TcpProvider>();
    app.register_network_message::<PlayerPosition, TcpProvider>();
}

fn handle_messages(mut messages: MessageReader<NetworkData<ChatMessage>>) {
    for msg in messages.read() {
        println!("{}: {}", msg.user, msg.message);
    }
}

fn send_message(net: Res<Network<TcpProvider>>) {
    // Send with the new simplified API
    net.send(conn_id, ChatMessage {
        user: "Player1".to_string(),
        message: "Hello!".to_string(),
    }).ok();
    
    // Broadcast to all
    net.broadcast(PlayerPosition { x: 1.0, y: 2.0 });
}
```

## Technical Details

### How It Works

The new system uses Rust's `std::any::type_name()` to automatically generate message identifiers. These names are cached using `OnceCell` for performance, so there's zero runtime overhead after the first access.

### Performance

- **First access**: Type name is computed and cached
- **Subsequent accesses**: Direct lookup from cache (same as const str)
- **No runtime overhead**: After caching, performance is identical to explicit names

### Compatibility

This release is **100% backward compatible**. All existing code continues to work without any changes. The old API methods are deprecated but will remain functional for the foreseeable future.

## Breaking Changes

**None!** This is a fully backward-compatible release.

## Version Updates

- `pl3xus`: 0.9.11 → **0.10.0**
- `pl3xus_common`: 0.2.8 → **0.3.0**
- `pl3xus_websockets`: 0.2.1 → **0.3.0**

## Documentation

- **Updated README** with new API examples and migration guide
- **New example**: `automatic_messages.rs` demonstrating the new API
- **Comprehensive examples README** with usage patterns and best practices
- **Full CHANGELOG** documenting all changes

## Testing

All features are thoroughly tested:
- ✅ 12 integration tests for automatic message registration
- ✅ 7 unit tests for type name generation and caching
- ✅ Tests for external types, generic types, and mixed registration
- ✅ Backward compatibility tests

## Getting Started

Update your `Cargo.toml`:

```toml
[dependencies]
pl3xus = "0.10"
serde = { version = "1.0", features = ["derive"] }
```

Then start using the new API:

```rust
#[derive(Serialize, Deserialize, Clone)]
struct MyMessage { /* fields */ }

app.register_network_message::<MyMessage, TcpProvider>();
net.send(conn_id, msg)?;
```

## Feedback

We'd love to hear your feedback on the new API! Please open an issue on GitHub if you have any questions, suggestions, or encounter any problems.

## Credits

This release was made possible by the Bevy Pl3xus community. Special thanks to all contributors and users who provided feedback and suggestions.

---

**Happy Networking! 🚀**

