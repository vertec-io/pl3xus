# Glossary

Terminology used throughout the pl3xus documentation.

## Core Concepts

### Connection
A single client-server link. Each connection has a unique `ConnectionId`.

### ConnectionId
A unique identifier for a network connection. Used to send messages to specific clients.

### Provider
A transport implementation. Examples: `TcpProvider`, `WebSocketProvider`.

### NetworkProvider
The trait that transport implementations must implement. Defines how to connect, listen, send, and receive data.

### NetworkPacket
The wire-level message format. Contains type name, schema hash, and serialized data.

### `NetworkData<T>`
A wrapper around received messages that includes the source `ConnectionId`.

### NetworkEvent
Events for connection lifecycle: `Connected`, `Disconnected`, `Error`.

## Sync Concepts

### Sync
Automatic synchronization of ECS components from server to clients.

### Subscription
A client's request to receive updates for a specific component type.

### Mutation
A client's request to modify a server-side component.

### Conflation
Combining multiple updates for the same entity/component into a single message. Reduces bandwidth.

### SyncSettings
Configuration for component synchronization: rate limiting, conflation, mutations.

### ComponentChangeEvent
Event emitted when a synchronized component changes.

### EntityDespawnEvent
Event emitted when a synchronized entity is despawned.

## Client Concepts

### ClientTypeRegistry
Client-side registry mapping type names to deserializers.

### SyncProvider
Leptos component that provides sync context to children.

### use_sync_component
Hook for subscribing to component updates.

### use_sync_component_store
Hook for fine-grained reactive access to component data.

### use_sync_component_write
Hook for sending mutations to the server.

## Control Concepts

### Exclusive Control
Pattern where only one client can control an entity at a time.

### EntityControl
Component tracking which client has control of an entity.

### ExclusiveControlPlugin
Plugin that implements the exclusive control pattern.

### Control Transfer
The process of one client releasing control and another acquiring it.

## Message Concepts

### Pl3xusMessage
Trait for types that can be sent over the network. Automatically implemented for `Serialize + Deserialize + Send + Sync + 'static`.

### NetworkMessage
Legacy trait for explicit message naming. Still supported for versioning.

### OutboundMessage
Event for scheduled message sending via `MessageWriter`.

### MessageReader
Bevy event reader for network messages.

### MessageWriter
Bevy event writer for outbound messages.

## Transport Concepts

### TCP
Transmission Control Protocol. Reliable, ordered delivery. Native only.

### WebSocket
Full-duplex communication over HTTP. Works in browsers (WASM).

### WASM
WebAssembly. Allows Rust code to run in web browsers.

### Bincode
Binary serialization format used for wire messages. Compact and fast.

## Architecture Concepts

### ECS
Entity Component System. Bevy's core architecture pattern.

### Plugin
Bevy's modular extension mechanism.

### Resource
Bevy's global state container.

### System
Bevy's unit of logic that operates on entities and components.

### SystemSet
Bevy's mechanism for ordering and grouping systems.

## Related Documentation

- [Troubleshooting](./troubleshooting.md) - Common issues
- [FAQ](./faq.md) - Frequently asked questions
- [Architecture](../architecture/) - System design

