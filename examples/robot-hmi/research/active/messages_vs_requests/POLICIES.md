# Authorization Policies Deep Dive

## Current Policy Types

### EntityAccessPolicy (for targeted messages)
Controls which clients can send messages TO specific entities.

```rust
pub struct EntityAccessPolicy {
    inner: Arc<dyn EntityAccessAuthorizer>,
}

pub trait EntityAccessAuthorizer: Send + Sync + 'static {
    fn authorize(&self, ctx: &EntityAccessContext) -> AuthResult;
}

// Context provides: world, source (ConnectionId), entity (Entity)
```

Built-in policies:
- `EntityAccessPolicy::allow_all()` - No check
- `EntityAccessPolicy::server_only()` - Only server can send
- `EntityAccessPolicy::from_fn(|world, source, entity| ...)` - Custom

### MessageAccessPolicy (for non-targeted messages)
Controls which clients can send specific message TYPES.

```rust
pub struct MessageAccessPolicy {
    inner: Arc<dyn MessageAccessAuthorizer>,
}

pub trait MessageAccessAuthorizer: Send + Sync + 'static {
    fn authorize(&self, ctx: &MessageAccessContext) -> AuthResult;
}

// Context provides: world, source (ConnectionId)
```

Built-in policies:
- `MessageAccessPolicy::allow_all()` - No check
- `MessageAccessPolicy::server_only()` - Only server can send
- `MessageAccessPolicy::from_fn(|world, source| ...)` - Custom

## Current Builder Methods

```rust
// For targeted messages
app.message::<T, NP>()
   .targeted()
   .with_default_entity_access()     // Use DefaultEntityAccessPolicy resource
   .register();

app.message::<T, NP>()
   .targeted()
   .with_entity_access(custom_policy)  // Use custom policy
   .register();

// For non-targeted messages
app.message::<T, NP>()
   .with_message_access(custom_policy)
   .register();
```

## The Problem

The API is asymmetric:
- `with_default_entity_access()` exists but `with_default_message_access()` doesn't
- No equivalent of `DefaultMessageAccessPolicy` resource

And for requests, we need the SAME policies but the current request API has none:
```rust
// Current: no policy support
app.listen_for_request_message::<R, WebSocketProvider>();
```

## Proposed Unified Policy API

### Symmetric Policy Resources

```rust
// Already exists
#[derive(Resource)]
pub struct DefaultEntityAccessPolicy(pub EntityAccessPolicy);

// NEW: Should exist for symmetry
#[derive(Resource)]
pub struct DefaultMessageAccessPolicy(pub MessageAccessPolicy);
```

### Unified Builder Methods

For both messages AND requests:

```rust
// Option A: Separate methods (current pattern)
.with_entity_access(policy)          // Custom entity policy
.with_default_entity_access()        // Use default entity policy
.with_message_access(policy)         // Custom message policy
.with_default_message_access()       // Use default message policy (NEW)

// Option B: Single method with enum (alternative)
.with_policy(AccessPolicy::Entity(policy))
.with_policy(AccessPolicy::DefaultEntity)
.with_policy(AccessPolicy::Message(policy))
.with_policy(AccessPolicy::DefaultMessage)
```

I prefer **Option A** because:
1. Type-safe: can't apply MessageAccessPolicy to a targeted message
2. Explicit: clear what kind of policy you're setting
3. Composable: could add both entity AND message policy if needed

### Full Request Builder API

```rust
// Non-targeted request (no entity)
app.request::<ListPrograms, NP>()
   .register();  // No auth, anyone can call

app.request::<AdminSettings, NP>()
   .with_message_access(MessageAccessPolicy::server_only())
   .register();  // Only server can call

app.request::<AdminSettings, NP>()
   .with_default_message_access()  // Use DefaultMessageAccessPolicy
   .register();

// Targeted request (entity-specific)
app.request::<SetSpeedOverride, NP>()
   .targeted()
   .with_default_entity_access()   // Use DefaultEntityAccessPolicy
   .register();

app.request::<SpecialCommand, NP>()
   .targeted()
   .with_entity_access(EntityAccessPolicy::from_fn(|world, source, entity| {
       // Custom logic: check role, ownership, etc.
       Ok(())
   }))
   .register();
```

## Common Policy Patterns

### 1. Exclusive Control (Default)
```rust
// Already provided by ExclusiveControlPlugin
EntityAccessPolicy::from_fn(|world, source, entity| {
    check_entity_control(world, source, entity)
})
```

### 2. Role-Based Access
```rust
EntityAccessPolicy::from_fn(|world, source, entity| {
    let roles = world.get_resource::<ClientRoles>()?;
    if roles.has_role(source, "admin") {
        Ok(())
    } else {
        Err("Admin role required")
    }
})
```

### 3. Owner-Only
```rust
EntityAccessPolicy::from_fn(|world, source, entity| {
    let owner = world.get::<Owner>(entity)?;
    if owner.0 == source {
        Ok(())
    } else {
        Err("Not the owner")
    }
})
```

### 4. Read-Only Entities
```rust
EntityAccessPolicy::from_fn(|world, source, entity| {
    if world.get::<ReadOnly>(entity).is_some() {
        Err("Entity is read-only")
    } else {
        Ok(())
    }
})
```

## Implementation Checklist

- [ ] Add `DefaultMessageAccessPolicy` resource
- [ ] Add `.with_default_message_access()` to message builder
- [ ] Create `RequestRegistration` builder (parallel to `MessageRegistration`)
- [ ] Add `.targeted()` to request builder
- [ ] Add `.with_entity_access(policy)` to request builder
- [ ] Add `.with_default_entity_access()` to request builder
- [ ] Add `.with_message_access(policy)` to request builder
- [ ] Add `.with_default_message_access()` to request builder
- [ ] Create `AuthorizedRequest<R>` handler type
- [ ] Add authorization middleware for targeted requests
- [ ] Add `use_targeted_request()` client hook

