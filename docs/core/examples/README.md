# Examples

Working example walkthroughs for the pl3xus ecosystem.

## Contents

| Example | Description |
|---------|-------------|
| [Basic Example](./basic-example.md) | Simple client-server chat |
| [Control Demo](./control-demo.md) | Exclusive control transfer pattern |
| [Industrial Example](./industrial-example.md) | Robotics/industrial use case |

## Running Examples

### Core Pl3xus Examples

Located in `crates/pl3xus/examples/`:

```bash
# TCP chat server
cargo run --example server -p pl3xus

# TCP chat client (Bevy UI)
cargo run --example client -p pl3xus

# Automatic message registration demo
cargo run --example automatic_messages -p pl3xus -- server
cargo run --example automatic_messages -p pl3xus -- client
```

### WebSocket Examples

Located in `crates/pl3xus_websockets/examples/`:

```bash
# WebSocket server
cargo run --example server -p pl3xus_websockets

# WebSocket client
cargo run --example client -p pl3xus_websockets

# Hybrid server (HTTP + WebSocket)
cargo run --example hybrid_server -p pl3xus_websockets
```

### Control Demo

Located in `examples/control-demo/`:

```bash
# Start the server
cargo run -p control-demo-server

# Start the client (in another terminal)
cd examples/control-demo/client
trunk serve
```

Then open http://localhost:8080 in your browser.

## Example Architecture

### Basic Chat Example

```
┌─────────────────┐     TCP/WebSocket     ┌─────────────────┐
│   Chat Server   │◄────────────────────►│   Chat Client   │
│   (Bevy App)    │                       │   (Bevy App)    │
└─────────────────┘                       └─────────────────┘
```

### Control Demo

```
┌─────────────────┐                       ┌─────────────────┐
│  Control Server │     WebSocket         │  Web Client 1   │
│   (Bevy App)    │◄────────────────────►│   (Leptos)      │
│                 │                       └─────────────────┘
│  - Robots       │                       ┌─────────────────┐
│  - Control      │◄────────────────────►│  Web Client 2   │
│    Transfer     │                       │   (Leptos)      │
└─────────────────┘                       └─────────────────┘
```

## Key Patterns Demonstrated

### 1. Message Registration

```rust
// Automatic (recommended)
app.register_network_message::<ChatMessage, TcpProvider>();

// Explicit (for versioning)
app.listen_for_message::<ChatMessage, TcpProvider>();
```

### 2. Component Synchronization

```rust
// Server: Register components for sync
app.sync_component::<Position>(None);
app.sync_component::<EntityControl>(Some(SyncSettings {
    allow_mutations: true,
    ..default()
}));
```

### 3. Reactive Subscriptions

```rust
// Client: Subscribe to components
let positions = use_sync_component::<Position>();

view! {
    <For each=move || positions.get().iter()...>
        // Render each position
    </For>
}
```

### 4. Exclusive Control

```rust
// Request control of an entity
let request_control = use_request_control();
request_control(entity_id);

// Check if we have control
let has_control = use_has_control(entity_id);
```

## Related Documentation

- [Getting Started](../getting-started/) - Quick start guides
- [Guides](../guides/) - In-depth tutorials
- [API Reference](../api/) - Detailed API docs

