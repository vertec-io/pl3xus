# Implementation Plan: Targeted Requests with Authorization

This document provides a step-by-step implementation guide for adding targeted request authorization to pl3xus.

## Executive Summary

After reviewing meteorite's registration patterns and pl3xus's existing `MutationAuthorizer` system, we propose:

1. **Pluggable authorization policy** - `TargetedMessageAuthorizer` trait (mirrors `MutationAuthorizer`)
2. **Convenience registration functions** - `register_message<T, NP, S>()` with opt-in scheduling
3. **ExclusiveControlPlugin enhancement** - Implements the authorization policy for EntityControl
4. **Deprecate SubscriptionMessage** - `sync_component` is the better pattern

## Core Design Principles

1. **Opt-in scheduling** - `register_message` requires system set, `register_message_unscheduled` doesn't
2. **Policy-based auth** - Users can implement their own authorization, ExclusiveControl is just one policy
3. **Eliminate boilerplate** - One call registers everything correctly
4. **Play nice with sync_component** - These are complementary, not competing patterns

## Scope

The implementation spans four crates:
1. **pl3xus_common** - Wire format types (`TargetedRequest<R>`, `AuthorizedMessage<T>`)
2. **pl3xus** - Registration helpers + authorization middleware
3. **pl3xus_sync** - `TargetedMessageAuthorizer` trait + ExclusiveControl policy implementation
4. **pl3xus_client** - Client hook for targeted requests

Estimated effort: **3-4 sessions**

---

## Phase 0: Authorization Policy Trait (Foundation)

### Step 0.1: Add TargetedMessageAuthorizer trait

**File:** `crates/pl3xus_sync/src/authorization.rs` (new file)

```rust
//! Pluggable authorization policies for targeted messages.
//!
//! This mirrors the MutationAuthorizer pattern but for targeted network messages.

use bevy::prelude::*;
use pl3xus_common::ConnectionId;
use std::sync::Arc;

/// Context passed to the authorizer when checking targeted messages.
pub struct TargetedAuthContext<'a> {
    pub world: &'a World,
    pub source: ConnectionId,
    pub target_entity: Entity,
}

/// Pluggable policy for deciding if a client can send to a target entity.
///
/// Implementations can inspect arbitrary application state via the world reference.
pub trait TargetedMessageAuthorizer: Send + Sync + 'static {
    /// Returns Ok(()) if authorized, Err(reason) if not.
    fn authorize(&self, ctx: &TargetedAuthContext) -> Result<(), String>;
}

/// Resource wrapping the active targeted message authorization policy.
///
/// If this resource is not present, all targeted messages are allowed.
#[derive(Resource)]
pub struct TargetedAuthorizerResource {
    pub inner: Arc<dyn TargetedMessageAuthorizer>,
}

impl TargetedAuthorizerResource {
    /// Create from a closure (most common usage).
    pub fn from_fn<F>(f: F) -> Self
    where
        F: Fn(&World, ConnectionId, Entity) -> Result<(), String> + Send + Sync + 'static,
    {
        struct ClosureAuthorizer<F>(F);
        impl<F> TargetedMessageAuthorizer for ClosureAuthorizer<F>
        where
            F: Fn(&World, ConnectionId, Entity) -> Result<(), String> + Send + Sync + 'static,
        {
            fn authorize(&self, ctx: &TargetedAuthContext) -> Result<(), String> {
                (self.0)(ctx.world, ctx.source, ctx.target_entity)
            }
        }
        Self { inner: Arc::new(ClosureAuthorizer(f)) }
    }

    /// Allow all targeted messages (no authorization check).
    pub fn allow_all() -> Self {
        Self::from_fn(|_, _, _| Ok(()))
    }

    /// Only server can send targeted messages.
    pub fn server_only() -> Self {
        Self::from_fn(|_, source, _| {
            if source.is_server() { Ok(()) }
            else { Err("Only server can send targeted messages".to_string()) }
        })
    }
}
```

### Step 0.2: ExclusiveControlPlugin implements the policy

**File:** `crates/pl3xus_sync/src/control.rs` (modify)

