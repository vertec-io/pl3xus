# WebSocket Patterns Guide

This guide covers common WebSocket architectures and production patterns when using pl3xus_websockets.

---

## Overview

`pl3xus_websockets` provides WebSocket transport for the pl3xus networking library. It supports:

- **Native servers** using async-tungstenite
- **Native clients** using async-tungstenite
- **WASM clients** using ws_stream_wasm
- **Binary message encoding** using bincode for efficiency

---

## Basic Server Setup

### Minimal Server

```rust
use bevy::prelude::*;
use bevy::tasks::{TaskPool, TaskPoolBuilder};
use pl3xus::{Pl3xusPlugin, Pl3xusRuntime, Network, NetworkEvent};
use pl3xus_websockets::{NetworkSettings, WebSocketProvider};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins(bevy::log::LogPlugin::default())
        // Add pl3xus with WebSocket provider
        .add_plugins(Pl3xusPlugin::<WebSocketProvider, TaskPool>::default())
        // Configure the async runtime
        .insert_resource(Pl3xusRuntime(
            TaskPoolBuilder::new().num_threads(2).build()
        ))
        // Configure WebSocket settings
        .insert_resource(NetworkSettings::default())
        // Start listening on startup
        .add_systems(Startup, setup_networking)
        .add_systems(Update, handle_connections)
        .run();
}

fn setup_networking(
    mut net: ResMut<Network<WebSocketProvider>>,
    settings: Res<NetworkSettings>,
    task_pool: Res<Pl3xusRuntime<TaskPool>>,
) {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    
    match net.listen(addr, &task_pool.0, &settings) {
        Ok(_) => info!("Server listening on ws://{}/", addr),
        Err(err) => {
            error!("Failed to start server: {}", err);
            panic!("Could not bind to address");
        }
    }
}

fn handle_connections(mut events: EventReader<NetworkEvent>) {
    for event in events.read() {
        match event {
            NetworkEvent::Connected(id) => info!("Client {} connected", id),
            NetworkEvent::Disconnected(id) => info!("Client {} disconnected", id),
            NetworkEvent::Error(id, err) => error!("Client {} error: {:?}", id, err),
        }
    }
}
```

---

## Network Settings

### Native Server/Client Settings

```rust
use pl3xus_websockets::NetworkSettings;
use async_tungstenite::tungstenite::protocol::WebSocketConfig;

let settings = NetworkSettings {
    // WebSocket protocol configuration
    websocket_config: WebSocketConfig {
        max_message_size: Some(64 * 1024 * 1024),  // 64MB max message
        max_frame_size: Some(16 * 1024 * 1024),    // 16MB max frame
        ..Default::default()
    },
    // Message queue capacity per connection
    // At 60 FPS, 500 = ~8 seconds of buffering
    channel_capacity: 500,
    // Warn when queue is 80% full
    channel_warning_threshold: 80,
};

app.insert_resource(settings);
```

### WASM Client Settings

```rust
// WASM clients have simpler settings
let settings = NetworkSettings {
    max_message_size: 64 * 1024 * 1024,  // 64MB
    channel_capacity: 500,
    channel_warning_threshold: 80,
};
```

### Production Recommendations

| Setting | Development | Production |
|---------|-------------|------------|
| `channel_capacity` | 500 | 1000-2000 |
| `max_message_size` | 64MB | Based on your data |
| `channel_warning_threshold` | 80 | 70-80 |

---

## Client Connection Patterns

### Native Bevy Client

```rust
use pl3xus::{Network, Pl3xusRuntime};
use pl3xus_websockets::{NetworkSettings, WebSocketProvider};
use url::Url;

fn connect_to_server(
    mut net: ResMut<Network<WebSocketProvider>>,
    settings: Res<NetworkSettings>,
    task_pool: Res<Pl3xusRuntime<TaskPool>>,
) {
    let url = Url::parse("ws://127.0.0.1:8080").unwrap();
    
    match net.connect(url, &task_pool.0, &settings) {
        Ok(_) => info!("Connecting to server..."),
        Err(err) => error!("Connection failed: {}", err),
    }
}
```

### Leptos Web Client (with pl3xus_client)

```rust
use pl3xus_client::{SyncProvider, ClientTypeRegistry};
use std::sync::Arc;

#[component]
fn App() -> impl IntoView {
    let registry = Arc::new(
        ClientTypeRegistry::builder()
            .register::<Position>()
            .build()
    );

    view! {
        // SyncProvider handles WebSocket connection automatically
        <SyncProvider url="ws://localhost:8080/sync" registry=registry>
            <MyApp />
        </SyncProvider>
    }
}
```

### Leptos Web Client (manual WebSocket)

For custom WebSocket handling without pl3xus_client:

```rust
use leptos::prelude::*;
use leptos_use::use_websocket_with_options;
use pl3xus_websockets::Pl3xusBincodeSingleMsgCodec;

#[component]
fn ChatClient() -> impl IntoView {
    let UseWebSocketReturn { message, send, ready_state, .. } = 
        use_websocket_with_options::<MyMessage, ServerMessage, Pl3xusBincodeSingleMsgCodec, _, _>(
            "ws://127.0.0.1:8080",
            UseWebSocketOptions::default()
                .on_open(|_| log::info!("Connected!"))
                .on_close(|_| log::info!("Disconnected")),
        );

    // Use Effect to watch signals (not callbacks)
    Effect::new(move || {
        if let Some(msg) = message.get() {
            log::info!("Received: {:?}", msg);
        }
    });

    view! {
        <button on:click=move |_| send(&MyMessage { text: "Hello".into() })>
            "Send"
        </button>
    }
}
```

