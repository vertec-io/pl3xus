# Entity Policies Reference

## Overview

Entity policies control access to entity-targeted requests. They determine whether a client can perform an action on a specific entity.

## Default Entity Policy

The default policy requires the client to have `EntityControl` of the target entity:

```rust
app.request::<UpdatePosition, WebSocketProvider>()
    .targeted()
    .with_default_entity_policy()  // Requires EntityControl
    .register();
```

## EntityControl Component

```rust
use pl3xus_sync::EntityControl;

#[derive(Component, Clone, Serialize, Deserialize, Default)]
pub struct EntityControl {
    pub client_id: Option<ConnectionId>,
    pub granted_at: Option<SystemTime>,
}
```

## Policy Types

### Default Policy (Requires Control)

```rust
// Client must have EntityControl to modify
app.request::<UpdateConfig, WebSocketProvider>()
    .targeted()
    .with_default_entity_policy()
    .register();
```

### No Policy (Read-Only)

```rust
// Anyone can query - no authorization needed
app.request::<GetRobotInfo, WebSocketProvider>()
    .targeted()
    // No .with_entity_policy() - open access
    .register();
```

### Custom Policy

```rust
app.request::<AdminCommand, WebSocketProvider>()
    .targeted()
    .with_entity_policy(EntityAccessPolicy::from_fn(|world, source, entity| {
        // Custom authorization logic
        let admins = world.resource::<AdminList>();
        if admins.contains(&source) {
            Ok(())
        } else {
            Err(AuthError::Forbidden("Admin access required".into()))
        }
    }))
    .register();
```

## Hierarchical Control

Control can propagate through entity hierarchies:

```rust
use pl3xus_sync::has_hierarchical_control;

fn check_control(world: &World, client: ConnectionId, entity: Entity) -> bool {
    has_hierarchical_control(world, client, entity)
}
```

### Hierarchy Setup

```rust
// System entity (parent) - controls all children
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

### Control Flow

```
Client requests control of System
         │
         ▼
┌─────────────────┐
│     System      │ ◀── EntityControl { client_id: Some(A) }
│   (parent)      │
└────────┬────────┘
         │ ChildOf
         ▼
┌─────────────────┐
│    Robot-1      │ ◀── Inherits control from parent
│   (child)       │
└─────────────────┘
```

## Control Request Pattern

### Request Type

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ControlRequest {
    Take { entity_bits: u64 },
    Release { entity_bits: u64 },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ControlResponse {
    pub granted: bool,
    pub error: Option<String>,
}
```

### Server Handler

```rust
fn handle_control_request(
    mut messages: MessageReader<NetworkData<NetworkRequest<ControlRequest>>>,
    mut controls: Query<&mut EntityControl>,
) {
    for request in messages.read() {
        match &request.message.request {
            ControlRequest::Take { entity_bits } => {
                let entity = Entity::from_bits(*entity_bits);
                if let Ok(mut control) = controls.get_mut(entity) {
                    if control.client_id.is_none() {
                        control.client_id = Some(request.source);
                        control.granted_at = Some(SystemTime::now());
                        let _ = request.respond(ControlResponse {
                            granted: true,
                            error: None,
                        });
                    } else {
                        let _ = request.respond(ControlResponse {
                            granted: false,
                            error: Some("Already controlled".into()),
                        });
                    }
                }
            }
            ControlRequest::Release { entity_bits } => {
                let entity = Entity::from_bits(*entity_bits);
                if let Ok(mut control) = controls.get_mut(entity) {
                    if control.client_id == Some(request.source) {
                        control.client_id = None;
                        control.granted_at = None;
                        let _ = request.respond(ControlResponse {
                            granted: true,
                            error: None,
                        });
                    }
                }
            }
        }
    }
}
```

## Client-Side Control Check

```rust
#[component]
fn ControlStatus(entity_id: u64) -> impl IntoView {
    let ctx = use_context::<SyncContext>().unwrap();
    let control = use_entity_component::<EntityControl>(entity_id);
    
    let has_control = move || {
        control.get()
            .and_then(|c| c.client_id)
            .map(|id| Some(id) == ctx.my_connection_id.get())
            .unwrap_or(false)
    };
    
    let controlled_by_other = move || {
        control.get()
            .and_then(|c| c.client_id)
            .map(|id| Some(id) != ctx.my_connection_id.get())
            .unwrap_or(false)
    };
    
    view! {
        <Show when=has_control>
            <span class="badge success">"You have control"</span>
        </Show>
        <Show when=controlled_by_other>
            <span class="badge warning">"Controlled by another client"</span>
        </Show>
    }
}
```

## Error Types

```rust
pub enum AuthError {
    Forbidden(String),      // No permission
    NotFound,               // Entity doesn't exist
    NoControl,              // No EntityControl component
    ControlledByOther,      // Another client has control
}
```

