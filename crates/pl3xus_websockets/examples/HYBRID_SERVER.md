# Hybrid Server Example

## Overview

The `hybrid_server` example demonstrates how to run **both TCP and WebSocket protocols simultaneously** in a single Bevy server, allowing clients from either protocol to connect and share a common chat room.

This example showcases **two different architectural patterns** using feature flags:
- **Scheduled Pattern** (default) - Decoupled, deterministic message handling
- **Immediate Pattern** - Direct, simple message handling

## Architecture

The hybrid server uses Bevy's resource system to maintain two separate `Network<T>` resources:

- `Network<TcpProvider>` - Handles TCP connections on port **3030**
- `Network<WebSocketProvider>` - Handles WebSocket connections on port **8081**

Both networks are registered with the same message types, and the message handling approach depends on which feature flag is enabled.

## Pattern 1: Scheduled (Default) - Decoupled Architecture

The **scheduled pattern** uses the built-in `register_outbound_message` method to create a two-stage relay system that completely decouples application logic from network infrastructure.

### How It Works

**Stage 1: Application Logic (AppLogic SystemSet)**
Application logic reads incoming messages and writes `OutboundMessage<T>` events. It has **zero dependencies** on `Network` resources.

```rust
fn handle_messages(
    mut new_messages: MessageReader<NetworkData<UserChatMessage>>,
    mut outbound: MessageWriter<OutboundMessage<NewChatMessage>>,
) {
    for message in new_messages.read() {
        // Determine protocol using provider_name field
        let provider = message.provider_name();  // "TCP" or "WebSocket"

        // Create broadcast message
        let broadcast_message = NewChatMessage {
            name: format!("{}-{}", provider, message.source()),
            message: message.message.clone(),
        };

        // Write outbound - the built-in relay system handles broadcasting!
        outbound.write(OutboundMessage {
            name: "chat".to_string(),
            message: broadcast_message,
            for_client: None,  // None = broadcast to all
        });
    }
}
```

**Stage 2: Built-in Relay (NetworkRelay SystemSet)**
The framework automatically sets up relay systems for each provider when you call `register_outbound_message`:

```rust
// Register outbound messages for BOTH providers
// This automatically sets up relay_outbound systems for each provider
app.register_outbound_message::<NewChatMessage, TcpProvider, _>(NetworkRelay.clone());
app.register_outbound_message::<NewChatMessage, WebSocketProvider, _>(NetworkRelay.clone());
```

Each relay system reads `OutboundMessage<T>` events and broadcasts them via its respective `Network<T>` resource.

### System Set Ordering

```rust
app.configure_sets(Update, (
    AppLogic,
    NetworkRelay.after(AppLogic),
));
```

This ensures:
- ✅ All application logic completes before messages are sent
- ✅ Messages are sent at a deterministic point in the frame
- ✅ Can use `.apply_deferred()` before NetworkRelay to sync world state

### Benefits

✅ **Complete Decoupling**: Application logic has zero dependencies on `Network` resources
✅ **Determinism**: All messages sent at the same point in the frame
✅ **Testability**: Application logic can be tested without network infrastructure
✅ **Flexibility**: Easy to add new protocols without changing application logic
✅ **Uses Framework Correctly**: Leverages built-in `register_outbound_message` method

## Pattern 2: Immediate - Direct Control

The **immediate pattern** gives you direct control by having application logic directly broadcast to both network providers.

### How It Works

Application logic directly uses `Network<TcpProvider>` and `Network<WebSocketProvider>` resources:

```rust
fn handle_messages(
    mut new_messages: MessageReader<NetworkData<UserChatMessage>>,
    tcp_net: Res<Network<TcpProvider>>,
    ws_net: Res<Network<WebSocketProvider>>,
) {
    for message in new_messages.read() {
        let provider = message.provider_name();

        let broadcast_message = NewChatMessage {
            name: format!("{}-{}", provider, message.source()),
            message: message.message.clone(),
        };

        // Immediate pattern: Directly broadcast to both networks
        tcp_net.broadcast(broadcast_message.clone());
        ws_net.broadcast(broadcast_message);
    }
}
```

### Benefits

✅ **Simple and Direct**: No intermediate events or relay systems
✅ **Maximum Control**: You decide exactly when and how to send messages
✅ **Easy to Understand**: Straightforward message flow
✅ **Good for Prototyping**: Quick to implement and iterate

## Switching Between Patterns

Use feature flags to select which pattern to use:

### Scheduled Pattern (Default)
```bash
cargo run --example hybrid_server --package pl3xus_websockets
```

### Immediate Pattern
```bash
cargo run --example hybrid_server --package pl3xus_websockets --features immediate
```

The server will log which pattern is active:
- "🚀 Starting hybrid server with SCHEDULED message pattern"
- "🚀 Starting hybrid server with IMMEDIATE message pattern"

## Provider Identification

Both patterns use the `provider_name` field in `NetworkData<T>` to identify which protocol a message came from:

```rust
let provider = message.provider_name();  // "TCP" or "WebSocket"
```

This allows application logic to determine the protocol **without** needing access to `Network` resources, achieving complete decoupling (especially important in the scheduled pattern).

## Running the Example

### Start the Hybrid Server

**Scheduled Pattern (Default):**
```bash
cargo run --example hybrid_server --package pl3xus_websockets
```

**Immediate Pattern:**
```bash
cargo run --example hybrid_server --package pl3xus_websockets --features immediate
```

The server will listen on:
- 📡 **TCP**: `127.0.0.1:3030`
- 🌐 **WebSocket**: `127.0.0.1:8081`

