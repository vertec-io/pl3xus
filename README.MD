# pl3xus

A server-authoritative real-time synchronization framework for building industrial-grade applications with Bevy ECS servers and Leptos WASM clients.

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## Documentation

**[pl3xus Documentation](https://www.pl3xus.com/docs)** - Complete guides, API reference, and tutorials.

## Why pl3xus?

The Entity Component System (ECS) architecture has proven itself in game engines, but its benefits extend far beyond games. ECS provides the foundation for building high-performance, reliable, and maintainable real-time systems:

- **Data-oriented design** enables predictable, cache-friendly performance even with thousands of entities
- **Concurrent system execution** handles complex machinery with many independent subsystems
- **The component model** naturally maps to industrial concepts: a robot arm is an entity, its position, velocity, and status are components, and control logic lives in systems

pl3xus builds on this foundation to make developing industrial applications with Rust and modern web-based interfaces easier than ever.

### The Problem with Existing Solutions

Traditional networking libraries for Bevy focus on game development patterns: client-side prediction, lag compensation, and peer-to-peer architectures. These patterns don't fit industrial and automation applications where the server must be the single source of truth.

pl3xus takes a different approach. It's built for **server-authoritative architectures** where you need:

- **Real-time HMI/SCADA interfaces**: Build web-based control panels for robots, PLCs, and industrial equipment
- **Multi-operator systems**: Multiple users viewing and controlling shared resources with proper authorization
- **Live monitoring dashboards**: Stream system state to browsers without polling
- **Development tooling**: Built-in DevTools for entity browsing, component inspection, and real-time debugging

The core idea is simple: register a Bevy component for synchronization on the server, subscribe to it on the client, and pl3xus handles change detection, serialization, transmission, and reactive UI updates automatically.

```rust
// Server: one line to sync a component
app.sync_component::<RobotPosition>(None);

// Client: one line to subscribe and display
let positions = use_components::<RobotPosition>();
```

## How It Differs

| Capability | pl3xus | bevy_renet | Matchbox |
|------------|--------|------------|----------|
| ECS Component Sync | Yes | No | No |
| Leptos Web Client | Yes | No | No |
| Browser Support | Yes | No | Yes |
| Game Prediction | No | Yes | Yes |
| Server-Authoritative | Yes | Yes | No |

If you're building multiplayer games with client prediction, use bevy_renet or Matchbox. If you're building control systems, dashboards, or industrial applications, pl3xus is designed for you.

## Features

- **Transport Agnostic**: Use TCP, WebSockets, or implement your own transport layer
- **Cross-Platform**: Works on Linux, Windows, macOS, and WASM
- **Event-Driven**: Integrates with Bevy's ECS event system
- **ECS Synchronization**: Automatic server-to-client component sync with change detection
- **Modular Architecture**: Lightweight core with optional features and transport providers
- **Type-Safe**: Strongly typed messages with compile-time guarantees
- **Async Runtime Agnostic**: Works with any async runtime (Bevy TaskPool, Tokio, etc.)
- **Reactive Web Client**: Leptos-based client for building real-time web UIs

## AI-Native Development

pl3xus is built as an AI-native framework. The [`/skills`](./skills) directory contains a collection of reusable AI agent skills following [Anthropic's skills protocol](https://github.com/anthropics/skills), giving your agents the tools they need to write expert-level industrial and automation applications.

The skills system includes:

- **Skills Registry**: Agent-optimized pattern reference with mandatory production patterns
- **Framework Skills**: Server setup, client hooks, queries, mutations, and authorization
- **Foundation Skills**: Bevy ECS fundamentals and Leptos UI patterns for industrial applications
- **Complete Workflows**: End-to-end guidance from architecture through deployment

When you invoke a skill like `"Use the pl3xus-development skill to create a robot control application"`, your agent receives the same patterns and practices a senior engineer with deep pl3xus expertise would apply.

## Core Crates

| Crate | Description |
| :---- | :---------- |
| [`pl3xus`](./crates/pl3xus) | Core networking library |
| [`pl3xus_common`](./crates/pl3xus_common) | Shared types and utilities |
| [`pl3xus_macros`](./crates/pl3xus_macros) | Procedural macros |
| [`pl3xus_async`](./crates/pl3xus_async) | Async runtime integration |

## Transport Providers

| Provider | Platforms | WASM | Crate |
| :------- | :-------- | :--: | :---- |
| TCP | Linux, Windows, macOS | No | Built-in |
| WebSocket | Linux, Windows, macOS, WASM | Yes | [`pl3xus_websockets`](./crates/pl3xus_websockets) |
| Memory | All | Yes | [`pl3xus_memory`](./crates/pl3xus_memory) |

## Sync and Client

| Crate | Description |
| :---- | :---------- |
| [`pl3xus_sync`](./crates/pl3xus_sync) | Server-side ECS component synchronization |
| [`pl3xus_client`](./crates/pl3xus_client) | Leptos-based reactive web client |

## Bevy Version Compatibility

| pl3xus | Bevy | Rust |
| :----: | :--: | :--: |
| 1.2 | 0.19 | 1.96 (nightly) |
| 1.1 | 0.17 | 1.88 (nightly) |

## Supported Platforms

| Platform | Status |
| :------- | :----- |
| Linux | Fully Supported |
| Windows | Fully Supported |
| macOS | Supported |
| WASM | Supported (WebSocket transport) |

## License

Licensed under the MIT License. See [LICENSE](LICENSE) for details.

Copyright 2025 Vertec
