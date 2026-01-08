# pl3xus Skills Registry

> **Agent Reference**: This document maps all available pl3xus APIs, their purposes, and when to use each.
> The goal is to help agents choose the **most appropriate pattern** for each use case.

## Core Principle: Server-Authoritative Architecture

pl3xus is a server-authoritative framework. The server owns all state; clients display and request changes.

- **Components** are ECS state that syncs automatically from server → clients
- **Messages** are fire-and-forget (no response expected)
- **Requests** are request/response pairs (client expects a reply)
- **Targeting** means the operation is directed at a specific entity
- **Authorization** means the server validates the client has permission

---

## Server-Side Registration Decision Tree

### 1. Message vs Request?

| Need a response from server? | Use |
|------------------------------|-----|
| **No** - fire-and-forget (streaming commands, events) | `app.message::<T, NP>()` |
| **Yes** - client expects a reply | `app.request::<T, NP>()` |

### 2. Targeted vs Non-Targeted?

| Is operation directed at a specific entity? | Add |
|---------------------------------------------|-----|
| **No** - global operation (ListPrograms, GetSettings) | `.register()` |
| **Yes** - entity-specific (SetSpeedOverride for Robot #3) | `.targeted().register()` |

### 3. Needs Authorization?

| Does server need to verify client permission? | Add |
|-----------------------------------------------|-----|
| **No** - anyone can call (read-only queries, public info) | `.register()` |
| **Yes** - must have control/permission | `.with_default_entity_policy().register()` or custom policy |

---

## Server Registration API Reference

### Messages (fire-and-forget, no response)

```rust
// Simple broadcast message - anyone can send, no targeting
app.message::<Ping, NP>().register();

// Targeted message without auth - anyone can query any entity
// Use for: read-only entity queries, public entity info
app.message::<GetEntityInfo, NP>()
   .targeted()
   .register();

// Targeted message WITH auth - must have control of entity
// Use for: control commands (jog, motion), state changes
app.message::<JogCommand, NP>()
   .targeted()
   .with_default_entity_policy()
   .register();

// Non-targeted WITH auth - role-based or custom check
// Use for: admin commands, role-restricted operations
app.message::<AdminCommand, NP>()
   .with_message_policy(RolePolicy::admin_only())
   .register();
```

### Requests (request/response)

```rust
// Simple non-targeted request - anyone can call
// Use for: global queries (ListPrograms), config fetches
app.request::<ListPrograms, NP>().register();

// Targeted request without auth - query entity without control
// Use for: reading entity-specific data anyone can see
app.request::<ReadGin, NP>()
   .targeted()
   .register();

// Targeted request WITH auth - must have control
// Use for: entity state changes, mutations requiring permission
app.request::<SetSpeedOverride, NP>()
   .targeted()
   .with_default_entity_policy()
   .register();

// Batch registration - register multiple with same config
// Use for: related operations (program lifecycle, CRUD sets)
app.requests::<(
    StartProgram,
    PauseProgram,
    ResumeProgram,
    StopProgram,
), NP>()
    .targeted()
    .with_default_entity_policy()
    .with_error_response();  // Auto-respond on auth failure
```

### Component Synchronization

```rust
// Basic sync - server changes auto-push to clients
// Clients can mutate (default behavior)
app.sync_component::<Position>(None);

// Read-only - clients cannot mutate
// Use for: server-computed state, sensor readings
app.sync_component_builder::<RobotStatus>()
    .read_only()
    .build();

// With mutation handler - server validates/processes changes
// Use for: when mutations need validation or side effects
app.sync_component_builder::<JogSettings>()
    .with_handler::<NP, _, _>(handle_jog_settings_mutation)
    .build();

// With handler + authorization - must have control to mutate
// Use for: controlled components requiring entity permission
app.sync_component_builder::<FrameToolData>()
    .with_handler::<NP, _, _>(handle_frame_tool_mutation)
    .targeted()
    .with_default_entity_policy()
    .build();
```

---

## Client-Side Hooks Decision Tree

### 1. What are you accessing?

| Data Type | Category |
|-----------|----------|
| Real-time ECS component data | Component Hooks |
| One-time data fetch | Query Hooks |
| State-changing operation | Mutation Hooks |
| Broadcast messages | Message Hooks |
| Fire-and-forget commands | Send Hooks |

### 2. Component Hooks - Which one?

| Scenario | Hook |
|----------|------|
| All entities with component | `use_components::<T>()` → `HashMap<u64, T>` |
| Filtered entities (client-side) | `use_components_where::<T, _>(filter)` |
| Single entity, static ID | `use_entity::<T>(id)` → `Option<T>` |
| Single entity, reactive ID | `use_entity_reactive::<T, _>(\|\| Some(id))` |
| Single entity with existence check | `use_entity_component::<T, _>(\|\| Some(id))` → `(T, bool)` |
| With mutation capability | `use_mut_component::<T, _>(\|\| Some(id))` → `MutComponentHandle` |
| Fine-grained reactivity (stores) | `use_component_store::<T>()` or `use_entity_component_store` |

### 3. Query Hooks (read operations)

| Scenario | Hook |
|----------|------|
| Fetch with caching + auto-invalidation | `use_query(request)` → `QueryHandle` |
| Manual fetch control | `use_request::<R>()` → `(send_fn, state)` |
| Fetch with response handler | `use_request_with_handler::<R, _>(handler)` |
| Fetch with keyed cache | `use_keyed_query(key, request_fn)` |

### 4. Mutation Hooks (write operations)

| Scenario | Hook |
|----------|------|
| Non-targeted mutation | `use_mutation::<R>(handler)` → `MutationHandle` |
| Entity-targeted mutation | `use_mutation_targeted::<R>(handler)` → `TargetedMutationHandle` |
| Targeted request (low-level) | `use_targeted_request::<R>()` |

### 5. Other Hooks

| Scenario | Hook |
|----------|------|
| Broadcast messages from server | `use_message::<T>()` |
| Send targeted message (no response) | `use_send_targeted::<T>(entity_bits)` |
| WebSocket connection control | `use_connection()` |
| Mutation state tracking | `use_mutations()` |
| Query cache management | `use_query_client()` |

---

## Client-Side Hook Examples

### Component Access Patterns

```rust
// All robots - for lists, iteration
let robots = use_components::<RobotStatus>();
// → HashMap<u64, RobotStatus>

// Single robot - when you know the ID
let (status, exists) = use_entity_component::<RobotStatus, _>(move || Some(robot_id));
// → (RobotStatus, bool)

// Selected robot - reactive to selection changes
let selected_id: RwSignal<Option<u64>> = ...;
let status = use_entity_reactive::<RobotStatus, _>(move || selected_id.get());
// → Option<RobotStatus>

// With mutation - when you need to change component values
let handle = use_mut_component::<JogSettings, _>(move || Some(robot_id));
handle.mutate(new_settings);
// → MutComponentHandle { value, exists, mutation_state, mutate() }
```

### Query Patterns

```rust
// Cached query with auto-refetch on invalidation
let programs = use_query(ListPrograms);
// programs.data(), programs.is_loading(), programs.refetch()

// Keyed query (different cache per key)
let program = use_keyed_query(
    move || program_id.get(),
    |id| GetProgram { id },
);
```

### Mutation Patterns

```rust
// Non-targeted - global operation
let create = use_mutation::<CreateProgram>(move |result| {
    match result {
        Ok(r) if r.success => toast.success("Created!"),
        Ok(r) => toast.error(r.error.unwrap_or_default()),
        Err(e) => toast.error(e),
    }
});
create.send(CreateProgram { name: "test".into() });

// Targeted - entity-specific operation
let set_speed = use_mutation_targeted::<SetSpeedOverride>(move |result| {
    match result {
        Ok(r) if r.success => toast.success("Speed set"),
        Ok(r) => toast.error(r.error.unwrap_or_default()),
        Err(e) => toast.error(e),
    }
});
set_speed.send(robot_id, SetSpeedOverride { value: 50.0 });

// Fire-and-forget targeted (no response)
let send_jog = use_send_targeted::<JogCommand>(robot_id);
send_jog(JogCommand { axis: 0, direction: 1 });
```

---

## Authorization Deep Dive

Authorization in pl3xus is handled through **policies** that determine whether a client can perform an action. For **targeted** operations (directed at a specific entity), use **EntityAccessPolicy**. For **non-targeted** operations (global), use **MessageAccessPolicy**.

### ExclusiveControlPlugin and EntityControl

pl3xus provides a built-in authorization system via the `ExclusiveControlPlugin`. This plugin:

1. **Manages entity control** - Clients can take/release control of entities
2. **Installs a default policy** - The `DefaultEntityAccessPolicy` resource
3. **Handles hierarchy** - Control of a parent entity grants control of children
4. **Times out inactive clients** - Configurable automatic release

**Setup:**
```rust
use pl3xus_sync::control::ExclusiveControlPlugin;

app.add_plugins(
    ExclusiveControlPlugin::builder()
        .timeout_seconds(300.0)         // 5 min timeout (default: 30 min)
        .propagate_to_children(true)    // Parent control → child control
        .build::<WebSocketProvider>()
);
```

**EntityControl Component:**
```rust
use pl3xus_sync::EntityControl;

// Spawn entity with control tracking
commands.spawn((
    Robot { name: "Robot-1".into() },
    EntityControl::default(),  // No one controls initially (client_id = 0)
));

// EntityControl fields:
pub struct EntityControl {
    pub client_id: ConnectionId,        // Who has control (0 = no one)
    pub sub_connection_ids: Vec<ConnectionId>, // Related connections (tabs)
    pub last_activity: f32,             // For timeout detection
}
```

**Control Flow:**
1. Client sends `ControlRequest::Take(entity_bits)` to take control
2. Server grants control if entity is available (client_id == 0)
3. Server updates `EntityControl.client_id` to the requesting client
4. Client can now send targeted messages/requests to that entity
5. Client sends `ControlRequest::Release(entity_bits)` when done

### Default Entity Policy: `with_default_entity_policy()`

The shortcut `.with_default_entity_policy()` uses the policy installed by `ExclusiveControlPlugin`:

```rust
// This...
app.request::<SetSpeed, NP>()
    .targeted()
    .with_default_entity_policy()
    .register();

// ...is equivalent to:
app.request::<SetSpeed, NP>()
    .targeted()
    .with_entity_policy(ExclusiveControlPlugin::<NP>::control_based_policy())
    .register();
```

**What the default policy checks:**
1. Is the source the server? → Always authorized
2. Does the target entity exist?
3. Does the entity have an `EntityControl` component?
4. Does `EntityControl.client_id` match the requesting client?
5. Is the client in `EntityControl.sub_connection_ids`?
6. If hierarchy is enabled: Does any ancestor satisfy the above?

**Denied responses:**
- "No client has control of this entity. Take control first." (client_id == 0)
- "Entity controlled by client X (you are Y)" (different client)
- "Entity has no control component" (missing EntityControl)

### Custom Entity Policies: `with_entity_policy()`

For operations that need different authorization logic, use custom policies:

```rust
use pl3xus_sync::{EntityAccessPolicy, AuthResult};

// Admin-only access
app.request::<AdminCommand, NP>()
    .targeted()
    .with_entity_policy(EntityAccessPolicy::from_fn(|world, source, entity| {
        let admins = world.resource::<AdminList>();
        if admins.contains(&source) {
            Ok(())
        } else {
            Err("Admin access required".into())
        }
    }))
    .register();

// Role-based access
app.request::<SupervisorCommand, NP>()
    .targeted()
    .with_entity_policy(EntityAccessPolicy::from_fn(|world, source, entity| {
        let roles = world.resource::<UserRoles>();
        match roles.get(&source) {
            Some(Role::Supervisor) | Some(Role::Admin) => Ok(()),
            _ => Err("Supervisor or Admin role required".into()),
        }
    }))
    .register();

// Check entity state + control
app.request::<WriteValue, NP>()
    .targeted()
    .with_entity_policy(EntityAccessPolicy::from_fn(|world, source, entity| {
        // Check control
        if !has_hierarchical_control::<EntityControl, _>(
            world, entity,
            |control| control.has_control(source)
        ) {
            return Err("No control of entity".into());
        }

        // Check entity is not locked
        if let Some(state) = world.get::<RobotState>(entity) {
            if state.is_locked {
                return Err("Entity is locked".into());
            }
        }

        Ok(())
    }))
    .register();

// Allow all (for read-only targeted queries)
app.request::<GetEntityInfo, NP>()
    .targeted()
    .with_entity_policy(EntityAccessPolicy::allow_all())
    .register();

// Server only
app.request::<InternalSync, NP>()
    .targeted()
    .with_entity_policy(EntityAccessPolicy::server_only())
    .register();
```

### Policy Priority

The authorization middleware checks policies in this order:

1. **Per-message policy** (set with `.with_entity_policy(policy)`)
2. **Default policy** (if `.with_default_entity_policy()` was called)
3. **No authorization** (if neither was set)

```rust
// Per-message policy takes precedence
app.request::<SpecialCommand, NP>()
    .targeted()
    .with_entity_policy(custom_policy)    // ← This is used
    .with_default_entity_policy()          // ← Ignored
    .register();
```

### Non-Targeted Authorization: MessageAccessPolicy

For global operations (not entity-targeted), use `MessageAccessPolicy`:

```rust
use pl3xus_sync::MessageAccessPolicy;

// Only authenticated users
app.request::<ListSecrets, NP>()
    .with_message_policy(MessageAccessPolicy::from_fn(|world, source| {
        let auth = world.resource::<AuthenticatedClients>();
        if auth.is_authenticated(source) {
            Ok(())
        } else {
            Err("Authentication required".into())
        }
    }))
    .register();

// Server-only (internal operations)
app.message::<InternalSync, NP>()
    .with_message_policy(MessageAccessPolicy::server_only())
    .register();

// Use default message policy
app.request::<ProtectedQuery, NP>()
    .with_default_message_policy()  // Uses DefaultMessageAccessPolicy resource
    .register();
```

### Hierarchical Control

When `propagate_to_children` is enabled (default), controlling a parent grants control of all children:

```rust
// Entity hierarchy
// System (controlled by Client A)
//   ├── Robot-1
//   └── Robot-2

// Client A can send commands to System, Robot-1, and Robot-2

// Custom hierarchical check
use pl3xus_sync::has_control_hierarchical;

let authorized = has_control_hierarchical::<EntityControl, _>(
    world,
    child_entity,
    |control| control.has_control(client_id)
);
```

### Authorization Decision Guide

| Scenario | Policy to Use |
|----------|---------------|
| Entity mutation, requires control | `.with_default_entity_policy()` |
| Entity query, anyone can read | `.targeted()` only (no policy) |
| Entity operation, admin only | `.with_entity_policy(admin_policy)` |
| Entity operation, check entity state | `.with_entity_policy(custom_fn)` |
| Global operation, authenticated only | `.with_message_policy(auth_policy)` |
| Global operation, server only | `.with_message_policy(MessageAccessPolicy::server_only())` |
| Global operation, anyone | `.register()` (no policy) |

### Reference Documentation

For more details, see:
- **Skill**: `pl3xus-authorization` - Authorization patterns and examples
- **Guide**: `docs/core/guides/authorization.md` - Authorization flow and best practices
- **Guide**: `docs/core/guides/entity-control.md` - EntityControl patterns
- **API**: `crates/pl3xus_sync/src/authorization.rs` - Policy types and registration
- **API**: `crates/pl3xus_sync/src/control.rs` - ExclusiveControlPlugin implementation

---

## Pattern Selection Guide

### When to use Synced Components vs Requests

| Scenario | Use |
|----------|-----|
| Real-time state that all clients need | Synced component |
| Client needs to mutate with validation | Synced component + mutation handler |
| One-time fetch (lists, config) | Request/Query |
| CRUD operations | Request |
| High-frequency streaming commands | Targeted message (no response) |

### When to use Authorization

| Scenario | Authorization |
|----------|---------------|
| Read-only query anyone can call | None |
| Global settings anyone can read | None |
| Entity control commands (jog, motion) | `.with_default_entity_policy()` |
| Entity state mutations | `.with_default_entity_policy()` |
| Role-restricted operations | Custom message policy |
| Admin-only commands | Custom policy |

### When to use Batch Registration

| Scenario | Recommendation |
|----------|----------------|
| Related operations (CRUD for one resource) | Batch: `app.requests::<(Create, Read, Update, Delete), NP>()` |
| Lifecycle operations (Start, Pause, Stop) | Batch with same auth config |
| Unrelated operations | Individual registration is fine |

---

## Skill Index

| Skill | Purpose | Use When |
|-------|---------|----------|
| `pl3xus-development` | End-to-end workflow | Starting projects, major features |
| `pl3xus-project-structure` | Project organization | New projects or plugins, structure decisions |
| `pl3xus-server` | Server-side patterns | Server implementation |
| `pl3xus-client` | Client-side patterns | UI implementation |
| `pl3xus-queries` | Request/response patterns | Read operations |
| `pl3xus-mutations` | State-changing operations | Write operations |
| `pl3xus-authorization` | Entity policies | Multi-user, permissions |
| `bevy-ecs` | Bevy ECS fundamentals | Server systems |
| `leptos-ui` | Leptos UI patterns | Client components |

---

## Reference Examples

| Pattern | Example Location |
|---------|------------------|
| Non-targeted requests | `examples/robot-hmi/server/src/plugins/requests.rs` |
| Targeted messages with auth | `examples/robot-hmi/server/src/plugins/sync.rs` |
| Targeted requests with auth | `examples/robot-hmi/server/src/plugins/sync.rs` |
| Component mutation handlers | `examples/robot-hmi-advanced/` |
| Client hooks | `examples/robot-hmi/app/src/` |
| Plugin module structure | `examples/meteorite/plugins/microwave/` |

