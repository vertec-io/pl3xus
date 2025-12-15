# Frequently Asked Questions

Common questions about pl3xus.

## General

### What is pl3xus?

pl3xus is a modular, event-driven networking solution for Bevy applications. It provides:

- Core networking with TCP and WebSocket support
- Automatic ECS component synchronization
- Reactive web client library for Leptos

### How does it compare to other Bevy networking solutions?

| Feature | pl3xus | bevy_renet | bevy_replicon |
|---------|---------------|------------|---------------|
| Transport | TCP, WebSocket | UDP (QUIC) | UDP |
| WASM | ✅ (WebSocket) | ❌ | ❌ |
| Auto Sync | ✅ | ❌ | ✅ |
| Web Client | ✅ (Leptos) | ❌ | ❌ |

### Why Rust nightly?

Bevy 0.17 requires Rust 1.88.0 features (Edition 2024). Once Rust 1.88 is released to stable (expected Q1 2026), you can switch back to stable.

## Architecture

### Can I use pl3xus without pl3xus_sync?

Yes! The core `pl3xus` crate works standalone for basic networking. Use `pl3xus_sync` only if you need automatic component synchronization.

### Can I use pl3xus_client without Leptos?

Currently, `pl3xus_client` is designed for Leptos. For other frameworks, you can use the raw WebSocket connection with `pl3xus_common` types.

### What serialization format is used?

Bincode for all wire messages. It's compact, fast, and type-safe.

## Usage

### How do I send messages to specific clients?

Use `net.send(connection_id, message)`:

```rust
fn send_to_client(net: Res<Network<TcpProvider>>) {
    net.send(ConnectionId { id: 0 }, MyMessage { ... });
}
```

### How do I broadcast to all clients?

Use `net.broadcast(message)`:

```rust
fn broadcast(net: Res<Network<TcpProvider>>) {
    net.broadcast(MyMessage { ... });
}
```

### How do I know when a client connects?

Listen for `NetworkEvent::Connected`:

```rust
fn handle_connections(mut events: MessageReader<NetworkEvent>) {
    for event in events.read() {
        if let NetworkEvent::Connected(id) = event {
            info!("Client connected: {:?}", id);
        }
    }
}
```

### How do I sync only some components?

Register only the components you want to sync:

```rust
// Only Position and Velocity will sync
app.sync_component::<Position>(None);
app.sync_component::<Velocity>(None);
// Health won't sync - not registered
```

### How do I allow clients to modify components?

Enable mutations in `SyncSettings`:

```rust
app.sync_component::<Position>(Some(SyncSettings {
    allow_mutations: true,
    ..default()
}));
```

## Performance

### How many clients can it handle?

Depends on your use case. For typical applications:
- Hundreds of clients with moderate sync rates
- Thousands with careful optimization (conflation, batching)

### How do I reduce bandwidth?

1. **Conflation** - Combine updates for same entity
2. **Rate limiting** - Reduce sync frequency
3. **Selective sync** - Only sync needed components

### Is it suitable for real-time applications?

Yes, with appropriate configuration. Used in robotics and industrial control where low latency is critical.

## Troubleshooting

### Why aren't my messages being received?

See [Troubleshooting - Messages Not Received](./troubleshooting.md#messages-not-received).

### Why won't my WASM build compile?

See [Troubleshooting - WASM Build Fails](./troubleshooting.md#wasm-build-fails).

### Where can I get help?

- **Discord**: [Bevy Discord](https://discord.gg/bevy) - look for `@SirCarter`
- **Issues**: [GitHub Issues](https://github.com/jamescarterbell/pl3xus/issues)
- **Documentation**: [docs.rs](https://docs.rs/pl3xus)

## Contributing

### How can I contribute?

See the [Contributing Guide](../../README.MD#contributing) in the main README.

### Can I add a new transport provider?

Yes! Implement the `NetworkProvider` trait. See existing providers for examples.

