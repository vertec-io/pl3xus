---
name: pl3xus-mutations
description: Mutation patterns for pl3xus applications. Covers state-changing operations, invalidation, optimistic updates, and mutation handlers. Use when implementing write operations.
allowed-tools:
  - view
  - codebase-retrieval
  - save-file
  - str-replace-editor
---

# pl3xus Mutations Skill

## Purpose

This skill covers mutation patterns in pl3xus. Mutations are state-changing operations that modify server state and can trigger query invalidation.

## When to Use

Use this skill when:
- Implementing state-changing operations
- Setting up mutation handlers
- Configuring query invalidation
- Handling mutation responses

## Mutation Types

### Defining Mutation Types

```rust
// shared/src/requests.rs
use pl3xus_common::RequestMessage;
use pl3xus_macros::Invalidates;

#[derive(Clone, Debug, Serialize, Deserialize, Invalidates)]
#[invalidates("ListPrograms")]
pub struct CreateProgram {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateProgramResponse {
    pub success: bool,
    pub program_id: Option<u64>,
    pub error: Option<String>,
}

impl RequestMessage for CreateProgram {
    type ResponseMessage = CreateProgramResponse;
}
```

### Invalidation Derive Macro

Use `#[invalidates(...)]` to declare which queries to invalidate:

```rust
// Single query invalidation
#[derive(Invalidates)]
#[invalidates("ListPrograms")]
pub struct CreateProgram { ... }

// Multiple query invalidation
#[derive(Invalidates)]
#[invalidates("ListPrograms", "GetProgram", "GetProgramStats")]
pub struct DeleteProgram { ... }
```

## Server Registration

### Targeted Mutation

```rust
app.request::<CreateProgram, WebSocketProvider>()
    .targeted()
    .with_default_entity_policy()
    .register();
```

### Batch Mutation Registration

```rust
app.requests::<(
    CreateProgram,
    UpdateProgram,
    DeleteProgram,
    LoadProgram,
    UnloadProgram,
), WebSocketProvider>()
    .targeted()
    .with_default_entity_policy()
    .with_error_response();
```

## Server Handlers

### Mutation Handler with Invalidation

```rust
use pl3xus_sync::broadcast_invalidations_for;

fn handle_create_program(
    mut messages: MessageReader<NetworkData<TargetedRequest<CreateProgram>>>,
    mut commands: Commands,
    net: Res<Network<WebSocketProvider>>,
) {
    for request in messages.read() {
        let program = commands.spawn((
            Name::new(&request.message.request.name),
            ProgramData {
                description: request.message.request.description.clone(),
                ..default()
            },
        )).id();
        
        let response = CreateProgramResponse {
            success: true,
            program_id: Some(program.to_bits()),
            error: None,
        };
        
        if let Ok(()) = request.respond(response.clone()) {
            if response.success {
                // Broadcast invalidation to all clients
                broadcast_invalidations_for::<CreateProgram, _>(&net, None);
            }
        }
    }
}
```

### Error Handling Pattern

```rust
fn handle_delete_program(
    mut messages: MessageReader<NetworkData<TargetedRequest<DeleteProgram>>>,
    mut commands: Commands,
    programs: Query<Entity, With<ProgramMarker>>,
    net: Res<Network<WebSocketProvider>>,
) {
    for request in messages.read() {
        let entity = Entity::from_bits(request.message.target_entity);
        
        let response = if programs.get(entity).is_ok() {
            commands.entity(entity).despawn();
            DeleteProgramResponse { success: true, error: None }
        } else {
            DeleteProgramResponse {
                success: false,
                error: Some("Program not found".into()),
            }
        };
        
        if let Ok(()) = request.respond(response.clone()) {
            if response.success {
                broadcast_invalidations_for::<DeleteProgram, _>(&net, None);
            }
        }
    }
}
```

## Client Usage

### Targeted Mutation with Handler

```rust
let create = use_mutation_targeted::<CreateProgram>(move |result| {
    match result {
        Ok(r) if r.success => {
            toast.success("Program created");
            // Query cache is automatically invalidated
        }
        Ok(r) => toast.error(r.error.unwrap_or_default()),
        Err(e) => toast.error(e),
    }
});

// Send mutation to system entity
create.send(system_id, CreateProgram {
    name: "New Program".into(),
    description: None,
});
```

### Mutation State

```rust
let delete = use_mutation_targeted::<DeleteProgram>(|_| {});

view! {
    <button
        disabled=move || delete.is_pending()
        on:click=move |_| delete.send(program_id, DeleteProgram)
    >
        {move || if delete.is_pending() { "Deleting..." } else { "Delete" }}
    </button>
}
```

## Invalidation Flow

```
Client                    Server                    All Clients
   │                         │                           │
   │──CreateProgram─────────▶│                           │
   │                         │ (create entity)           │
   │                         │                           │
   │◀──Response──────────────│                           │
   │                         │──QueryInvalidation───────▶│
   │                         │  (ListPrograms)           │
   │                         │                           │
   │ (refetch ListPrograms)  │                           │
```

## Related Skills

- **pl3xus-queries**: For read operations
- **pl3xus-authorization**: For access control
- **pl3xus-server**: Server-side patterns

## Reference

- [Mutation Handlers](./references/mutation-handlers.md)
- [Invalidation](./references/invalidation.md)

