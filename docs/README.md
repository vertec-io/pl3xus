# pl3xus Documentation

Welcome to the comprehensive documentation for the pl3xus ecosystem!

## 🚀 Quick Start

**New to pl3xus?** Start here:

1. **[Installation](core/installation.md)** - Add pl3xus to your project
2. **[Server-Side Sync](sync/index.md)** - Synchronize ECS components to clients
3. **[Client-Side Reactive UI](client/index.md)** - Build reactive web UIs with Leptos
4. **[Core Networking](core/getting-started.md)** - TCP-based networking with Bevy

---

## 📚 Documentation Structure

This documentation is organized into the following sections:

### getting-started/
Step-by-step guides to get you up and running quickly.

### architecture/
Deep dives into system architecture and design.

### guides/
How-to guides for specific tasks and features:
- [Server Setup](core/guides/server-setup.md) - Configure Pl3xusSyncPlugin
- [Hooks](core/guides/hooks.md) - All 9 Leptos hooks for reactive sync
- [Subscriptions](core/guides/subscriptions.md) - Component subscription lifecycle
- [Mutations](core/guides/mutations.md) - Client-driven component mutations
- [Type Registry](core/guides/type-registry.md) - ClientTypeRegistry configuration
- [Connection Management](core/guides/connection-management.md) - Connection lifecycle
- [Shared Types](core/guides/shared-types.md) - Sharing types between server/client
- [WebSocket Patterns](core/guides/websocket-patterns.md) - Production WebSocket patterns
- [DevTools](core/guides/devtools.md) - DevTools setup and usage
- [Sending Messages](core/guides/sending-messages.md) - Direct vs scheduled messaging

### api/
API reference and quick reference guides.

### examples/
Detailed walkthroughs of example applications.

### migration/
Migration guides for upgrading between versions.

### reference/
Reference materials, glossary, troubleshooting, and FAQ.

---

## 🔑 Key Concepts

### The pl3xus Ecosystem

**pl3xus** is a modular networking ecosystem for Bevy applications:

```
┌─────────────────────────────────────────────────────────────────┐
│                        Your Application                          │
├─────────────────────────────────────────────────────────────────┤
│  pl3xus_client    │  pl3xus_sync    │  pl3xus          │
│  (Leptos Web UI)     │  (Server Sync)     │  (Core Networking)  │
├─────────────────────────────────────────────────────────────────┤
│                    pl3xus_websockets                          │
│                    (Transport Provider)                          │
├─────────────────────────────────────────────────────────────────┤
│                    pl3xus_common                              │
│                    (Shared Types)                                │
└─────────────────────────────────────────────────────────────────┘
```

- **pl3xus** - Core networking library (TCP, WebSocket, custom transports)
- **pl3xus_sync** - Server-side ECS component synchronization
- **pl3xus_client** - Reactive Leptos client library for web UIs
- **pl3xus_websockets** - WebSocket transport provider
- **pl3xus_memory** - In-memory transport for testing

### Core Features

**Automatic Message Registration** (pl3xus)
- Zero boilerplate networking
- Just derive `Serialize + Deserialize`
- Type-safe message handling

**Bincode-Based Sync** (pl3xus_sync)
- Automatic component synchronization
- Opt-in per component type
- Configurable sync settings
- Mutation authorization

**Reactive Subscriptions** (pl3xus_client)
- Automatic subscription management
- Fine-grained reactivity with Leptos
- Focus-retaining editable fields
- Built-in DevTools

---

## 📖 Documentation Status

| Section | Status |
|---------|--------|
| Getting Started | ✅ Complete |
| Guides | ✅ Complete (10 guides) |
| API Reference | 🔧 In Progress |
| Examples | 🔧 In Progress |
| Contributing | ✅ Complete |

**Last Updated**: 2025-12-07
**pl3xus Version**: 1.1.1
**Bevy Version**: 0.17.2
