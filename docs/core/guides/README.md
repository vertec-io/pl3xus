# Guides

In-depth tutorials and patterns for using pl3xus effectively.

---

## Core Concepts

| Guide | Description |
|-------|-------------|
| [Hooks Reference](./hooks.md) | Complete client hook documentation |
| [Mutations](./mutations.md) | TanStack Query-inspired mutations |
| [Requests & Queries](./requests.md) | Request/response and query patterns |
| [Authorization](./authorization.md) | Control who can do what |
| [Entity Control](./entity-control.md) | Exclusive control patterns |

## Setup & Configuration

| Guide | Description |
|-------|-------------|
| [Server Setup](./server-setup.md) | Setting up a Bevy server |
| [Type Registry](./type-registry.md) | Client-side type registration |
| [Shared Types](./shared-types.md) | Sharing types between server and client |
| [Connection Management](./connection-management.md) | Connection lifecycle |

## Features

| Guide | Description |
|-------|-------------|
| [DevTools](./devtools.md) | Built-in debugging tools |
| [Subscriptions](./subscriptions.md) | How component sync works |
| [Sending Messages](./sending-messages.md) | Direct message patterns |
| [WebSocket Patterns](./websocket-patterns.md) | Architecture patterns |

---

## Quick Example

### Server

```rust
use pl3xus_sync::{Pl3xusSyncPlugin, AppPl3xusSyncExt};

app.add_plugins(Pl3xusSyncPlugin::default())
   .sync_component::<Position>(None)
   .request::<UpdatePosition, NP>()
       .targeted()
       .with_default_entity_policy()
       .register();
```

### Client

```rust
use pl3xus_client::{use_components, use_mutation_targeted};

#[component]
fn App() -> impl IntoView {
    let positions = use_components::<Position>();
    let mutation = use_mutation_targeted::<UpdatePosition>(|_| {});

    view! {
        <For each=move || positions.get().into_iter() key=|(id, _)| *id>
            {|(id, pos)| view! {
                <div>
                    {format!("({:.1}, {:.1})", pos.x, pos.y)}
                    <button on:click=move |_| mutation.send(id, UpdatePosition { x: 0.0, y: 0.0 })>
                        "Reset"
                    </button>
                </div>
            }}
        </For>
    }
}
```

---

## Related

- [Quick Start](../getting-started.md) - Get running in 5 minutes
- [Server Development](../../sync/index.md) - Server-side APIs
- [Client Development](../../client/index.md) - Client-side APIs

