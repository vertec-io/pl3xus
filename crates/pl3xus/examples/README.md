# Pl3xus Examples

This directory contains examples demonstrating how to use Bevy Pl3xus.

## Examples

### automatic_messages.rs

**Demonstrates the new automatic message registration API** - the easiest way to use Pl3xus!

This example shows how to use any `Serialize + Deserialize` type as a network message without needing to implement the `NetworkMessage` trait. This is the recommended approach for new code.

**Features demonstrated:**
- Automatic message registration with `register_network_message()`
- No boilerplate - just derive `Serialize` and `Deserialize`
- Works with types from external crates
- Simple `send()` and `broadcast()` methods

**Run the server:**
```bash
cargo run --package pl3xus --example automatic_messages -- server
```

**Run the client:**
```bash
cargo run --package pl3xus --example automatic_messages -- client
```

**Controls:**
- Server: Press SPACE to broadcast an announcement
- Client: Press SPACE to send a chat message, ENTER to send position update

### client.rs & server.rs

**Traditional examples using explicit NetworkMessage implementation**

These examples demonstrate the traditional approach where you manually implement the `NetworkMessage` trait with an explicit `NAME` constant. This approach is still supported and useful when you need:
- Explicit control over message names (e.g., for versioning)
- Compatibility with existing code
- Cross-language protocol definitions

**Run the server:**
```bash
cargo run --package pl3xus --example server
```

**Run the client:**
```bash
cargo run --package pl3xus --example client
```

## Choosing Between Approaches

### Use `register_network_message()` (Automatic) When:
- ✅ You want the simplest API
- ✅ You're working with types from external crates
- ✅ You don't need explicit control over message names
- ✅ You're starting a new project

### Use `listen_for_message()` (Explicit) When:
- ✅ You need explicit message names for versioning
- ✅ You're maintaining existing code
- ✅ You need cross-language protocol compatibility
- ✅ You want to decouple message names from Rust type names

## Code Comparison

### Automatic Approach (Recommended for New Code)

```rust
use serde::{Serialize, Deserialize};

// Just derive Serialize and Deserialize - that's it!
#[derive(Serialize, Deserialize, Clone)]
struct ChatMessage {
    username: String,
    message: String,
}

// Register with automatic naming
app.register_network_message::<ChatMessage, TcpProvider>();

// Send and receive
net.send(conn_id, ChatMessage { ... })?;
net.broadcast(ChatMessage { ... });
```

### Explicit Approach (Traditional)

```rust
use pl3xus::NetworkMessage;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
struct ChatMessage {
    username: String,
    message: String,
}

// Manually implement NetworkMessage with explicit name
impl NetworkMessage for ChatMessage {
    const NAME: &'static str = "chat:v1:Message";
}

// Register with explicit name
#[allow(deprecated)]
app.listen_for_message::<ChatMessage, TcpProvider>();

// Send and receive (same as automatic)
net.send(conn_id, ChatMessage { ... })?;
net.broadcast(ChatMessage { ... });
```

## Common Patterns

### Receiving Messages

Both approaches use the same pattern for receiving:

```rust
fn handle_messages(
    mut messages: MessageReader<NetworkData<ChatMessage>>,
) {
    for msg in messages.read() {
        println!("From {}: {}", msg.source(), msg.username);

        // Access message data via Deref
        let username = &msg.username;

        // Or clone the inner message
        let message = (**msg).clone();
    }
}
```

### Sending Messages

```rust
fn send_message(net: Res<Network<TcpProvider>>) {
    let msg = ChatMessage {
        username: "Player1".to_string(),
        message: "Hello!".to_string(),
    };
    
    // Send to specific client
    net.send(ConnectionId { id: 0 }, msg.clone())?;
    
    // Broadcast to all clients
    net.broadcast(msg);
}
```

### Connection Events

```rust
fn handle_connections(mut events: MessageReader<NetworkEvent>) {
    for event in events.read() {
        match event {
            NetworkEvent::Connected(conn_id) => {
                println!("Client connected: {:?}", conn_id);
            }
            NetworkEvent::Disconnected(conn_id) => {
                println!("Client disconnected: {:?}", conn_id);
            }
            NetworkEvent::Error(err) => {
                eprintln!("Network error: {:?}", err);
            }
        }
    }
}
```

## Tips

1. **Start with automatic messages** - They're simpler and work for most use cases
2. **Use explicit names for versioning** - If you need protocol versioning, use `NetworkMessage` with explicit names like `"chat:v2:Message"`
3. **Test locally first** - Run server and client on localhost (127.0.0.1) before testing over network
4. **Handle connection events** - Always listen for `NetworkEvent` to know when clients connect/disconnect
5. **Clone when broadcasting** - The `broadcast()` method requires `Clone` since it sends to multiple clients

## Troubleshooting

### "Could not find `Network`" Error
Make sure you add the `Pl3xusPlugin` before registering messages:
```rust
app.add_plugins(Pl3xusPlugin::<TcpProvider, TaskPool>::default());
app.insert_resource(Pl3xusRuntime(...));
app.insert_resource(NetworkSettings::default());
// Now you can register messages
app.register_network_message::<MyMessage, TcpProvider>();
```

### Messages Not Received
- Ensure both client and server register the same message types
- Check that you're reading the `NetworkData<T>` events in your systems
- Verify the connection is established by listening for `NetworkEvent::Connected`

### Duplicate Registration Panic
Each message type can only be registered once. If you see this error, you're calling `register_network_message` or `listen_for_message` multiple times for the same type.

