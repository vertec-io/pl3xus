---
title: Installation
---
# Installation

This guide covers installing pl3xus and its dependencies.

## Prerequisites

### Rust Nightly

Bevy 0.17 requires Rust 1.88.0 (nightly). Create a `rust-toolchain.toml` in your project root:

```toml
[toolchain]
channel = "nightly"
```

Then run:

```bash
rustup update
```

### Bevy

pl3xus 1.1.x requires Bevy 0.17:

```toml
[dependencies]
bevy = "0.17"
```

## Core Installation

Add the core pl3xus crate:

```toml
[dependencies]
bevy = "0.17"
pl3xus = "1.1"
serde = { version = "1.0", features = ["derive"] }
```

## Transport Providers

Choose a transport provider based on your needs:

### WebSocket (Recommended for Web)

```toml
[dependencies]
pl3xus_websockets = "1.1"
```

Supports:
- ✅ Native (Linux, Windows, macOS)
- ✅ WASM (Web browsers)

### TCP (Built-in)

TCP is included in the core `pl3xus` crate. No additional dependency needed.

Supports:
- ✅ Native (Linux, Windows, macOS)
- ❌ WASM (not supported)

### Memory (Testing)

```toml
[dependencies]
pl3xus_memory = "1.1"
```

For in-memory testing without network overhead.

## Sync & Client

For ECS component synchronization:

### Server-Side (Bevy)

```toml
[dependencies]
pl3xus_sync = "1.1"
```

### Client-Side (Leptos)

```toml
[dependencies]
pl3xus_client = "1.1"
leptos = "0.8"
```

## Complete Example

### Server Cargo.toml

```toml
[package]
name = "my-server"
version = "0.1.0"
edition = "2024"

[dependencies]
bevy = "0.17"
pl3xus = "1.1"
pl3xus_websockets = "1.1"
pl3xus_sync = "1.1"
serde = { version = "1.0", features = ["derive"] }
```

### Client Cargo.toml

```toml
[package]
name = "my-client"
version = "0.1.0"
edition = "2024"

[dependencies]
leptos = "0.8"
pl3xus_client = "1.1"
serde = { version = "1.0", features = ["derive"] }
wasm-bindgen = "0.2"
```

## Verifying Installation

Create a simple test to verify everything is working:

```rust
use bevy::prelude::*;
use pl3xus::{Pl3xusPlugin, Pl3xusRuntime};
use pl3xus_websockets::WebSocketProvider;
use bevy::tasks::TaskPoolBuilder;

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins(Pl3xusPlugin::<WebSocketProvider, bevy::tasks::TaskPool>::default())
        .insert_resource(Pl3xusRuntime(
            TaskPoolBuilder::new().num_threads(2).build()
        ))
        .run();
}
```

If this compiles and runs without errors, you're ready to go!

## Next Steps

- [Core Pl3xus Guide](./index.md) - Learn the basics
- [Server Sync Guide](../../sync/index.md) - Set up component synchronization
- [Client Guide](../../client/index.md) - Build reactive web UIs