### Connect TCP Clients

```bash
cargo run -p pl3xus --example client
```

### Connect WebSocket Clients

**Bevy Client:**
```bash
cargo run --example client --package pl3xus_websockets
```

**Leptos WASM Client:**
```bash
cd crates/pl3xus_websockets/leptos_client_example
trunk serve --port 8082
# Open http://127.0.0.1:8082 in your browser
```

## Message Flow

### Scheduled Pattern
1. Message received by appropriate `Network<T>` resource
2. Message written to Bevy's global `MessageWriter<NetworkData<UserChatMessage>>` with provider name
3. Application logic reads message and writes `OutboundMessage<NewChatMessage>` (AppLogic set)
4. Built-in relay systems (one per provider) read outbound messages and broadcast (NetworkRelay set)

### Immediate Pattern
1. Message received by appropriate `Network<T>` resource
2. Message written to Bevy's global `MessageWriter<NetworkData<UserChatMessage>>` with provider name
3. Application logic reads message and directly broadcasts to both `Network<TcpProvider>` and `Network<WebSocketProvider>`

Both patterns create a unified chat room where TCP and WebSocket clients can communicate seamlessly!

## Code Structure

The example is split into three files:

### `hybrid_server.rs` (Main File)
- Uses feature flags to select which plugin to load
- `setup_networking()` - Starts both TCP and WebSocket servers
- Registers incoming messages for both providers

### `scheduled_messages.rs` (Scheduled Pattern Plugin)
- `ScheduledMsgPlugin` - Sets up system sets and registers outbound messages
- `handle_connection_events()` - Unified connection handler
- `handle_messages()` - Application logic that writes `OutboundMessage<T>` events
- Uses built-in `relay_outbound` systems (registered via `register_outbound_message`)

### `immediate_messages.rs` (Immediate Pattern Plugin)
- `ImmediateMsgPlugin` - Sets up systems
- `handle_connection_events()` - Unified connection handler
- `handle_messages()` - Application logic that directly broadcasts to both networks

## Which Pattern Should You Use?

### Use **Scheduled Pattern** if you want:
- ✅ Complete decoupling of application logic from network infrastructure
- ✅ Deterministic message timing (all messages sent at same point in frame)
- ✅ Easy testing (application logic has no network dependencies)
- ✅ Production-ready architecture
- ✅ Ability to use `.apply_deferred()` before sending messages

### Use **Immediate Pattern** if you want:
- ✅ Simple, straightforward code
- ✅ Maximum control over when messages are sent
- ✅ Quick prototyping
- ✅ Direct access to network resources

## Summary of Benefits

✅ **Protocol Flexibility**: Clients can choose their preferred protocol (TCP or WebSocket)
✅ **Unified Chat Room**: All clients see messages from all other clients regardless of protocol
✅ **Zero Code Duplication**: Message types are defined once and work for both protocols
✅ **Hybrid Schema Hash**: Cross-protocol communication works even with different module paths
✅ **Provider Identification**: Determine protocol using `message.provider_name()`
✅ **Two Architectural Patterns**: Choose between scheduled (decoupled) or immediate (direct) patterns
✅ **Scalable**: Can easily add more protocols (e.g., UDP, QUIC) using the same pattern

## Example Output

### Scheduled Pattern
```
🚀 Starting hybrid server with SCHEDULED message pattern
📡 TCP server listening on 127.0.0.1:3030
🌐 WebSocket server listening on 127.0.0.1:8081
🚀 Hybrid server started! Accepting both TCP and WebSocket connections.

🌐 WebSocket client connected: Connection with ID=1
🌐 WebSocket connection added: Connection with ID=1 (Total TCP: 0, WS: 1)
🌐 Received WebSocket message from Connection with ID=1: Hello from WebSocket!

📡 TCP client connected: Connection with ID=1
📡 TCP connection added: Connection with ID=1 (Total TCP: 1, WS: 1)
📡 Received TCP message from Connection with ID=1: Hello from TCP!
```

### Immediate Pattern
```
🚀 Starting hybrid server with IMMEDIATE message pattern
📡 TCP server listening on 127.0.0.1:3030
🌐 WebSocket server listening on 127.0.0.1:8081
🚀 Hybrid server started! Accepting both TCP and WebSocket connections.

🌐 WebSocket client connected: Connection with ID=1
🌐 WebSocket connection added: Connection with ID=1 (Total TCP: 0, WS: 1)
🌐 Received WebSocket message from Connection with ID=1: Hello from WebSocket!

📡 TCP client connected: Connection with ID=1
📡 TCP connection added: Connection with ID=1 (Total TCP: 1, WS: 1)
📡 Received TCP message from Connection with ID=1: Hello from TCP!
```

## Conclusion

The hybrid server demonstrates that **pl3xus's architecture is flexible enough to support multiple protocols simultaneously** with clean, maintainable designs.

The example showcases **two different architectural approaches**:

### Scheduled Pattern (Recommended for Production)
- **Clean separation** between application logic and network infrastructure
- **Deterministic behavior** with predictable message timing
- **Easy testing** since application logic has no network dependencies
- **Uses framework correctly** by leveraging built-in `register_outbound_message`
- **Scalability** to add new protocols without changing existing code

### Immediate Pattern (Good for Prototyping)
- **Simple and direct** code that's easy to understand
- **Maximum control** over message sending
- **Quick to implement** for rapid prototyping
- **Straightforward** message flow

Both patterns are production-ready and fully tested with cross-protocol communication!

