# Comparison: Current vs Proposed Query Invalidation

## Current Pattern: Manual Invalidation

### Example: CreateProgram Handler

```rust
fn handle_create_program(
    mut requests: MessageReader<Request<CreateProgram>>,
    db: Option<Res<DatabaseResource>>,
    net: Res<Network<WebSocketProvider>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        
        let result = db.as_ref()
            .map(|db| db.create_program(&inner.name, inner.description.as_deref()))
            .unwrap_or(Err(anyhow::anyhow!("Database not available")));

        match result {
            Ok(program_id) => {
                // BOILERPLATE: Manually invalidate related queries
                net.broadcast(SyncServerMessage::QueryInvalidation(QueryInvalidation {
                    query_types: vec!["ListPrograms".to_string()],
                    keys: None,
                }));
                
                request.respond(CreateProgramResponse {
                    success: true,
                    program_id: Some(program_id),
                    error: None,
                });
            }
            Err(e) => {
                request.respond(CreateProgramResponse {
                    success: false,
                    program_id: None,
                    error: Some(e.to_string()),
                });
            }
        }
    }
}
```

**Lines for invalidation**: ~6 (inside success branch)

### Same Pattern Repeated

```rust
// handle_update_configuration
net.broadcast(SyncServerMessage::QueryInvalidation(QueryInvalidation {
    query_types: vec!["GetRobotConfigurations".to_string()],
    keys: None,
}));

// handle_delete_configuration
net.broadcast(SyncServerMessage::QueryInvalidation(QueryInvalidation {
    query_types: vec!["GetRobotConfigurations".to_string()],
    keys: None,
}));

// handle_create_configuration
net.broadcast(SyncServerMessage::QueryInvalidation(QueryInvalidation {
    query_types: vec!["GetRobotConfigurations".to_string()],
    keys: None,
}));
```

**Total**: ~24 lines of repeated boilerplate across 4 handlers (just for configs)

---

## Proposed Pattern A: Registration-Time Declaration

### Plugin Setup

```rust
impl Plugin for RequestsPlugin {
    fn build(&self, app: &mut App) {
        // Handlers
        app.listen_for_request_message::<CreateProgram, WebSocketProvider>();
        app.listen_for_request_message::<DeleteProgram, WebSocketProvider>();
        
        // Invalidation rules (declared once)
        app.invalidation_rules()
            .on_success::<CreateProgram>().invalidate::<ListPrograms>()
            .on_success::<DeleteProgram>().invalidate::<ListPrograms>()
            .on_success::<CreateConfiguration>().invalidate::<GetRobotConfigurations>()
            .on_success::<UpdateConfiguration>().invalidate::<GetRobotConfigurations>()
            .on_success::<DeleteConfiguration>().invalidate::<GetRobotConfigurations>();
    }
}
```

### Handler (No Invalidation Code!)

```rust
fn handle_create_program(
    mut requests: MessageReader<Request<CreateProgram>>,
    db: Option<Res<DatabaseResource>>,
    // Note: `net` not needed for invalidation anymore
) {
    for request in requests.read() {
        let inner = request.get_request();
        
        let result = db.as_ref()
            .map(|db| db.create_program(&inner.name, inner.description.as_deref()))
            .unwrap_or(Err(anyhow::anyhow!("Database not available")));

        let response = match result {
            Ok(program_id) => CreateProgramResponse {
                success: true,
                program_id: Some(program_id),
                error: None,
            },
            Err(e) => CreateProgramResponse {
                success: false,
                program_id: None,
                error: Some(e.to_string()),
            },
        };
        
        // Framework handles invalidation based on response.success
        request.respond(response);
    }
}
```

**Lines saved per handler**: ~6

---

## Summary Comparison

| Aspect | Manual | Automatic (Registration) |
|--------|--------|--------------------------|
| Handler complexity | Includes invalidation logic | Pure business logic |
| Where rules live | Scattered in handlers | Centralized in plugin |
| Forgot to invalidate? | Runtime bug, stale data | Compiler won't catch, but centralized review |
| Conditional invalidation | Full control | Opt-out mechanism needed |
| New query types | Update all related handlers | Update registration |
| Lines per handler | +6 for invalidation | 0 |
| Total boilerplate | O(handlers × queries) | O(relationships) |

---

## Proposed Pattern B: Attribute-Based

### Type Definition

```rust
#[derive(Serialize, Deserialize, RequestMessage)]
#[invalidates(ListPrograms)]
pub struct CreateProgram {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize, RequestMessage)]
#[invalidates(ListPrograms)]
pub struct DeleteProgram {
    pub program_id: i64,
}

#[derive(Serialize, Deserialize, RequestMessage)]
#[invalidates(GetRobotConfigurations)]
pub struct CreateConfiguration {
    pub name: String,
    // ...
}
```

### Pros/Cons

**Pros**:
- Self-documenting: relationship visible at type definition
- Compile-time checked (attribute must reference valid type)
- No separate registration needed

**Cons**:
- Couples message types to query types (both must be in scope)
- Less flexible (can't change relationships at runtime)
- Macro complexity

---

## Recommendation

**Start with Pattern A (Registration-Time)**:
- Centralized, explicit rules
- No changes to message types
- Framework can implement without derive macros
- Easy to extend later

**Consider Pattern B later** if the pattern stabilizes and we want tighter coupling.

