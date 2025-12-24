---
name: bevy-ecs
description: Bevy ECS fundamentals for industrial applications. Covers entities, components, systems, resources, and Bevy 0.17 patterns. Use when learning ECS concepts or implementing server logic.
allowed-tools:
  - view
  - codebase-retrieval
  - web-search
  - web-fetch
---

# Bevy ECS Skill

## Purpose

This skill covers Bevy ECS fundamentals as they apply to industrial applications. Bevy's ECS architecture provides excellent patterns for real-time systems with many entities.

## When to Use

Use this skill when:
- Learning ECS concepts
- Implementing server-side logic
- Understanding Bevy 0.17 patterns
- Designing entity structures

## ECS Overview

### Entity-Component-System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         World                                │
│  ┌─────────────────────────────────────────────────────────┐│
│  │ Entities (IDs)                                          ││
│  │  Entity(0)  Entity(1)  Entity(2)  Entity(3)             ││
│  └─────────────────────────────────────────────────────────┘│
│  ┌─────────────────────────────────────────────────────────┐│
│  │ Components (Data)                                       ││
│  │  Position   Velocity   Name   RobotMarker   Health      ││
│  │  [0,1,2]    [0,1]      [0,1]  [0,1]         [2,3]       ││
│  └─────────────────────────────────────────────────────────┘│
│  ┌─────────────────────────────────────────────────────────┐│
│  │ Resources (Global State)                                ││
│  │  Time   Network   Config   SystemState                  ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────┐
│ Systems (Logic)                                              │
│  update_positions()  process_commands()  sync_state()        │
└─────────────────────────────────────────────────────────────┘
```

## Core Concepts

### Entities

Entities are just IDs. They have no data themselves:

```rust
// Spawn an entity with components
let robot = commands.spawn((
    Name::new("Robot-1"),
    Position::default(),
    Velocity::default(),
    RobotMarker,
)).id();

// Get entity bits for network transmission
let entity_id: u64 = robot.to_bits();

// Reconstruct entity from bits
let entity = Entity::from_bits(entity_id);
```

### Components

Components are data attached to entities:

```rust
#[derive(Component, Clone, Debug, Serialize, Deserialize, Default)]
pub struct Position {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Component)]
pub struct RobotMarker;  // Marker component (no data)
```

### Systems

Systems are functions that process entities:

```rust
fn update_positions(
    time: Res<Time>,
    mut query: Query<(&mut Position, &Velocity)>,
) {
    for (mut pos, vel) in query.iter_mut() {
        pos.x += vel.x * time.delta_secs_f64();
        pos.y += vel.y * time.delta_secs_f64();
        pos.z += vel.z * time.delta_secs_f64();
    }
}
```

### Resources

Resources are global singleton data:

```rust
#[derive(Resource)]
pub struct SystemConfig {
    pub max_speed: f64,
    pub update_rate: f64,
}

fn read_config(config: Res<SystemConfig>) {
    println!("Max speed: {}", config.max_speed);
}
```

## Bevy 0.17 Patterns

### MessageReader/MessageWriter

Bevy 0.17 uses Messages instead of Events for network data:

```rust
// ✅ Correct - Bevy 0.17
fn handle_commands(
    mut messages: MessageReader<NetworkData<RobotCommand>>,
) {
    for msg in messages.read() {
        // Process message
    }
}

// ❌ Deprecated - Don't use
fn handle_commands(
    mut events: EventReader<NetworkData<RobotCommand>>,
) { ... }
```

### Entity Hierarchies

Use `ChildOf` for parent-child relationships:

```rust
// Parent
let system = commands.spawn((Name::new("System"),)).id();

// Child - use ChildOf component
commands.spawn((
    Name::new("Robot-1"),
    ChildOf(system),
));

// Query children
fn find_children(
    query: Query<(Entity, &ChildOf)>,
) {
    for (entity, child_of) in query.iter() {
        let parent = child_of.0;
    }
}
```

### Name Component

Use Bevy's built-in `Name` component:

```rust
use bevy::prelude::Name;

commands.spawn((
    Name::new("Robot-1"),
    RobotMarker,
));

// Query by name
fn find_by_name(query: Query<(Entity, &Name)>) {
    for (entity, name) in query.iter() {
        if name.as_str() == "Robot-1" {
            // Found it
        }
    }
}
```

## Query Patterns

### Basic Query

```rust
fn process_robots(
    query: Query<(&Position, &Velocity), With<RobotMarker>>,
) {
    for (pos, vel) in query.iter() {
        // Process each robot
    }
}
```

### Mutable Query

```rust
fn update_robots(
    mut query: Query<&mut Position, With<RobotMarker>>,
) {
    for mut pos in query.iter_mut() {
        pos.x += 1.0;
    }
}
```

### Query Filters

```rust
// With - entity must have component
Query<&Position, With<RobotMarker>>

// Without - entity must not have component
Query<&Position, Without<Disabled>>

// Changed - component was modified this frame
Query<&Position, Changed<Position>>

// Added - component was added this frame
Query<&Position, Added<Position>>
```

## Plugin Organization

Organize code into plugins:

```rust
pub struct RobotPlugin;

impl Plugin for RobotPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_robots)
           .add_systems(Update, (
               update_robot_state,
               process_robot_commands,
           ));
    }
}

// In main.rs
app.add_plugins((
    MinimalPlugins,
    RobotPlugin,
    ControlPlugin,
));
```

## Related Skills

- **pl3xus-server**: Server-side pl3xus patterns
- **industrial-systems**: ECS vs traditional frameworks

## Reference

- [ECS Concepts](./references/ecs-concepts.md)
- [Bevy 0.17 Patterns](./references/bevy-0.17-patterns.md)

