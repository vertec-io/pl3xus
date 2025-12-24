---
name: pl3xus-authorization
description: Authorization patterns for pl3xus applications. Covers entity policies, control patterns, hierarchical access, and message authorization. Use when implementing access control.
allowed-tools:
  - view
  - codebase-retrieval
  - save-file
  - str-replace-editor
---

# pl3xus Authorization Skill

## Purpose

This skill covers authorization patterns in pl3xus. Authorization ensures that clients can only access and modify entities they have permission to control.

## When to Use

Use this skill when:
- Implementing entity access control
- Setting up control hierarchies
- Creating custom authorization policies
- Handling multi-user scenarios

## Entity Control

### EntityControl Component

The `EntityControl` component tracks which client controls an entity:

```rust
use pl3xus_sync::EntityControl;

// Synced automatically when registered
app.sync_component::<EntityControl>(None);
```

### Control Flow

```
Client A                    Server                    Client B
   │                           │                          │
   │──RequestControl──────────▶│                          │
   │                           │ (grant control)          │
   │◀──ControlGranted─────────│                          │
   │                           │──EntityControl sync─────▶│
   │                           │  (client_id: A)          │
   │                           │                          │
   │──UpdatePosition──────────▶│                          │
   │                           │ (authorized - has ctrl)  │
   │◀──Response───────────────│                          │
   │                           │                          │
   │                           │                          │
   │                           │◀──UpdatePosition────────│
   │                           │ (denied - no control)    │
   │                           │──ErrorResponse──────────▶│
```

## Entity Policies

### Default Entity Policy

Use for most targeted requests:

```rust
app.request::<UpdatePosition, WebSocketProvider>()
    .targeted()
    .with_default_entity_policy()  // Requires EntityControl
    .register();
```

### Custom Entity Policy

For custom authorization logic:

```rust
app.request::<AdminCommand, WebSocketProvider>()
    .targeted()
    .with_entity_policy(EntityAccessPolicy::from_fn(|world, source, entity| {
        // Check if source has admin role
        let admins = world.resource::<AdminList>();
        if admins.contains(&source) {
            Ok(())
        } else {
            Err(AuthError::Forbidden("Admin access required".into()))
        }
    }))
    .register();
```

### No Authorization (Read-Only)

For queries that don't require control:

```rust
app.request::<GetRobotInfo, WebSocketProvider>()
    .targeted()
    // No .with_entity_policy() - anyone can query
    .register();
```

## Hierarchical Control

### Parent-Child Hierarchy

Control propagates through entity hierarchies:

```rust
// System entity (parent)
let system = commands.spawn((
    Name::new("System"),
    EntityControl::default(),
)).id();

// Robot entity (child) - inherits control from parent
commands.spawn((
    Name::new("Robot-1"),
    ChildOf(system),
    EntityControl::default(),
));
```

### Hierarchical Control Check

```rust
use pl3xus_sync::has_hierarchical_control;

fn handle_robot_command(
    mut messages: MessageReader<NetworkData<TargetedRequest<RobotCommand>>>,
    world: &World,
) {
    for request in messages.read() {
        let entity = Entity::from_bits(request.message.target_entity);
        
        if has_hierarchical_control(world, request.source, entity) {
            // Process command
        } else {
            // Deny - no control
        }
    }
}
```

## Message Policies

### Message-Level Authorization

For non-targeted messages:

```rust
app.message::<AdminBroadcast, WebSocketProvider>()
    .with_message_policy(MessagePolicy::from_fn(|world, source| {
        // Check admin status
        Ok(())
    }))
    .register();
```

## Client-Side Control

### Checking Control Status

```rust
#[component]
fn RobotControls(robot_id: u64) -> impl IntoView {
    let ctx = use_context::<SyncContext>().unwrap();
    let control = use_entity_component::<EntityControl>(robot_id);
    
    let has_control = move || {
        control.get()
            .and_then(|c| c.client_id)
            .map(|id| Some(id) == ctx.my_connection_id.get())
            .unwrap_or(false)
    };
    
    view! {
        <Show when=has_control>
            <ControlPanel />
        </Show>
        <Show when=move || !has_control()>
            <p>"Another client has control"</p>
        </Show>
    }
}
```

### Requesting Control

```rust
let request_control = use_mutation_targeted::<RequestControl>(move |result| {
    match result {
        Ok(r) if r.granted => toast.success("Control granted"),
        Ok(r) => toast.warning("Control denied"),
        Err(e) => toast.error(e),
    }
});

view! {
    <button on:click=move |_| request_control.send(system_id, RequestControl)>
        "Request Control"
    </button>
}
```

## Authorization Errors

### Error Types

```rust
pub enum AuthError {
    Forbidden(String),      // No permission
    NotFound,               // Entity doesn't exist
    NoControl,              // No EntityControl on entity
    ControlledByOther,      // Another client has control
}
```

### Error Responses

When authorization fails, the framework automatically sends error responses:

```rust
// Client receives MutationResponse with status
pub enum MutationStatus {
    Ok,
    Forbidden,
    NotFound,
    ValidationError,
    InternalError,
}
```

## Related Skills

- **pl3xus-server**: Server-side patterns
- **pl3xus-mutations**: Mutation handling
- **pl3xus-queries**: Query patterns

## Reference

- [Entity Policies](./references/entity-policies.md)
- [Control Patterns](./references/control-patterns.md)

