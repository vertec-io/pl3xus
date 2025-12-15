# pl3xus_client

Reactive Leptos client library for building web UIs that synchronize with Bevy ECS servers.

[![Crates.io](https://img.shields.io/crates/v/pl3xus_client.svg)](https://crates.io/crates/pl3xus_client)
[![Documentation](https://docs.rs/pl3xus_client/badge.svg)](https://docs.rs/pl3xus_client)
[![License](https://img.shields.io/crates/l/pl3xus_client.svg)](https://github.com/vertec-io/pl3xus/blob/main/LICENSE)

---

## Overview

pl3xus_client is a reactive Leptos library for building web UIs that display and edit ECS components synchronized from Bevy servers via `pl3xus_sync`. It's designed for control panels, dashboards, and web-based tools for robotics, industrial automation, and networked applications.

### Key Features

- Reactive hooks for subscribing to components with automatic updates
- Compile-time type checking with Rust's type system
- Focus retention for editable fields during server updates
- Built-in component inspector for debugging
- Automatic subscription management (subscribe on mount, unsubscribe on unmount)
- No Bevy dependency, runs in WASM/browser

---

## Quick Start

### Installation

```toml
[dependencies]
leptos = "0.8"
pl3xus_client = "0.1"
serde = { version = "1.0", features = ["derive"] }
```

Install Trunk for building WASM:

```bash
cargo install trunk
rustup target add wasm32-unknown-unknown
```

### Shared Crate Pattern (Recommended)

Use the same shared crate as your server. See [pl3xus_sync README](../pl3xus_sync/README.md) for how to create it.

**Client `Cargo.toml`**:
```toml
[dependencies]
leptos = "0.8"
pl3xus_client = "0.1"
# Import shared types WITHOUT the "server" feature
shared_types = { path = "../shared_types" }
```

This pattern enables:
- Client builds without Bevy dependency (no "server" feature)
- Guaranteed type compatibility with server
- WASM compilation without Bevy
- Automatic `SyncComponent` trait implementation for all `Serialize + Deserialize` types

### Basic Usage

```rust
use leptos::prelude::*;
use pl3xus_client::{
    SyncProvider, use_sync_component, ClientRegistryBuilder
};
use shared_types::Position;

#[component]
pub fn App() -> impl IntoView {
    let registry = ClientRegistryBuilder::new()
        .register::<Position>()
        .build();

    view! {
        <SyncProvider url="ws://localhost:8082" registry=registry>
            <AppView/>
        </SyncProvider>
    }
}

#[component]
fn AppView() -> impl IntoView {
    // Automatically subscribes to Position components
    let positions = use_sync_component::<Position>();

    view! {
        <div class="app-view">
            <h1>"Entities"</h1>
            <For
                each=move || {
                    positions.get()
                        .iter()
                        .map(|(id, pos)| (*id, pos.clone()))
                        .collect::<Vec<_>>()
                }
                key=|(id, _)| *id
                let:item
            >
                {
                    let (entity_id, position) = item;
                    view! {
                        <div class="entity">
                            "Entity " {entity_id} ": "
                            "x=" {position.x} ", y=" {position.y}
                        </div>
                    }
                }
            </For>
        </div>
    }
}
```



### Editable Fields

```rust
use pl3xus_client::SyncFieldInput;

#[component]
fn PositionEditor(entity_id: u64) -> impl IntoView {
    view! {
        <div class="editor">
            <label>
                "X: "
                <SyncFieldInput
                    entity_id=entity_id
                    field_accessor=|pos: &Position| pos.x
                    field_mutator=|pos: &Position, new_x: f32| {
                        Position { x: new_x, y: pos.y }
                    }
                    input_type="number"
                />
            </label>
        </div>
    }
}
```

Features:
- Input retains focus when server updates arrive
- Press Enter to send mutation to server
- Click away to discard changes and revert to server value

### DevTools

```rust
use pl3xus_client::DevTools;

view! {
    <SyncProvider url="ws://localhost:8082" registry=registry>
        <AppView/>
        <DevTools/>  // Add DevTools
    </SyncProvider>
}
```

---

## Build and Run

Create `index.html`:

```html
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8"/>
    <title>My Client</title>
</head>
<body></body>
</html>
```

Build and serve:

```bash
trunk serve --port 8080
```

Open `http://localhost:8080` in your browser.

---

## Documentation

- **[Getting Started Guide](../../docs/getting-started/pl3xus-client.md)** - Step-by-step tutorial
- **[API Documentation](https://docs.rs/pl3xus_client)** - Complete API reference
- **[Mutations Guide](../../docs/guides/mutations.md)** - Advanced mutation patterns
- **[DevTools Guide](../../docs/guides/devtools.md)** - DevTools features
- **[Examples](./examples/)** - Working code examples

---

## Examples

See the `examples/` directory for complete working examples:

- **`basic_client/`** - Minimal getting started example
- **`devtools_demo/`** - DevTools integration example

Run an example:

```bash
# Terminal 1: Start server
cargo run -p pl3xus_client --example basic_server

# Terminal 2: Start client
cd crates/pl3xus_client/examples/basic_client
trunk serve --port 8080
```

---

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.

---

**Part of the [pl3xus](https://github.com/vertec-io/pl3xus) ecosystem**

