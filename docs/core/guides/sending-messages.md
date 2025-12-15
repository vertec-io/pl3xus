# Sending Messages in Pl3xus

Pl3xus provides **two approaches** for sending messages, each optimized for different use cases.

## Option A: Direct Sending (Immediate) ⚡

Use the `Network` resource for immediate message sending. This is the simplest and most common approach.

### Example

```rust
use pl3xus::Network;
use pl3xus::tcp::TcpProvider;

fn send_messages(
    net: Res<Network<TcpProvider>>,
    connection_id: ConnectionId,
) {
    let message = ChatMessage {
        user: "Player1".to_string(),
        message: "Hello!".to_string(),
    };

    // Send to specific connection - processes immediately
    net.send(connection_id, message.clone()).ok();

    // Broadcast to all connections - processes immediately  
    net.broadcast(message);
}
```

### When to Use

✅ **Use Direct Sending when:**
- You want simple, straightforward message sending
- Messages should be sent immediately
- You don't need fine-grained control over network scheduling
- You're building a typical client-server application

### How It Works

1. You call `net.send()` or `net.broadcast()`
2. The message is immediately serialized and queued for sending
3. The network provider sends it as soon as possible
4. No additional setup or registration required

---

## Option B: Outbound Messages (Scheduled) 🎯

Use `OutboundMessage` events with `MessageWriter` for precise control over when messages are sent. This allows you to schedule network operations in specific system sets.

### Example

```rust
use pl3xus::{OutboundMessage, AppNetworkMessage};
use pl3xus_common::Pl3xusMessage;
use pl3xus::tcp::TcpProvider;
use bevy::prelude::*;

// Step 1: Register the outbound message with your desired system set
fn setup_networking(app: &mut App) {
    // Messages will be sent during PostUpdate
    app.register_outbound_message::<ChatMessage, TcpProvider, _>(PostUpdate);
}

// Step 2: Send messages using MessageWriter
fn send_messages(
    mut outbound: MessageWriter<OutboundMessage<ChatMessage>>,
    connection_id: ConnectionId,
) {
    let message = ChatMessage {
        user: "Player1".to_string(),
        message: "Hello!".to_string(),
    };

    // Broadcast to all connections (default behavior)
    outbound.write(OutboundMessage::new(
        ChatMessage::type_name().to_string(),
        message.clone(),
    ));

    // Or target a specific connection
    outbound.write(
        OutboundMessage::new(
            ChatMessage::type_name().to_string(),
            message,
        )
        .for_client(connection_id)
    );
}
```

### When to Use

✅ **Use Outbound Messages when:**
- You need precise control over network scheduling
- You want to batch network operations in specific system sets
- You're implementing custom network flow control
- You want to separate message creation from message sending
- You need to coordinate network sends with other application systems

### How It Works

1. You register the message type with `register_outbound_message()`, specifying a system set
2. You send `OutboundMessage` events using `MessageWriter` in your systems
3. The `relay_outbound_notifications` system runs in your specified system set
4. Messages are sent over the network during that system set's execution
5. This gives you fine-grained control over when network I/O happens

### Advanced Use Cases

**Example: Batching messages in a specific phase**

```rust
// Send all network messages during a custom NetworkSync phase
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
struct NetworkSync;

fn setup(app: &mut App) {
    app.configure_sets(PostUpdate, NetworkSync);
    
    // All outbound messages will be sent during NetworkSync
    app.register_outbound_message::<PlayerPosition, TcpProvider, _>(NetworkSync);
    app.register_outbound_message::<PlayerAction, TcpProvider, _>(NetworkSync);
    app.register_outbound_message::<AppState, TcpProvider, _>(NetworkSync);
}

// Now all these messages will be batched and sent together
fn update_player(mut outbound: MessageWriter<OutboundMessage<PlayerPosition>>) {
    outbound.write(OutboundMessage::new(
        PlayerPosition::type_name().to_string(),
        PlayerPosition { x: 1.0, y: 2.0, z: 3.0 },
    ));
}

fn handle_action(mut outbound: MessageWriter<OutboundMessage<PlayerAction>>) {
    outbound.write(OutboundMessage::new(
        PlayerAction::type_name().to_string(),
        PlayerAction::Jump,
    ));
}
```

