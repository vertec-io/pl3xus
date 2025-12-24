---
title: What is pl3xus?
---
# What is pl3xus?

pl3xus is a **server-authoritative real-time synchronization framework** for building industrial-grade applications with Bevy ECS servers and Leptos WASM clients.

## The Problem

Building real-time applications that synchronize state between a server and multiple clients is hard:

- **State Management**: Keeping client state in sync with server state requires careful coordination
- **Boilerplate**: Traditional approaches require writing serialization, message handlers, and state management code for every piece of data
- **Authorization**: Controlling who can read or modify what data adds complexity
- **Real-time Updates**: Efficiently pushing changes to clients without polling is non-trivial
- **Type Safety**: Maintaining type safety across the network boundary is challenging

## The Solution

pl3xus solves these problems with a **component-centric synchronization model**:

```rust
// Server: Register a component for synchronization
app.sync_component::<RobotPosition>(None);

// Client: Subscribe and display - that's it!
let positions = use_components::<RobotPosition>();
```

**One line on the server. One line on the client.** Changes are automatically detected, serialized, transmitted, and reactively updated in your UI.

---

## Core Principles

### 1. Server-Authoritative

The server is the **single source of truth**. Clients display server state and can request mutations, but the server always has the final say. This eliminates entire classes of bugs related to state synchronization.

### 2. Component-Centric

Components are the unit of synchronization. Register a Bevy component for sync, and pl3xus handles:
- Change detection
- Serialization
- Network transmission
- Client-side reactive updates

### 3. Zero Boilerplate

No manual event handlers. No state machines. No serialization code. Just register components and use hooks.

### 4. Type-Safe

Your Rust types define the contract. Compile-time type checking ensures server and client agree on data shapes.

### 5. TanStack Query-Inspired API

Familiar patterns for developers coming from React/TanStack Query:

```rust
// Mutations with loading states
let mutation = use_mutation::<UpdatePosition>(|result| {
    match result {
        Ok(_) => log!("Updated!"),
        Err(e) => log!("Error: {e}"),
    }
});

mutation.send(UpdatePosition { x: 10.0, y: 20.0 });

// Check loading state
if mutation.is_loading() {
    // Show spinner
}
```

---

## When to Use pl3xus

### ✅ Industrial Control Systems

Build web-based HMIs (Human-Machine Interfaces) for robots, PLCs, and industrial equipment. Real-time state display with authorized control.

### ✅ Real-Time Dashboards

Monitor live system state in web browsers. Perfect for IoT, simulation monitoring, or any application requiring real-time visibility.

### ✅ Multi-Client Applications

Multiple operators viewing and controlling shared resources with proper authorization and control handoff.

### ✅ Development Tools

The built-in DevTools provide entity browsing, component inspection, and real-time state editing during development.

---

## When NOT to Use pl3xus

### ❌ Simple REST APIs

If you just need request/response endpoints, use Axum or Actix-web. pl3xus is for persistent, bidirectional connections.

### ❌ Game Networking

For games requiring client-side prediction and lag compensation, use dedicated game networking libraries. pl3xus focuses on authoritative server patterns.

### ❌ Static CRUD Applications

If your data rarely changes and you don't need real-time updates, a traditional database-backed API is simpler.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     Bevy Server                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   Systems   │  │ Components  │  │   pl3xus_sync       │  │
│  │  (Update)   │→ │  (Change)   │→ │  (Detect & Send)    │  │
│  └─────────────┘  └─────────────┘  └──────────┬──────────┘  │
└───────────────────────────────────────────────┼─────────────┘
                                                │
                              WebSocket         │
                                                ↓
┌─────────────────────────────────────────────────────────────┐
│                     Leptos Client                           │
│  ┌─────────────────────┐  ┌─────────────────────────────┐   │
│  │   pl3xus_client     │  │   Reactive UI (Leptos)      │   │
│  │  (Receive & Parse)  │→ │   (Display & Edit)          │   │
│  └─────────────────────┘  └─────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

---

## Next Steps

Ready to build? Start with the [Quick Start Guide](./getting-started.md).

Want to understand the concepts first? Read about [Core Concepts](./guides/index.md).

