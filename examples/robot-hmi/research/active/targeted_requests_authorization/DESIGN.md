# Targeted Requests with Authorization - Design Document

## Executive Summary

This document proposes adding **pluggable targeted message authorization** to pl3xus, enabling:
1. A policy-based authorization system (`TargetedMessageAuthorizer` trait) that mirrors `MutationAuthorizer`
2. Convenience registration functions that bundle all required setup
3. `ExclusiveControlPlugin` as one implementation of the authorization policy

## Goals

1. **Eliminate boilerplate** - Systems shouldn't manually check authorization
2. **Pluggable policies** - Users can implement their own auth logic, not just EntityControl
3. **Type-safe** - Compile-time guarantees for message types
4. **Hierarchical support** - Control of parent = control of children (when using ExclusiveControl)
5. **Additive** - Don't break existing APIs
6. **Consistent** - Match patterns established by `MutationAuthorizer`

## Proposed API

### Authorization Policy (pl3xus_sync)

The foundation is a pluggable policy trait that mirrors `MutationAuthorizer`:

```rust
/// Context for authorization decisions
pub struct TargetedAuthContext<'a> {
    pub world: &'a World,
    pub source: ConnectionId,
    pub target_entity: Entity,
}

/// Pluggable policy for deciding if a client can send to a target entity.
pub trait TargetedMessageAuthorizer: Send + Sync + 'static {
    fn authorize(&self, ctx: &TargetedAuthContext) -> Result<(), String>;
}

/// Resource wrapping the active policy.
#[derive(Resource)]
pub struct TargetedAuthorizerResource {
    pub inner: Arc<dyn TargetedMessageAuthorizer>,
}

impl TargetedAuthorizerResource {
    pub fn from_fn<F>(f: F) -> Self { ... }
    pub fn allow_all() -> Self { ... }
    pub fn server_only() -> Self { ... }
}
```

**ExclusiveControlPlugin installs its own policy:**
```rust
impl Plugin for ExclusiveControlPlugin {
    fn build(&self, app: &mut App) {
        // ... existing setup ...

        // Install EntityControl-based authorization
        app.insert_resource(TargetedAuthorizerResource::from_fn(
            move |world, source, entity| {
                // Check EntityControl component
                if source.is_server() { return Ok(()); }

                match world.get::<EntityControl>(entity) {
                    Some(control) => {
                        if control.client_id == source || control.client_id.id == 0 {
                            Ok(())
                        } else {
                            Err("Entity controlled by another client".to_string())
                        }
                    }
                    None => Ok(()), // No control component = allow
                }
            }
        ));
    }
}
```

**Custom policies are easy:**
```rust
// Role-based access control
app.insert_resource(TargetedAuthorizerResource::from_fn(
    |world, source, entity| {
        if world.get_resource::<AdminClients>()?.contains(source) {
            return Ok(());
        }
        // ... other checks
    }
));
```

### Registration Convenience Functions (pl3xus)

```rust
/// Complete registration with system-set scheduling
pub fn register_message<T, NP, S>(app: &mut App, system_set: S)
where
    T: Pl3xusMessage + Clone + 'static,
    NP: NetworkProvider,
    S: SystemSet + Clone,
{
    // 1. Incoming (plain T)
    app.register_network_message::<T, NP>();

    // 2. Incoming (TargetedMessage<T>)
    app.register_targeted_message::<T, NP>();

    // 3. Outbound (system-set controlled)
    app.register_outbound_message::<T, NP, S>(system_set);

    // 4. Authorized event type
    app.add_message::<AuthorizedMessage<T>>();

    // 5. Authorization middleware
    app.add_systems(PreUpdate, authorize_targeted_messages::<T, NP>);
}

/// Simple incoming-only registration (no auth, no targeting, no outbound)
pub fn register_message_unscheduled<T, NP>(app: &mut App) { ... }
```

### Client Side

#### New Hook: `use_request_targeted<R>()`

```rust
/// Hook for sending requests targeted at a specific entity.
/// The server middleware will automatically check if the client has control.
pub fn use_request_targeted<R>() -> (
    impl Fn(u64, R) + Clone,           // (entity_bits, request)
    Signal<UseRequestState<R::ResponseMessage>>,
)
where
    R: RequestMessage + Clone + 'static;
```

**Usage:**
```rust
// In a Leptos component
let (jog, jog_state) = use_request_targeted::<JogCommand>();

// Entity ID comes from synced component
let robot_id = robot.entity_id;

// Send targeted request
view! {
    <button on:click=move |_| jog(robot_id, JogCommand { axis: Axis::X, velocity: 0.5 })>
        "Jog +X"
    </button>
}
```

### Wire Format

#### New Type: `TargetedRequest<R>`

