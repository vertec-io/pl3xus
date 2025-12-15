# Architecture

This section covers the architecture and design of the pl3xus ecosystem.

## Contents

- **[Ecosystem Overview](./ecosystem.md)** - How the crates work together
- **[Sync Protocol](./sync-protocol.md)** - Wire protocol for component synchronization
- **[Message Flow](./message-flow.md)** - How messages flow through the system
- **[Control Transfer](./control-transfer.md)** - Exclusive control pattern

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     Bevy Server Application                      │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                    pl3xus_sync                            ││
│  │  - Component change detection                                ││
│  │  - Subscription management                                   ││
│  │  - Mutation authorization                                    ││
│  │  - Conflation & batching                                     ││
│  └─────────────────────────────────────────────────────────────┘│
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                      pl3xus                               ││
│  │  - Message serialization (bincode)                           ││
│  │  - Connection management                                     ││
│  │  - Event-driven message handling                             ││
│  └─────────────────────────────────────────────────────────────┘│
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                 pl3xus_websockets                         ││
│  │  - WebSocket transport                                       ││
│  │  - Native + WASM support                                     ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ WebSocket (bincode)
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Web Client (Leptos)                          │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                   pl3xus_client                           ││
│  │  - Reactive subscriptions                                    ││
│  │  - Type registry                                             ││
│  │  - DevTools                                                  ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

## Key Design Principles

### 1. Transport Agnostic

The core `pl3xus` crate is transport-agnostic. You can use:
- TCP (built-in)
- WebSockets (`pl3xus_websockets`)
- In-memory (`pl3xus_memory`)
- Custom transports (implement `NetworkProvider`)

### 2. Opt-In Synchronization

Components are only synchronized if explicitly registered:

```rust
app.sync_component::<Position>(None);  // Sync Position to all clients
```

### 3. Binary Wire Format

All messages use bincode serialization for:
- Compact wire format
- Fast serialization/deserialization
- Type safety

### 4. Event-Driven

The system integrates with Bevy's ECS event system:
- Messages arrive as `NetworkData<T>` events
- Connection events via `NetworkEvent`
- Component changes via `ComponentChangeEvent`

## Related Documentation

- [Getting Started](../getting-started/) - Quick start guides
- [Guides](../guides/) - In-depth tutorials
- [API Reference](../api/) - Detailed API docs

