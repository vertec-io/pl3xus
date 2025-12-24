# Component Mutation Handlers Migration Inventory

## Overview

This document inventories all candidates for migration to the Component Mutation Handler pattern and tracks boilerplate reduction metrics.

## Current State

### Already Migrated ✅

| Component | Client Hook | Server Handler | Boilerplate Saved |
|-----------|-------------|----------------|-------------------|
| `JogSettingsState` | `use_mut_component` | `handle_jog_settings_mutation` | ~80 lines |

### Migration Candidates

#### Priority 1: Direct Component Mutations (High Value)

These components are already synced and could benefit from mutation handlers:

| Component | Current Pattern | Migration Benefit | Estimated Savings |
|-----------|-----------------|-------------------|-------------------|
| `FrameToolDataState` | `use_mutation<SetActiveFrameTool>` + separate sync | Unified read/write, no separate request type | ~60 lines |
| `ActiveConfigState` | `sync_component` (no handler) + `LoadConfiguration` request | Add validation handler, remove separate request | ~40 lines |
| `IoConfigState` | `use_mutation<UpdateIoConfig>` + separate sync | Unified read/write | ~50 lines |

#### Priority 2: Request/Response → Component Mutation

These use request/response patterns that could be replaced with component mutations:

| Request Type | Related Component | Current Lines | Potential Savings |
|--------------|-------------------|---------------|-------------------|
| `UpdateJogSettings` | `JogSettingsState` | 45 lines | **ALREADY MIGRATED** |
| `SetActiveFrameTool` | `FrameToolDataState` | 55 lines | ~40 lines |
| `UpdateIoConfig` | `IoConfigState` | 40 lines | ~30 lines |

#### Priority 3: Not Suitable for Migration

These should remain as request/response patterns:

| Request Type | Reason |
|--------------|--------|
| `CreateProgram`, `DeleteProgram` | Database operations, not component state |
| `CreateRobotConnection`, etc. | Database CRUD, not ECS components |
| `LoadProgram`, `StartProgram`, etc. | Commands, not state mutations |
| `WriteFrameData`, `WriteToolData` | Robot I/O operations, not component state |
| `ReadDin`, `WriteDout`, etc. | Direct robot I/O, not component state |

## Detailed Analysis

### FrameToolDataState Migration

**Current Pattern (frame_panel.rs + tool_panel.rs):**
```rust
// Client: Two separate hooks
let (frame_tool_state, _) = use_entity_component::<FrameToolDataState, _>(...);
let set_frame_tool = use_mutation::<SetActiveFrameTool>(...);

// Server: Separate request handler + component sync
app.sync_component::<FrameToolDataState>(None);
// Plus ~55 lines in handle_set_active_frame_tool
```

**After Migration:**
```rust
// Client: Single unified hook
let handle = use_mut_component::<FrameToolDataState, _>(...);
// handle.value for reading, handle.mutate() for writing

// Server: Component sync with handler
app.sync_component_builder::<FrameToolDataState>()
    .with_handler::<WebSocketProvider, _, _>(handle_frame_tool_mutation)
    .build();
```

**Savings:**
- Remove `SetActiveFrameTool` request type (~15 lines in shared_types)
- Remove `handle_set_active_frame_tool` handler (~55 lines)
- Simplify client code (~20 lines across frame_panel.rs and tool_panel.rs)
- **Total: ~90 lines removed, ~30 lines added = ~60 lines saved**

### ActiveConfigState Migration

**Current Pattern:**
- Component synced without handler
- `LoadConfiguration` request for changing active config
- Client uses `use_entity_component` + `use_mutation<LoadConfiguration>`

**After Migration:**
- Add mutation handler for validation
- Keep `LoadConfiguration` for database-backed config loading (different use case)
- **Savings: ~40 lines** (validation logic consolidated)

### IoConfigState Migration

**Current Pattern:**
- `UpdateIoConfig` request handler
- Separate component sync

**After Migration:**
- Unified mutation handler
- **Savings: ~50 lines**

## Boilerplate Metrics

### Lines of Code Analysis

| Category | Before Migration | After Migration | Reduction |
|----------|------------------|-----------------|-----------|
| JogSettingsState (completed) | 125 lines | 45 lines | **80 lines (64%)** |
| FrameToolDataState (planned) | 110 lines | 50 lines | ~60 lines (55%) |
| IoConfigState (planned) | 90 lines | 40 lines | ~50 lines (56%) |
| **Total Projected** | **325 lines** | **135 lines** | **~190 lines (58%)** |

### Pattern Comparison

**Old Pattern (per component):**
1. Define request type in shared_types (~15 lines)
2. Define response type in shared_types (~10 lines)
3. Register request listener in server (~1 line)
4. Write request handler in server (~40-60 lines)
5. Sync component separately (~1 line)
6. Client: use_entity_component + use_mutation (~15 lines)

**New Pattern (per component):**
1. Component already defined in shared_types (no change)
2. Register with mutation handler (~3 lines)
3. Write mutation handler (~25-35 lines)
4. Client: use_mut_component (~8 lines)

**Per-component savings: ~40-60 lines (50-60%)**

## Migration Order

1. **FrameToolDataState** - High value, clear mapping
2. **IoConfigState** - Medium value, straightforward
3. **ActiveConfigState** - Lower priority, may need to keep LoadConfiguration

## Next Steps

1. [ ] Migrate FrameToolDataState to mutation handler pattern
2. [ ] Migrate IoConfigState to mutation handler pattern  
3. [ ] Evaluate ActiveConfigState migration (may be partial)
4. [ ] Update metrics after each migration
5. [ ] Document lessons learned

## Notes

- The mutation handler pattern is best for components where:
  - Client needs both read and write access
  - Server needs to validate/transform mutations
  - The mutation is a direct state change (not a command)
  
- Keep request/response for:
  - Database CRUD operations
  - Commands that trigger complex workflows
  - Operations that don't map to component state

