# Registration Patterns Analysis

This document analyzes how message registration works in meteorite vs pl3xus, identifies gaps, and proposes a pluggable authorization policy system.

---

## pl3xus Existing Authorization System

pl3xus already has a pluggable authorization policy for **mutations**:

### `MutationAuthorizer` Trait (in `pl3xus_sync/src/registry.rs`)
```rust
/// Pluggable policy for deciding whether a queued mutation is allowed.
pub trait MutationAuthorizer: Send + Sync + 'static {
    fn authorize(&self, ctx: &MutationAuthContext, mutation: &QueuedMutation) -> MutationStatus;
}

/// Resource wrapping the active mutation authorization policy.
#[derive(Resource)]
pub struct MutationAuthorizerResource {
    pub inner: Arc<dyn MutationAuthorizer>,
}
```

### Built-in Policies:
- `ServerOnlyMutationAuthorizer` - Only server can mutate
- `MutationAuthorizerResource::from_fn(|world, mutation| ...)` - Custom closure

### Helper for Hierarchy-Aware Auth:
```rust
pub fn has_control_hierarchical<C, F>(world: &World, entity: Entity, predicate: F) -> bool
where
    C: Component,
    F: Fn(&C) -> bool,
```

This checks if an entity or any ancestor has a control component matching a predicate.

---

## Key Insight: Extend This Pattern to Requests

The `MutationAuthorizer` pattern is exactly what we need for targeted requests!

We should create an analogous:
- `RequestAuthorizer` trait
- `RequestAuthorizerResource`
- Helper for hierarchy-aware request authorization

The `ExclusiveControlPlugin` then becomes just one **implementation** of this policy interface, not the only option.

---

## Meteorite Registration Functions (Reference)

Meteorite bundles multiple registration steps into convenience functions:

| Function | Bundles |
|----------|---------|
| `register_network_message<T>` | Incoming + Outbound |
| `register_authorized_message<T>` | Targeted + Outbound + AuthorizedNetworkData + Middleware |
| `register_subscription_message<T>` | Subscribe/Unsubscribe + Outbound |
| `register_network_request<T>` | Request only (no auth) |

**Note:** `register_subscription_message` is considered **deprecated** in pl3xus since `sync_component` provides a better pattern.

---

## Proposed pl3xus Registration API

### Principle: Eliminate Boilerplate, Default to Correct Behavior

```rust
// COMPLETE registration - includes targeted, authorized, outbound
app.register_message::<ChatMessage, WebSocketProvider, _>(MySchedule::Notify);

// UNSCHEDULED registration - for when you don't need system-set control
app.register_message_unscheduled::<ChatMessage, WebSocketProvider>();
```

### What Each Does Internally:

**`register_message<T, NP, S>(app, system_set)`:**
```rust
pub fn register_message<T, NP, S>(app: &mut App, system_set: S)
where
    T: Pl3xusMessage + Clone + 'static,
    NP: NetworkProvider,
    S: SystemSet + Clone,
{
    // 1. Register incoming (can receive T from wire)
    app.register_network_message::<T, NP>();

    // 2. Register targeted variant (can receive TargetedMessage<T>)
    app.register_targeted_message::<T, NP>();

    // 3. Register outbound (can send via MessageWriter in system set)
    app.register_outbound_message::<T, NP, S>(system_set);

    // 4. Register AuthorizedMessage<T> for downstream handlers
    app.add_message::<AuthorizedMessage<T>>();

    // 5. Add authorization middleware (checks RequestAuthorizerResource)
    app.add_systems(PreUpdate, authorize_targeted_messages::<T, NP>);
}
```

**`register_message_unscheduled<T, NP>(app)`:**
```rust
pub fn register_message_unscheduled<T, NP>(app: &mut App)
where
    T: Pl3xusMessage + Clone + 'static,
    NP: NetworkProvider,
{
    // Just incoming - no outbound, no targeting, no auth
    app.register_network_message::<T, NP>();
}
```

---

## Proposed Authorization Policy System

### New Trait: `TargetedMessageAuthorizer`

```rust
/// Context passed to the authorizer when checking targeted messages.
pub struct TargetedAuthContext<'a> {
    pub world: &'a World,
    pub source: ConnectionId,
    pub target_entity: Entity,
}

/// Pluggable policy for deciding if a client can send to a target entity.
pub trait TargetedMessageAuthorizer: Send + Sync + 'static {
    /// Returns Ok(()) if authorized, Err(reason) if not.
    fn authorize(&self, ctx: &TargetedAuthContext) -> Result<(), String>;
}

/// Resource wrapping the active targeted message authorization policy.
#[derive(Resource)]
pub struct TargetedAuthorizerResource {
    pub inner: Arc<dyn TargetedMessageAuthorizer>,
}

impl TargetedAuthorizerResource {
    /// Create from a closure
    pub fn from_fn<F>(f: F) -> Self
    where
        F: Fn(&World, ConnectionId, Entity) -> Result<(), String> + Send + Sync + 'static,
    { ... }

    /// Allow all targeted messages (no auth check)
    pub fn allow_all() -> Self { ... }

    /// Only server can send targeted messages
    pub fn server_only() -> Self { ... }
}
```

### ExclusiveControlPlugin Implements This Policy

```rust
impl Plugin for ExclusiveControlPlugin {
    fn build(&self, app: &mut App) {
        // ... existing setup ...

        // Register the exclusive control authorization policy
        app.insert_resource(TargetedAuthorizerResource::from_fn(
            |world, source, entity| {
                // Use the existing has_control_hierarchical helper
                if has_control_hierarchical::<EntityControl, _>(
                    world,
                    entity,
                    |control| control.client_id == source || control.client_id.id == 0
                ) {
                    Ok(())
                } else {
                    Err("No control of entity".to_string())
                }
            }
        ));
    }
}
```

### Custom Authorization Policies

Users can build their own policies:

```rust
// Role-based access control
app.insert_resource(TargetedAuthorizerResource::from_fn(
    |world, source, entity| {
        // Check if client has admin role
        if let Some(roles) = world.get_resource::<ClientRoles>() {
            if roles.is_admin(source) {
                return Ok(());
            }
        }

        // Check entity-level permissions
        if let Some(perms) = world.get::<EntityPermissions>(entity) {
            if perms.allowed_clients.contains(&source) {
                return Ok(());
            }
        }

        Err("Access denied".to_string())
    }
));
```

---

## Deprecation: SubscriptionMessage Pattern

The legacy `SubscriptionMessage` pattern (from Eventwork era) is deprecated. Users should use `sync_component` instead:

| Legacy | Modern Replacement |
|--------|-------------------|
| `register_subscription::<T>()` | `sync_component::<T>(config)` |
| Manual Subscribe/Unsubscribe handling | Automatic subscription management |
| User writes all subscription logic | Framework handles sync |

---

## Summary: Complete Registration Types

| Registration Function | Incoming | Targeted | Authorized | Outbound | Use Case |
|----------------------|----------|----------|------------|----------|----------|
| `register_message<T, NP, S>()` | ✅ | ✅ | ✅ | ✅ | Full bidirectional with auth |
| `register_message_unscheduled<T, NP>()` | ✅ | ❌ | ❌ | ❌ | Simple receive-only |
| `register_request<T, NP>()` | ✅ | ❌ | ❌ | ❌ | Request/response (no entity) |
| `register_targeted_request<T, NP>()` | ✅ | ✅ | ✅ | ❌ | Request to specific entity |

Note: `sync_component<T>()` handles its own registration and doesn't need these functions.

