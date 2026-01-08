# Phase 3: Truly Automatic Query Invalidation

## Problem Statement

Current Phase 2 implementation still requires **manual broadcast calls** in every handler:

```rust
fn handle_create_program(...) {
    // Business logic...
    if success {
        broadcast_invalidations_for::<CreateProgram, _>(&net, None);  // <-- Still manual!
    }
}
```

This is "70% still manual" - we moved the rules to the type but still call broadcast in handlers.

**Goal**: Zero additional lines in handlers for invalidation.

## Key Insight We're Missing

The challenge is that the framework doesn't have visibility into:
1. **Success/failure** of the response (response struct varies by type)
2. **When to trigger** invalidation (after response is sent)

### Industry Patterns

| Framework | Pattern | How it works |
|-----------|---------|--------------|
| TanStack Query | `mutationFn` return | Invalidation triggered after promise resolves |
| Apollo Client | `refetchQueries` option | Specified at mutation call site, executes after mutation |
| SWR | `mutate` revalidates | Key-based, optimistic updates + revalidation |
| RTK Query | `invalidatesTags` | Declarative, framework intercepts responses |

The common pattern: **Framework observes completion, not handler**.

## Potential Solutions

### Option A: Response Wrapping

Make all responses implement a trait that indicates success:

```rust
pub trait MutationResponse {
    fn is_success(&self) -> bool;
}

// Derive macro or manual impl
#[derive(MutationResponse)]
pub struct CreateProgramResponse {
    pub success: bool,  // Macro looks for this field
    pub program: Option<Program>,
    pub error: Option<String>,
}
```

Framework wraps the respond() call to check success and broadcast.

**Problem**: `request.respond()` is called inside handler, can't wrap it.

### Option B: Response Tracking Resource

Handler marks response in a resource, framework broadcasts in a later system:

```rust
fn handle_create_program(
    mut requests: MessageReader<Request<CreateProgram>>,
    mut tracker: ResMut<ResponseTracker>,
    // ...
) {
    for request in requests.read() {
        // Business logic...
        let response = CreateProgramResponse { success: true, ... };
        
        // Still manual but cleaner
        tracker.track_success::<CreateProgram>(&request);
        request.respond(response);
    }
}
```

**Problem**: Still requires manual call, just a different one.

### Option C: Handler Return Value Pattern

Handler returns response instead of calling respond():

```rust
fn handle_create_program(
    In(request): In<Request<CreateProgram>>,
    db: Res<DatabaseResource>,
) -> CreateProgramResponse {
    // Business logic...
    CreateProgramResponse { success: true, ... }
}

// Framework sends response AND broadcasts invalidation
app.request::<CreateProgram, WS>()
    .handler(handle_create_program)  // Framework wraps this
    .auto_invalidate()
    .register();
```

**Pros**: 
- Handler is pure business logic
- Framework controls response sending
- Framework can check success and broadcast

