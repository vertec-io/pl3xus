# Automatic Query Invalidation - Implementation Status

## Overall Status: ✅ Complete (Phase 3 - Truly Automatic)

## Checklist

### Phase 1: Builder Pattern (Completed, Now Deprecated)
- [x] Define `InvalidationRules` resource
- [x] Define `InvalidationRule` struct with mutation type, query name, key extractor
- [x] Add `invalidation_rules()` builder method to App
- [x] Implement `on_success::<T>().invalidate("QueryName")` API
- [x] Support keyed invalidation via `broadcast_invalidations` keys parameter
- [x] ~~Create `MutationResponse` trait for success detection~~ (Removed - orphan rules)
- [x] Implement `broadcast_invalidations` helper function

### Phase 2: Trait-Based with Derive Macro (Complete)
- [x] Define `Invalidates` trait in `pl3xus_sync`
- [x] Implement `#[derive(Invalidates)]` macro in `pl3xus_macros`
- [x] Add `#[invalidates("QueryName", ...)]` attribute support
- [x] Add `broadcast_invalidations_for<T, NP>()` function (uses trait)
- [x] Add `pl3xus_sync` and `pl3xus_macros` as optional deps to `fanuc_replica_types`
- [x] Migrate all request types to use `#[derive(Invalidates)]`
- [x] Remove builder pattern registration from handlers
- [x] Update handlers to use `broadcast_invalidations_for`

### Phase 3: Truly Automatic with respond_and_invalidate (Current - Complete)
- [x] Define `HasSuccess` trait in `pl3xus_common`
- [x] Implement `#[derive(HasSuccess)]` macro in `pl3xus_macros` (validates `success: bool` field)
- [x] Add `RequestInvalidateExt` extension trait in `pl3xus_sync`
- [x] Add `respond_and_invalidate()` method for global invalidation
- [x] Add `respond_and_invalidate_with_keys()` method for keyed invalidation
- [x] Add `#[derive(HasSuccess)]` to all 11 response types
- [x] Migrate all 11 handlers to use `respond_and_invalidate` pattern
- [x] Remove manual `broadcast_invalidations_for` calls from handlers

### Integration
- [x] Handlers use `request.respond_and_invalidate(response, &net)` - single line!
- [x] Success checked automatically via `HasSuccess::is_success()`
- [x] Invalidation targets read from `T::invalidates()` trait method
- [x] Support both targeted and non-targeted requests

### fanuc_rmi_replica Migration
- [x] Add `#[derive(DeriveInvalidates)]` to all 11 mutation request types
- [x] Add `#[derive(HasSuccess)]` to all 11 mutation response types
- [x] All 11 handlers use `respond_and_invalidate` pattern
- [x] Zero manual `broadcast_invalidations_for` calls remain

### Testing
- [x] Playwright E2E test: Create program → ListPrograms invalidated ✅
- [x] Playwright E2E test: Create configuration → GetRobotConfigurations invalidated ✅
- [x] Playwright E2E test: Delete configuration → GetRobotConfigurations invalidated ✅
- [x] Fixed `use_query` hook to properly subscribe to invalidation signal reactively
- [x] Fixed `use_query_targeted` hook to properly subscribe to invalidation signal reactively

### Documentation
- [x] Update STATUS.md with implementation details
- [x] Document API in research folder
- [x] Update API_DESIGN_RESEARCH.md with final decision

## Progress Log

### 2025-12-23 - Started Implementation
- Created status tracking document
- Analyzed current codebase for invalidation patterns
- Identified 12+ handlers with manual invalidation code
- Beginning server-side infrastructure

### 2025-12-23 - Phase 1 Complete (Builder Pattern)
- Implemented `InvalidationRules` resource and builder API in `pl3xus_sync`
- Removed trait-based approach due to Rust orphan rules
- Implemented `broadcast_invalidations` helper function
- Migrated all 11 handlers to use automatic invalidation
- Removed manual `QueryInvalidation` construction from all handlers