---

## Message Encoding

pl3xus uses **bincode** for binary message encoding, providing:

- Compact wire format (smaller than JSON)
- Fast serialization/deserialization
- Type-safe message handling

### Message Registration

```rust
use pl3xus::AppNetworkMessageExt;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
struct ChatMessage {
    sender: String,
    content: String,
}

// Register on both server and client
app.register_network_message::<ChatMessage, WebSocketProvider>();
```

### Receiving Messages

```rust
use pl3xus::NetworkData;

fn handle_chat(mut messages: EventReader<NetworkData<ChatMessage>>) {
    for msg in messages.read() {
        info!("From {}: {}", msg.sender, msg.content);
    }
}
```

---

## Connection Management

### Tracking Connected Clients

```rust
use pl3xus::{ConnectionId, NetworkEvent};
use bevy::utils::HashSet;

#[derive(Resource, Default)]
struct ConnectedClients(HashSet<ConnectionId>);

fn track_connections(
    mut events: EventReader<NetworkEvent>,
    mut clients: ResMut<ConnectedClients>,
) {
    for event in events.read() {
        match event {
            NetworkEvent::Connected(id) => {
                clients.0.insert(*id);
                info!("Client {} connected. Total: {}", id, clients.0.len());
            }
            NetworkEvent::Disconnected(id) => {
                clients.0.remove(id);
                info!("Client {} disconnected. Total: {}", id, clients.0.len());
            }
            _ => {}
        }
    }
}
```

### Graceful Disconnection

```rust
fn disconnect_client(
    mut net: ResMut<Network<WebSocketProvider>>,
    client_id: ConnectionId,
) {
    net.disconnect(client_id);
}
```

---

## Production Patterns

### 1. Health Check Endpoint

For load balancers and monitoring:

```rust
// Consider running a separate HTTP server for health checks
// WebSocket connections are long-lived and not ideal for health probes
```

### 2. Connection Limits

Prevent resource exhaustion:

```rust
#[derive(Resource)]
struct ConnectionLimits {
    max_connections: usize,
}

fn check_connection_limit(
    events: EventReader<NetworkEvent>,
    clients: Res<ConnectedClients>,
    limits: Res<ConnectionLimits>,
    mut net: ResMut<Network<WebSocketProvider>>,
) {
    // Disconnect new clients if at capacity
    for event in events.read() {
        if let NetworkEvent::Connected(id) = event {
            if clients.0.len() > limits.max_connections {
                warn!("Connection limit reached, disconnecting {}", id);
                net.disconnect(*id);
            }
        }
    }
}
```

### 3. Message Rate Limiting

Protect against spam:

```rust
use std::time::Instant;
use bevy::utils::HashMap;

#[derive(Resource, Default)]
struct MessageRateLimiter {
    last_message: HashMap<ConnectionId, Instant>,
    min_interval_ms: u64,
}

fn rate_limit_messages<T: Send + Sync + 'static>(
    mut messages: EventReader<NetworkData<T>>,
    mut limiter: ResMut<MessageRateLimiter>,
) {
    let now = Instant::now();

    for msg in messages.read() {
        if let Some(last) = limiter.last_message.get(&msg.source()) {
            if now.duration_since(*last).as_millis() < limiter.min_interval_ms as u128 {
                warn!("Rate limiting client {}", msg.source());
                continue;
            }
        }
        limiter.last_message.insert(msg.source(), now);
        // Process message...
    }
}
```

### 4. Graceful Shutdown

```rust
fn shutdown_server(
    mut net: ResMut<Network<WebSocketProvider>>,
    clients: Res<ConnectedClients>,
) {
    info!("Shutting down, disconnecting {} clients", clients.0.len());

    for client_id in clients.0.iter() {
        net.disconnect(*client_id);
    }
}
```

---

## Debugging

### Enable Tracing

```rust
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::new("pl3xus=debug,pl3xus_websockets=debug"))
        .init();

    // ... app setup
}
```

### Common Log Messages

| Message | Meaning |
|---------|---------|
| "Beginning connection" | Client starting WebSocket handshake |
| "Message read" | Received a message from wire |
| "Message deserialized and sent to pl3xus" | Message decoded and queued |
| "Channel depth warning" | Message queue filling up |

---

## Troubleshooting

### Connection Refused

**Symptom**: Client can't connect, "Connection refused" error.

**Causes**:
- Server not running
- Wrong port number
- Firewall blocking connection

**Fix**: Verify server is listening on the expected address.

### Messages Not Arriving

**Symptom**: Messages sent but not received.

**Causes**:
- Message type not registered
- Serialization mismatch

**Fix**: Ensure both sides register the same message types.

### High Latency

**Symptom**: Noticeable delay in message delivery.

**Causes**:
- Channel queue backing up
- Network congestion

**Fix**: Check for "Channel depth warning" logs, increase `channel_capacity`.

---

## Related Documentation

- [Sending Messages](./sending-messages.md) - Message patterns
- [Shared Types](./shared-types.md) - Type sharing between server/client
- [pl3xus_websockets README](https://github.com/vertec-io/pl3xus/tree/main/crates/pl3xus_websockets)
- [API Reference](https://docs.rs/pl3xus_websockets)

---

**Last Updated**: 2025-12-07
**pl3xus_websockets Version**: 0.17
```


