---
title: Leptos Hooks Guide
---
# Leptos Hooks Guide

This guide covers all the reactive hooks provided by pl3xus_client for building synchronized Leptos applications.

---

## Overview

pl3xus_client provides a suite of hooks that integrate with Leptos's reactive system. These hooks:

- **Automatically subscribe** to server data when components mount
- **Reactively update** when the server sends changes
- **Clean up subscriptions** when components unmount
- **Deduplicate subscriptions** when multiple components use the same data

All hooks must be called within a `SyncProvider` context.

---

## Hook Reference

| Hook | Purpose |
|------|---------|
| `use_sync_component` | Subscribe to all entities with a component type |
| `use_sync_component_where` | Filtered subscription with a predicate |
| `use_sync_entity` | Subscribe to a single entity's component |
| `use_sync_connection` | Access connection state and controls |
| `use_sync_context` | Access the full SyncContext API |
| `use_sync_mutations` | Track mutation request states |
| `use_sync_field_editor` | Build controlled input fields |
| `use_sync_message` | Subscribe to broadcast messages |
| `use_sync_untracked` | Handle incremental/append-only data |

---

## use_sync_component

Subscribe to all entities that have a specific component type.

```rust
use pl3xus_client::use_sync_component;
use std::collections::HashMap;

#[component]
fn PositionList() -> impl IntoView {
    // Returns HashMap<entity_id, Position>
    let positions: ReadSignal<HashMap<u64, Position>> = use_sync_component::<Position>();

    view! {
        <ul>
            <For
                each=move || positions.get().into_iter()
                key=|(id, _)| *id
                children=|(id, pos)| view! {
                    <li>{format!("Entity {}: ({:.1}, {:.1})", id, pos.x, pos.y)}</li>
                }
            />
        </ul>
    }
}
```

**Key Points:**
- Returns `ReadSignal<HashMap<u64, T>>` where u64 is the entity ID
- Automatically subscribes on mount, unsubscribes on unmount
- Multiple calls with the same type share one subscription (deduplication)
- Updates trigger re-renders of dependent views

---

## use_sync_component_where

Subscribe with a filter predicate to get a subset of entities.

```rust
use pl3xus_client::use_sync_component_where;

#[component]
fn ActiveRobots() -> impl IntoView {
    // Only get robots that are currently active
    let active_robots = use_sync_component_where::<Robot, _>(|robot| robot.is_active);

    view! {
        <div>
            <h2>"Active Robots: " {move || active_robots.get().len()}</h2>
            <For
                each=move || active_robots.get().into_iter()
                key=|(id, _)| *id
                children=|(id, robot)| view! {
                    <div>{format!("{}: {}", id, robot.name)}</div>
                }
            />
        </div>
    }
}
```

**Key Points:**
- Filter runs on every update (keep predicates fast)
- Derived from `use_sync_component` internally
- Great for showing subsets without manual filtering in views

---

## use_sync_entity

Subscribe to a single entity's component by ID.

```rust
use pl3xus_client::use_sync_entity;

#[component]
fn RobotDetail(entity_id: u64) -> impl IntoView {
    // Returns Signal<Option<Robot>>
    let robot = use_sync_entity::<Robot>(entity_id);

    view! {
        <Show
            when=move || robot.get().is_some()
            fallback=|| view! { <p>"Robot not found"</p> }
        >
            {move || {
                let r = robot.get().unwrap();
                view! {
                    <div class="robot-detail">
                        <h2>{r.name.clone()}</h2>
                        <p>"Status: " {format!("{:?}", r.status)}</p>
                        <p>"Battery: " {r.battery_level} "%"</p>
                    </div>
                }
            }}
        </Show>
    }
}
```

**Key Points:**
- Returns `Signal<Option<T>>` - None if entity doesn't exist or lacks the component
- More efficient than filtering the full map in your view
- Entity ID is typically received from parent component or route params

---

## use_sync_connection

Access WebSocket connection state and controls.

