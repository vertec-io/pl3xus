# pl3xus_websockets

[![Crates.io](https://img.shields.io/crates/v/pl3xus_websockets)](https://crates.io/crates/pl3xus_websockets)
[![Docs.rs](https://docs.rs/pl3xus_websockets/badge.svg)](https://docs.rs/pl3xus_websockets)
[![License](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/jamescarterbell/pl3xus)

WebSocket transport provider for [pl3xus](https://github.com/jamescarterbell/pl3xus) with full WASM and native support.

## Supported Platforms

- WASM
- Windows
- Linux
- Mac

## Features

- ✅ **WASM Support** - Works in web browsers
- ✅ **Native Support** - Works on Linux, Windows, macOS
- ✅ **Async Runtime** - Uses `async-std` for cross-platform compatibility
- ✅ **Drop-in Replacement** - Easy to switch from TCP to WebSockets

## Getting Started

Add the dependency to your `Cargo.toml`:

```toml
[dependencies]
bevy = "0.17"
pl3xus = "1.1"
pl3xus_websockets = "1.1"
serde = { version = "1.0", features = ["derive"] }
```

**Important**: Bevy 0.17 requires Rust 1.88.0 (nightly). Create `rust-toolchain.toml`:
```toml
[toolchain]
channel = "nightly"
```

## Version Compatibility

| pl3xus_websockets | pl3xus | Bevy | Rust |
| :------------------: | :-------: | :--: | :--: |
| 1.1.1 | 1.1.1 | 0.17 | 1.88 (nightly) |
| 1.1.0 | 1.1.0 | 0.17 | 1.88 (nightly) |
| 0.2.0 | 0.9.0 | 0.16 | 1.85 |

### Basic Usage

```rust
use bevy::prelude::*;
use bevy::tasks::TaskPoolBuilder;
use pl3xus::{AppNetworkMessage, Pl3xusPlugin, Pl3xusRuntime};
use pl3xus_websockets::{WebSocketProvider, NetworkSettings};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Add the Pl3xusPlugin with WebSocketProvider
        .add_plugins(Pl3xusPlugin::<WebSocketProvider, bevy::tasks::TaskPool>::default())
        // Configure network settings
        .insert_resource(NetworkSettings::default())
        // Set up the async runtime
        .insert_resource(Pl3xusRuntime(
            TaskPoolBuilder::new().num_threads(2).build()
        ))
        // Register your messages
        .register_network_message::<YourMessage, WebSocketProvider>()
        .run();
}
```

### Network Settings

Configure the WebSocket connection:

```rust
use pl3xus_websockets::NetworkSettings;

// Default settings (localhost:3000)
app.insert_resource(NetworkSettings::default());

// Custom settings
app.insert_resource(NetworkSettings {
    ip: "127.0.0.1".to_string(),
    port: 8080,
});
```

## Examples

Check out the [examples directory](./examples) for complete working examples:

- **`server.rs`** - WebSocket chat server
- **`client.rs`** - WebSocket chat client with Bevy UI

Run the examples:
```bash
# Terminal 1 - Start the server
cargo run --example server -p pl3xus_websockets

# Terminal 2 - Start a client
cargo run --example client -p pl3xus_websockets
```

## WASM Compilation

To compile for WASM:

```bash
# Add the WASM target
rustup target add wasm32-unknown-unknown

# Build for WASM
cargo build --target wasm32-unknown-unknown --example client -p pl3xus_websockets
```

No special features or configuration needed - it just works! ✨

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
