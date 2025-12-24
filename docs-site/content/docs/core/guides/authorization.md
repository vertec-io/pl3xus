---
title: Authorization
---
# Authorization

Control who can read and modify what in your application.

---

## Overview

pl3xus provides a flexible authorization system:

- **Default Entity Policy** - Built-in `EntityControl` checking
- **Custom Policies** - Define your own authorization logic
- **Hierarchical Control** - Parent control grants child access

---

## Authorization Flow

```
Client Request → Server Receives → Authorization Check → Handler (if authorized)
                                         ↓
                                   Denied Response (if not authorized)
```

---

## Default Entity Policy

The simplest and most common pattern:

```rust
app.request::<SetSpeed, NP>()
    .targeted()
    .with_default_entity_policy()
    .register();
```

This checks:
1. Does the target entity exist?
2. Does the entity have an `EntityControl` component?
3. Does the client's `ConnectionId` match the control holder?
4. If not direct match, check parent entities (hierarchical)

---

## Custom Authorization

### Simple Closure

```rust
use pl3xus_sync::{EntityAccessPolicy, AuthResult};

app.request::<AdminCommand, NP>()
    .targeted()
    .with_entity_policy(EntityAccessPolicy::from_fn(|world, ctx, entity| {
        // ctx.connection_id - the requesting client
        // entity - the target entity
        
        if is_admin(world, ctx.connection_id) {
            AuthResult::Authorized
        } else {
            AuthResult::Denied("Admin access required".into())
        }
    }))
    .register();
```

### Check Multiple Conditions

```rust
.with_entity_policy(EntityAccessPolicy::from_fn(|world, ctx, entity| {
    // Check entity exists
    let Some(entity_ref) = world.get_entity(entity) else {
        return AuthResult::Denied("Entity not found".into());
    };
    
    // Check entity is controllable
    let Some(control) = entity_ref.get::<EntityControl>() else {
        return AuthResult::Denied("Entity not controllable".into());
    };
    
    // Check client has control
    if control.connection_id != Some(ctx.connection_id) {
        return AuthResult::Denied("No control of entity".into());
    }
    
    // Check entity is in valid state
    let Some(state) = entity_ref.get::<RobotState>() else {
        return AuthResult::Denied("Invalid entity state".into());
    };
    
    if state.is_locked {
        return AuthResult::Denied("Entity is locked".into());
    }
    
    AuthResult::Authorized
}))
```

---

## AuthResult

```rust
pub enum AuthResult {
    Authorized,
    Denied(String),  // Reason sent to client
}
```

---

## Handler Receives Authorized Request

When authorization passes, your handler receives an `AuthorizedRequest`:

```rust
fn handle_set_speed(
    mut requests: MessageReader<AuthorizedRequest<SetSpeed>>,
    mut query: Query<&mut Speed>,
) {
    for request in requests.read() {
        // entity() is guaranteed to exist and be authorized
        let entity = request.entity();
        
        if let Ok(mut speed) = query.get_mut(entity) {
            speed.value = request.value;
        }
    }
}
```

### AuthorizedRequest Methods

| Method | Description |
|--------|-------------|
| `entity()` | The authorized target entity |
| `source()` | The client's ConnectionId |
| `into_inner()` | Extract the request payload |
| `Deref` | Access request fields directly |

---

## Component Mutations

For synced component mutations:

```rust
app.sync_component_builder::<JogSettings>()
    .with_handler::<NP>(handle_jog_settings)
    .targeted()
    .with_default_entity_policy()
    .build();

fn handle_jog_settings(
    mut mutations: MessageReader<AuthorizedComponentMutation<JogSettings>>,
    mut query: Query<&mut JogSettings>,
) {
    for mutation in mutations.read() {
        let entity = mutation.entity();
        if let Ok(mut settings) = query.get_mut(entity) {
            *settings = mutation.into_inner();
        }
    }
}
```

---

## Messages (No Response)

For one-way messages:

```rust
app.message::<JogCommand, NP>()
    .targeted()
    .with_default_entity_policy()
    .register();

fn handle_jog_command(
    mut messages: MessageReader<AuthorizedMessage<JogCommand>>,
    mut query: Query<&mut JogState>,
) {
    for message in messages.read() {
        let entity = message.entity();
        if let Ok(mut state) = query.get_mut(entity) {
            state.apply(message.into_inner());
        }
    }
}
```

---

## Best Practices

### 1. Always Use Authorization for Mutations

```rust
// ❌ No authorization - anyone can modify
app.request::<SetSpeed, NP>().register();

// ✅ With authorization
app.request::<SetSpeed, NP>()
    .targeted()
    .with_default_entity_policy()
    .register();
```

### 2. Validate in Handler Too

Authorization checks access, but validate the data:

```rust
fn handle_set_speed(mut requests: MessageReader<AuthorizedRequest<SetSpeed>>) {
    for request in requests.read() {
        // Validate even though authorized
        if request.value < 0.0 || request.value > 1000.0 {
            // Send error response
            continue;
        }
        // Apply...
    }
}
```

### 3. Use Hierarchical Control for Complex Systems

```rust
// System entity controls all children
commands.spawn((
    System { name: "Main".into() },
    EntityControl::default(),
)).with_children(|parent| {
    parent.spawn(Robot { name: "Robot-1".into() });
    parent.spawn(Robot { name: "Robot-2".into() });
});

// Control of System grants control of all Robots
```

---

## Related

- [Entity Control](./entity-control.md) - Control handoff patterns
- [Mutations](./mutations.md) - Mutation patterns
- [Server Development](../sync/index.md) - Server-side APIs

