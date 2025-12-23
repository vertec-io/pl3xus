# Research Questions: Component Mutation Handlers

## Core Design Questions

### 1. Where Should Handlers Run?

**Option A: Bevy System**
- Handler is a normal Bevy system that reads `ComponentMutation<T>` events
- Pro: Access to full World, other components, resources
- Pro: Familiar pattern for Bevy developers
- Con: Must run in correct schedule phase

**Option B: Callback Closure**
- Handler is a closure registered at sync time
- Pro: Simpler registration
- Con: Limited World access

**Recommendation**: Option A - Bevy systems provide the flexibility needed for real use cases.

### 2. When Are Mutations Applied?

**Option A: Before Handler (handler validates)**
- Mutation applied immediately, handler can rollback on failure
- Pro: Simpler flow
- Con: Race condition - other clients see value before validation

**Option B: After Handler (handler controls)**
- Handler decides when/if to apply mutation
- Pro: No invalid states visible to clients
- Con: Handler must explicitly apply

**Option C: Hybrid (handler can do either)**
- Config flag: `ApplyMode::Immediate | ApplyMode::HandlerControlled`
- Pro: Flexibility
- Con: Complexity

**Recommendation**: Option B - Handler controlled. Matches request pattern semantics.

### 3. How Do Responses Work?

Should handlers be able to send responses back to the mutating client?

**Current mutation responses**:
- `MutationStatus::Ok | Forbidden | NotFound | InternalError`
- No custom response data

**Proposed enhancement**:
- Handler can respond with custom data type
- Similar to request/response pattern

```rust
// Server
mutation.respond(FrameToolMutationResponse {
    success: true,
    actual_frame: 5,  // May differ from requested
});

// Client
frame_tool.mutate_with_response::<FrameToolMutationResponse>(
    |state| { state.active_frame = 5; },
    move |result| {
        match result {
            Ok(resp) => log!("Set to frame {}", resp.actual_frame),
            Err(e) => toast.error(e),
        }
    }
);
```

### 4. How Does This Interact with Authorization?

Current authorization happens before mutation processing:
1. `MutationAuthorizer::authorize()` runs
2. If authorized, `apply_mutation` fn runs
3. `MutationResponse` sent

With handlers:
1. `MutationAuthorizer::authorize()` runs
2. If authorized, **handler system** runs
3. Handler can apply or reject with custom response

**Question**: Should handlers have their own authorization hook, or reuse existing?

**Recommendation**: Reuse existing. Handlers are for business logic, not auth.

### 5. What About Partial Updates?

Current mutations replace entire component. What if client only wants to update one field?

**Option A: Full replacement (current)**
- Client sends entire component
- Simple, predictable
- Con: Race conditions if two clients update different fields

**Option B: Field-level patches**
- Client sends `{ active_frame: 5 }` not full component
- Handler receives partial update
- Con: Significant complexity

**Option C: Optimistic merge**
- Client sends full component, but handler can merge with current
- Handler sees both old and new values

**Recommendation**: Start with Option A. Option C as future enhancement.

## Implementation Questions

### 6. Registration API

```rust
// Option 1: Builder pattern
app.sync_component::<FrameToolDataState>()
    .with_handler(handler_system)
    .handler_controlled()
    .build();

// Option 2: Separate method
app.sync_component::<FrameToolDataState>();
app.add_mutation_handler::<FrameToolDataState>(handler_system);

// Option 3: Combined method
app.sync_component_with_handler::<FrameToolDataState>(handler_system);
```

### 7. Event Type Design

```rust
// What the handler receives
pub struct ComponentMutation<T> {
    connection_id: ConnectionId,
    request_id: Option<u64>,
    entity: Entity,
    new_value: T,
    old_value: Option<T>,  // If we can provide it
}

impl<T> ComponentMutation<T> {
    fn entity(&self) -> Entity;
    fn new_value(&self) -> &T;
    fn old_value(&self) -> Option<&T>;
    fn apply(&self, world: &mut World);  // Apply the mutation
    fn respond<R: Serialize>(&self, response: R);  // Send response
    fn reject(&self, message: &str);  // Reject with error
}
```

### 8. Client Hook Design

```rust
// Option 1: New hook
let handle = use_component_mutation::<FrameToolDataState>(entity_id, callback);
handle.mutate(|state| { state.active_frame = 5; });

// Option 2: Extend existing
let (state, _exists) = use_entity_component::<FrameToolDataState, _>(entity_fn);
// ...but how to add mutation capability?

// Option 3: Combined read+write hook
let handle = use_synced_mutation::<FrameToolDataState>(entity_fn, callback);
handle.state.get();  // Read
handle.mutate(|s| { ... });  // Write with handler
```

## What This Replaces

| Current Pattern | Replaced By |
|-----------------|-------------|
| `SetActiveFrameTool` request type | Component mutation on `FrameToolDataState` |
| `SetActiveFrameToolResponse` type | Generic mutation response or custom |
| `handle_set_active_frame_tool` handler | Component mutation handler |
| `use_mutation::<SetActiveFrameTool>` | `use_synced_mutation::<FrameToolDataState>` |

## What This Does NOT Replace

- Complex multi-step operations (e.g., `ExecuteProgram`)
- Operations not tied to a specific component
- Operations requiring multiple entities
- Resource-based operations (non-entity)

