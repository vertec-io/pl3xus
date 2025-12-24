# Message Handlers Reference

## Overview

Message handlers process incoming requests from clients. pl3xus uses Bevy 0.17's `MessageReader` API for handling network messages.

## Handler Types

### Targeted Request Handler

For requests that target a specific entity:

```rust
fn handle_update_position(
    mut messages: MessageReader<NetworkData<TargetedRequest<UpdatePosition>>>,
    mut positions: Query<&mut Position>,
) {
    for request in messages.read() {
        let entity = Entity::from_bits(request.message.target_entity);
        
        if let Ok(mut pos) = positions.get_mut(entity) {
            pos.x = request.message.request.x;
            pos.y = request.message.request.y;
            pos.z = request.message.request.z;
            
            let _ = request.respond(UpdatePositionResponse {
                success: true,
                error: None,
            });
        } else {
            let _ = request.respond(UpdatePositionResponse {
                success: false,
                error: Some("Entity not found".into()),
            });
        }
    }
}
```

### Non-Targeted Request Handler

For requests that don't target a specific entity:

```rust
fn handle_list_robots(
    mut messages: MessageReader<NetworkData<NetworkRequest<ListRobots>>>,
    robots: Query<(Entity, &Name), With<RobotMarker>>,
) {
    for request in messages.read() {
        let list: Vec<_> = robots.iter()
            .map(|(e, name)| RobotListItem {
                id: e.to_bits(),
                name: name.to_string(),
            })
            .collect();
        
        let _ = request.respond(ListRobotsResponse { robots: list });
    }
}
```

### Authorized Request Handler

For requests that require entity control:

```rust
fn handle_start_program(
    mut messages: MessageReader<NetworkData<AuthorizedRequest<StartProgram>>>,
    mut programs: Query<&mut ProgramState>,
) {
    for request in messages.read() {
        // Authorization already verified by framework
        let entity = Entity::from_bits(request.message.target_entity);
        
        if let Ok(mut state) = programs.get_mut(entity) {
            if state.can_start {
                state.state = ExecutionState::Running;
                state.can_start = false;
                state.can_pause = true;
                state.can_stop = true;
                
                let _ = request.respond(StartProgramResponse { success: true, error: None });
            } else {
                let _ = request.respond(StartProgramResponse {
                    success: false,
                    error: Some("Cannot start in current state".into()),
                });
            }
        }
    }
}
```

## Registration

### Targeted with Authorization

```rust
app.request::<UpdatePosition, WebSocketProvider>()
    .targeted()
    .with_default_entity_policy()
    .register();
```

### Targeted without Authorization (Read-Only)

```rust
app.request::<GetRobotInfo, WebSocketProvider>()
    .targeted()
    .register();
```

### Non-Targeted

```rust
app.request::<ListRobots, WebSocketProvider>()
    .register();
```

### Batch Registration

```rust
app.requests::<(
    StartProgram,
    PauseProgram,
    ResumeProgram,
    StopProgram,
), WebSocketProvider>()
    .targeted()
    .with_default_entity_policy()
    .with_error_response();
```

## Response Patterns

### Success Response

```rust
let _ = request.respond(UpdateResponse {
    success: true,
    error: None,
});
```

### Error Response

```rust
let _ = request.respond(UpdateResponse {
    success: false,
    error: Some("Validation failed: value out of range".into()),
});
```

### With Invalidation

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
            ProgramData::default(),
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

## Bevy 0.17 Note

Always use `MessageReader`, not `EventReader`:

```rust
// ✅ Correct - Bevy 0.17
fn handler(mut messages: MessageReader<NetworkData<T>>) { ... }

// ❌ Deprecated
fn handler(mut events: EventReader<NetworkData<T>>) { ... }
```

