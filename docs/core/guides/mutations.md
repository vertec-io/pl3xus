# Component Mutations Guide

This guide covers how clients can modify server-side ECS components through the mutation system in pl3xus_sync and pl3xus_client.

---

## Overview

Mutations allow web clients to request changes to server-side component data. The flow is:

1. **Client** sends a `MutateComponent` message with the new component value
2. **Server** validates the mutation through the `MutationAuthorizer` (if configured)
3. **Server** applies the change to the ECS world (if authorized)
4. **Server** sends back a `MutationResponse` with the result
5. **Server** broadcasts the updated component to all subscribers

---

## Server-Side Configuration

### Basic Setup

Register components for synchronization in your Bevy server:

```rust
use bevy::prelude::*;
use pl3xus_sync::{AppPl3xusSyncExt, Pl3xusSyncPlugin};
use pl3xus_websockets::WebSocketProvider;

fn main() {
    let mut app = App::new();
    
    // Add the sync plugin
    app.add_plugins(Pl3xusSyncPlugin::<WebSocketProvider>::default());
    
    // Register components for sync (mutations enabled by default)
    app.sync_component::<Position>(None);
    app.sync_component::<Velocity>(None);
    
    app.run();
}
```

### Mutation Authorization

By default, all mutations from any client are allowed. For production systems, you should implement authorization.

#### Option 1: Server-Only Mode

Disable all client mutations - only server-side code can modify components:

```rust
use pl3xus_sync::MutationAuthorizerResource;

app.insert_resource(MutationAuthorizerResource::server_only());
```

#### Option 2: Custom Authorization with Closure

Use a closure for simple authorization logic:

```rust
use pl3xus_sync::{MutationAuthorizerResource, MutationStatus};
use bevy::prelude::*;

app.insert_resource(MutationAuthorizerResource::from_fn(
    |world, mutation| {
        // Only allow Position mutations
        if mutation.component_type == "Position" {
            MutationStatus::Ok
        } else {
            MutationStatus::Forbidden
        }
    }
));
```

#### Option 3: Implement MutationAuthorizer Trait

For complex authorization logic, implement the trait directly:

```rust
use pl3xus_sync::{
    MutationAuthorizer, MutationAuthContext, MutationAuthorizerResource, 
    MutationStatus, QueuedMutation
};
use std::sync::Arc;

struct MyAuthorizer {
    allowed_components: Vec<&'static str>,
}

impl MutationAuthorizer for MyAuthorizer {
    fn authorize(&self, ctx: &MutationAuthContext, mutation: &QueuedMutation) -> MutationStatus {
        // Check if component type is allowed
        if self.allowed_components.contains(&mutation.component_type.as_str()) {
            MutationStatus::Ok
        } else {
            MutationStatus::Forbidden
        }
    }
}

// Install the authorizer
app.insert_resource(MutationAuthorizerResource {
    inner: Arc::new(MyAuthorizer {
        allowed_components: vec!["Position", "Velocity"],
    }),
});
```

### Hierarchy-Aware Authorization

For entity hierarchies where control of a parent grants control over children:

```rust
use pl3xus_sync::{has_control_hierarchical, MutationAuthorizerResource, MutationStatus};
use bevy::prelude::*;

// Your control marker component
#[derive(Component)]
struct EntityOwner {
    connection_id: pl3xus_common::ConnectionId,
}

app.insert_resource(MutationAuthorizerResource::from_fn(
    |world, mutation| {
        let entity = bevy::prelude::Entity::from_bits(mutation.entity.bits);
        
        // Check if this connection owns the entity or any ancestor
        if has_control_hierarchical::<EntityOwner, _>(
            world,
            entity,
            |owner| owner.connection_id == mutation.connection_id
        ) {
            MutationStatus::Ok
        } else {
            MutationStatus::Forbidden
        }
    }
));
```

---

## Client-Side Implementation