```rust
// In ExclusiveControlPlugin::build()
impl Plugin for ExclusiveControlPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.config.clone());
        app.add_message::<ControlRequest>();
        app.add_message::<ControlResponse>();

        // NEW: Install the exclusive control authorization policy
        let propagate = self.config.propagate_to_children;
        app.insert_resource(TargetedAuthorizerResource::from_fn(
            move |world, source, entity| {
                exclusive_control_check(world, source, entity, propagate)
            }
        ));
    }
}

fn exclusive_control_check(
    world: &World,
    source: ConnectionId,
    entity: Entity,
    check_hierarchy: bool,
) -> Result<(), String> {
    if source.is_server() {
        return Ok(()); // Server always authorized
    }

    let check = |control: &EntityControl| {
        control.client_id == source || control.client_id.id == 0
    };

    let authorized = if check_hierarchy {
        has_control_hierarchical::<EntityControl, _>(world, entity, check)
    } else {
        world.get::<EntityControl>(entity).map(check).unwrap_or(true)
    };

    if authorized {
        Ok(())
    } else {
        Err("No control of entity".to_string())
    }
}
```

---

## Phase 1: Registration Convenience Functions

### Step 1.1: Create registration module

**File:** `crates/pl3xus/src/managers/registration.rs` (new file)

```rust
//! Convenience registration functions for network messages.
//!
//! These bundle multiple registration steps into single calls,
//! eliminating boilerplate and ensuring correct setup.

use bevy::prelude::*;
use pl3xus_common::Pl3xusMessage;
use crate::{NetworkProvider, AppNetworkMessage};

/// Register a complete bidirectional message with system-set controlled sending.
///
/// Bundles:
/// - `register_network_message` (incoming)
/// - `register_targeted_message` (targeted incoming)
/// - `register_outbound_message` (outgoing in system set)
/// - `AuthorizedMessage<T>` event type
/// - Authorization middleware (if TargetedAuthorizerResource exists)
pub fn register_message<T, NP, S>(app: &mut App, system_set: S)
where
    T: Pl3xusMessage + Clone + 'static,
    NP: NetworkProvider,
    S: SystemSet + Clone,
{
    // Incoming (plain)
    app.register_network_message::<T, NP>();

    // Incoming (targeted)
    app.register_targeted_message::<T, NP>();

    // Outbound (system-set controlled)
    app.register_outbound_message::<T, NP, S>(system_set);

    // Authorized message event
    app.add_message::<AuthorizedMessage<T>>();

    // Authorization middleware
    app.add_systems(PreUpdate, authorize_targeted_messages::<T, NP>);
}

/// Register an incoming-only message (no outbound, no targeting, no auth).
///
/// Use this when you only need to receive messages, not send them.
pub fn register_message_unscheduled<T, NP>(app: &mut App)
where
    T: Pl3xusMessage + Clone + 'static,
    NP: NetworkProvider,
{
    app.register_network_message::<T, NP>();
}
```

### Step 1.2: AuthorizedMessage and middleware

**File:** `crates/pl3xus/src/managers/registration.rs` (continued)

```rust
/// A targeted message that has passed authorization.
///
/// Systems should read this instead of `NetworkData<TargetedMessage<T>>`.
#[derive(Message, Debug, Clone)]
pub struct AuthorizedMessage<T: Pl3xusMessage> {
    pub message: T,
    pub source: ConnectionId,
    pub target_entity: Entity,
}

fn authorize_targeted_messages<T, NP>(
    mut incoming: MessageReader<NetworkData<TargetedMessage<T>>>,
    auth_res: Option<Res<TargetedAuthorizerResource>>,
    mut authorized: MessageWriter<AuthorizedMessage<T>>,
    world: &World,
)
where
    T: Pl3xusMessage + Clone + 'static,
    NP: NetworkProvider,
{
    for msg in incoming.read() {
        let source = *msg.source();
        let entity = Entity::from_bits(msg.target_entity_bits);

        // If no authorizer resource, allow all
        let is_authorized = match &auth_res {
            Some(res) => {
                let ctx = TargetedAuthContext { world, source, target_entity: entity };
                res.inner.authorize(&ctx).is_ok()
            }
            None => true,
        };

        if is_authorized {
            authorized.write(AuthorizedMessage {
                message: msg.inner.message.clone(),
                source,
                target_entity: entity,
            });
        }
        // If not authorized, silently drop (or log/send error in future)
    }
}
```

---

## Phase 2: Wire Format (pl3xus_common)

### Step 1.1: Add TargetedRequest type

**File:** `crates/pl3xus_common/src/messages.rs`

