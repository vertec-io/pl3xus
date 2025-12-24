# December 2025 Session: Accomplishments

## Summary

This session focused on **API redesign** for the pl3xus framework, moving from low-level request/response patterns to a TanStack Query-inspired API with proper caching, server-side invalidation, and ergonomic hooks.

---

## 1. TanStack Query-Inspired API Implementation

### New Hooks Created

| Hook | Purpose | Notes |
|------|---------|-------|
| `use_query<R>()` | Fetch data on mount, cached until invalidation | Returns `QueryHandle<R>` |
| `use_query_keyed<R, K>()` | Fetch with dynamic key, null key skips fetch | For conditional fetching |
| `use_query_targeted<R>()` | Entity-targeted queries | Includes entity_id in request |
| `use_mutation<R>(callback)` | Fire-and-forget with response handler | Returns `MutationHandle<R>` |
| `use_mutation_targeted<R>(callback)` | Entity-targeted mutations | Includes authorization |

### Key Features

- **Automatic caching**: Queries with same type share cached data
- **Server-side invalidation**: Server pushes `QueryInvalidation` messages
- **Deduplication**: Multiple components requesting same query share one request
- **Stale-while-revalidate**: Shows cached data while refetching

### Files Changed

- `crates/pl3xus_client/src/hooks.rs` - Added all new hooks
- `crates/pl3xus_sync/src/messages.rs` - Added `QueryInvalidation` message type
- `crates/pl3xus_sync/src/registration.rs` - Added invalidation helpers

---

## 2. Server-Side Query Invalidation

Implemented server-side helpers that broadcast invalidation to all clients:

```rust
// Invalidate all queries of a type
invalidate_queries::<ListPrograms>(&mut sync_state);

// Invalidate specific keys
invalidate_queries_with_keys::<GetProgram, _>(&mut sync_state, &[program_id]);

// Invalidate all queries (nuclear option)
invalidate_all_queries(&mut sync_state);
```

### Files Changed

- `crates/pl3xus_sync/src/lib.rs` - Added exports
- `examples/fanuc_rmi_replica/server/src/plugins/requests.rs` - Added to all mutation handlers

---

## 3. Fixed Message Batching Bug

**Problem**: Multiple messages sent in a single WebSocket frame were being dropped after the first.

**Root Cause**: `SyncClientMessage` decoder was only extracting the first message, ignoring remaining bytes.

**Solution**: Added batch message extraction in provider.rs using length-prefixed decoding.

### Files Changed

- `crates/pl3xus_client/src/provider.rs` - Added `extract_batch_messages()` function

---

## 4. Fixed Entity Targeting Issues

### Problem 1: ConnectionState on Wrong Entity

Client components were subscribing to `ConnectionState` on the **system entity**, but it actually lives on the **robot entity**.

**Fixed Files** (all changed from `system_entity_id` to `robot_entity_id`):
- `layout/top_bar.rs` - TopBar, ConnectionDropdown, QuickSettingsButton, ConnectionStateHandler
- `layout/right_panel.rs` - RightPanel
- `components/status_panel.rs` - StatusPanel
- `control/mod.rs`, `quick_commands.rs`, `command_input.rs`, `joint_jog.rs`
- `info/mod.rs`, `active_config.rs`, `jog_defaults.rs`, `frame_panel.rs`, `tool_panel.rs`
- `info/frame_display.rs`, `tool_display.rs`

### Problem 2: Quick Settings Popup Closing Immediately

**Root Cause**: Effect handling ConnectToRobot response was running on mount with stale data.

**Solution**: Added guard to only process responses when `connecting_to_id` is set.

---

## 5. Hook Renaming for Clarity

Renamed hooks to be more intuitive:

| Old Name | New Name |
|----------|----------|
| `use_sync_component` | `use_components` |
| `use_sync_entity_component` | `use_entity_component` |
| `use_sync_component_store` | `use_component_store` |

---

## 6. Marker Component Renaming

Renamed marker components to Active* pattern:

| Old Name | New Name | Purpose |
|----------|----------|---------|
| `SystemMarker` | `ActiveSystem` | Marks the active system entity |
| `RobotMarker` | `ActiveRobot` | Marks the active robot entity |

This allows the server to move the marker to indicate which robot is currently selected/active.

---

## 7. Builder Pattern Registration API

Implemented ergonomic message/request registration:

```rust
app.message::<JogCommand>()
    .targeted()
    .with_entity_policy(ExclusiveControlPolicy)
    .register();

app.request::<SetSpeedOverride>()
    .targeted()
    .with_entity_policy(ExclusiveControlPolicy)
    .register();
```

---

## 8. ExclusiveControlPlugin Enhancement

Added sub-connection support for multiple browser tabs from same user.

---

## 9. Migrated All Client Code to New API

Migrated all use_request calls to appropriate patterns:
- Queries → `use_query` / `use_query_keyed`
- Mutations → `use_mutation` / `use_mutation_targeted`
- Imperative triggers → `use_request` (kept for I/O polling pattern)

---

## Documentation Created

| Document | Location |
|----------|----------|
| Query API Research | `docs/research/QUERY_API_RESEARCH.md` |
| This session docs | `research/active/december_2024_session/` |

