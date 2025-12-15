# Getting Started with pl3xus_client

pl3xus_client is a reactive Leptos library for building web UIs that synchronize with Bevy ECS servers via pl3xus_sync.

Time: 30-45 minutes
Difficulty: Intermediate
Prerequisites: Basic Leptos knowledge, pl3xus_sync server running

---

## Overview

pl3xus_client provides:
- Reactive hooks for subscribing to components with automatic updates
- Compile-time type checking
- Focus retention for editable fields during server updates
- Built-in component inspector
- Automatic subscription management (subscribe on mount, unsubscribe on unmount)

---

## Installation

Add to your `Cargo.toml`:

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

---

## Quick Start

### Step 1: Use the Shared Crate

Use the same shared crate as your server (see [pl3xus_sync Getting Started](../../sync/index.md) for how to create it).

**Client `Cargo.toml`**:

```toml
[dependencies]
leptos = "0.8"
pl3xus_client = "0.1"
serde = { version = "1.0", features = ["derive"] }
# Import shared types WITHOUT the "server" feature
shared_types = { path = "../shared_types" }
```

This pattern enables:
- Client builds without Bevy dependency (no "server" feature)
- Identical types on server and client (`Position`, `Velocity`, etc.)
- WASM compilation without Bevy
- Compile-time type safety guarantees

### Step 2: Automatic SyncComponent Implementation

The `SyncComponent` trait is automatically implemented for all types that are `Serialize + Deserialize + Send + Sync + 'static`.

Simply derive `Serialize` and `Deserialize` on your types:

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}
```

The implementation:
- Provides a blanket implementation of `SyncComponent`
- Extracts component names using `std::any::type_name::<T>()`
- Caches names for performance (approximately 500ns first call, 50-100ns subsequent)
- Matches the server-side behavior in pl3xus_sync

### Step 3: Set Up the Client Registry

Create a registry that maps type names to deserializers:

```rust
use leptos::prelude::*;
use pl3xus_client::{SyncProvider, ClientRegistryBuilder};
use shared_types::{Position, Velocity};

#[component]
pub fn App() -> impl IntoView {
    let registry = ClientRegistryBuilder::new()
        .register::<Position>()
        .register::<Velocity>()
        .build();

    view! {
        <SyncProvider
            url="ws://localhost:8082"
            registry=registry
        >
            <AppView/>
        </SyncProvider>
    }
}
```

### Step 4: Subscribe to Components

Use the `use_sync_component` hook to subscribe and display data:

```rust
use pl3xus_client::use_sync_component;

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

### Step 5: Create index.html

```html
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8"/>
    <title>My Web Client</title>
</head>
<body></body>
</html>
```

### Step 6: Build and Run

```bash
trunk serve --port 8080
```

Open `http://localhost:8080` in your browser to see the synchronized entities.

---

## Editable Fields

To allow users to edit component values:

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
            <label>
                "Y: "
                <SyncFieldInput
                    entity_id=entity_id
                    field_accessor=|pos: &Position| pos.y
                    field_mutator=|pos: &Position, new_y: f32| {
                        Position { x: pos.x, y: new_y }
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

---

## DevTools

pl3xus_client includes built-in DevTools for inspecting entities and components:

```rust
use pl3xus_client::DevTools;

#[component]
fn App() -> impl IntoView {
    let registry = ClientRegistryBuilder::new()
        .register::<Position>()
        .build();
    
    view! {
        <SyncProvider url="ws://localhost:8082" registry=registry>
            <AppView/>
            <DevTools/>  // Add DevTools
        </SyncProvider>
    }
}
```

Press the DevTools button to inspect entities, view component values, and edit fields in real-time.

---

## Next Steps

- **[Mutations Guide](../core/guides/mutations.md)** - Advanced mutation patterns
- **[DevTools Guide](../core/guides/devtools.md)** - DevTools features and customization
- **[Type Registry Guide](../core/guides/type-registry.md)** - Advanced registry patterns

---

## Complete Example

See `crates/pl3xus_client/examples/basic_client/` for a complete working example.

**Run it**:
```bash
# Terminal 1: Start server
cargo run -p pl3xus_client --example basic_server

# Terminal 2: Start client
cd crates/pl3xus_client/examples/basic_client
trunk serve --port 8080
```

---

**Last Updated**: 2025-11-22  
**pl3xus_client Version**: 0.1  
**Leptos Version**: 0.8

