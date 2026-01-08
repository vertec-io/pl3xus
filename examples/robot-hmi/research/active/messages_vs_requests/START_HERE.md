# Messages vs Requests: Architectural Analysis

## The Core Question

When should a network operation be a **Message** vs a **Request**? And how should **targeting** (entity ID) and **authorization** (control checks) be applied to each?

## Current State in pl3xus

### Messages
```rust
// Registration
app.message::<JogCommand, NP>()
   .targeted()                    // Optional: expects TargetedMessage<T> wrapper
   .with_default_entity_access()  // Optional: uses ExclusiveControlPlugin policy
   .register();

// Server handling
fn handle_jog(mut msgs: MessageReader<AuthorizedTargetedMessage<JogCommand>>) {
    for msg in msgs.read() {
        // msg.entity, msg.source, msg.message
        // No response mechanism - fire-and-forget
    }
}
```

### Requests
```rust
// Registration
app.listen_for_request_message::<StartProgram, WebSocketProvider>();

// Server handling  
fn handle_start(mut requests: MessageReader<Request<StartProgram>>) {
    for req in requests.read() {
        // req.get_request(), req.source()
        // MUST call req.respond() to complete the transaction
        req.respond(StartProgramResponse { success: true });
    }
}
```

## The Problem

Currently:
1. **Messages** have targeting + authorization middleware (via `.targeted().with_entity_access()`)
2. **Requests** have response mechanism but NO targeting/authorization middleware
3. This creates an awkward split where:
   - `JogCommand` is a targeted message (has entity, has auth, no response)
   - `StartProgram` is a request (has response, no entity targeting, no auth middleware)

## Why This Matters

For `SetSpeedOverride`:
- Needs to target a specific robot entity ✓ (message feature)
- Needs authorization check ✓ (message feature)  
- Needs to return success/failure ✓ (request feature)
- Needs correlation ID for UI loading state ✓ (request feature)

Currently implemented as a targeted message, but the client can't know if it succeeded!

## Semantic Analysis

### When to Use Messages (Fire-and-Forget)

Messages are appropriate when:
1. **Realtime streaming** - High-frequency updates where responses would create backpressure
2. **Idempotent state** - Repeated sends have no adverse effect
3. **Observable via sync** - Success is visible through synced component changes
4. **Low criticality** - Failure is acceptable (will retry naturally)

Examples:
- `JogCommand` - Continuous jogging, 50Hz, visible via position sync
- Position/velocity updates - Streaming data
- Heartbeat/keepalive - Periodic signals

### When to Use Requests (Response Required)

Requests are appropriate when:
1. **Confirmation needed** - Client needs to know success/failure
2. **One-time operations** - Not naturally retried
3. **Complex errors** - Need to communicate specific failure reasons
4. **UI feedback** - Loading states, error toasts, retry logic
5. **Transactions** - Operations that must succeed or rollback

Examples:
- `StartProgram` - Must confirm execution started
- `LoadConfiguration` - Must confirm config loaded
- `SetSpeedOverride` - Must confirm speed changed
- `InitializeRobot` - Must confirm init completed
- `ConnectToRobot` - Must confirm connection established

## The Unification Question

Should all targeted operations be requests? Consider:

### Option A: Keep Separation
- Messages for realtime/streaming
- Requests for one-shot commands
- Add targeting + auth to requests

### Option B: Unify to Requests
- All commands become requests
- Messages only for subscriptions/streaming
- Simpler mental model

### Option C: Action Pattern (ROS2-style)
- Simple commands → Requests
- Long-running operations → Actions (with progress/cancel)
- Streaming → Messages