```rust
/// Wire format for entity-targeted requests with correlation ID.
/// Used for requests that require control authorization.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(bound = "R: RequestMessage")]
pub struct TargetedRequest<R: RequestMessage> {
    /// Target entity as raw bits (Entity::to_bits())
    pub entity_bits: u64,
    /// Request correlation ID (for response matching)
    pub request_id: u64,
    /// The request payload
    pub request: R,
}

impl<R: RequestMessage> TargetedRequest<R> {
    pub fn type_name() -> &'static str {
        // Cache pattern similar to TargetedMessage
        ...
    }
}
```

### Step 1.2: Add TargetedRequestError response

**File:** `crates/pl3xus_common/src/messages.rs`

```rust
/// Error response for rejected targeted requests
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TargetedRequestError {
    NotAuthorized { controlled_by: ConnectionId },
    EntityNotFound,
    NotControlled,  // Entity exists but has no EntityControl component
}
```

---

## Phase 2: Server-Side Middleware (pl3xus + pl3xus_sync)

### Step 2.1: Add AuthorizedRequest type

**File:** `crates/pl3xus/src/managers/targeted_request.rs` (new file)

```rust
use bevy::prelude::*;
use async_channel::Sender;
use pl3xus_common::{ConnectionId, NetworkPacket, RequestMessage};
use pl3xus_sync::EntityControl;

/// Request that has passed authorization middleware.
/// Systems only receive this after control validation.
#[derive(Message, Debug, Clone)]
pub struct AuthorizedRequest<R: RequestMessage> {
    pub request: R,
    pub entity: Entity,
    pub source: ConnectionId,
    pub control: EntityControl,
    response_tx: Sender<NetworkPacket>,
    request_id: u64,
}

impl<R: RequestMessage> AuthorizedRequest<R> {
    pub fn respond(self, response: R::ResponseMessage) -> Result<(), NetworkError> {
        // Similar to Request::respond() in network_request.rs
    }
}
```

### Step 2.2: Add middleware system

**File:** `crates/pl3xus/src/managers/targeted_request.rs`

```rust
fn targeted_request_middleware<R, NP>(
    mut raw: MessageReader<NetworkData<TargetedRequest<R>>>,
    entities: Query<&EntityControl>,
    config: Res<ExclusiveControlConfig>,
    net: Res<Network<NP>>,
    mut authorized: MessageWriter<AuthorizedRequest<R>>,
)
where
    R: RequestMessage + Clone + Debug + 'static,
    NP: NetworkProvider,
{
    for request in raw.read() {
        let source = *request.source();
        let entity = Entity::from_bits(request.entity_bits);
        
        match entities.get(entity) {
            Ok(control) => {
                let has_control = 
                    control.client_id == source 
                    || control.client_id.id == 0
                    || source.is_server();
                
                if has_control {
                    authorized.write(AuthorizedRequest { ... });
                } else {
                    // Send error response
                    send_error_response(net, source, request.request_id, 
                        TargetedRequestError::NotAuthorized { 
                            controlled_by: control.client_id 
                        });
                }
            }
            Err(_) => {
                send_error_response(net, source, request.request_id,
                    TargetedRequestError::EntityNotFound);
            }
        }
    }
}
```

### Step 2.3: Add registration extension trait

**File:** `crates/pl3xus/src/managers/targeted_request.rs`

```rust
pub trait AppTargetedRequestExt {
    fn add_targeted_request<R, NP>(&mut self) -> &mut Self
    where
        R: RequestMessage + Clone + Debug + 'static,
        NP: NetworkProvider;
}

impl AppTargetedRequestExt for App {
    fn add_targeted_request<R, NP>(&mut self) -> &mut Self {
        // 1. Register TargetedRequest<R> as network message
        self.register_network_message::<TargetedRequest<R>, NP>();
        
        // 2. Add message type for AuthorizedRequest<R>
        self.add_message::<AuthorizedRequest<R>>();
        
        // 3. Add middleware system
        self.add_systems(PreUpdate, targeted_request_middleware::<R, NP>);
        
        self
    }
}
```

### Step 2.4: Export from pl3xus

**File:** `crates/pl3xus/src/managers/mod.rs`

```rust
pub mod targeted_request;
pub use targeted_request::{AuthorizedRequest, AppTargetedRequestExt};
```

---

## Phase 3: Client-Side Hook (pl3xus_client)

