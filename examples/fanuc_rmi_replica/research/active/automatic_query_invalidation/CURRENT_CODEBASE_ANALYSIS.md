# Current Codebase Analysis: Query Invalidation

## Files Analyzed

- `examples/fanuc_rmi_replica/server/src/plugins/requests.rs`
- `crates/pl3xus_sync/src/lib.rs` (invalidation helpers)
- `crates/pl3xus_client/src/hooks.rs` (client handling)

## Current Invalidation Points

### In `requests.rs`

| Handler | Invalidates | Lines of Code |
|---------|-------------|---------------|
| `handle_create_configuration` | `GetRobotConfigurations` | 6 |
| `handle_update_configuration` | `GetRobotConfigurations` | 6 |
| `handle_delete_configuration` | `GetRobotConfigurations` | 6 |
| `handle_load_configuration` | `GetRobotConfigurations` | 6 |
| `handle_create_program` | `ListPrograms` | 5 |
| `handle_delete_program` | `ListPrograms` | 5 |

**Total**: ~34 lines of boilerplate

### Code Pattern (Repeated 6+ Times)

```rust
// Invalidate configuration queries so all clients refetch
if should_invalidate {
    if let Some(net) = net {
        let invalidation = QueryInvalidation {
            query_types: vec!["GetRobotConfigurations".to_string()],
            keys: None,
        };
        net.broadcast(SyncServerMessage::QueryInvalidation(invalidation));
        info!("📢 Broadcast query invalidation for GetRobotConfigurations");
    }
}
```

## Missing Invalidations

### Potential Issues Found

1. **`handle_update_program_settings`** - Should it invalidate `ListPrograms`?
   - If program name/description changes, list should update
   - Currently: Unknown if invalidation exists

2. **`handle_upload_csv`** - Changes program data
   - Should invalidate `GetProgram` for specific program
   - Currently: Unknown

## Available Helper Functions

### In `crates/pl3xus_sync/src/lib.rs`

```rust
/// Broadcast query invalidations to all connected clients.
pub fn invalidate_queries<NP: NetworkProvider>(world: &World, query_types: &[&str])

/// Invalidate queries for specific keys.
pub fn invalidate_queries_with_keys<NP: NetworkProvider>(
    world: &World,
    query_types: &[&str],
    keys: &[String],
)

/// Invalidate all queries on all clients.
pub fn invalidate_all_queries<NP: NetworkProvider>(world: &World)
```

**Note**: These helpers exist but handlers don't always use them. Instead, they manually construct the message.

## Relationship Map

Based on current codebase:

```
CreateProgram       → invalidates → ListPrograms
DeleteProgram       → invalidates → ListPrograms
UpdateProgramSettings → should → ListPrograms, GetProgram(id)

CreateConfiguration → invalidates → GetRobotConfigurations
UpdateConfiguration → invalidates → GetRobotConfigurations
DeleteConfiguration → invalidates → GetRobotConfigurations
LoadConfiguration   → invalidates → GetRobotConfigurations

UploadCsv          → should → GetProgram(id)
```

## Opportunities for Automatic Invalidation

### High Value (Clear 1:1 Mapping)

| Mutation | Query | Key |
|----------|-------|-----|
| CreateProgram | ListPrograms | None |
| DeleteProgram | ListPrograms | None |
| CreateConfiguration | GetRobotConfigurations | None |
| UpdateConfiguration | GetRobotConfigurations | None |
| DeleteConfiguration | GetRobotConfigurations | None |

### Medium Value (Needs Keyed Invalidation)

| Mutation | Query | Key Extraction |
|----------|-------|----------------|
| UpdateProgramSettings | GetProgram | `req.program_id` |
| UploadCsv | GetProgram | `req.program_id` |

### Complex (May Not Benefit)

- Operations with conditional invalidation
- Operations where invalidation depends on response content

## Framework Support Needed

### Current State

- ✅ `QueryInvalidation` message exists
- ✅ Client handles invalidation and refetches
- ✅ Helper functions exist (`invalidate_queries`)
- ❌ No automatic relationship tracking
- ❌ No success-based invalidation triggering
- ❌ No registration-time declaration

### Required Additions

1. **InvalidationRules Resource**
```rust
#[derive(Resource, Default)]
struct InvalidationRules {
    rules: Vec<InvalidationRule>,
}

struct InvalidationRule {
    mutation_type: TypeId,
    mutation_name: String,
    query_name: String,
    key_extractor: Option<Box<dyn Fn(&[u8]) -> Option<String>>>,
}
```

2. **Registration Extension**
```rust
trait AppInvalidationExt {
    fn invalidation_rules(&mut self) -> InvalidationRuleBuilder;
}
```

3. **Response Interception**
- Hook into `request.respond()` to check rules
- If response indicates success, trigger invalidations

## Estimated Impact

| Metric | Current | With Auto-Invalidation |
|--------|---------|------------------------|
| Lines of invalidation code in handlers | ~34 | ~5 (registration only) |
| Places to update when adding query | N handlers | 1 (registration) |
| Risk of forgotten invalidation | High | Low |
| Flexibility for complex cases | Full | Opt-out needed |

