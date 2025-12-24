---
name: leptos-ui
description: Leptos UI patterns for industrial interfaces. Covers reactive signals, components, contexts, and WASM patterns. Use when building client-side UI.
allowed-tools:
  - view
  - codebase-retrieval
  - web-search
  - web-fetch
---

# Leptos UI Skill

## Purpose

This skill covers Leptos UI patterns for building industrial application interfaces. Leptos provides fine-grained reactivity for efficient WASM applications.

## When to Use

Use this skill when:
- Building client-side UI components
- Understanding Leptos reactivity
- Implementing forms and inputs
- Creating reusable components

## Leptos Overview

### Reactive Signals

```rust
use leptos::prelude::*;

#[component]
fn Counter() -> impl IntoView {
    let count = RwSignal::new(0);
    
    view! {
        <button on:click=move |_| count.update(|c| *c += 1)>
            "Count: " {move || count.get()}
        </button>
    }
}
```

### Signal Types

```rust
// Read-write signal
let value = RwSignal::new(0);
value.set(5);
let current = value.get();

// Read-only signal (derived)
let doubled = Signal::derive(move || value.get() * 2);

// Memo (cached computation)
let expensive = Memo::new(move |_| {
    // Only recomputes when dependencies change
    compute_expensive(value.get())
});
```

## Component Patterns

### Basic Component

```rust
#[component]
fn RobotCard(
    #[prop(into)] name: String,
    #[prop(into)] status: Signal<RobotStatus>,
) -> impl IntoView {
    view! {
        <div class="robot-card">
            <h3>{name}</h3>
            <p>"Status: " {move || format!("{:?}", status.get())}</p>
        </div>
    }
}
```

### Component with Children

```rust
#[component]
fn Panel(
    #[prop(into)] title: String,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="panel">
            <h2>{title}</h2>
            <div class="panel-content">
                {children()}
            </div>
        </div>
    }
}

// Usage
view! {
    <Panel title="Robot Controls">
        <button>"Start"</button>
        <button>"Stop"</button>
    </Panel>
}
```

## Context Pattern

### Providing Context

```rust
#[derive(Clone)]
pub struct RobotContext {
    pub id: u64,
    pub name: RwSignal<String>,
}

#[component]
fn RobotProvider(robot_id: u64, children: Children) -> impl IntoView {
    provide_context(RobotContext {
        id: robot_id,
        name: RwSignal::new(String::new()),
    });
    
    children()
}
```

### Consuming Context

```rust
#[component]
fn RobotControls() -> impl IntoView {
    let ctx = expect_context::<RobotContext>();
    
    view! {
        <p>"Controlling robot: " {ctx.id}</p>
    }
}
```

## Control Flow

### Show/Hide

```rust
view! {
    <Show
        when=move || is_loading.get()
        fallback=|| view! { <Content /> }
    >
        <Spinner />
    </Show>
}
```

### For Loop

```rust
view! {
    <For
        each=move || items.get()
        key=|item| item.id
        children=|item| view! {
            <ItemCard item=item />
        }
    />
}
```

### Match/Switch

```rust
view! {
    {move || match status.get() {
        Status::Loading => view! { <Spinner /> }.into_any(),
        Status::Error(e) => view! { <Error message=e /> }.into_any(),
        Status::Ready(data) => view! { <Content data=data /> }.into_any(),
    }}
}
```

## Form Patterns

### Text Input (Preferred over Number)

```rust
#[component]
fn NumericInput(
    value: RwSignal<f64>,
    #[prop(optional)] label: Option<String>,
) -> impl IntoView {
    let text = RwSignal::new(value.get().to_string());
    let is_valid = RwSignal::new(true);
    
    view! {
        <div class="input-group">
            {label.map(|l| view! { <label>{l}</label> })}
            <input
                type="text"
                class:invalid=move || !is_valid.get()
                prop:value=move || text.get()
                on:input=move |ev| {
                    let val = event_target_value(&ev);
                    text.set(val.clone());
                    match val.parse::<f64>() {
                        Ok(num) => {
                            value.set(num);
                            is_valid.set(true);
                        }
                        Err(_) => is_valid.set(false),
                    }
                }
            />
        </div>
    }
}
```

### Controlled Input

```rust
#[component]
fn ControlledInput(value: RwSignal<String>) -> impl IntoView {
    view! {
        <input
            type="text"
            prop:value=move || value.get()
            on:input=move |ev| value.set(event_target_value(&ev))
        />
    }
}
```

## Effects

### Side Effects

```rust
#[component]
fn DataLoader(id: u64) -> impl IntoView {
    let data = RwSignal::new(None);
    
    Effect::new(move |_| {
        // Runs when id changes
        spawn_local(async move {
            let result = fetch_data(id).await;
            data.set(Some(result));
        });
    });
    
    view! {
        <Show when=move || data.get().is_some()>
            {move || format!("{:?}", data.get().unwrap())}
        </Show>
    }
}
```

## Anti-Patterns

| Anti-Pattern | Problem | Solution |
|--------------|---------|----------|
| `input type="number"` | Hard with decimals/negatives | Text input + validation |
| Cloning signals in closures | Unnecessary overhead | Use `move` closures |
| Large component functions | Hard to maintain | Split into smaller components |
| Prop drilling | Verbose, fragile | Use context |

## Related Skills

- **pl3xus-client**: pl3xus client patterns
- **pl3xus-queries**: Data fetching

## Reference

- [Leptos Documentation](https://leptos.dev/)
- [Component Patterns](./references/component-patterns.md)

