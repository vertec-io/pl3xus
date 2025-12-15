# API Reference

Detailed API documentation for the pl3xus ecosystem.

## Online Documentation

For the most up-to-date API documentation, see the docs.rs pages:

- **[pl3xus](https://docs.rs/pl3xus)** - Core networking library
- **[pl3xus_common](https://docs.rs/pl3xus_common)** - Shared types
- **[pl3xus_sync](https://docs.rs/pl3xus_sync)** - Server-side sync
- **[pl3xus_client](https://docs.rs/pl3xus_client)** - Leptos client
- **[pl3xus_websockets](https://docs.rs/pl3xus_websockets)** - WebSocket transport
- **[pl3xus_macros](https://docs.rs/pl3xus_macros)** - Procedural macros

## Contents

| Reference | Description |
|-----------|-------------|
| [Hooks Reference](./hooks-reference.md) | All pl3xus_client hooks |
| [Control Plugin](./control-plugin.md) | ExclusiveControlPlugin API |

## Quick Reference

### pl3xus_client Hooks

| Hook | Purpose |
|------|---------|
| `use_sync_component::<T>()` | Subscribe to component updates |
| `use_sync_component_store::<T>()` | Fine-grained reactive store |
| `use_sync_component_write::<T>()` | Write mutations to server |
| `use_connection_status()` | Monitor connection state |
| `use_sync_context()` | Access raw sync context |

### pl3xus_sync Extension Trait

| Method | Purpose |
|--------|---------|
| `app.sync_component::<T>(settings)` | Register component for sync |
| `app.register_network_message::<T, P>()` | Register message type |

### pl3xus Network Resource

| Method | Purpose |
|--------|---------|
| `net.send(conn_id, msg)` | Send to specific connection |
| `net.broadcast(msg)` | Send to all connections |
| `net.disconnect(conn_id)` | Disconnect a client |

## Building Local Documentation

Generate documentation locally:

```bash
# All crates
cargo doc --workspace --open

# Specific crate
cargo doc -p pl3xus_client --open
```

## Related Documentation

- [Getting Started](../getting-started/) - Quick start guides
- [Guides](../guides/) - In-depth tutorials
- [Architecture](../architecture/) - System design