### Step 3.1: Add request_targeted method to SyncContext

**File:** `crates/pl3xus_client/src/context.rs`

```rust
impl SyncContext {
    /// Send a targeted request to a specific entity.
    /// The server will validate control authorization before processing.
    pub fn request_targeted<R: RequestMessage + Clone>(&self, entity_bits: u64, request: R) -> u64 {
        let request_id = self.next_request_id.fetch_add(1, Ordering::SeqCst);

        let targeted = TargetedRequest {
            entity_bits,
            request_id,
            request,
        };

        // Send via WebSocket (same as regular request)
        self.send_message(targeted);

        // Track in request state map
        self.requests.update(|map| {
            map.insert(request_id, RequestState {
                status: RequestStatus::Pending,
                timestamp: now(),
            });
        });

        request_id
    }
}
```

### Step 3.2: Add use_request_targeted hook

**File:** `crates/pl3xus_client/src/hooks.rs`

```rust
/// Hook to send a targeted request to a specific entity.
///
/// The server middleware automatically validates that the client has control
/// of the target entity. Unauthorized requests receive an error response.
///
/// # Type Parameters
/// - `R`: The request type (must implement `RequestMessage`)
///
/// # Returns
/// A tuple of:
/// - A function to send the request: `(entity_bits: u64, request: R)`
/// - A signal with the current request state
pub fn use_request_targeted<R>() -> (
    impl Fn(u64, R) + Clone,
    Signal<UseRequestState<R::ResponseMessage>>,
)
where
    R: RequestMessage + Clone + 'static,
{
    let ctx = expect_context::<SyncContext>();
    let current_request_id = RwSignal::new(None::<u64>);

    let state = {
        let ctx = ctx.clone();
        Signal::derive(move || {
            // Same logic as use_request, derive state from request map
            match current_request_id.get() {
                None => UseRequestState::idle(),
                Some(id) => ctx.get_request_state::<R>(id),
            }
        })
    };

    let send = move |entity_bits: u64, request: R| {
        let id = ctx.request_targeted(entity_bits, request);
        current_request_id.set(Some(id));
    };

    (send, state)
}
```

---

## Phase 4: Integration & Testing

### Step 4.1: Add to fanuc_rmi_replica example

**Server (main.rs):**
```rust
app.add_plugins(ExclusiveControlPlugin::default())
   .add_exclusive_control_systems::<WebSocketProvider>()
   .add_targeted_request::<JogCommand, WebSocketProvider>()
   .add_systems(Update, handle_jog);
```

**Client component:**
```rust
#[component]
fn JogControls(robot_id: u64) -> impl IntoView {
    let (jog, jog_state) = use_request_targeted::<JogCommand>();

    view! {
        <button
            on:click=move |_| jog(robot_id, JogCommand::new(Axis::X, 0.5))
            disabled=move || jog_state.get().is_loading()
        >
            "Jog +X"
        </button>
    }
}
```

### Step 4.2: Unit tests

