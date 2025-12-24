---
title: Client Development
---
# Client Development

Build reactive web UIs that synchronize with your Bevy ECS server in real-time.

---

## Overview

pl3xus_client provides:

- **TanStack Query-inspired hooks** - `use_mutation`, `use_query`, `use_mut_component`
- **Automatic subscriptions** - Subscribe on mount, unsubscribe on unmount
- **Real-time updates** - UI reactively updates when server state changes
- **Loading & error states** - Built-in state management for async operations
- **DevTools** - Inspect entities and components in real-time

---

## Installation

```toml
[dependencies]
leptos = "0.8"
pl3xus_client = "0.1"
serde = { version = "1.0", features = ["derive"] }
shared_types = { path = "../shared" }  # Your shared types crate
```

```bash
cargo install trunk
rustup target add wasm32-unknown-unknown
```

---

## Quick Start

### 1. Set Up the Provider

```rust
use leptos::prelude::*;
use pl3xus_client::{SyncProvider, ClientTypeRegistry};
use shared::{Robot, Position};

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    // Register types the client will receive
    let registry = ClientTypeRegistry::builder()
        .register::<Robot>()
        .register::<Position>()
        .build();

    view! {
        <SyncProvider url="ws://localhost:8080" registry=registry>
            <Dashboard/>
        </SyncProvider>
    }
}
```

### 2. Display Synced Components

```rust
use pl3xus_client::use_components;

#[component]
fn Dashboard() -> impl IntoView {
    // Subscribe to all Robot components
    let robots = use_components::<Robot>();

    view! {
        <h1>"Robots"</h1>
        <For
            each=move || robots.get().into_iter()
            key=|(id, _)| *id
            children=|(id, robot)| view! {
                <RobotCard entity_id=id robot=robot/>
            }
        />
    }
}
```

### 3. Display Single Entity

```rust
use pl3xus_client::use_entity_component;

#[component]
fn RobotCard(entity_id: u64, robot: Robot) -> impl IntoView {
    // Subscribe to this entity's Position
    let position = use_entity_component::<Position>(entity_id.into());

    view! {
        <div class="robot-card">
            <h2>{robot.name}</h2>
            <Show when=move || position.get().is_some()>
                {move || {
                    let pos = position.get().unwrap();
                    format!("Position: ({:.1}, {:.1})", pos.x, pos.y)
                }}
            </Show>
        </div>
    }
}
```

---

## Mutations

Send changes to the server with loading states and error handling.

### Basic Mutation

```rust
use pl3xus_client::use_mutation;

#[component]
fn CreateButton() -> impl IntoView {
    let mutation = use_mutation::<CreateRobot>(|result| {
        match result {
            Ok(response) => log!("Created: {}", response.id),
            Err(e) => log!("Error: {}", e),
        }
    });

    view! {
        <button
            on:click=move |_| mutation.send(CreateRobot { name: "New".into() })
            disabled=move || mutation.is_loading()
        >
            {move || if mutation.is_loading() { "Creating..." } else { "Create" }}
        </button>
    }
}
```

### Targeted Mutation (with Authorization)

```rust
use pl3xus_client::use_mutation_targeted;

#[component]
fn SpeedControl(entity_id: u64) -> impl IntoView {
    let mutation = use_mutation_targeted::<SetSpeed>(|result| {
        if let Err(e) = result {
            toast.error(format!("Failed: {e}"));
        }
    });

    view! {
        <button on:click=move |_| mutation.send(entity_id, SetSpeed { value: 100.0 })>
            "Set Speed"
        </button>
    }
}
```

### Component Mutation

For mutating synced components directly:

```rust
use pl3xus_client::use_mut_component;

#[component]
fn SettingsEditor(entity_id: u64) -> impl IntoView {
    let handle = use_mut_component::<Settings>(entity_id.into());

    let on_save = move |_| {
        if let Some(current) = handle.value().get() {
            handle.mutate(Settings { speed: 100.0, ..current });
        }
    };

    view! {
        <button on:click=on_save disabled=move || handle.is_loading()>
            "Save"
        </button>
    }
}
```

---

## Queries

Fetch data with caching and loading states.

```rust
use pl3xus_client::use_query;

#[component]
fn ProgramList() -> impl IntoView {
    let query = use_query::<ListPrograms>();

    Effect::new(move || {
        query.fetch(ListPrograms {});
    });

    view! {
        <Show when=move || query.is_loading()>
            <p>"Loading..."</p>
        </Show>
        <Show when=move || query.is_success()>
            <For
                each=move || query.data().map(|d| d.programs.clone()).unwrap_or_default()
                key=|p| p.id
                children=|program| view! { <div>{program.name}</div> }
            />
        </Show>
    }
}
```

---

## DevTools

Add the DevTools component to inspect your application:

```rust
use pl3xus_client::DevTools;

view! {
    <SyncProvider url="ws://localhost:8080" registry=registry>
        <App/>
        <DevTools/>
    </SyncProvider>
}
```

Features:
- Entity browser with component inspection
- Real-time value editing
- Connection status monitoring
- Subscription tracking

---

## Hook Reference

| Hook | Purpose |
|------|---------|
| `use_components<T>()` | Subscribe to all entities with component T |
| `use_components_where<T>(predicate)` | Filtered subscription |
| `use_entity_component<T>(entity_id)` | Single entity's component |
| `use_mut_component<T>(entity_id)` | Read and mutate component |
| `use_mutation<R>(handler)` | Send mutations |
| `use_mutation_targeted<R>(handler)` | Targeted mutations |
| `use_request<R>(handler)` | One-off requests |
| `use_targeted_request<R>(handler)` | Targeted requests |
| `use_query<R>()` | Cached queries |
| `use_query_keyed<R, K>()` | Keyed queries |
| `use_query_targeted<R>()` | Targeted queries |
| `use_connection()` | Connection state |
| `use_sync_context()` | Full context access |

---

## Next Steps

- **[Hooks Reference](../core/guides/hooks.md)** - Complete hook documentation
- **[Mutations Guide](../core/guides/mutations.md)** - Advanced mutation patterns
- **[DevTools Guide](../core/guides/devtools.md)** - DevTools features

