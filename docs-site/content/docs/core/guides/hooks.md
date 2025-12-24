---
title: Client Hooks Reference
---
# Client Hooks Reference

Complete reference for all pl3xus_client hooks. These hooks integrate with Leptos's reactive system for automatic subscription management and real-time updates.

---

## Quick Reference

| Hook | Purpose |
|------|---------|
| **Component Sync** | |
| `use_components` | Subscribe to all entities with a component type |
| `use_components_where` | Filtered subscription with a predicate |
| `use_entity_component` | Subscribe to a single entity's component |
| `use_mut_component` | Read and mutate a component |
| **Requests & Mutations** | |
| `use_mutation` | Send mutations with loading/error states |
| `use_mutation_targeted` | Send targeted mutations (with authorization) |
| `use_request` | Send one-off requests |
| `use_targeted_request` | Send targeted requests |
| `use_query` | Cached queries with auto-refresh |
| `use_query_keyed` | Keyed queries (like TanStack Query) |
| `use_query_targeted` | Targeted queries |
| **Context & Connection** | |
| `use_sync_context` | Access the full SyncContext API |
| `use_connection` | Access connection state and controls |
| `use_entity` | Get entity metadata |
| `use_entity_reactive` | Reactive entity signal |

---

## Component Sync Hooks

### use_components

Subscribe to all entities that have a specific component type.

```rust
use pl3xus_client::use_components;

#[component]
fn RobotList() -> impl IntoView {
    // Returns HashMap<entity_id, Robot>
    let robots = use_components::<Robot>();

    view! {
        <For
            each=move || robots.get().into_iter()
            key=|(id, _)| *id
            children=|(id, robot)| view! {
                <div>{format!("Robot {}: {}", id, robot.name)}</div>
            }
        />
    }
}
```

**Returns:** `ReadSignal<HashMap<u64, T>>`

**Key Points:**
- Automatically subscribes on mount, unsubscribes on unmount
- Multiple calls with the same type share one subscription (deduplication)
- Updates trigger re-renders of dependent views

---

### use_components_where

Subscribe with a filter predicate.

```rust
use pl3xus_client::use_components_where;

#[component]
fn ActiveRobots() -> impl IntoView {
    let active = use_components_where::<Robot, _>(|robot| robot.is_active);

    view! {
        <h2>"Active: " {move || active.get().len()}</h2>
    }
}
```

---

### use_entity_component

Subscribe to a single entity's component by ID.

```rust
use pl3xus_client::use_entity_component;

#[component]
fn RobotDetail(entity_id: Signal<u64>) -> impl IntoView {
    // Returns Signal<Option<Robot>>
    let robot = use_entity_component::<Robot>(entity_id);

    view! {
        <Show when=move || robot.get().is_some()>
            {move || robot.get().unwrap().name.clone()}
        </Show>
    }
}
```

**Returns:** `Signal<Option<T>>`

**Note:** The entity_id parameter accepts `Into<Signal<u64>>`, so you can pass:
- A raw `u64` value
- A `Signal<u64>` for reactive entity selection
- A `Memo<u64>` or other signal types

---

### use_mut_component

Read and mutate a component with authorization.

```rust
use pl3xus_client::use_mut_component;

#[component]
fn SettingsEditor(entity_id: u64) -> impl IntoView {
    let handle = use_mut_component::<JogSettings>(entity_id.into());

    let on_save = move |_| {
        if let Some(current) = handle.value().get() {
            handle.mutate(JogSettings {
                speed: 100.0,
                ..current
            });
        }
    };

    view! {
        <button on:click=on_save disabled=move || handle.is_loading()>
            {move || if handle.is_loading() { "Saving..." } else { "Save" }}
        </button>
    }
}
```

**Returns:** `MutComponentHandle<T>` with:
- `value()` → `Signal<Option<T>>` - Current component value
- `mutate(new_value)` - Send mutation to server
- `is_loading()` → `bool` - True while mutation is pending
- `state()` → `ComponentMutationState` - Idle, Loading, Success, or Error

---

## Mutation Hooks

### use_mutation

Send mutations with TanStack Query-style loading states.

```rust
use pl3xus_client::use_mutation;

#[component]
fn CreateRobotButton() -> impl IntoView {
    let mutation = use_mutation::<CreateRobot>(|result| {
        match result {
            Ok(response) => log!("Created: {}", response.robot_id),
            Err(e) => log!("Error: {}", e),
        }
    });

    view! {
        <button
            on:click=move |_| mutation.send(CreateRobot { name: "New Robot".into() })
            disabled=move || mutation.is_loading()
        >
            {move || if mutation.is_loading() { "Creating..." } else { "Create Robot" }}
        </button>
    }
}
```

**Returns:** `MutationHandle<R>` with:
- `send(request)` - Send the mutation
- `is_loading()` → `bool`
- `is_success()` → `bool`
- `is_error()` → `bool`
- `data()` → `Option<&Response>`
- `error()` → `Option<&str>`
- `reset()` - Reset to idle state

---

### use_mutation_targeted

Send mutations to a specific entity (with authorization).

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

**Returns:** `TargetedMutationHandle<R>` with:
- `send(entity_id, request)` - Send to specific entity
- Same state methods as `MutationHandle`

