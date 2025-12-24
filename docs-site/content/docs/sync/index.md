---
title: Server Development
---
# Server Development

Build Bevy ECS servers that synchronize state to web clients in real-time.

---

## Overview

pl3xus_sync provides:

- **Automatic component synchronization** - Register once, sync forever
- **Builder pattern API** - Fluent configuration for components and requests
- **Authorization system** - Control who can read and modify what
- **Targeted requests** - Entity-specific operations with authorization
- **Hierarchical control** - Parent-child entity control inheritance

---

## Quick Start

### 1. Register Components for Sync

```rust
use pl3xus_sync::{Pl3xusSyncPlugin, AppPl3xusSyncExt};

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins(Pl3xusSyncPlugin::default())
        // Simple registration
        .sync_component::<Position>(None)
        .sync_component::<Velocity>(None)
        // That's it! Changes are automatically synced
        .run();
}
```

### 2. Spawn Entities

```rust
fn setup(mut commands: Commands) {
    commands.spawn((
        Position { x: 0.0, y: 0.0 },
        Velocity { x: 1.0, y: 0.0 },
    ));
}
```

Any changes to `Position` or `Velocity` are automatically detected and sent to subscribed clients.

---

## Component Registration

### Basic Registration

```rust
// Simple - no handler, no authorization
app.sync_component::<Position>(None);
```

### Builder Pattern

For more control, use the builder:

```rust
app.sync_component_builder::<JogSettings>()
    .with_handler::<NP>(handle_jog_settings_mutation)  // Custom mutation handler
    .targeted()                                         // Entity-specific
    .with_default_entity_policy()                       // Requires EntityControl
    .build();
```

### With Custom Handler

```rust
fn handle_jog_settings_mutation(
    mut mutations: MessageReader<AuthorizedComponentMutation<JogSettings>>,
    mut query: Query<&mut JogSettings>,
) {
    for mutation in mutations.read() {
        let entity = mutation.entity();  // Already authorized!
        if let Ok(mut settings) = query.get_mut(entity) {
            *settings = mutation.into_inner();
        }
    }
}
```

---

## Request Registration

### Non-Targeted Requests

For operations that don't target a specific entity:

```rust
// Registration
app.request::<CreateRobot, NP>().register();

// Handler
fn handle_create_robot(
    mut requests: MessageReader<Request<CreateRobot>>,
    mut commands: Commands,
    net: Res<Network<NP>>,
) {
    for request in requests.read() {
        let entity = commands.spawn(Robot { name: request.name.clone() }).id();
        net.send(request.source(), CreateRobotResponse {
            robot_id: entity.to_bits()
        });
    }
}
```

### Targeted Requests (with Authorization)

For entity-specific operations:

```rust
// Registration
app.request::<SetSpeed, NP>()
    .targeted()
    .with_default_entity_policy()  // Requires EntityControl
    .register();

// Handler receives AuthorizedRequest
fn handle_set_speed(
    mut requests: MessageReader<AuthorizedRequest<SetSpeed>>,
    mut query: Query<&mut Speed>,
) {
    for request in requests.read() {
        let entity = request.entity();  // Already authorized!
        if let Ok(mut speed) = query.get_mut(entity) {
            speed.value = request.value;
        }
    }
}
```

### Targeted Requests (without Authorization)

For read-only entity-specific operations:

```rust
// Registration - no authorization policy
app.request::<GetStatus, NP>()
    .targeted()
    .register();

// Handler receives TargetedRequest
fn handle_get_status(
    mut requests: MessageReader<Request<TargetedRequest<GetStatus>>>,
    query: Query<&Status>,
    net: Res<Network<NP>>,
) {
    for request in requests.read() {
        let target_id = &request.target_id;
        let entity = Entity::from_bits(target_id.parse::<u64>().unwrap());

        if let Ok(status) = query.get(entity) {
            net.send(request.source(), GetStatusResponse {
                status: status.clone()
            });
        }
    }
}
```

---

## Authorization

### EntityControl Component

The `EntityControl` component tracks which client has control:

```rust
use pl3xus_sync::EntityControl;

// Spawn with control tracking
commands.spawn((
    Robot { name: "Robot-1".into() },
    EntityControl::default(),  // No one has control initially
));
```

### Default Entity Policy

The most common pattern - requires the client to have `EntityControl`:

```rust
app.request::<WriteValue, NP>()
    .targeted()
    .with_default_entity_policy()
    .register();
```

### Custom Authorization

```rust
use pl3xus_sync::{EntityAccessPolicy, AuthResult};

app.request::<AdminCommand, NP>()
    .targeted()
    .with_entity_policy(EntityAccessPolicy::from_fn(|world, ctx, entity| {
        // Custom authorization logic
        if is_admin(world, ctx.connection_id) {
            AuthResult::Authorized
        } else {
            AuthResult::Denied("Admin access required".into())
        }
    }))
    .register();
```

### Hierarchical Control

Control of a parent entity grants control over children:

```rust
use pl3xus_sync::has_hierarchical_control;

// Check if client has control of entity or any ancestor
if has_hierarchical_control::<EntityControl, _>(
    world,
    entity,
    |control| control.connection_id == Some(client_id)
) {
    // Authorized
}
```

---

## Message Registration

For one-way messages (no response):

```rust
// Registration
app.message::<JogCommand, NP>()
    .targeted()
    .with_default_entity_policy()
    .register();

// Handler
fn handle_jog_command(
    mut messages: MessageReader<AuthorizedMessage<JogCommand>>,
    mut query: Query<&mut JogState>,
) {
    for message in messages.read() {
        let entity = message.entity();
        if let Ok(mut state) = query.get_mut(entity) {
            state.apply_jog(message.into_inner());
        }
    }
}
```

---

## Best Practices

### 1. Use Builder Pattern for Complex Registration

```rust
// ❌ Hard to read
app.sync_component::<Settings>(Some(ComponentSyncConfig { ... }));

// ✅ Clear and extensible
app.sync_component_builder::<Settings>()
    .with_handler::<NP>(handle_settings)
    .targeted()
    .with_default_entity_policy()
    .build();
```

### 2. Use Targeted Requests for Entity Operations

```rust
// ❌ Entity ID in request body - no authorization
struct UpdateRobot { entity_id: u64, speed: f32 }

// ✅ Targeted request with authorization
struct UpdateRobotSpeed { speed: f32 }
app.request::<UpdateRobotSpeed, NP>()
    .targeted()
    .with_default_entity_policy()
    .register();
```

### 3. Always Validate in Handlers

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

---

## Related

- [Client Development](../client/index.md) - Build Leptos clients
- [Mutations](../core/guides/mutations.md) - Mutation patterns
- [Authorization](../core/guides/authorization.md) - Deep dive into authorization