---

## Comparison Table

| Feature | Direct Sending (`net.send()`) | Outbound Messages (`MessageWriter`) |
|---------|-------------------------------|-----------------------------------|
| **Simplicity** | ✅ Very simple | ⚠️ More setup required |
| **Timing** | Immediate | Scheduled in system set |
| **Control** | Limited | Full scheduling control |
| **Use Case** | General purpose | Advanced network scheduling |
| **Setup** | None | Register with system set |
| **Batching** | ❌ No | ✅ Yes, in system sets |
| **Coordination** | ❌ Limited | ✅ Full control |

---

## Best Practices

### For Most Applications

**Use Direct Sending** (`net.send()` / `net.broadcast()`) for:
- Chat messages
- Player actions
- Simple client-server communication
- Prototyping and getting started

### For Advanced Applications

**Use Outbound Messages** (`MessageWriter<OutboundMessage<T>>`) for:
- Networked applications with complex state synchronization
- Applications that need deterministic network timing
- Systems that batch multiple message types together
- Applications with custom network scheduling requirements

### Mixing Both Approaches

You can use both approaches in the same application! For example:
- Use **Direct Sending** for urgent, one-off messages (chat, commands)
- Use **Outbound Messages** for regular state updates (positions, application state)

```rust
fn handle_chat(
    net: Res<Network<TcpProvider>>,
    mut chat_events: MessageReader<ChatEvent>,
) {
    for event in chat_events.read() {
        // Chat is urgent - send immediately
        net.broadcast(ChatMessage {
            user: event.user.clone(),
            message: event.message.clone(),
        });
    }
}

fn sync_positions(
    mut outbound: MessageWriter<OutboundMessage<PlayerPosition>>,
    players: Query<(&Transform, &PlayerId)>,
) {
    for (transform, player_id) in players.iter() {
        // Position updates are batched in PostUpdate
        outbound.write(OutboundMessage::new(
            PlayerPosition::type_name().to_string(),
            PlayerPosition {
                id: player_id.0,
                x: transform.translation.x,
                y: transform.translation.y,
                z: transform.translation.z,
            },
        ));
    }
}
```

---

## Common Pitfalls

### ❌ Don't: Mix registration approaches

```rust
// BAD: Registering the same message type with both approaches
app.register_network_message::<ChatMessage, TcpProvider>();
app.register_outbound_message::<ChatMessage, TcpProvider, _>(PostUpdate);
```

### ✅ Do: Choose one approach per message type

```rust
// GOOD: Use one approach per message type
app.register_network_message::<ChatMessage, TcpProvider>();  // For receiving
app.register_outbound_message::<PlayerPosition, TcpProvider, _>(PostUpdate);  // For sending with control
```

### ❌ Don't: Forget to register outbound messages

```rust
// BAD: Sending without registration
fn send_message(mut outbound: MessageWriter<OutboundMessage<MyMessage>>) {
    outbound.write(OutboundMessage::new(...));  // Won't work!
}
```

### ✅ Do: Always register before using

```rust
// GOOD: Register in setup
fn setup(app: &mut App) {
    app.register_outbound_message::<MyMessage, TcpProvider, _>(PostUpdate);
}

fn send_message(mut outbound: MessageWriter<OutboundMessage<MyMessage>>) {
    outbound.write(OutboundMessage::new(...));  // Works!
}
```

---

## Summary

- **Direct Sending** = Simple, immediate, great for most use cases
- **Outbound Messages** = Advanced, scheduled, great for complex applications
- Both approaches are valid and can be used together
- Choose based on your application's needs

For more examples, see the [examples directory](https://github.com/jamescarterbell/pl3xus/tree/master/crates/pl3xus/examples).

