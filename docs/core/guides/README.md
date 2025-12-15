# Guides

In-depth tutorials and patterns for using pl3xus effectively.

---

## Contents

### Core Concepts

| Guide | Description |
|-------|-------------|
| [Server Setup](./server-setup.md) | Setting up a Bevy server with pl3xus_sync |
| [Type Registry](./type-registry.md) | ClientTypeRegistry for client-side deserialization |
| [Subscriptions](./subscriptions.md) | How component subscriptions work |
| [Hooks](./hooks.md) | All Leptos hooks for reactive sync |
| [Connection Management](./connection-management.md) | Connection lifecycle and reconnection |

### Features

| Guide | Description |
|-------|-------------|
| [Sending Messages](./sending-messages.md) | Direct vs scheduled message sending |
| [Mutations](./mutations.md) | Client-driven component mutations |
| [DevTools](./devtools.md) | Using the built-in DevTools |
| [Shared Types](./shared-types.md) | Sharing types between server and client |
| [WebSocket Patterns](./websocket-patterns.md) | Common WebSocket architectures |

---

## Quick Start

### 1. Server Setup

```rust
use pl3xus_sync::{Pl3xusSyncPlugin, AppPl3xusSyncExt};

app.add_plugins(Pl3xusSyncPlugin::<WebSocketProvider>::default())
   .sync_component::<Position>(None)
   .sync_component::<Velocity>(None);
```

See [Server Setup](./server-setup.md) for the complete guide.

### 2. Client Setup

```rust
use pl3xus_client::{SyncProvider, ClientTypeRegistry};

let registry = ClientTypeRegistry::builder()
    .register::<Position>()
    .register::<Velocity>()
    .build();

view! {
    <SyncProvider url="ws://localhost:8080" registry=registry>
        <MyApp/>
    </SyncProvider>
}
```

See [Type Registry](./type-registry.md) for details.

### 3. Subscribe to Data

```rust
use pl3xus_client::use_sync_component;

#[component]
fn PositionList() -> impl IntoView {
    let positions = use_sync_component::<Position>();

    view! {
        <For each=move || positions.get().into_iter() key=|(id, _)| *id>
            {|(id, pos)| view! { <div>{format!("{}: ({}, {})", id, pos.x, pos.y)}</div> }}
        </For>
    }
}
```

See [Hooks](./hooks.md) for all available hooks.

---

## Best Practices

### 1. Use Shared Types

Create a shared crate for types used by both server and client:

```
my-app/
├── shared/           # Shared types crate
│   └── src/lib.rs    # Position, Velocity, etc.
├── server/           # Bevy server
│   └── Cargo.toml    # depends on shared
└── client/           # Leptos client
    └── Cargo.toml    # depends on shared
```

See [Shared Types](./shared-types.md) for the complete pattern.

### 2. Register All Types

Both server and client must register the same component types:

```rust
// Server
app.sync_component::<Position>(None);
app.sync_component::<Velocity>(None);

// Client
let registry = ClientTypeRegistry::builder()
    .register::<Position>()
    .register::<Velocity>()
    .build();
```

### 3. Handle Connection State

Show users the connection status:

```rust
let connection = use_sync_connection();

view! {
    <Show when=move || connection.ready_state.get() != ConnectionReadyState::Open>
        <div class="reconnecting">"Reconnecting..."</div>
    </Show>
}
```

See [Connection Management](./connection-management.md) for reconnection patterns.

---

## Related Documentation

- [Getting Started](../getting-started/) - Quick start guides
- [Architecture](../architecture/) - System design
- [API Reference](../api/) - Detailed API docs