**Cons**:
- Different handler signature than current
- Doesn't fit Bevy's system model (systems can't easily return values)
- Need to handle MessageReader loop internally

### Option D: ECS-Native with Marker Components

After handler runs, a follow-up system checks for success markers:

```rust
// Handler adds a marker component when successful
fn handle_create_program(
    mut commands: Commands,
    mut requests: MessageReader<Request<CreateProgram>>,
    // ...
) {
    for request in requests.read() {
        // Business logic...
        if success {
            commands.spawn(SuccessfulMutation::<CreateProgram>::default());
        }
        request.respond(response);
    }
}

// Framework system runs after all handlers
fn broadcast_pending_invalidations<T: Invalidates>(
    mut commands: Commands,
    markers: Query<Entity, With<SuccessfulMutation<T>>>,
    net: Res<Network<WebSocketProvider>>,
) {
    for entity in markers.iter() {
        broadcast_invalidations_for::<T, _>(&net, None);
        commands.entity(entity).despawn();
    }
}
```

**Problem**: Generic system per request type - explosion of systems.

### Option E: Request Extension Method (Most Promising?)

Extend `Request<T>` with a method that combines respond + invalidate:

```rust
impl<T: RequestMessage + Invalidates> Request<T> {
    pub fn respond_and_invalidate<R: MutationResponse, NP: NetworkProvider>(
        self,
        response: R,
        net: &Network<NP>,
    ) -> Result<(), ...> {
        let is_success = response.is_success();
        self.respond(response)?;
        
        if is_success {
            broadcast_invalidations_for::<T, NP>(net, None);
        }
        Ok(())
    }
}

// Usage - one line instead of if block!
fn handle_create_program(...) {
    for request in requests.read() {
        let response = CreateProgramResponse { ... };
        request.respond_and_invalidate(response, &net);
    }
}
```

**Pros**:
- Single line in handler
- Natural extension of existing pattern
- Type-safe (only works for `T: Invalidates`)

**Cons**:
- Requires `MutationResponse` trait on responses
- Still technically manual (just less verbose)

## Recommendation

**Option E (Request Extension)** seems most promising for near-term:
- Minimal API change
- Fits existing handler patterns
- Type-safe
- Single line per handler

For truly zero-touch, **Option C** is the ideal long-term goal but requires bigger architectural changes.

## Research Questions

1. ✅ Can we implement Option E without the `MutationResponse` trait by using a convention (e.g., all responses have `success: bool`)?
   - **ANSWERED**: Yes! We implemented `HasSuccess` trait with a derive macro that validates `success: bool` field exists at compile time.

2. Could we use Bevy's system piping to wrap handlers?
   - Still open for future exploration

3. What if `app.request::<T>().auto_invalidate().register()` added a post-system that runs after the handler system?
   - Still open for future exploration (Option C long-term goal)

## Implementation Complete ✅

**Option E has been implemented!**

### New API

```rust
// In pl3xus_common/src/messages.rs
pub trait HasSuccess {
    fn is_success(&self) -> bool;
}

// In pl3xus_macros/src/lib.rs
#[derive(HasSuccess)]  // Validates success: bool field at compile time

// In pl3xus_sync/src/invalidation.rs
pub trait RequestInvalidateExt<T: RequestMessage> {
    fn respond_and_invalidate<NP>(self, response: T::ResponseMessage, net: &Network<NP>) -> Result<(), NetworkError>;
    fn respond_and_invalidate_with_keys<NP>(self, response: T::ResponseMessage, net: &Network<NP>, keys: Vec<String>) -> Result<(), NetworkError>;
}
```

### Usage

```rust
// Before (Phase 2 - manual broadcast)
let success = response.success;
if let Err(e) = request.clone().respond(response) {
    error!("Failed to send response: {:?}", e);
}
if success {
    broadcast_invalidations_for::<CreateProgram, _>(&net, None);
}

// After (Phase 3 - single line!)
if let Err(e) = request.clone().respond_and_invalidate(response, &net) {
    error!("Failed to send response: {:?}", e);
}

// With keyed invalidation
if let Err(e) = request.clone().respond_and_invalidate_with_keys(response, &net, vec![program_id.to_string()]) {
    error!("Failed to send response: {:?}", e);
}
```

### Type Definitions

```rust
// Request type (unchanged)
#[cfg_attr(feature = "server", derive(Invalidates))]
#[cfg_attr(feature = "server", invalidates("ListPrograms"))]
pub struct CreateProgram { ... }

// Response type (add HasSuccess)
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "server", derive(HasSuccess))]
pub struct CreateProgramResponse {
    pub success: bool,  // Required field
    pub program_id: Option<i64>,
    pub error: Option<String>,
}
```

### Handlers Migrated

All 11 mutation handlers with `#[derive(Invalidates)]` have been migrated:
- CreateProgram, DeleteProgram, UpdateProgramSettings, UploadCsv
- CreateRobotConnection, UpdateRobotConnection, DeleteRobotConnection
- CreateConfiguration, UpdateConfiguration, DeleteConfiguration, SetDefaultConfiguration

## Status

**Phase**: ✅ COMPLETE
**Priority**: Done
**Dependencies**: Phase 2 complete

## Future Work

- **Option C (Handler Return Value)**: For truly zero-touch invalidation, explore making handlers return responses instead of calling respond(). This would allow the framework to fully control response sending and invalidation.
- **`app.request::<T>().auto_invalidate().register()`**: Could add a post-system that automatically wraps handlers.