---

## Request Hooks

### use_request

Send one-off requests (non-cached).

```rust
use pl3xus_client::use_request;

#[component]
fn DataLoader() -> impl IntoView {
    let (data, set_data) = signal(None);

    let request = use_request::<GetData>(move |result| {
        if let Ok(response) = result {
            set_data.set(Some(response));
        }
    });

    Effect::new(move || {
        request.send(GetData { id: 123 });
    });

    view! { /* ... */ }
}
```

---

### use_targeted_request

Send requests to a specific entity.

```rust
use pl3xus_client::use_targeted_request;

#[component]
fn FrameDataLoader(entity_id: u64) -> impl IntoView {
    let (frame_data, set_frame_data) = signal(None);

    let request = use_targeted_request::<GetFrameData>(move |result| {
        if let Ok(data) = result {
            set_frame_data.set(Some(data));
        }
    });

    Effect::new(move || {
        request.send(entity_id, GetFrameData { frame_number: 1 });
    });

    view! { /* ... */ }
}
```

---

## Query Hooks

### use_query

Cached queries with automatic refresh (TanStack Query-inspired).

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

### use_query_keyed

Keyed queries for parameterized data.

```rust
use pl3xus_client::use_query_keyed;

#[component]
fn ProgramDetail(program_id: Signal<u64>) -> impl IntoView {
    let query = use_query_keyed::<GetProgram, u64>();

    Effect::new(move || {
        let id = program_id.get();
        query.fetch(id, GetProgram { id });
    });

    view! {
        <Show when=move || query.is_success()>
            {move || query.data().map(|p| p.name.clone()).unwrap_or_default()}
        </Show>
    }
}
```

---

### use_query_targeted

Targeted queries for entity-specific data.

```rust
use pl3xus_client::use_query_targeted;

#[component]
fn RobotStatus(entity_id: u64) -> impl IntoView {
    let query = use_query_targeted::<GetConnectionStatus>();

    Effect::new(move || {
        query.fetch(entity_id, GetConnectionStatus {});
    });

    view! {
        <Show when=move || query.is_success()>
            {move || format!("{:?}", query.data())}
        </Show>
    }
}
```

---

## Context & Connection Hooks

### use_sync_context

Access the full SyncContext for advanced operations.

```rust
use pl3xus_client::use_sync_context;

#[component]
fn AdvancedComponent() -> impl IntoView {
    let ctx = use_sync_context();

    // Access connection state
    let is_connected = move || ctx.ready_state.get() == ConnectionReadyState::Open;

    // Send raw messages
    let send_custom = move |_| {
        ctx.send_message(MyCustomMessage { data: "hello".into() });
    };

    view! { /* ... */ }
}
```

---

### use_connection

Access WebSocket connection state and controls.

```rust
use pl3xus_client::use_connection;

#[component]
fn ConnectionStatus() -> impl IntoView {
    let connection = use_connection();

    view! {
        <div class=move || if connection.is_connected() { "connected" } else { "disconnected" }>
            {move || match connection.ready_state.get() {
                ConnectionReadyState::Open => "Connected",
                ConnectionReadyState::Connecting => "Connecting...",
                _ => "Disconnected",
            }}
        </div>
    }
}
```

---

## Best Practices

### 1. Use Targeted Hooks for Entity Operations

```rust
// ❌ Don't pass entity_id in request body
let mutation = use_mutation::<UpdateRobot>(|_| {});
mutation.send(UpdateRobot { entity_id: 123, speed: 100.0 });

// ✅ Use targeted mutation
let mutation = use_mutation_targeted::<UpdateRobotSpeed>(|_| {});
mutation.send(123, UpdateRobotSpeed { speed: 100.0 });
```

### 2. Use use_entity_component for Single Entities

```rust
// ❌ Less efficient
let all = use_components::<Robot>();
let my_robot = move || all.get().get(&entity_id).cloned();

// ✅ More efficient
let my_robot = use_entity_component::<Robot>(entity_id.into());
```

### 3. Handle Loading and Error States

```rust
let mutation = use_mutation::<SaveData>(|_| {});

view! {
    <button disabled=move || mutation.is_loading()>
        {move || match () {
            _ if mutation.is_loading() => "Saving...",
            _ if mutation.is_error() => "Failed ✗",
            _ if mutation.is_success() => "Saved ✓",
            _ => "Save",
        }}
    </button>
}
```

### 4. Keep Hooks at Component Top Level

```rust
// ✅ Good
#[component]
fn MyComponent() -> impl IntoView {
    let robots = use_components::<Robot>();
    let mutation = use_mutation::<UpdateRobot>(|_| {});
    view! { /* ... */ }
}

// ❌ Bad - hooks inside conditionals
#[component]
fn MyComponent(show: bool) -> impl IntoView {
    view! {
        {move || if show {
            let robots = use_components::<Robot>();  // Don't do this!
            // ...
        }}
    }
}
```

---

## Related

- [Mutations](./mutations.md) - Mutation patterns and authorization
- [Requests](./requests.md) - Request/response patterns
- [Component Sync](../sync/index.md) - Server-side component registration