```rust
use pl3xus_client::use_sync_connection;
use leptos_use::core::ConnectionReadyState;

#[component]
fn ConnectionStatus() -> impl IntoView {
    let connection = use_sync_connection();

    let status_text = move || match connection.ready_state.get() {
        ConnectionReadyState::Connecting => "Connecting...",
        ConnectionReadyState::Open => "Connected",
        ConnectionReadyState::Closing => "Disconnecting...",
        ConnectionReadyState::Closed => "Disconnected",
    };

    let status_class = move || match connection.ready_state.get() {
        ConnectionReadyState::Open => "status-connected",
        _ => "status-disconnected",
    };

    view! {
        <div class=status_class>
            <span>{status_text}</span>
            <Show when=move || connection.ready_state.get() == ConnectionReadyState::Closed>
                <button on:click=move |_| (connection.open)()>"Reconnect"</button>
            </Show>
        </div>
    }
}
```

**Returns `SyncConnection` with:**
- `ready_state: Signal<ConnectionReadyState>` - Current connection state
- `open: Arc<dyn Fn()>` - Function to open the connection
- `close: Arc<dyn Fn()>` - Function to close the connection

---

## use_sync_context

Access the full SyncContext for advanced operations like mutations.

```rust
use pl3xus_client::use_sync_context;

#[component]
fn PositionUpdater(entity_id: u64) -> impl IntoView {
    let ctx = use_sync_context();

    let update_position = move |_| {
        let new_pos = Position { x: 100.0, y: 200.0 };
        let request_id = ctx.mutate(entity_id, new_pos);
        leptos::logging::log!("Mutation sent: {}", request_id);
    };

    view! {
        <button on:click=update_position>"Move to (100, 200)"</button>
    }
}
```

**Key Methods:**
- `mutate(entity_id, component)` - Send a mutation request, returns request ID
- `connection()` - Get the SyncConnection interface
- `ready_state` - Current connection state signal

---

## use_sync_mutations

Track the status of mutation requests.

```rust
use pl3xus_client::{use_sync_context, use_sync_mutations, MutationState};
use pl3xus_sync::MutationStatus;

#[component]
fn MutateWithFeedback(entity_id: u64) -> impl IntoView {
    let ctx = use_sync_context();
    let mutations = use_sync_mutations();
    let (last_request, set_last_request) = signal(None::<u64>);

    let update = move |_| {
        let request_id = ctx.mutate(entity_id, Position { x: 50.0, y: 50.0 });
        set_last_request.set(Some(request_id));
    };

    let status_text = move || {
        last_request.get().and_then(|id| {
            mutations.get().get(&id).map(|state| {
                match &state.status {
                    Some(MutationStatus::Ok) => "✓ Saved".to_string(),
                    Some(MutationStatus::Forbidden) => "✗ Not authorized".to_string(),
                    Some(status) => format!("Error: {:?}", status),
                    None => "⏳ Pending...".to_string(),
                }
            })
        }).unwrap_or_default()
    };

    view! {
        <button on:click=update>"Update Position"</button>
        <span class="status">{status_text}</span>
    }
}
```

**Returns `ReadSignal<HashMap<u64, MutationState>>`:**
- Key is the request_id from `ctx.mutate()`
- `MutationState.status` is `None` while pending, `Some(MutationStatus)` when server responds

---

## use_sync_field_editor

Build controlled input fields that integrate with server state.

```rust
use pl3xus_client::use_sync_field_editor;

#[component]
fn PositionXEditor(entity_id: u64) -> impl IntoView {
    let (input_ref, is_focused, initial_value, on_keydown, on_blur) =
        use_sync_field_editor::<Position, f32, _, _>(
            entity_id,
            |pos| pos.x,                              // Field accessor
            |pos, new_x| Position { x: new_x, y: pos.y }  // Field mutator
        );

    view! {
        <input
            node_ref=input_ref
            type="number"
            value=initial_value
            on:focus=move |_| is_focused.set(true)
            on:blur=move |_| {
                is_focused.set(false);
                on_blur();
            }
            on:keydown=on_keydown
        />
    }
}
```

**Behavior:**
- Input retains focus when server updates arrive
- **Enter** sends mutation to server
- **Blur** reverts to server value (discards local changes)
- Prevents update loops with controlled input pattern

**Returns tuple:**
1. `NodeRef<Input>` - Reference to bind to input element
2. `RwSignal<bool>` - Focus state tracker
3. `String` - Initial value for the input
4. `Fn(KeyboardEvent)` - Keydown handler (Enter to submit)
5. `Fn()` - Blur handler (revert on blur)