```rust
/// Wire format for targeted requests (distinct from existing TargetedMessage)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TargetedRequest<R: RequestMessage> {
    pub entity_bits: u64,        // Target entity (u64 for efficiency)
    pub request_id: u64,         // Correlation ID for response
    pub request: R,              // The actual request payload
}
```

**Rationale for `u64` vs `String`:**
- Clients already know entity IDs from synced components (u64 format)
- No lookup required on server side - direct `Entity::from_bits()`
- More efficient on wire (8 bytes vs variable-length string)
- Type-safe - no parsing errors possible

### Server Side

#### New Type: `AuthorizedRequest<R>`

```rust
/// Wrapper emitted by middleware for authorized requests.
/// Systems only see requests that passed control validation.
#[derive(Message, Debug, Clone)]
pub struct AuthorizedRequest<R: RequestMessage> {
    /// The original request payload
    pub request: R,
    /// Target entity (validated to exist)
    pub entity: Entity,
    /// Client that sent the request
    pub source: ConnectionId,
    /// Control state at time of authorization
    pub control: EntityControl,
    /// Internal: channel to send response back
    response_tx: Sender<NetworkPacket>,
}

impl<R: RequestMessage> AuthorizedRequest<R> {
    /// Send a response back to the client
    pub fn respond(self, response: R::ResponseMessage) -> Result<(), NetworkError> {
        // ... serialize and send
    }
}
```

#### Registration Extension Trait

```rust
pub trait AppTargetedRequestExt {
    /// Register a targeted request type with automatic authorization.
    ///
    /// This sets up:
    /// 1. Network message registration for `TargetedRequest<R>`
    /// 2. Middleware system that checks control and emits `AuthorizedRequest<R>`
    /// 3. Automatic rejection with error response for unauthorized requests
    fn add_targeted_request<R, NP>(&mut self) -> &mut Self
    where
        R: RequestMessage + Clone + Debug + 'static,
        NP: NetworkProvider;
}
```

**Usage:**
```rust
// Server setup
app.add_plugins(ExclusiveControlPlugin::default())
   .add_exclusive_control_systems::<WebSocketProvider>()
   .add_targeted_request::<JogCommand, WebSocketProvider>()
   .add_systems(Update, handle_jog);

// System - no manual control check!
fn handle_jog(mut requests: MessageReader<AuthorizedRequest<JogCommand>>) {
    for req in requests.read() {
        let velocity = req.request.velocity;
        // ... execute jog on req.entity
        req.respond(JogResponse::Success);
    }
}
```

### Middleware Implementation

```rust
fn targeted_request_authorization_middleware<R, NP>(
    mut raw_requests: MessageReader<NetworkData<TargetedRequest<R>>>,
    entities: Query<(Entity, &EntityControl, Option<&Children>)>,
    config: Res<ExclusiveControlConfig>,
    net: Res<Network<NP>>,
    mut authorized: MessageWriter<AuthorizedRequest<R>>,
)
where
    R: RequestMessage + Clone + Debug + 'static,
    NP: NetworkProvider,
{
    for request in raw_requests.read() {
        let source = *request.source();
        let entity = Entity::from_bits(request.entity_bits);
        
        // Check if entity exists and client has control
        match entities.get(entity) {
            Ok((_, control, _)) => {
                // Use hierarchical control check
                let has_control = control.client_id == source
                    || control.client_id.id == 0  // Uncontrolled = anyone can command
                    || source.is_server();
                
                if has_control {
                    // Emit authorized request
                    authorized.write(AuthorizedRequest {
                        request: request.request.clone(),
                        entity,
                        source,
                        control: control.clone(),
                        response_tx: /* from connection */,
                    });
                } else {
                    // Send rejection response
                    send_rejection(net, source, request.request_id, 
                        "Not authorized: entity controlled by another client");
                }
            }
            Err(_) => {
                send_rejection(net, source, request.request_id,
                    "Entity not found");
            }
        }
    }
}
```

## Key Design Decisions

### 1. Reject at Middleware vs Pass-Through with Flag

**Chosen: Reject at Middleware**

- Systems only see authorized requests (cleaner code)
- Unauthorized requests get automatic error response
- No risk of forgetting to check the flag
- Matches mutation behavior (rejected mutations don't trigger systems)

### 2. Entity ID Format: u64 vs String

**Chosen: u64 (entity bits)**

- Matches existing synced component pattern (clients receive entity IDs as u64)
- More efficient (no string parsing, no HashMap lookup)
- Type-safe (String "robot-1" could have typos, u64 from server is always valid)
- Meteorite uses String because it has manual node IDs; pl3xus uses ECS entities

### 3. Separate Type vs Extend Existing

**Chosen: New `TargetedRequest<R>` type (separate from `TargetedMessage<T>`)**

- `TargetedMessage<T>` is a general wrapper for any message
- `TargetedRequest<R>` is specific to request/response with correlation IDs
- Clearer semantics for registration and handling
- Allows different behavior (requests need response channel, messages don't)

