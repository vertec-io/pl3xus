# Partial Component Field Updates Research

## Problem Statement

Currently, pl3xus component mutations send the entire component when any field changes:

```rust
// Current: Client sends entire JogSettingsState to update just cartesian_jog_speed
MutateComponent {
    entity_id: 12345,
    type_name: "JogSettingsState",
    new_value: { cartesian_jog_speed: 15.0, cartesian_jog_step: 1.0, joint_jog_speed: 0.1, ... }
}
```

This is inefficient and doesn't support:
1. **Concurrent field edits** - Two clients editing different fields simultaneously
2. **Large components** - Components with many fields or nested data
3. **Bandwidth optimization** - Sending only what changed
4. **Field-level validation** - Server handlers knowing which specific field was modified

## Use Cases

### ActiveConfigState Individual Field Editing
Currently `ActiveConfigState` has 12 fields. User might want to edit just `u_frame_number`:

```rust
pub struct ActiveConfigState {
    pub loaded_from_id: Option<i64>,
    pub loaded_from_name: Option<String>,
    pub changes_count: u32,
    pub u_frame_number: i32,  // <-- User only wants to edit this
    pub u_tool_number: i32,
    pub front: i32,
    pub up: i32,
    pub left: i32,
    pub flip: i32,
    pub turn4: i32,
    pub turn5: i32,
    pub turn6: i32,
}
```

### IoConfigState per-port configuration
Setting display name for a single I/O port shouldn't require sending entire HashMap.

### Future: Large nested data structures
Programs with many instructions, robot paths, etc.

## Proposed Solutions

### Option A: JSON Patch (RFC 6902)

Use standard JSON Patch format:
```rust
MutateComponentFields {
    entity_id: 12345,
    type_name: "ActiveConfigState",
    patches: [
        { op: "replace", path: "/u_frame_number", value: 5 }
    ]
}
```

**Pros:**
- Industry standard (RFC 6902)
- Well-understood semantics
- Library support (`json-patch` crate)

**Cons:**
- JSON path strings are stringly-typed
- Harder to validate at compile time
- Overhead for simple single-field updates

### Option B: Field ID enum + value

Generate field enum at compile time:
```rust
#[derive(PartialMutate)]
pub struct ActiveConfigState { ... }

// Generated:
enum ActiveConfigStateField {
    LoadedFromId,
    UFrameNumber,
    // ...
}

MutateComponentField {
    entity_id: 12345,
    type_name: "ActiveConfigState",
    field: "u_frame_number",
    value: Json(5)
}
```

**Pros:**
- Type-safe field names
- Simple single-field case
- Server can validate specific field

**Cons:**
- Only handles flat field updates
- Doesn't support nested paths

### Option C: Hybrid - Simple field + optional path

For simple cases, just field name. For complex cases, optional JSON path:
```rust
MutateComponentField {
    entity_id: 12345,
    type_name: "ActiveConfigState",
    field: "u_frame_number",
    path: None,  // Simple field
    value: Json(5)
}

MutateComponentField {
    entity_id: 12345,
    type_name: "IoConfigState",
    field: "configs",
    path: Some("[\"DIN\",1].display_name"),  // Nested path
    value: Json("Limit Switch")
}
```

## Key Research Questions

1. **Conflict resolution**: How do we handle concurrent edits to same/different fields?
2. **Validation**: How do mutation handlers get field-level context?
3. **Client API**: What does `use_mut_component` look like for partial updates?
4. **Derive macro**: Can we generate field update helpers automatically?
5. **Backwards compatibility**: Can we support both full and partial mutations?

## Related Work

- **GraphQL mutations**: Field-level updates with explicit input types
- **Firebase Realtime DB**: Path-based updates with merge semantics
- **Redux**: Action types with partial payload
- **CRDTs**: Conflict-free field-level updates

## Files to Read

- `crates/pl3xus_sync/src/messages.rs` - Current `MutateComponent` message
- `crates/pl3xus_sync/src/registry.rs` - Mutation handling infrastructure
- `crates/pl3xus_client/src/hooks/mutation.rs` - Client-side mutation hooks

## Status

**Phase**: Research
**Priority**: Medium (enables better ActiveConfigState UX)
**Dependencies**: None