### Using SyncFieldInput (Recommended)

The easiest way to enable mutations is with the `SyncFieldInput` component:

```rust
use pl3xus_client::SyncFieldInput;
use leptos::prelude::*;

#[component]
fn PositionEditor(entity_id: u64) -> impl IntoView {
    view! {
        <div class="editor">
            <label>
                "X: "
                <SyncFieldInput<Position, f32>
                    entity_id=entity_id
                    field_accessor=|pos: &Position| pos.x
                    field_mutator=|pos: &Position, new_x: f32| {
                        Position { x: new_x, y: pos.y }
                    }
                    input_type="number"
                />
            </label>
        </div>
    }
}
```

**Features:**
- Input retains focus when server updates arrive
- Press **Enter** to send mutation to server
- Click away to **discard changes** and revert to server value
- Controlled input pattern prevents update loops

### Using SyncContext Directly

For more control, use the `SyncContext` directly:

```rust
use pl3xus_client::{use_sync_context, SyncComponent};
use leptos::prelude::*;

#[component]
fn ManualMutation(entity_id: u64) -> impl IntoView {
    let ctx = use_sync_context();

    let send_mutation = move |_| {
        let new_position = Position { x: 100.0, y: 200.0 };

        // Send mutation and get request ID for tracking
        let request_id = ctx.mutate(entity_id, new_position);

        // Optionally track the mutation status
        leptos::logging::log!("Mutation sent with request_id: {}", request_id);
    };

    view! {
        <button on:click=send_mutation>"Set Position to (100, 200)"</button>
    }
}
```

---

## Mutation Status Values

The server responds with one of these statuses:

| Status | Meaning |
|--------|---------|
| `Ok` | Mutation was applied successfully |
| `Forbidden` | Authorization denied the mutation |
| `NotFound` | Entity or component not found |
| `ValidationError` | Value failed validation |
| `InternalError` | Server-side error occurred |

---

## Best Practices

### 1. Always Implement Authorization in Production

Never deploy without a `MutationAuthorizer`:

```rust
// ❌ BAD: No authorization in production
app.sync_component::<Position>(None);

// ✅ GOOD: Explicit authorization
app.insert_resource(MutationAuthorizerResource::from_fn(|world, mutation| {
    // Your authorization logic here
    validate_mutation(world, mutation)
}));
```

### 2. Use Optimistic Updates Carefully

The client doesn't update its local state until the server confirms. This prevents desync but means there's a brief delay:

```rust
// Server confirms → broadcasts update → client receives
// No local optimistic update by design
```

### 3. Handle Mutation Failures Gracefully

```rust
// Track mutation status in your UI
let mutations = use_mutation_status();

view! {
    <Show when=move || mutations.get(&request_id).map(|s| s.is_error()).unwrap_or(false)>
        <div class="error">"Failed to save changes"</div>
    </Show>
}
```

### 4. Validate on Server, Not Just Client

Client-side validation improves UX, but always validate on the server:

```rust
app.insert_resource(MutationAuthorizerResource::from_fn(|world, mutation| {
    // Validate the component value
    if let Ok(pos) = bincode::deserialize::<Position>(&mutation.value) {
        if pos.x < 0.0 || pos.y < 0.0 {
            return MutationStatus::ValidationError;
        }
    }
    MutationStatus::Ok
}));
```

---

## Complete Example

See the control demo for a complete working example:

```bash
# Start the server
cargo run -p control_demo_server

# Start the client (in another terminal)
cd examples/control-demo/client && trunk serve
```

---

## Related Documentation

- [Sending Messages](./sending-messages.md) - Direct message sending
- [DevTools](./devtools.md) - Inspect mutations in DevTools
- [Shared Types](./shared-types.md) - Setting up shared component types
- [API Reference](https://docs.rs/pl3xus_sync) - Full API documentation

---

**Last Updated**: 2025-12-07
**pl3xus_sync Version**: 0.1


