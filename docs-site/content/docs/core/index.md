---
title: Core Pl3xus Guide
---
# Core Pl3xus Guide

This guide covers the core `pl3xus` crate for event-driven networking in Bevy applications.

## Overview

`pl3xus` provides transport-agnostic networking for Bevy:

- **Transport Agnostic** - Use TCP, WebSocket, or custom transports
- **Type-Safe Messages** - Strongly typed with compile-time guarantees
- **Event-Driven** - Integrates with Bevy's ECS via `MessageReader`/`MessageWriter`
- **Industrial Ready** - Built for reliable, low-latency communication

## Quick Start

### 1. Add Dependencies

```toml
[dependencies]
bevy = "0.17"
pl3xus = "1.1"
pl3xus_websockets = "1.1"  # Or pl3xus with "tcp" feature
serde = { version = "1.0", features = ["derive"] }
```

### 2. Define Messages

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
struct ChatMessage {
    user: String,
    content: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct StatusUpdate {
    entity_id: u64,
    status: String,
}
```

### 3. Set Up the Server

```rust
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use bevy::prelude::*;
use bevy::tasks::{TaskPool, TaskPoolBuilder};
use pl3xus::{Pl3xusPlugin, Pl3xusRuntime, Network, NetworkEvent, AppNetworkMessage};
use pl3xus_websockets::{WebSocketProvider, NetworkSettings};

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        // Add pl3xus with WebSocket transport
        .add_plugins(Pl3xusPlugin::<WebSocketProvider, TaskPool>::default())
        .insert_resource(Pl3xusRuntime(TaskPoolBuilder::new().num_threads(2).build()))
        .insert_resource(NetworkSettings::default())
        // Register message types
        .register_network_message::<ChatMessage, WebSocketProvider>()
        .register_network_message::<StatusUpdate, WebSocketProvider>()
        // Add systems
        .add_systems(Startup, start_server)
        .add_systems(Update, (handle_connections, handle_messages))
        .run();
}

fn start_server(
    net: Res<Network<WebSocketProvider>>,
    settings: Res<NetworkSettings>,
    task_pool: Res<Pl3xusRuntime<TaskPool>>,
) {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    
    match net.listen(addr, &task_pool.0, &settings) {
        Ok(_) => info!("Server listening on {addr}"),
        Err(e) => error!("Failed to start server: {e}"),
    }
}
```

### 4. Handle Connections

```rust
use pl3xus::MessageReader;

fn handle_connections(mut events: MessageReader<NetworkEvent>) {
    for event in events.read() {
        match event {
            NetworkEvent::Connected(conn_id) => {
                info!("Client connected: {:?}", conn_id);
            }
            NetworkEvent::Disconnected(conn_id) => {
                info!("Client disconnected: {:?}", conn_id);
            }
            NetworkEvent::Error(err) => {
                error!("Network error: {:?}", err);
            }
        }
    }
}
```

### 5. Handle Messages

```rust
use pl3xus::NetworkData;

fn handle_messages(
    mut messages: MessageReader<NetworkData<ChatMessage>>,
    net: Res<Network<WebSocketProvider>>,
) {
    for msg in messages.read() {
        let sender = msg.source();  // Get the ConnectionId
        info!("{}: {}", msg.user, msg.content);
        
        // Broadcast to all connected clients
        net.broadcast(ChatMessage {
            user: msg.user.clone(),
            content: msg.content.clone(),
        });
    }
}
```

### 6. Set Up a Client

```rust
fn start_client(
    net: Res<Network<WebSocketProvider>>,
    settings: Res<NetworkSettings>,
    task_pool: Res<Pl3xusRuntime<TaskPool>>,
) {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    
    match net.connect(addr, &task_pool.0, &settings) {
        Ok(_) => info!("Connecting to server..."),
        Err(e) => error!("Failed to connect: {e}"),
    }
}

fn send_message(net: Res<Network<WebSocketProvider>>) {
    net.broadcast(ChatMessage {
        user: "Client".to_string(),
        content: "Hello, server!".to_string(),
    });
}
```

## Key Concepts

### `NetworkData<T>`

Wraps received messages with connection metadata:

```rust
fn process_messages(mut messages: MessageReader<NetworkData<StatusUpdate>>) {
    for msg in messages.read() {
        let source = msg.source();       // ConnectionId of sender
        let status = &msg.status;        // Access fields via Deref
        let inner = msg.into_inner();    // Extract the inner message
    }
}
```

### Network Resource

The `Network<NP>` resource is your interface for sending messages:

```rust
fn send_examples(net: Res<Network<WebSocketProvider>>) {
    // Send to a specific client
    net.send(connection_id, message.clone());
    
    // Broadcast to all connected clients
    net.broadcast(message);
}
```

### MessageReader / MessageWriter

Unlike Bevy's `EventReader`/`EventWriter`, pl3xus uses `MessageReader` and `MessageWriter` for network messages:

```rust
// Reading incoming messages
fn receive(mut messages: MessageReader<NetworkData<MyMessage>>) {
    for msg in messages.read() { /* ... */ }
}

// Writing outbound messages (alternative to net.send)
fn send(mut writer: MessageWriter<OutboundMessage<MyMessage>>) {
    writer.send(OutboundMessage::broadcast(my_message));
}
```

## Next Steps

- [Installation Guide](./installation.md) - Detailed setup instructions
- [Getting Started Tutorial](./getting-started.md) - Step-by-step walkthrough
- [Sending Messages](./guides/sending-messages.md) - Advanced messaging patterns
- [Server Setup](./core/guides/server-setup.md) - Production server configuration
