# Quick Start

Get a real-time synchronized application running in 5 minutes.

## Prerequisites

- Rust (stable or nightly)
- Bevy 0.17+
- Leptos 0.8+
- Trunk (`cargo install trunk`)
- wasm32 target (`rustup target add wasm32-unknown-unknown`)

---

## 1. Create the Shared Types

First, create a shared crate for types used by both server and client:

```rust
// shared/src/lib.rs
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Component, Clone, Debug, Serialize, Deserialize)]
pub struct Counter {
    pub value: i32,
}
```

---

## 2. Create the Server

```rust
// server/src/main.rs
use bevy::prelude::*;
use pl3xus::prelude::*;
use pl3xus_sync::{AppPl3xusSyncExt, Pl3xusSyncPlugin};
use shared::Counter;

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins(Pl3xusSyncPlugin::default())
        // Register Counter for automatic synchronization
        .sync_component::<Counter>(None)
        .add_systems(Startup, setup)
        .add_systems(Update, increment_counter)
        .run();
}

fn setup(mut commands: Commands) {
    // Spawn an entity with a Counter component
    commands.spawn(Counter { value: 0 });
}

fn increment_counter(mut query: Query<&mut Counter>, time: Res<Time>) {
    for mut counter in &mut query {
        // Increment every second
        if time.elapsed_secs() as i32 > counter.value {
            counter.value += 1;
        }
    }
}
```

That's it for the server! The `sync_component::<Counter>(None)` call registers the component for automatic synchronization. Any changes are automatically detected and sent to subscribed clients.

---

## 3. Create the Client

```rust
// client/src/main.rs
use leptos::prelude::*;
use pl3xus_client::{SyncProvider, ClientTypeRegistry, use_components};
use shared::Counter;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    // Register types the client will receive
    let registry = ClientTypeRegistry::builder()
        .register::<Counter>()
        .build();

    view! {
        <SyncProvider url="ws://localhost:8080" registry=registry>
            <CounterDisplay/>
        </SyncProvider>
    }
}

#[component]
fn CounterDisplay() -> impl IntoView {
    // Subscribe to all Counter components - automatically updates!
    let counters = use_components::<Counter>();

    view! {
        <h1>"Real-time Counters"</h1>
        <For
            each=move || counters.get().into_iter()
            key=|(entity_id, _)| *entity_id
            children=|(entity_id, counter)| {
                view! {
                    <div>
                        "Entity " {entity_id} ": " {counter.value}
                    </div>
                }
            }
        />
    }
}
```

---

## 4. Run It

Terminal 1 - Start the server:
```bash
cd server && cargo run
```

Terminal 2 - Start the client:
```bash
cd client && trunk serve
```

Open `http://localhost:8080` and watch the counter update in real-time!

---

## What Just Happened?

1. **Server** spawned an entity with a `Counter` component
2. **Server** registered `Counter` for sync with `sync_component::<Counter>(None)`
3. **Client** connected via WebSocket and subscribed to `Counter`
4. **Server** detected changes to `Counter` and sent updates
5. **Client** received updates and reactively updated the UI

**Zero boilerplate. Zero manual serialization. Zero event handlers.**

---

## Next Steps

Now that you have the basics working:

| Topic | Description |
|-------|-------------|
| [Mutations](./guides/mutations.md) | Let clients modify server state |
| [Requests & Queries](./guides/requests.md) | Request/response patterns |
| [Authorization](./guides/authorization.md) | Control who can do what |
| [Entity Control](./guides/entity-control.md) | Exclusive control patterns |
| [DevTools](./guides/devtools.md) | Debug your application |

---

## Learning Path

For a deeper understanding, follow this path:

1. **[What is pl3xus?](./what-is-pl3xus.md)** - Philosophy and architecture
2. **[Core Concepts](./guides/index.md)** - Fundamental patterns
3. **[Server Development](../sync/index.md)** - Server-side APIs
4. **[Client Development](../client/index.md)** - Client-side hooks
5. **[Examples](./examples/index.md)** - Real-world applications

