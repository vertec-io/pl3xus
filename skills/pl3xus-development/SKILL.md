---
name: pl3xus-development
description: Complete end-to-end workflow for building production-grade industrial applications with pl3xus (Bevy ECS server + Leptos WASM client). Use when starting a new pl3xus project or implementing major features.
allowed-tools:
  - view
  - codebase-retrieval
  - save-file
  - str-replace-editor
  - launch-process
  - read-process
  - add_tasks
  - update_tasks
  - view_tasklist
---

# pl3xus Development Skill

> **⚠️ CRITICAL: Before developing, review `../SKILLS_REGISTRY.md` for mandatory patterns.**

## Purpose

This skill provides a complete workflow for building production-grade industrial applications using the pl3xus framework. It covers architecture, server implementation, client implementation, and integration.

## When to Use

Use this skill when:
- Starting a new pl3xus application
- Implementing a major feature end-to-end
- Need guidance on production patterns
- Unsure which specific skill to use

## Architecture Overview

pl3xus is a **server-authoritative** framework for real-time industrial applications:

```
┌─────────────────────────────────────────────────────────────┐
│                     Bevy ECS Server                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │ Components  │  │  Systems    │  │  Message Handlers   │  │
│  │ (Position,  │  │ (physics,   │  │ (requests, queries, │  │
│  │  Velocity)  │  │  control)   │  │  mutations)         │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
│                         │                                    │
│                    pl3xus_sync                               │
│              (component sync, auth)                          │
└─────────────────────────┬───────────────────────────────────┘
                          │ WebSocket (bincode)
┌─────────────────────────┴───────────────────────────────────┐
│                   Leptos WASM Client                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   Hooks     │  │ Components  │  │     Contexts        │  │
│  │ (use_entity │  │ (reactive   │  │ (SyncProvider,      │  │
│  │  _component)│  │  views)     │  │  EntityContext)     │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Workflow

### Phase 1: Project Structure

Choose a project structure pattern. See [pl3xus-project-structure skill](../pl3xus-project-structure/SKILL.md) for details.

**Two supported patterns:**

1. **Shared Types Crate** (simpler) - Separate `shared/` crate for types
2. **Plugin-Based** (modular) - Feature-gated types in `plugins/` crate

To detect existing pattern in a project:
```bash
# Plugin-based if this returns results:
grep -r "feature = \"ecs\"" plugins/src/ 2>/dev/null

# Shared types if this exists:
ls shared/src/lib.rs types/src/lib.rs 2>/dev/null
```

### Phase 2: Shared Types

Define types in `shared/` that both server and client use:

```rust
// shared/src/components.rs
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Position {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

// shared/src/requests.rs
use pl3xus_common::RequestMessage;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdatePosition {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdatePositionResponse {
    pub success: bool,
    pub error: Option<String>,
}

impl RequestMessage for UpdatePosition {
    type ResponseMessage = UpdatePositionResponse;
}
```

### Phase 3: Server Implementation

See [pl3xus-server skill](../pl3xus-server/SKILL.md) for detailed patterns.

**Key patterns:**
```rust
// main.rs - Plugin-based organization
app.add_plugins((
    Pl3xusSyncPlugin::<WebSocketProvider>::default(),
    RobotPlugin,
    ControlPlugin,
))
.sync_component::<Position>(None)
.request::<UpdatePosition, WebSocketProvider>()
    .targeted()
    .with_default_entity_policy()
    .register();
```

### Phase 4: Client Implementation

See [pl3xus-client skill](../pl3xus-client/SKILL.md) for detailed patterns.

**Key patterns:**
```rust
// Use entity-specific hooks for multi-entity scenarios
let position = use_entity_component::<Position>(robot_id);

// Use targeted mutations with handlers
let update = use_mutation_targeted::<UpdatePosition>(|result| {
    match result {
        Ok(r) if r.success => log::info!("Updated"),
        Ok(r) => log::error!("Failed: {:?}", r.error),
        Err(e) => log::error!("Error: {e}"),
    }
});
```

## Production Patterns (CRITICAL)

> **See `../SKILLS_REGISTRY.md` for complete pattern reference.**

### ✅ ALWAYS Use

| Pattern | Why |
|---------|-----|
| `.targeted().with_default_entity_policy().register()` | Required for entity-targeted requests |
| `app.requests::<(A, B, C), WS>()` batch registration | Consistent config, less boilerplate |
| `use_entity_component::<T>(id)` | Guaranteed entity identity |
| `MessageReader`/`MessageWriter` | Bevy 0.17 standard |
| Server-driven `can_*` flags | Server is authoritative |
| Synced components with mutation handlers | Preferred over separate request types |

### ❌ NEVER Use

| Pattern | Why |
|---------|-----|
| `use_components().values().next()` | No entity guarantee |
| `<input type="number">` | Browser inconsistencies |
| Client-side state logic | Server is authoritative |
| `EventReader`/`EventWriter` | Deprecated in Bevy 0.17 |
| Individual `.request::<T>().register()` | Use batch registration |

### Synced Components with Mutation Handlers (PREFERRED)

When a component value change requires side effects (sending to IO, device commands):

```rust
// Server: Sync component + register mutation
app.sync_component::<RobotConfig>(None);
app.request::<UpdateRobotConfig, WebSocketProvider>()
    .targeted()
    .with_default_entity_policy()
    .register();

// Handler: Mutate component + trigger side effects
fn handle_update_config(
    mut messages: MessageReader<NetworkData<TargetedRequest<UpdateRobotConfig>>>,
    mut configs: Query<&mut RobotConfig>,
    mut commands: MessageWriter<RobotCommand>,
) {
    for request in messages.read() {
        let entity = Entity::from_bits(request.message.target_entity);
        if let Ok(mut config) = configs.get_mut(entity) {
            *config = request.message.request.config.clone();
            commands.write(RobotCommand::ApplyConfig(config.clone()));
            let _ = request.respond(UpdateConfigResponse { success: true, error: None });
        }
    }
}
```

## Related Skills

- **SKILLS_REGISTRY.md**: Complete skill registry with mandatory patterns (START HERE)
- **pl3xus-project-structure**: Project organization and plugin structure patterns
- **pl3xus-server**: Server-side patterns in depth
- **pl3xus-client**: Client-side patterns in depth
- **pl3xus-queries**: Request/response patterns
- **pl3xus-mutations**: Mutation and invalidation patterns
- **pl3xus-authorization**: Entity policies and control

## Reference

- [Common Patterns](./references/common-patterns.md)
- [SKILLS_REGISTRY.md](../SKILLS_REGISTRY.md) - Agent-optimized pattern reference

