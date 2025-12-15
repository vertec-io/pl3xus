---
title: Reference
---
# Reference

Reference materials, glossary, troubleshooting, and FAQ.

## Contents

| Reference | Description |
|-----------|-------------|
| [Glossary](./glossary.md) | Terminology definitions |
| [Troubleshooting](./troubleshooting.md) | Common issues and solutions |
| [FAQ](./faq.md) | Frequently asked questions |

## Quick Troubleshooting

### Connection Issues

**Problem**: Client can't connect to server

1. Check the server is running and listening on the correct port
2. Verify the URL in the client matches the server address
3. Check for firewall rules blocking the connection
4. For WASM clients, ensure the server supports WebSocket

### Message Not Received

**Problem**: Messages sent but not received

1. Ensure both sides register the same message type
2. Check that you're reading `NetworkData<T>` events in your systems
3. Verify the connection is established via `NetworkEvent::Connected`

### Component Not Syncing

**Problem**: Component changes not appearing on client

1. Verify the component is registered with `sync_component::<T>()`
2. Check the client has subscribed to the component type
3. Ensure the component type is registered in the client's `ClientTypeRegistry`

### WASM Build Errors

**Problem**: Build fails for WASM target

1. Use `pl3xus_websockets` (TCP doesn't work in WASM)
2. Add the WASM target: `rustup target add wasm32-unknown-unknown`
3. Check for dependencies that don't support WASM

## Glossary Quick Reference

| Term | Definition |
|------|------------|
| **Connection** | A single client-server link |
| **Provider** | Transport implementation (TCP, WebSocket) |
| **Sync** | Automatic component synchronization |
| **Subscription** | Client request to receive component updates |
| **Mutation** | Client request to modify server component |
| **Conflation** | Combining multiple updates into one |

## Version Compatibility

| pl3xus | Bevy | Rust | Notes |
| :------------: | :--: | :--: | :---: |
| 1.1.x | 0.17 | 1.88 (nightly) | Current |
| 0.9.x | 0.16 | 1.85 | Maintenance |

## Getting Help

- **Discord**: [Bevy Discord](https://discord.gg/bevy) - look for `@SirCarter`
- **Issues**: [GitHub Issues](https://github.com/jamescarterbell/pl3xus/issues)
- **Documentation**: [docs.rs](https://docs.rs/pl3xus)

## Related Documentation

- [Getting Started](../getting-started/) - Quick start guides
- [Guides](../guides/) - In-depth tutorials
- [API Reference](../api/) - Detailed API docs

