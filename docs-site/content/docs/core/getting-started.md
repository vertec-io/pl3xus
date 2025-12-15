---
title: Getting Started with pl3xus
---
# Getting Started with pl3xus

Welcome! This guide will help you get started with the pl3xus ecosystem.

---

## 📖 Learning Path

We recommend following this learning path:

### 1. Core Networking (pl3xus)
**Time**: 15-30 minutes  
**Guide**: [pl3xus Getting Started](./index.md)

Learn the basics of networking with Bevy using pl3xus. This is the foundation for everything else.

**You'll learn**:
- Setting up a TCP server and client
- Sending and receiving messages
- Automatic message registration
- Type-safe networking

### 2. Server-Side Synchronization (pl3xus_sync)
**Time**: 30-45 minutes  
**Guide**: [pl3xus_sync Getting Started](../../sync/index.md)

Learn how to automatically synchronize ECS components from your Bevy server to clients.

**You'll learn**:
- Adding the Pl3xusSyncPlugin
- Registering components for sync
- Configuring sync settings
- Handling mutations

### 3. Client-Side Reactive UI (pl3xus_client)
**Time**: 30-45 minutes  
**Guide**: [pl3xus_client Getting Started](../../client/index.md)

Learn how to build reactive web UIs that display and edit synchronized data.

**You'll learn**:
- Setting up the SyncProvider
- Subscribing to components
- Displaying data reactively
- Implementing editable fields
- Using DevTools

### 4. Full Stack Application
**Time**: 45-60 minutes
**Guide**: [Control Demo Example](./examples/control-demo.md)

Put it all together by building a complete client-server application.

**You'll learn**:
- Project structure
- Shared types
- Complete server implementation
- Complete client implementation
- Running and testing

---

## 🎯 Quick Start by Use Case

### "I just want to send messages between Bevy systems"

→ Start with [pl3xus Getting Started](./index.md)

You only need the core `pl3xus` crate. Skip the sync and client guides.

### "I want to build a web-based control panel for my Bevy app"

→ Follow the full learning path:
1. [pl3xus Getting Started](./index.md)
2. [pl3xus_sync Getting Started](../../sync/index.md)
3. [pl3xus_client Getting Started](../../client/index.md)
4. [Control Demo Example](./examples/control-demo.md)

### "I want to build a distributed application"

→ Start with [pl3xus Getting Started](./index.md), then:
- Read the [Sending Messages Guide](./guides/sending-messages.md)
- Check out the [Examples](./examples/README.md)

You may not need pl3xus_sync/pl3xus_client for some applications - consider using pl3xus directly for more control.

---

## 📋 Prerequisites

### For All Guides

- **Rust**: Stable or nightly (nightly recommended for Bevy)
- **Bevy**: 0.17 or later
- **Basic Bevy knowledge**: Understanding of ECS, systems, and plugins

### For pl3xus_client Guides

- **Leptos**: 0.8
- **Trunk**: For building WASM applications
- **Basic web development knowledge**: HTML, CSS, JavaScript concepts

### Installation

**Trunk** (for client-side development):
```bash
cargo install trunk
```

**wasm32 target** (for client-side development):
```bash
rustup target add wasm32-unknown-unknown
```

---

## 🔑 Key Concepts

Before diving in, familiarize yourself with these key concepts:

### Automatic Message Registration (pl3xus)

pl3xus automatically registers message types - no boilerplate needed:

```rust
#[derive(Serialize, Deserialize)]
struct MyMessage {
    data: String,
}

// That's it! No manual registration required.
```

### Component Synchronization (pl3xus_sync)

`pl3xus_sync` automatically serializes and synchronizes components across the network:

```rust
#[derive(Component, Serialize, Deserialize)]
struct Position {
    x: f32,
    y: f32,
}

// Register for sync
app.register_sync_component::<Position>();
```

### Reactive Subscriptions (pl3xus_client)

pl3xus_client provides reactive hooks that automatically manage subscriptions:

```rust
// Subscribe to all Position components
let positions = use_sync_component::<Position>();

// Automatically updates when server sends changes
view! {
    <For each=move || positions.get() ...>
}
```

---

## 🚀 Ready to Start?

Choose your starting point:

- **[Installation](./installation.md)** - Add dependencies
- **[pl3xus Getting Started](./index.md)** - Core networking
- **[pl3xus_sync Getting Started](../../sync/index.md)** - Server-side sync
- **[pl3xus_client Getting Started](../../client/index.md)** - Client-side UI

---

## 📚 Additional Resources

- **[Architecture Overview](./architecture/README.md)** - Understand how it all works
- **[User Guides](./guides/README.md)** - Task-specific how-to guides
- **[API Reference](https://docs.rs/pl3xus)** - Detailed API documentation
- **[Examples](./examples/README.md)** - Real-world example applications

---

**Last Updated**: 2025-12-07
**Difficulty**: Beginner to Intermediate
**Estimated Time**: 2-3 hours for full learning path

