---
name: pl3xus-client
description: Client-side Leptos patterns for pl3xus applications. Covers reactive hooks, component patterns, context usage, and UI best practices. Use when implementing client-side UI.
allowed-tools:
  - view
  - codebase-retrieval
  - save-file
  - str-replace-editor
  - launch-process
  - read-process
---

# pl3xus Client Skill

## Purpose

This skill covers client-side implementation patterns for pl3xus applications using Leptos. The client reflects server state reactively and sends requests/mutations to the server.

## When to Use

Use this skill when:
- Setting up a new pl3xus client
- Implementing reactive UI components
- Using hooks for component sync
- Handling user interactions

## Client Setup

### Basic Client Structure

```rust
// src/main.rs
use leptos::prelude::*;
use pl3xus_client::SyncProvider;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    view! {
        <SyncProvider url="ws://localhost:8080/ws">
            <MainContent />
        </SyncProvider>
    }
}
```

### Type Registry

Register shared types for deserialization:

```rust
use pl3xus_client::ClientTypeRegistry;

fn create_registry() -> ClientTypeRegistry {
    let mut registry = ClientTypeRegistry::new();
    registry.register::<Position>();
    registry.register::<Velocity>();
    registry.register::<RobotState>();
    registry
}

// In SyncProvider
<SyncProvider url="ws://localhost:8080/ws" registry=create_registry()>
```

## Hooks Reference

### Component Hooks

**`use_entity_component<T>(entity_id)` - Preferred for multi-entity**

```rust
#[component]
fn RobotPosition(robot_id: u64) -> impl IntoView {
    let position = use_entity_component::<Position>(robot_id);
    
    view! {
        <Show when=move || position.get().is_some()>
            {move || {
                let pos = position.get().unwrap();
                format!("({:.2}, {:.2}, {:.2})", pos.x, pos.y, pos.z)
            }}
        </Show>
    }
}
```

**`use_components<T>()` - For listing all entities**

```rust
#[component]
fn RobotList() -> impl IntoView {
    let robots = use_components::<RobotInfo>();
    
    view! {
        <For
            each=move || robots.get().into_iter()
            key=|(id, _)| *id
            children=|(id, info)| view! {
                <RobotCard id=id info=info />
            }
        />
    }
}
```

### Request Hooks

**`use_request<R>()` - Basic request**

```rust
let (fetch, state) = use_request::<ListRobots>();

Effect::new(move |_| {
    fetch(ListRobots);
});

view! {
    <Show when=move || state.get().is_loading()>
        <Spinner />
    </Show>
}
```

**`use_request_with_handler<R, F>()` - With callback**

```rust
let load = use_request_with_handler::<LoadProgram, _>(move |result| {
    match result {
        Ok(r) if r.success => toast.success("Loaded"),
        Ok(r) => toast.error(r.error.unwrap_or_default()),
        Err(e) => toast.error(e),
    }
});

load(LoadProgram { id: 42 });
```

### Mutation Hooks

**`use_mutation_targeted<R>(handler)` - Targeted mutation**

```rust
let update = use_mutation_targeted::<UpdatePosition>(move |result| {
    match result {
        Ok(r) if r.success => log::info!("Updated"),
        Ok(r) => log::error!("Failed: {:?}", r.error),
        Err(e) => log::error!("Error: {e}"),
    }
});

// Send mutation to specific entity
update.send(robot_id, UpdatePosition { x: 1.0, y: 2.0, z: 3.0 });
```

## Component Patterns

### Entity Context Pattern

Provide entity ID via context for child components:

```rust
#[component]
fn RobotPanel(robot_id: u64) -> impl IntoView {
    provide_context(RobotContext { id: robot_id });
    
    view! {
        <RobotHeader />
        <RobotControls />
        <RobotStatus />
    }
}

#[component]
fn RobotControls() -> impl IntoView {
    let ctx = expect_context::<RobotContext>();
    let state = use_entity_component::<RobotState>(ctx.id);
    
    // Use state.can_* flags from server
    view! {
        <button
            disabled=move || !state.get().map(|s| s.can_start).unwrap_or(false)
            on:click=move |_| start_robot(ctx.id)
        >
            "Start"
        </button>
    }
}
```

### Server-Driven UI State

Never compute UI state client-side. Use server-provided flags:

```rust
// ❌ Wrong - client-side logic
let can_start = move || state.get().map(|s| s.state == Idle).unwrap_or(false);

// ✅ Correct - server-driven
let can_start = move || state.get().map(|s| s.can_start).unwrap_or(false);
```

## Input Patterns

### Text Input with Validation (Not Number Input)

```rust
#[component]
fn NumericInput(value: RwSignal<f64>) -> impl IntoView {
    let text = RwSignal::new(value.get().to_string());
    
    view! {
        <input
            type="text"
            prop:value=move || text.get()
            on:input=move |ev| {
                let val = event_target_value(&ev);
                text.set(val.clone());
                if let Ok(num) = val.parse::<f64>() {
                    value.set(num);
                }
            }
        />
    }
}
```

## Anti-Patterns to Avoid

| Anti-Pattern | Problem | Solution |
|--------------|---------|----------|
| `use_components().values().next()` | No entity guarantee | `use_entity_component(id)` |
| `input type="number"` | Hard to use with decimals | Text input + validation |
| Client-side state logic | Server is authoritative | Use server `can_*` flags |
| Hardcoded entity IDs | Breaks multi-entity | Use context or props |

## Related Skills

- **pl3xus-queries**: Request/response patterns
- **pl3xus-mutations**: Mutation patterns
- **leptos-ui**: Leptos fundamentals

## Reference

- [Hooks Reference](./references/hooks-reference.md)
- [Context Patterns](./references/context-patterns.md)
- [Component Patterns](./references/component-patterns.md)