---

## use_sync_message

Subscribe to broadcast messages (not component sync).

```rust
use pl3xus_client::use_sync_message;

#[derive(Clone, Default, Serialize, Deserialize)]
struct ServerNotification {
    message: String,
    level: String,  // "info", "warning", "error"
}

#[component]
fn NotificationBanner() -> impl IntoView {
    let notification = use_sync_message::<ServerNotification>();

    view! {
        <Show when=move || !notification.get().message.is_empty()>
            <div class=move || format!("notification {}", notification.get().level)>
                {move || notification.get().message}
            </div>
        </Show>
    }
}
```

**Key Points:**
- For one-way server → client broadcasts (not entity components)
- Use cases: notifications, events, video frames, real-time data
- Message type must implement `SyncComponent` trait (auto-derived for Serialize + Deserialize)

---

## use_sync_untracked

Handle incremental/append-only data like logs or event streams.

```rust
use pl3xus_client::use_sync_untracked;

#[derive(Clone, Default, Serialize, Deserialize)]
struct LogBuffer {
    entries: Vec<LogEntry>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct LogEntry {
    timestamp: u64,
    level: String,
    message: String,
}

#[component]
fn LogViewer() -> impl IntoView {
    // Append new entries to the buffer
    let (full_logs, latest_entry) = use_sync_untracked::<LogBuffer, LogEntry>(
        |buffer, entry| buffer.entries.push(entry)
    );

    view! {
        <div class="log-viewer">
            <For
                each=move || full_logs.get().entries.clone()
                key=|entry| entry.timestamp
                children=|entry| view! {
                    <div class=format!("log-{}", entry.level)>
                        {format!("[{}] {}", entry.timestamp, entry.message)}
                    </div>
                }
            />
        </div>
    }
}
```

**Parameters:**
- `TFull` - The accumulated state type (e.g., LogBuffer with all entries)
- `TIncremental` - The incremental update type (e.g., single LogEntry)
- `append_fn` - Function to merge incremental updates into full state

**Returns:**
- `Signal<TFull>` - The accumulated state
- `Signal<Option<TIncremental>>` - The most recent incremental update

---

## Best Practices

### 1. Keep Hook Calls at Component Top Level

```rust
// ✅ Good - hooks at top level
#[component]
fn MyComponent() -> impl IntoView {
    let positions = use_sync_component::<Position>();
    let robots = use_sync_component::<Robot>();

    view! { /* ... */ }
}

// ❌ Bad - hooks inside conditionals
#[component]
fn MyComponent(show_positions: bool) -> impl IntoView {
    let robots = use_sync_component::<Robot>();

    view! {
        {move || if show_positions {
            let positions = use_sync_component::<Position>();  // Don't do this!
            // ...
        }}
    }
}
```

### 2. Use Specific Hooks for Specific Needs

```rust
// ❌ Inefficient - filtering full map in view
let all = use_sync_component::<Robot>();
let active = move || all.get().into_iter().filter(|(_, r)| r.active).collect();

// ✅ Better - use the filtering hook
let active = use_sync_component_where::<Robot, _>(|r| r.active);
```

### 3. Prefer use_sync_entity for Single Entities

```rust
// ❌ Less efficient
let all = use_sync_component::<Robot>();
let my_robot = move || all.get().get(&entity_id).cloned();

// ✅ More efficient
let my_robot = use_sync_entity::<Robot>(entity_id);
```

### 4. Use SyncFieldInput Component When Possible

For simple field editing, the `SyncFieldInput` component is often easier than `use_sync_field_editor`:

```rust
use pl3xus_client::SyncFieldInput;

view! {
    <SyncFieldInput<Position, f32>
        entity_id=entity_id
        field_accessor=|pos| pos.x
        field_mutator=|pos, x| Position { x, y: pos.y }
        input_type="number"
    />
}
```

---

## Related Documentation

- [Mutations](./mutations.md) - Server-side mutation authorization
- [Subscriptions](./subscriptions.md) - How subscriptions work
- [Type Registry](./type-registry.md) - Client type registration
- [API Reference](https://docs.rs/pl3xus_client) - Full API documentation

---

**Last Updated**: 2025-12-07
**pl3xus_client Version**: 0.1


