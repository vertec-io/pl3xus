---
name: pl3xus-server
description: Server-side Bevy ECS patterns for pl3xus applications. Covers component synchronization, message handlers, plugin organization, and entity management. Use when implementing server-side logic.
allowed-tools:
  - view
  - codebase-retrieval
  - save-file
  - str-replace-editor
  - launch-process
  - read-process
---

# pl3xus Server Skill

## Purpose

This skill covers server-side implementation patterns for pl3xus applications using Bevy ECS. The server is the authoritative source of truth for all application state.

## When to Use

Use this skill when:
- Setting up a new pl3xus server
- Implementing component synchronization
- Creating message/request handlers
- Organizing server code with plugins

## Server Setup

### Basic Server Structure

```rust
use bevy::prelude::*;
use pl3xus_websockets::WebSocketProvider;
use pl3xus_sync::{Pl3xusSyncPlugin, AppPl3xusSyncExt};

fn main() {
    App::new()
        .add_plugins((
            MinimalPlugins,
            Pl3xusSyncPlugin::<WebSocketProvider>::default(),
        ))
        // Component sync
        .sync_component::<Position>(None)
        .sync_component::<Velocity>(None)
        // Request registration
        .request::<UpdatePosition, WebSocketProvider>()
            .targeted()
            .with_default_entity_policy()
            .register()
        .run();
}
```

### Plugin Organization

Organize server code into Bevy plugins:

```rust
// src/plugins/robot.rs
pub struct RobotPlugin;

impl Plugin for RobotPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            update_robot_state,
            process_robot_commands,
        ));
    }
}

// src/main.rs
app.add_plugins((
    Pl3xusSyncPlugin::<WebSocketProvider>::default(),
    RobotPlugin,
    ControlPlugin,
    ProgramPlugin,
));
```

## Component Synchronization

### Registering Components

```rust
// Sync to all connected clients
app.sync_component::<Position>(None);

// Sync with custom settings
app.sync_component::<Velocity>(Some(SyncSettings {
    send_on_connect: true,
    ..default()
}));
```

### Synced Component Requirements

Components must implement:
- `Clone`
- `Serialize` + `Deserialize` (serde)
- `Default` (recommended)

```rust
#[derive(Clone, Debug, Serialize, Deserialize, Default, Component)]
pub struct Position {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
```

## Request Handlers

### Targeted Request Pattern (Production)

```rust
use bevy::prelude::*;
use pl3xus::prelude::*;

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

### Batch Request Registration

Register multiple related requests with the same configuration:

```rust
app.requests::<(
    SetSpeedOverride,
    ResetRobot,
    InitializeRobot,
    AbortMotion,
), WebSocketProvider>()
    .targeted()
    .with_default_entity_policy()
    .with_error_response();
```

## Entity Management

### Spawning Entities

```rust
fn spawn_robot(mut commands: Commands) {
    commands.spawn((
        Name::new("Robot-1"),
        Position::default(),
        Velocity::default(),
        RobotMarker,
    ));
}
```

### Entity Hierarchies

Use Bevy's built-in hierarchy components:

```rust
// Parent entity
let system = commands.spawn((
    Name::new("System"),
    SystemMarker,
)).id();

// Child entity
commands.spawn((
    Name::new("Robot-1"),
    ChildOf(system),  // Bevy 0.17 hierarchy
    RobotMarker,
));
```

## Server-Driven UI State

Include state and action flags in synced components:

```rust
#[derive(Clone, Serialize, Deserialize, Component)]
pub struct ProgramState {
    pub state: ExecutionState,
    // Action flags - server determines what's valid
    pub can_start: bool,
    pub can_pause: bool,
    pub can_resume: bool,
    pub can_stop: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum ExecutionState {
    NoProgram,
    Idle,
    Running,
    Paused,
    Completed,
    Error(String),
}
```

## Related Skills

- **pl3xus-queries**: Request/response patterns in depth
- **pl3xus-mutations**: Mutation handling
- **pl3xus-authorization**: Entity policies and control
- **bevy-ecs**: Bevy ECS fundamentals

## Reference

- [Component Sync](./references/component-sync.md)
- [Message Handlers](./references/message-handlers.md)
- [Plugin Organization](./references/plugin-organization.md)

