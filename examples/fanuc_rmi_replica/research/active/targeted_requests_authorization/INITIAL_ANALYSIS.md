# Initial Analysis: Targeted Requests with Authorization

## Problem Statement

Currently in pl3xus:
- **Mutations** have authorization middleware (`MutationAuthorizerResource`)
- **Requests** do not have any authorization middleware
- Systems must manually check control for each request type

This leads to:
1. Boilerplate in every system that handles authorized requests
2. Inconsistent authorization patterns across systems
3. Risk of forgetting to check authorization

## Current Architecture

### Client Side (pl3xus_client)

```
use_request<R>()
    → SyncContext.request(R)
        → RequestInternal<R> { id, request }
            → NetworkPacket
                → WebSocket
```

The `request` method wraps the request with a correlation ID but has no concept of a target entity.

### Server Side (pl3xus + pl3xus_sync)

```
NetworkData<RequestInternal<R>>
    → create_request_handlers system
        → Request<R> { request, source, request_id, response_tx }
            → Your system reads Request<R>
                → MANUAL CONTROL CHECK
                    → Execute or reject
```

Each system manually:
1. Extracts the source connection ID
2. Queries for EntityControl on the target entity
3. Checks if source matches controller
4. Handles the rejection case

## Meteorite Solution

Meteorite adds a layer that transforms messages before systems see them:

```
NetworkData<TargetedMessage<T>>
    → handle_authorized_messages<T> (middleware)
        → AuthorizedNetworkData<T> { inner, authorized, source, node_id, control_state }
            → Your system reads AuthorizedNetworkData<T>
                → Check authorized flag (one line)
```

### Key Design Decisions in Meteorite

1. **Uses String IDs, not Entity bits**
   - Nodes have `NetworkNode { id: String, ... }` component
   - Messages target by string ID
   - Middleware looks up entity from string

2. **Middleware is generic over message type**
   - `handle_authorized_messages<T: EventworkMessage>`
   - Must be registered per message type

3. **Authorization result is a flag, not rejection**
   - `authorized: bool` lets system decide how to handle
   - Not rejected at middleware level

4. **Rate limiting built-in**
   - Tracks request count per control holder
   - Rejects at middleware if rate exceeded

## Gap Analysis

### What pl3xus Has

| Feature | Status |
|---------|--------|
| `TargetedMessage<T>` struct | ✅ Exists in pl3xus_common |
| `RequestMessage` trait | ✅ Exists |
| `ExclusiveControlPlugin` | ✅ Handles take/release |
| `EntityControl` component | ✅ Synced to clients |
| `MutationAuthorizerResource` | ✅ For mutations only |
| `has_control_hierarchical()` | ✅ Helper function |

### What's Missing

| Feature | Status |
|---------|--------|
| Client hook for targeted requests | ❌ Missing |
| Server middleware for request auth | ❌ Missing |
| `AuthorizedRequest<T>` wrapper | ❌ Missing |
| Request registration with auth | ❌ Missing |

## Design Questions

### 1. Entity Identification

**Meteorite uses String IDs:**
```rust
pub target_id: String  // "robot-1", "system", etc.
```

**pl3xus currently uses Entity bits:**
```rust
EntityControl { client_id: ConnectionId, ... }
// Entity is implicit from ECS query
```

**Options:**
- A) Use String IDs like meteorite (more portable, needs lookup)
- B) Use Entity bits (faster, no lookup, but client must know bits)
- C) Use Either (flexible but complex)

**Recommendation:** Use Entity bits (u64). The client already knows entity IDs from synced components. String lookup adds complexity and potential for mismatches.

### 2. API Shape

**Option A: Entity at hook creation**
```rust
let (send, state) = use_request_targeted::<JogCommand>(|| entity_id);
send(JogCommand { velocity: 0.5 });
```
- Pro: Clean call site
- Con: Can't target different entities with same hook

**Option B: Entity per request**
```rust
let (send, state) = use_request_targeted::<JogCommand>();
send(entity_id, JogCommand { velocity: 0.5 });
```
- Pro: Flexible
- Con: Every call needs entity ID

**Option C: Both variants**
```rust
// Static target
let (send, state) = use_request_targeted::<JogCommand>(|| entity_id);

// Dynamic target
let (send, state) = use_request_targeted_dynamic::<JogCommand>();
send(entity_id, cmd);
```

**Recommendation:** Option B (entity per request). More flexible, and the entity ID is often already a signal anyway.

### 3. Server-Side Registration

**Meteorite approach:**
- No explicit registration per type
- Generic system handles all `TargetedMessage<T>`

**pl3xus pattern for requests:**
```rust
app.add_request::<MyRequest, WebSocketProvider>();
```

**Proposed:**
```rust
// New method for targeted requests with authorization
app.add_targeted_request::<JogCommand, WebSocketProvider>();

// This would:
// 1. Register TargetedRequest<JogCommand> as a message type
// 2. Add middleware system that produces AuthorizedRequest<JogCommand>
// 3. Systems read AuthorizedRequest<JogCommand> instead of Request<JogCommand>
```

### 4. Authorization vs Rejection

**Option A: Reject unauthorized at middleware**
- Pro: Systems don't see unauthorized requests
- Con: No custom error handling per system

**Option B: Pass through with flag (Meteorite approach)**
- Pro: Systems can log, respond differently
- Con: Systems must check flag

**Option C: Separate queues**
```rust
MessageReader<AuthorizedRequest<T>>   // Only authorized
MessageReader<UnauthorizedRequest<T>> // Only unauthorized
```
- Pro: Type-level separation
- Con: More complex

**Recommendation:** Option A for simplicity. Unauthorized requests get automatic rejection response.

## Proposed Architecture

### Client Side

```rust
// New hook
pub fn use_request_targeted<R>() -> (
    impl Fn(u64, R) + Clone,  // (entity_id, request)
    Signal<UseRequestState<R::ResponseMessage>>,
)
where
    R: RequestMessage + Clone + 'static,
```

### Wire Format

```rust
// Already exists
pub struct TargetedMessage<T: Pl3xusMessage> {
    pub target_id: String,  // We'd use entity bits as string: "4294967295"
    pub message: T,
}

// OR new type with entity bits directly
pub struct TargetedRequest<T: RequestMessage> {
    pub entity_bits: u64,
    pub id: u64,  // Correlation ID
    pub request: T,
}
```

### Server Side

```rust
// New type systems receive
#[derive(Message)]
pub struct AuthorizedRequest<T: RequestMessage> {
    pub request: T,
    pub entity: Entity,
    pub source: ConnectionId,
    pub control: EntityControl,  // The control state when authorized
    pub response_tx: Sender<NetworkPacket>,
}

impl<T: RequestMessage> AuthorizedRequest<T> {
    pub fn respond(self, response: T::ResponseMessage) { ... }
}
```

### Registration

```rust
// Server setup
app.add_plugins(ExclusiveControlPlugin::default());
app.add_targeted_request::<JogCommand, WebSocketProvider>();

// System
fn handle_jog(mut requests: MessageReader<AuthorizedRequest<JogCommand>>) {
    for req in requests.read() {
        // Already authorized! Just execute.
        let velocity = req.request.velocity;
        // ... apply jog
        req.respond(JogResponse::Success);
    }
}
```

## Next Steps

1. Validate this design with the user
2. Decide on entity ID format (u64 bits vs String)
3. Prototype client hook `use_request_targeted`
4. Prototype server middleware
5. Integration test with fanuc_rmi_replica jog commands
