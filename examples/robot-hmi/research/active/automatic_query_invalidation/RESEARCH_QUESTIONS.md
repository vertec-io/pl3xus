# Research Questions: Automatic Query Invalidation

## Core Design Questions

### 1. When Should Invalidation Occur?

**Option A: On Mutation Send (optimistic)**
- Invalidate when mutation request is received
- Pro: Faster perceived updates
- Con: May invalidate even if mutation fails

**Option B: On Response Send (confirmed)**
- Invalidate only when successful response is sent
- Pro: Only invalidates on confirmed success
- Con: Requires tracking response status

**Option C: Explicit in Handler (current)**
- Handler calls invalidate manually
- Pro: Full control
- Con: Boilerplate, error-prone

**Recommendation**: Option B - Invalidate on successful response. This matches user expectations.

### 2. How to Determine Success?

For automatic invalidation on success, we need to know if the mutation succeeded.

**Option A: Response Type Convention**
```rust
// Any response with `success: bool` field
pub struct CreateProgramResponse {
    pub success: bool,  // Framework checks this
    ...
}
```

**Option B: Trait-Based**
```rust
impl MutationResponse for CreateProgramResponse {
    fn is_success(&self) -> bool {
        self.success
    }
}
```

**Option C: Registration Predicate**
```rust
app.invalidation_rule()
    .when::<CreateProgram>(|resp| resp.success)
    .invalidate::<ListPrograms>();
```

**Recommendation**: Option B with Option C as fallback. Trait is cleanest.

### 3. How to Handle Keyed Queries?

Some queries have keys (e.g., `GetProgram { id: 5 }`). Should we:

**Option A: Invalidate All Keys**
```rust
// Any UpdateProgramSettings invalidates ALL GetProgram caches
app.invalidation_rule()
    .when::<UpdateProgramSettings>()
    .invalidate::<GetProgram>();  // All keys
```

**Option B: Key Extraction**
```rust
// Only invalidate the specific program
app.invalidation_rule()
    .when::<UpdateProgramSettings>()
    .invalidate_keyed::<GetProgram>(|req| req.program_id.to_string());
```

**Option C: Hybrid**
```rust
// Support both - default to all keys, allow specific
.invalidate::<GetProgram>()  // All
.invalidate_keyed::<GetProgram>(key_fn)  // Specific
```

**Recommendation**: Option C - Provide both options.

### 4. What About Cascading Invalidations?

When `CreateProgram` runs, should we:
- Invalidate `ListPrograms` (direct relationship)
- Also invalidate `GetProgramStatistics` (derived data)
- Also invalidate `DashboardSummary` (aggregate data)

**Option A: Explicit Only**
- Must declare each relationship
- Pro: Predictable, no surprises
- Con: Verbose for complex data models

**Option B: Category-Based**
```rust
app.register_query::<ListPrograms>()
    .category("programs");
    
app.invalidation_rule()
    .when::<CreateProgram>()
    .invalidate_category("programs");  // All program-related queries
```

**Recommendation**: Start with Option A. Add categories as patterns emerge.

### 5. What About Entity-Targeted Mutations?

Current targeted mutations use `use_mutation_targeted::<SetSpeedOverride>(robot_id, ...)`.

How do automatic invalidations work when the mutation targets a specific entity?

**Option A: Invalidate All Instances**
```rust
// SetSpeedOverride on robot 5 invalidates GetRobotStatus for ALL robots
.invalidate::<GetRobotStatus>()
```

**Option B: Entity-Scoped Invalidation**
```rust
// Only invalidate queries for the same entity
.invalidate_same_entity::<GetRobotStatus>()
```

**Recommendation**: Option B when possible. Need framework support.

## Implementation Questions

### 6. How to Track Relationships?

**Option A: HashMap at Registration**
```rust
#[derive(Resource)]
struct InvalidationRules {
    // mutation_type -> list of query_types to invalidate
    rules: HashMap<TypeId, Vec<InvalidationTarget>>,
}

struct InvalidationTarget {
    query_type: TypeId,
    query_type_name: String,
    key_extractor: Option<Box<dyn Fn(&[u8]) -> Option<String>>>,
}
```

**Option B: Message Trait**
```rust
pub trait MutationMessage: RequestMessage {
    fn invalidates() -> &'static [&'static str] {
        &[]  // Default: no invalidations
    }
}
```

### 7. Where Does Invalidation Logic Run?

**Option A: In Response Processing**
- When `request.respond()` is called, check invalidation rules
- Pro: Automatic, no handler changes needed
- Con: Must hook into response path

**Option B: Post-Handler System**
- Separate system runs after request handlers
- Checks which mutations completed successfully
- Broadcasts invalidations

### 8. How to Handle Batch Operations?

If `BatchUpdatePrograms` updates 100 programs, should we:
- Send 100 invalidations for `GetProgram`?
- Send 1 invalidation with 100 keys?
- Send 1 invalidation for all `GetProgram` queries?

**Recommendation**: Single invalidation for the query type, optionally with keys.

## What We Lose with Automatic Invalidation

### 1. Conditional Invalidation

Current pattern allows:
```rust
if some_condition {
    invalidate_queries(...)
}
```

With automatic, it's all-or-nothing based on success.

**Mitigation**: Allow `skip_auto_invalidation()` in handler.

### 2. Granular Timing Control

Current pattern allows invalidating at specific points in complex handlers.

**Mitigation**: Rarely needed. Complex handlers can opt out.

### 3. Cross-Request Relationships

Automatic rules are mutation → query. What about query → query?

**Example**: Fetching `GetRobotStatus` should invalidate cached `GetDashboardSummary`

**Answer**: This is a different pattern (cache dependencies), not mutation-based invalidation.

## Potential Pitfalls

### 1. Over-Invalidation
Automatic rules might invalidate too aggressively, causing unnecessary refetches.

**Mitigation**: Make rules explicit, not inferred.

### 2. Circular Dependencies
Can invalidation rules create loops?

**Answer**: No - invalidations trigger refetches, not mutations. No loops possible.

### 3. Performance
Many mutations might trigger many invalidations.

**Mitigation**: Batch invalidations into single message.

### 4. Debugging
Harder to trace why a query was invalidated.

**Mitigation**: Logging at debug level, include source mutation in message.

