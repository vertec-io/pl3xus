# Automatic Query Invalidation - Implementation Status

## Overall Status: ✅ Complete

## Checklist

### Server-Side Infrastructure
- [x] Define `InvalidationRules` resource
- [x] Define `InvalidationRule` struct with mutation type, query name, key extractor
- [x] Add `invalidation_rules()` builder method to App
- [x] Implement `on_success::<T>().invalidate("QueryName")` API
- [x] Support keyed invalidation via `broadcast_invalidations` keys parameter
- [x] ~~Create `MutationResponse` trait for success detection~~ (Removed - orphan rules)
- [x] Implement `broadcast_invalidations` helper function

### Integration
- [x] Handlers call `broadcast_invalidations` after successful response
- [x] Broadcast invalidations based on registered rules
- [x] Support both targeted and non-targeted requests

### fanuc_rmi_replica Migration
- [x] Register invalidation rules in RequestHandlerPlugin
- [x] Remove manual invalidation from `handle_create_program`
- [x] Remove manual invalidation from `handle_delete_program`
- [x] Remove manual invalidation from `handle_create_robot_connection`
- [x] Remove manual invalidation from `handle_update_robot_connection`
- [x] Remove manual invalidation from `handle_delete_robot_connection`
- [x] Remove manual invalidation from `handle_create_configuration`
- [x] Remove manual invalidation from `handle_update_configuration`
- [x] Remove manual invalidation from `handle_delete_configuration`
- [x] Remove manual invalidation from `handle_set_default_configuration`
- [x] Remove manual invalidation from `handle_update_program_settings`
- [x] Remove manual invalidation from `handle_upload_csv`

### Testing
- [x] Playwright E2E test: Create program → ListPrograms invalidated ✅
- [x] Playwright E2E test: Create configuration → GetRobotConfigurations invalidated ✅
- [x] Playwright E2E test: Delete configuration → GetRobotConfigurations invalidated ✅
- [x] Fixed `use_query` hook to properly subscribe to invalidation signal reactively
- [x] Fixed `use_query_targeted` hook to properly subscribe to invalidation signal reactively

### Documentation
- [x] Update STATUS.md with implementation details
- [x] Document API in research folder

## Progress Log

### 2025-12-23 - Started Implementation
- Created status tracking document
- Analyzed current codebase for invalidation patterns
- Identified 12+ handlers with manual invalidation code
- Beginning server-side infrastructure

### 2025-12-23 - Completed Implementation
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

## Design Decisions

### 1. Registration-Time Declaration (Pattern A)
We chose Pattern A from the research because:
- Centralized rules in plugin setup
- No changes to message types required
- Framework can implement without derive macros
- Easy to extend later

### 2. Explicit Handler Call (Not Trait-Based)
Originally planned to use a `MutationResponse` trait, but Rust's orphan rules prevent
implementing a trait from `pl3xus_sync` on types from `fanuc_replica_types`.

Instead, handlers explicitly call `broadcast_invalidations` after successful responses:
```rust
if success {
    broadcast_invalidations::<CreateProgram, _>(&net, &rules, None);
}
```

This is still a significant improvement because:
- Rules are centralized in one place
- Handlers don't construct `QueryInvalidation` messages manually
- The relationship between mutations and queries is explicit

### 3. Invalidation Timing
Invalidations are sent AFTER the response is sent to the requesting client.
This ensures:
- The mutation is confirmed successful
- The requesting client gets their response first
- Other clients are notified to refetch

## API Design

### Registration
```rust
impl Plugin for RequestHandlerPlugin {
    fn build(&self, app: &mut App) {
        // Register request handlers
        app.listen_for_request_message::<CreateProgram, WebSocketProvider>();

        // Register invalidation rules
        app.invalidation_rules()
            .on_success::<CreateProgram>().invalidate("ListPrograms")
            .on_success::<DeleteProgram>().invalidate("ListPrograms")
            .on_success::<DeleteProgram>().invalidate("GetProgram")
            .on_success::<UpdateProgramSettings>().invalidate("GetProgram");
    }
}
```

### Handler (Minimal Invalidation Code)
```rust
fn handle_create_program(
    mut requests: MessageReader<Request<CreateProgram>>,
    db: Option<Res<DatabaseResource>>,
    net: Res<Network<WebSocketProvider>>,
    rules: Res<InvalidationRules>,
) {
    for request in requests.read() {
        // ... business logic ...
        let success = response.success;
        request.respond(response);

        if success {
            broadcast_invalidations::<CreateProgram, _>(&net, &rules, None);
        }
    }
}
```

### Keyed Invalidation
For mutations that affect specific resources:
```rust
if success {
    broadcast_invalidations::<UpdateProgramSettings, _>(
        &net,
        &rules,
        Some(vec![program_id.to_string()])
    );
}
```

