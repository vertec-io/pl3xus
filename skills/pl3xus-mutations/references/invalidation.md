# Query Invalidation Reference

## Overview

When a mutation modifies server state, related queries may become stale. pl3xus provides a mechanism to invalidate queries so clients refetch fresh data.

## Invalidation Flow

```
Client A                    Server                    All Clients
   │                           │                           │
   │──CreateProgram───────────▶│                           │
   │                           │ (create entity)           │
   │                           │                           │
   │◀──Response───────────────│                           │
   │                           │──QueryInvalidation───────▶│
   │                           │  (ListPrograms)           │
   │                           │                           │
   │ (refetch ListPrograms)    │                           │
   │                           │◀──ListPrograms───────────│
   │                           │──Response────────────────▶│
```

## Declaring Invalidations

### Using Derive Macro

```rust
use pl3xus_macros::Invalidates;

#[derive(Clone, Debug, Serialize, Deserialize, Invalidates)]
#[invalidates("ListPrograms")]
pub struct CreateProgram {
    pub name: String,
}
```

### Multiple Invalidations

```rust
#[derive(Clone, Debug, Serialize, Deserialize, Invalidates)]
#[invalidates("ListPrograms", "GetProgram", "GetProgramStats")]
pub struct DeleteProgram {
    pub program_id: u64,
}
```

## Broadcasting Invalidations

### After Successful Mutation

```rust
use pl3xus_sync::broadcast_invalidations_for;

fn handle_create_program(
    mut messages: MessageReader<NetworkData<TargetedRequest<CreateProgram>>>,
    mut commands: Commands,
    net: Res<Network<WebSocketProvider>>,
) {
    for request in messages.read() {
        let program = commands.spawn((
            Name::new(&request.message.request.name),
            ProgramData::default(),
        )).id();
        
        let response = CreateProgramResponse {
            success: true,
            program_id: Some(program.to_bits()),
            error: None,
        };
        
        if let Ok(()) = request.respond(response.clone()) {
            if response.success {
                // Broadcast to all clients
                broadcast_invalidations_for::<CreateProgram, _>(&net, None);
            }
        }
    }
}
```

### To Specific Client

```rust
// Invalidate only for the requesting client
broadcast_invalidations_for::<CreateProgram, _>(&net, Some(request.source));
```

## Client-Side Handling

### Automatic Refetch

When a query is invalidated, the client automatically marks it as stale. If the query is currently being observed (component is mounted), it will refetch.

### Manual Refetch

```rust
let (fetch, state) = use_request::<ListPrograms>();

// Refetch when needed
let refetch = move || {
    fetch(ListPrograms);
};

view! {
    <button on:click=move |_| refetch()>
        "Refresh"
    </button>
    <Show when=move || state.get().is_stale>
        <span class="stale-indicator">"Data may be outdated"</span>
    </Show>
}
```

## Invalidation Patterns

### Create Operations

```rust
#[derive(Invalidates)]
#[invalidates("ListItems")]
pub struct CreateItem { ... }
```

### Update Operations

```rust
#[derive(Invalidates)]
#[invalidates("GetItem", "ListItems")]
pub struct UpdateItem { ... }
```

### Delete Operations

```rust
#[derive(Invalidates)]
#[invalidates("GetItem", "ListItems", "GetItemStats")]
pub struct DeleteItem { ... }
```

### Batch Operations

```rust
#[derive(Invalidates)]
#[invalidates("ListItems", "GetItemStats", "GetBatchStatus")]
pub struct BatchUpdateItems { ... }
```

## Best Practices

### 1. Be Specific

Only invalidate queries that are actually affected:

```rust
// ✅ Good - specific invalidation
#[invalidates("ListPrograms")]
pub struct CreateProgram { ... }

// ❌ Bad - over-invalidation
#[invalidates("ListPrograms", "ListRobots", "GetSystemStatus", "GetConfig")]
pub struct CreateProgram { ... }
```

### 2. Consider Hierarchy

If updating a parent affects children:

```rust
#[invalidates("GetSystem", "ListRobots", "GetRobotStatus")]
pub struct UpdateSystemConfig { ... }
```

### 3. Conditional Invalidation

Only broadcast on success:

```rust
if response.success {
    broadcast_invalidations_for::<T, _>(&net, None);
}
```

## Query Cache

The client maintains a query cache that tracks:
- Query type name
- Query key (serialized request)
- Cached response data
- Stale status
- Last fetch time

When invalidated:
1. Query is marked as stale
2. If query is observed, refetch is triggered
3. UI shows stale indicator (optional)
4. Fresh data replaces cached data

