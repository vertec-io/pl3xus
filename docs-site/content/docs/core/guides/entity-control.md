---
title: Entity Control
---
# Entity Control

Exclusive control patterns for multi-client applications.

---

## Overview

In multi-client applications, you often need to ensure only one client can control an entity at a time. pl3xus provides the `EntityControl` component and hierarchical control patterns for this.

---

## EntityControl Component

The `EntityControl` component tracks which client has control of an entity:

```rust
use pl3xus_sync::EntityControl;

// Spawn with control tracking
commands.spawn((
    Robot { name: "Robot-1".into() },
    EntityControl::default(),  // No one has control initially
));
```

### EntityControl Fields

```rust
pub struct EntityControl {
    pub connection_id: Option<ConnectionId>,  // Who has control
    pub client_name: Option<String>,          // Display name
}
```

---

## Requesting Control

### Client: Request Control

```rust
use pl3xus_client::use_mutation;
use shared::ControlRequest;

#[component]
fn ControlButton(entity_id: u64) -> impl IntoView {
    let mutation = use_mutation::<ControlRequest>(|result| {
        match result {
            Ok(ControlResponse::Granted) => log!("Control granted!"),
            Ok(ControlResponse::Denied(reason)) => log!("Denied: {reason}"),
            Err(e) => log!("Error: {e}"),
        }
    });

    view! {
        <button on:click=move |_| mutation.send(ControlRequest::Take(entity_id))>
            "Take Control"
        </button>
    }
}
```

### Server: Handle Control Requests

```rust
fn handle_control_request(
    mut requests: MessageReader<Request<ControlRequest>>,
    mut query: Query<&mut EntityControl>,
    net: Res<Network<NP>>,
) {
    for request in requests.read() {
        match request.inner() {
            ControlRequest::Take(entity_bits) => {
                let entity = Entity::from_bits(*entity_bits);
                if let Ok(mut control) = query.get_mut(entity) {
                    if control.connection_id.is_none() {
                        // Grant control
                        control.connection_id = Some(request.source());
                        net.send(request.source(), ControlResponse::Granted);
                    } else {
                        net.send(request.source(), ControlResponse::Denied(
                            "Entity already controlled".into()
                        ));
                    }
                }
            }
            ControlRequest::Release(entity_bits) => {
                let entity = Entity::from_bits(*entity_bits);
                if let Ok(mut control) = query.get_mut(entity) {
                    if control.connection_id == Some(request.source()) {
                        control.connection_id = None;
                        net.send(request.source(), ControlResponse::Released);
                    }
                }
            }
        }
    }
}
```

---

## Hierarchical Control

Control of a parent entity grants control over all children. This is useful for systems like:

- Robot (parent) → Tools, Sensors (children)
- Machine (parent) → Axes, I/O (children)

### Check Hierarchical Control

```rust
use pl3xus_sync::has_hierarchical_control;

fn check_control(world: &World, entity: Entity, client_id: ConnectionId) -> bool {
    has_hierarchical_control::<EntityControl, _>(
        world,
        entity,
        |control| control.connection_id == Some(client_id)
    )
}
```

### Use in Authorization

```rust
use pl3xus_sync::{EntityAccessPolicy, AuthResult};

app.request::<WriteValue, NP>()
    .targeted()
    .with_entity_policy(EntityAccessPolicy::from_fn(|world, ctx, entity| {
        if has_hierarchical_control::<EntityControl, _>(
            world,
            entity,
            |control| control.connection_id == Some(ctx.connection_id)
        ) {
            AuthResult::Authorized
        } else {
            AuthResult::Denied("No control of entity".into())
        }
    }))
    .register();
```

---

## Default Entity Policy

The most common pattern is the default entity policy, which checks `EntityControl`:

```rust
app.request::<SetSpeed, NP>()
    .targeted()
    .with_default_entity_policy()  // Uses EntityControl automatically
    .register();
```

This is equivalent to:

```rust
.with_entity_policy(EntityAccessPolicy::from_fn(|world, ctx, entity| {
    if has_hierarchical_control::<EntityControl, _>(
        world,
        entity,
        |control| control.connection_id == Some(ctx.connection_id)
    ) {
        AuthResult::Authorized
    } else {
        AuthResult::Denied("No control".into())
    }
}))
```

---

## UI Patterns

### Show Control Status

```rust
#[component]
fn ControlStatus(entity_id: u64) -> impl IntoView {
    let control = use_entity_component::<EntityControl>(entity_id.into());

    view! {
        <div class="control-status">
            {move || match control.get() {
                Some(c) if c.connection_id.is_some() => {
                    format!("Controlled by: {}", c.client_name.unwrap_or("Unknown".into()))
                }
                _ => "Available".into()
            }}
        </div>
    }
}
```

### Disable Controls When Not In Control

```rust
#[component]
fn SpeedSlider(entity_id: u64, has_control: Signal<bool>) -> impl IntoView {
    let mutation = use_mutation_targeted::<SetSpeed>(|_| {});

    view! {
        <input
            type="range"
            disabled=move || !has_control.get()
            on:change=move |ev| {
                let value = event_target_value(&ev).parse().unwrap_or(0.0);
                mutation.send(entity_id, SetSpeed { value });
            }
        />
    }
}
```

---

## Related

- [Authorization](./authorization.md) - Custom authorization policies
- [Mutations](./mutations.md) - Authorized mutations
- [Server Development](../sync/index.md) - Server-side patterns

