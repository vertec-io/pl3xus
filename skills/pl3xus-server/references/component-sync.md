# Component Synchronization Reference

## Overview

pl3xus automatically synchronizes component changes from server to all connected clients. When a component is modified on the server, the change is pushed to clients via WebSocket.

## Registering Components

### Basic Registration

```rust
use pl3xus_sync::AppPl3xusSyncExt;

app.sync_component::<Position>(None);
```

### With Custom Settings

```rust
app.sync_component::<Position>(Some(SyncSettings {
    send_on_connect: true,  // Send current value when client connects
    ..default()
}));
```

## Component Requirements

Synced components must implement:

```rust
#[derive(Clone, Debug, Serialize, Deserialize, Default, Component)]
pub struct Position {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
```

Required traits:
- `Clone` - For copying data
- `Serialize` + `Deserialize` - For network transmission
- `Default` - Recommended for initialization
- `Component` - Bevy component marker

## Sync Flow

```
Server                              Client
   │                                   │
   │ (component modified)              │
   │                                   │
   │──ComponentUpdate<Position>───────▶│
   │  { entity: 123, data: {...} }     │
   │                                   │
   │                                   │ (signal updated)
   │                                   │ (UI re-renders)
```

## Change Detection

pl3xus uses Bevy's change detection to only sync modified components:

```rust
fn update_position(
    mut query: Query<&mut Position>,
) {
    for mut pos in query.iter_mut() {
        pos.x += 1.0;  // This triggers change detection
        // Component will be synced to clients
    }
}
```

## Entity Lifecycle

### Spawning Entities

When an entity with synced components is spawned, clients receive the initial state:

```rust
fn spawn_robot(mut commands: Commands) {
    commands.spawn((
        Name::new("Robot-1"),
        Position::default(),
        Velocity::default(),
    ));
    // Clients automatically receive Position and Velocity
}
```

### Despawning Entities

When an entity is despawned, clients are notified:

```rust
fn remove_robot(mut commands: Commands, entity: Entity) {
    commands.entity(entity).despawn();
    // Clients automatically remove the entity from their cache
}
```

## Type Registry

Clients must register types to deserialize them:

```rust
// Client-side
fn create_registry() -> ClientTypeRegistry {
    let mut registry = ClientTypeRegistry::new();
    registry.register::<Position>();
    registry.register::<Velocity>();
    registry.register::<RobotState>();
    registry
}

<SyncProvider url="ws://localhost:8080/ws" registry=create_registry()>
```

## Best Practices

### 1. Keep Components Small

```rust
// ✅ Good - focused component
#[derive(Clone, Serialize, Deserialize, Component)]
pub struct Position { pub x: f64, pub y: f64, pub z: f64 }

// ❌ Bad - too many concerns
#[derive(Clone, Serialize, Deserialize, Component)]
pub struct RobotEverything {
    pub position: Vec3,
    pub velocity: Vec3,
    pub config: Config,
    pub history: Vec<Event>,
    // ...
}
```

### 2. Use Marker Components

```rust
#[derive(Component)]
pub struct RobotMarker;  // Not synced, just for queries

#[derive(Clone, Serialize, Deserialize, Component)]
pub struct RobotState { ... }  // Synced
```

### 3. Server-Driven State

Include action flags in synced components:

```rust
#[derive(Clone, Serialize, Deserialize, Component)]
pub struct ProgramState {
    pub state: ExecutionState,
    // Server determines valid actions
    pub can_start: bool,
    pub can_pause: bool,
    pub can_resume: bool,
    pub can_stop: bool,
}
```

## Common Issues

### Component Not Syncing

1. Check component is registered: `app.sync_component::<T>(None)`
2. Check type is registered on client: `registry.register::<T>()`
3. Check component implements required traits

### Stale Data

1. Ensure WebSocket connection is active
2. Check for network errors in browser console
3. Verify server is running and accessible