**File:** `crates/pl3xus/src/managers/targeted_request.rs`

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_authorized_request_when_has_control() { ... }

    #[test]
    fn test_rejected_when_other_client_has_control() { ... }

    #[test]
    fn test_rejected_when_entity_not_found() { ... }

    #[test]
    fn test_allowed_when_entity_uncontrolled() { ... }

    #[test]
    fn test_server_always_authorized() { ... }
}
```

---

## Complete File Change Summary

### Phase 0: Authorization Policy Trait

| File | Change | Lines |
|------|--------|-------|
| `pl3xus_sync/src/authorization.rs` | **NEW** | ~80 |
| `pl3xus_sync/src/control.rs` | Modify | +30 |
| `pl3xus_sync/src/lib.rs` | Modify | +5 |

### Phase 1: Registration Helpers

| File | Change | Lines |
|------|--------|-------|
| `pl3xus/src/managers/registration.rs` | **NEW** | ~120 |
| `pl3xus/src/managers/mod.rs` | Modify | +3 |
| `pl3xus/src/lib.rs` | Modify | +5 |

### Phase 2: Wire Format + Request Types

| File | Change | Lines |
|------|--------|-------|
| `pl3xus_common/src/messages.rs` | Modify | +50 |

### Phase 3: Client Hook

| File | Change | Lines |
|------|--------|-------|
| `pl3xus_client/src/context.rs` | Modify | +25 |
| `pl3xus_client/src/hooks.rs` | Modify | +50 |
| `pl3xus_client/src/lib.rs` | Modify | +2 |

### Phase 4: Example Integration

| File | Change | Lines |
|------|--------|-------|
| `fanuc_rmi_replica/server/src/main.rs` | Modify | +15 |
| `fanuc_rmi_replica/shared/src/messages.rs` | Modify | +25 |
| `fanuc_rmi_replica/client/src/components/` | Modify | +40 |

**Total: ~450 lines across 13 files**

---

## Implementation Checklist

### Phase 0: Authorization Policy Trait
- [ ] Create `authorization.rs` with `TargetedMessageAuthorizer` trait
- [ ] Add `TargetedAuthorizerResource` with `from_fn()`, `allow_all()`, `server_only()`
- [ ] Add `TargetedAuthContext` struct
- [ ] Modify `ExclusiveControlPlugin` to install authorization policy
- [ ] Add `exclusive_control_check()` helper
- [ ] Export from `pl3xus_sync/src/lib.rs`

### Phase 1: Registration Helpers
- [ ] Create `registration.rs`
- [ ] Add `register_message<T, NP, S>()` (full: incoming + targeted + outbound + auth)
- [ ] Add `register_message_unscheduled<T, NP>()` (incoming only)
- [ ] Add `AuthorizedMessage<T>` event type
- [ ] Add `authorize_targeted_messages<T, NP>()` middleware system
- [ ] Export from `managers/mod.rs` and `lib.rs`

### Phase 2: Wire Format + Request Types
- [ ] Add `TargetedRequest<R>` struct (for request/response pattern)
- [ ] Add `TargetedRequestError` enum
- [ ] Implement `Pl3xusMessage` for wire types
- [ ] Add serialization tests

### Phase 3: Client Hook
- [ ] Add `request_targeted()` to SyncContext
- [ ] Implement `use_request_targeted<R>()` hook
- [ ] Handle targeted response routing
- [ ] Export from lib.rs

### Phase 4: Example Integration
- [ ] Add `JogCommand` request type to shared
- [ ] Use `register_message` in server
- [ ] Add server handler that reads `AuthorizedMessage<JogCommand>`
- [ ] Create client UI component with jog buttons
- [ ] End-to-end test

---

## Relationship to sync_component

`sync_component<T>()` and these registration functions are **complementary**:

| Pattern | Use Case |
|---------|----------|
| `sync_component<T>()` | Automatic state synchronization (subscriptions, mutations) |
| `register_message<T>()` | Command/event messages (jog, start, stop, etc.) |
| `register_message_unscheduled<T>()` | Simple receive-only messages |

**sync_component already handles its own message registration** - you don't need to call `register_message` for synced components.

---

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| No authorizer resource installed | Default to allow-all (same as no policy) |
| Custom auth conflicts with ExclusiveControl | Document that installing ExclusiveControlPlugin replaces any existing TargetedAuthorizerResource |
| Policy race with control changes | Accept - auth checked at middleware time, not request time |
| Breaking existing request API | Keep `use_request<R>()` unchanged, add new patterns |

---

## Testing Strategy

### Unit Tests
1. `TargetedMessageAuthorizer` trait with mock policies
2. `authorize_targeted_messages` middleware with various policies
3. `ExclusiveControlPlugin` authorization check
4. Wire format serialization

### Integration Tests
1. Full round-trip with ExclusiveControl policy
2. Custom policy (e.g., role-based)
3. No policy installed (allow-all behavior)
4. Policy rejection flow

### Example Validation
1. fanuc_rmi_replica jog commands work with control
2. Commands rejected without control
3. Error displayed to user

---

## Deprecation Notes

### SubscriptionMessage Pattern (Legacy)

The `register_subscription<T>()` pattern is from the Eventwork era and is **deprecated**. Use `sync_component<T>()` instead:

```rust
// DEPRECATED - requires manual subscription handling
app.register_subscription::<RobotState, WS>();

// RECOMMENDED - automatic subscription management
app.sync_component::<RobotState>(None);
```

---

## Future Enhancements

1. **Multiple policies** - Chain or compose authorization policies
2. **Rate limiting** - Per-entity request rate limits
3. **Audit logging** - Log all authorization decisions
4. **Action pattern** - Long-running operations with cancel/progress
5. **OutboundMessage for responses** - Route responses through system-set controlled sending

