---
title: What is Pl3xus
---
# What is Pl3xus

Pl3xus is an **event-driven networking framework** for Bevy applications. It enables real-time synchronization between Bevy ECS servers and web clients.

---

## Philosophy

### Transport Agnostic

Pl3xus doesn't tie you to a specific transport protocol. Use TCP for LAN applications, WebSockets for browser clients, or implement your own custom provider. The message-passing layer remains the same.

### ECS-Native Design

Unlike traditional networking libraries that bolt onto existing architectures, Pl3xus is built around Bevy's Entity Component System. Components are the primary unit of synchronization—changes are detected automatically and replicated to subscribers.

### Type Safety First

Every message is strongly typed at compile time. No magic strings, no runtime type checks, no serialization surprises. Your Rust types define the contract between server and client.

### Minimal Boilerplate

Register a component with `app.sync_component::<T>(None)` and it's automatically synchronized. No manual change tracking, no event handlers to wire up, no state machines to manage.

---

## When to Use Pl3xus

### ✅ Real-Time Dashboards

Display live ECS state in web browsers. Perfect for monitoring industrial systems, IoT devices, or any application where operators need real-time visibility.

### ✅ Control Interfaces

Build web-based control panels that can read and mutate server-side components. Includes authorization hooks for production-safe deployments.

### ✅ Development Tools

Inspect and modify your Bevy application's state in real-time during development. The built-in DevTools provide entity browsing, component editing, and hierarchy visualization.

### ✅ Distributed Simulations

Synchronize simulation state across multiple connected clients. The conflation system handles high-frequency updates efficiently.

---

## When NOT to Use Pl3xus

### ❌ Simple REST APIs

If you just need request/response endpoints, use Axum or Actix-web. Pl3xus is designed for persistent, bidirectional connections.

### ❌ Traditional Game Networking

For games requiring client-side prediction, lag compensation, and matchmaking, consider dedicated game networking libraries. Pl3xus focuses on authoritative server patterns.

### ❌ Simple CRUD Applications

If your data model is static and updates are infrequent, a traditional database-backed API is simpler. Pl3xus shines when state is dynamic and updates are continuous.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     Bevy Server                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   Systems   │  │ Components  │  │   Pl3xusSync     │  │
│  │  (Update)   │→ │  (Change)   │→ │  (Detect & Send)    │  │
│  └─────────────┘  └─────────────┘  └──────────┬──────────┘  │
└───────────────────────────────────────────────┼─────────────┘
                                                │
                              WebSocket / TCP   │
                                                ↓
┌─────────────────────────────────────────────────────────────┐
│                     Web Client                              │
│  ┌─────────────────────┐  ┌─────────────────────────────┐   │
│  │   SyncProvider      │  │   Reactive UI (Leptos)      │   │
│  │  (Receive & Parse)  │→ │   (Display & Edit)          │   │
│  └─────────────────────┘  └─────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

---

## Getting Started

Ready to build? Start with the [Quick Start Guide](./index.md).