### 2025-12-23 - Bug Fix: Query Hooks Not Refetching
- Discovered `use_query` and `use_query_targeted` hooks weren't refetching after invalidation
- Root cause: Effects used `ctx.query_needs_refetch()` which uses `get_untracked()` - no reactive subscription
- Fix: Changed to use `ctx.query_invalidations.get()` to create reactive subscription
- Added `last_invalidation` signal to track counter and prevent duplicate refetches
- Verified fix with Playwright: CreateProgram, CreateConfiguration, DeleteConfiguration all trigger refetch

### 2025-12-23 - Phase 2 Complete (Trait-Based with Derive Macro)
- Implemented `Invalidates` trait in `pl3xus_sync/src/invalidation.rs`
- Implemented `#[derive(Invalidates)]` macro in `pl3xus_macros`
- Added `#[invalidates("Query1", "Query2")]` attribute support
- Added `broadcast_invalidations_for<T, NP>()` function that reads from trait
- Added `pl3xus_sync` and `pl3xus_macros` as optional dependencies to `fanuc_replica_types`
- Migrated all 11 mutation request types to use derive macro
- Removed builder pattern registration from `RequestHandlerPlugin`
- Updated all handlers to use `broadcast_invalidations_for` (no `rules` param needed)

## Design Decisions

### 1. Trait-Based with Derive Macro (Final Decision)
After Phase 1 (builder pattern), we evolved to a trait-based approach:
- Invalidation rules are collocated with type definitions
- Derive macro reduces boilerplate
- No centralized registration needed
- Manual impl available as escape hatch

### 2. String-Based Type Names
Query types are specified as strings in the attribute:
```rust
#[invalidates("ListPrograms", "GetProgram")]
```
**Pros:** Simple, works now
**Cons:** No compile-time checking that the query type exists

We chose simplicity over compile-time safety. Can add validation later if needed.

### 3. Feature-Gated Dependencies
The `Invalidates` trait is only needed on the server:
```toml
[features]
server = ["dep:bevy", "dep:pl3xus_sync", "dep:pl3xus_macros"]
```

This keeps WASM client builds lean.

### 4. Invalidation Timing
Invalidations are sent AFTER the response is sent to the requesting client.
This ensures:
- The mutation is confirmed successful
- The requesting client gets their response first
- Other clients are notified to refetch

## Final API Design

### Type Definition (in shared_types)
```rust
#[cfg(feature = "server")]
use pl3xus_macros::Invalidates as DeriveInvalidates;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "server", derive(DeriveInvalidates))]
#[cfg_attr(feature = "server", invalidates("ListPrograms"))]
pub struct CreateProgram {
    pub name: String,
    pub description: Option<String>,
}

// Multiple invalidations
#[cfg_attr(feature = "server", derive(DeriveInvalidates))]
#[cfg_attr(feature = "server", invalidates("ListPrograms", "GetProgram"))]
pub struct DeleteProgram {
    pub program_id: i64,
}
```

### Handler (Minimal Code)
```rust
fn handle_create_program(
    mut requests: MessageReader<Request<CreateProgram>>,
    db: Option<Res<DatabaseResource>>,
    net: Res<Network<WebSocketProvider>>,
    // No rules parameter needed!
) {
    for request in requests.read() {
        // ... business logic ...
        let success = response.success;
        request.respond(response);

        if success {
            broadcast_invalidations_for::<CreateProgram, _>(&net, None);
        }
    }
}
```

### Keyed Invalidation
For mutations that affect specific resources:
```rust
if success {
    broadcast_invalidations_for::<UpdateProgramSettings, _>(
        &net,
        Some(vec![program_id.to_string()])
    );
}
```

### Manual Implementation (Escape Hatch)
```rust
#[cfg(feature = "server")]
impl pl3xus_sync::Invalidates for ComplexMutation {
    fn invalidates() -> &'static [&'static str] {
        // Complex conditional logic
        &["Query1", "Query2"]
    }
}
```

