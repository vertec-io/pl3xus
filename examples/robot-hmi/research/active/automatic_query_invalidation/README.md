# Automatic Query Invalidation Research

## Problem Statement

Currently, request handlers must manually invalidate queries when they make changes that affect cached data. This creates:

1. **Boilerplate**: Every mutation handler repeats the same invalidation pattern
2. **Error-prone**: Easy to forget to invalidate, causing stale data
3. **Coupling**: Handler must know which queries are affected by its changes

### Current Pattern

```rust
fn handle_create_program(
    mut requests: MessageReader<Request<CreateProgram>>,
    db: Option<Res<DatabaseResource>>,
    net: Res<Network<WebSocketProvider>>,
) {
    for request in requests.read() {
        let result = db.create_program(&inner.name, ...);
        
        match result {
            Ok(program_id) => {
                // MANUAL: Must remember to invalidate related queries
                net.broadcast(SyncServerMessage::QueryInvalidation(QueryInvalidation {
                    query_types: vec!["ListPrograms".to_string()],
                    keys: None,
                }));
                
                request.respond(CreateProgramResponse { success: true, ... });
            }
            Err(e) => request.respond(CreateProgramResponse { success: false, ... }),
        }
    }
}
```

**Problems**:
- 5+ lines of boilerplate per mutation
- Must manually track `CreateProgram` -> `ListPrograms` relationship
- Easy to forget when adding new handlers
- No compiler help if relationship changes

## Proposed Solution: Declarative Invalidation

### Concept

Declare query-mutation relationships at registration time:

```rust
// During plugin setup
app.register_query::<ListPrograms>()
    .invalidated_by::<CreateProgram>()
    .invalidated_by::<DeleteProgram>()
    .invalidated_by::<UpdateProgramSettings>();

app.register_query::<GetProgram>()
    .invalidated_by_keyed::<UpdateProgramSettings>(|req| req.program_id.to_string());
```

The framework automatically invalidates queries when the corresponding mutations complete successfully.

## Research Questions

See `RESEARCH_QUESTIONS.md` for detailed analysis.

## API Design Options

### Option A: Registration-Time Declaration

```rust
impl Plugin for RequestsPlugin {
    fn build(&self, app: &mut App) {
        // Standard request registration
        app.listen_for_request_message::<CreateProgram, WebSocketProvider>();
        
        // Declare invalidation relationships
        app.invalidation_rule()
            .when::<CreateProgram>(ResponsePredicate::Success)  // Only on success
            .invalidate::<ListPrograms>();
    }
}
```

### Option B: Attribute-Based

```rust
#[derive(Serialize, Deserialize)]
#[invalidates(ListPrograms)]  // Derive macro adds the relationship
pub struct CreateProgram {
    pub name: String,
}
```

### Option C: Handler Return Type

```rust
fn handle_create_program(...) -> impl IntoQueryInvalidation {
    // ... business logic ...
    
    if success {
        Ok(response).with_invalidations(["ListPrograms"])
    } else {
        Err(response)
    }
}
```

## Comparison

See `COMPARISON.md` for pros/cons of each approach.

## Implementation Considerations

See `IMPLEMENTATION.md` for technical details.

