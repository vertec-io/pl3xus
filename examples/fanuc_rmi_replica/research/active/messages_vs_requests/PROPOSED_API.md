# Proposed Unified API

## Design Principles

1. **Convention over configuration** - Sensible defaults, minimal boilerplate
2. **Explicit is better than implicit** - Clear indication of targeting/auth
3. **Symmetry** - Messages and Requests use parallel APIs
4. **Progressive disclosure** - Simple cases are simple, complex cases are possible
5. **Flexible policies** - Custom policies per-message/request when needed

## Server-Side API

### Current (Inconsistent)
```rust
// Messages: builder pattern with targeting + auth
app.message::<JogCommand, NP>()
   .targeted()
   .with_default_entity_access()
   .register();

// Requests: different API, no targeting, no auth
app.listen_for_request_message::<StartProgram, WebSocketProvider>();
```

### Proposed (Unified)
```rust
// Messages: same as current
app.message::<JogCommand, NP>()
   .targeted()
   .with_default_entity_access()
   .register();

// Requests: parallel builder API
app.request::<SetSpeedOverride, NP>()
   .targeted()
   .with_default_entity_access()
   .register();

// Non-targeted request (current behavior)
app.request::<ListPrograms, NP>()
   .register();

// Custom policies (works for both messages and requests)
app.request::<AdminCommand, NP>()
   .targeted()
   .with_entity_access(EntityAccessPolicy::from_fn(|world, source, entity| {
       // Custom authorization logic
       let roles = world.get_resource::<ClientRoles>()?;
       if roles.has_role(source, "admin") {
           Ok(())
       } else {
           Err("Admin role required".into())
       }
   }))
   .register();

// Non-targeted with message-level policy
app.request::<ServerOnlyCommand, NP>()
   .with_message_access(MessageAccessPolicy::server_only())
   .register();
```

### Handler Types

```rust
// Targeted Request with auth (new)
fn handle_speed(
    mut requests: MessageReader<AuthorizedRequest<SetSpeedOverride>>
) {
    for req in requests.read() {
        let entity = req.entity;        // Target entity (validated)
        let source = req.source;        // Client connection
        let request = req.get_request(); // Payload
        
        // Process...
        req.respond(SetSpeedOverrideResponse { 
            success: true,
            new_speed: 50,
        });
    }
}

// Non-targeted Request (existing)
fn handle_list_programs(
    mut requests: MessageReader<Request<ListPrograms>>
) {
    for req in requests.read() {
        req.respond(ListProgramsResponse { programs: vec![] });
    }
}
```

## Client-Side API

### Current (Inconsistent)
```rust
// Messages: send_message for plain, send_targeted for targeted
let ctx = use_sync_context();
ctx.send_targeted(entity_id, JogCommand { ... });

// Requests: use_request hook, no targeting support
let (send, state) = use_request::<StartProgram>();
send(StartProgram { program_id: 1 });
```

### Proposed (Unified)
```rust
// Targeted message (unchanged)
let ctx = use_sync_context();
ctx.send_targeted(entity_id, JogCommand { ... });

// Non-targeted request (unchanged)
let (send, state) = use_request::<ListPrograms>();
send(ListPrograms {});

// Targeted request (NEW)
let (send, state) = use_targeted_request::<SetSpeedOverride>();
send(entity_id, SetSpeedOverride { speed: 50 });

// state: Signal<RequestState<SetSpeedOverrideResponse>>
match state.get() {
    RequestState::Idle => {},
    RequestState::Pending => { /* show loading */ },
    RequestState::Success(response) => { /* handle response */ },
    RequestState::Error(msg) => { /* show error */ },
}
```

## Wire Format

### TargetedRequest<R>
```rust
/// Wire format for targeted requests
#[derive(Serialize, Deserialize)]
pub struct TargetedRequest<R: RequestMessage> {
    /// Target entity (using entity bits for efficiency)
    pub entity_bits: u64,
    /// Correlation ID for response matching
    pub request_id: u64,
    /// The actual request payload
    pub request: R,
}
```

### Response unchanged
```rust
/// Same response format, no changes needed
#[derive(Serialize, Deserialize)]
pub struct Response<R> {
    pub request_id: u64,
    pub result: Result<R::ResponseMessage, String>,
}
```

## Error Handling

Authorization failures automatically send error responses:

```rust
// Client receives this when auth fails
RequestState::Error("Entity controlled by another client")
RequestState::Error("No client has control. Take control first.")
RequestState::Error("Entity not found")
```

## Policy Methods Summary

| Method | Applies To | Description |
|--------|------------|-------------|
| `.with_entity_access(policy)` | Targeted only | Custom entity-level policy |
| `.with_default_entity_access()` | Targeted only | Use `DefaultEntityAccessPolicy` resource |
| `.with_message_access(policy)` | Non-targeted | Custom message-level policy |
| `.with_default_message_access()` | Non-targeted | Use `DefaultMessageAccessPolicy` resource |

These methods work identically for both `app.message()` and `app.request()`.

## Migration Path

1. **Phase 1**: Add `app.request()` builder API (parallel to `app.message()`)
2. **Phase 2**: Add `use_targeted_request()` client hook
3. **Phase 3**: Migrate existing commands (non-breaking, existing API remains)
4. **Phase 4**: Deprecate `listen_for_request_message()` in favor of `app.request()`

