---
title: 'Query API Research: TanStack Query / leptos_query Patterns for pl3xus'
---
# Query API Research: TanStack Query / leptos_query Patterns for pl3xus

## Executive Summary

This document analyzes TanStack Query, leptos_query, and leptos-fetch to design a best-in-class request/query API for pl3xus. The goal is to create an ergonomic, type-safe API that handles the unique requirements of pl3xus's WebSocket-based, entity-targeted, real-time synchronized architecture.

**Key Innovation: Server-Side Invalidation**

Unlike traditional HTTP-based query libraries that rely on client-side TTL/stale-while-revalidate patterns, pl3xus can leverage its persistent WebSocket connection for **server-pushed invalidation**. This provides:

1. **Always accurate data**: Server knows when data changes and tells clients immediately
2. **No wasted fetches**: No polling or background refetching needed
3. **Reduced complexity**: No client-side cache invalidation logic
4. **Industrial-grade reliability**: Critical for robotics/real-time applications

---

## The pl3xus Data Model

### Three Tiers of Server State

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        SERVER (Bevy ECS + Database)                      │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────┐  │
│  │  Synced State   │  │  Query State    │  │   Command Responses     │  │
│  │  (Components)   │  │  (DB/Computed)  │  │   (Mutations)           │  │
│  ├─────────────────┤  ├─────────────────┤  ├─────────────────────────┤  │
│  │ RobotStatus     │  │ ListPrograms    │  │ CreateProgram           │  │
│  │ JointPositions  │  │ ListRobots      │  │ DeleteConfiguration     │  │
│  │ IOStates        │  │ GetProgram      │  │ SetSpeedOverride        │  │
│  │ EntityControl   │  │ GetConfigs      │  │ InitializeRobot         │  │
│  └────────┬────────┘  └────────┬────────┘  └────────────┬────────────┘  │
│           │                    │                        │               │
└───────────┼────────────────────┼────────────────────────┼───────────────┘
            │                    │                        │
            │ PUSH               │ PULL+INVALIDATE        │ REQUEST/RESPONSE
            │ (real-time)        │ (on-demand)            │ (one-time)
            ▼                    ▼                        ▼
┌───────────────────────────────────────────────────────────────────────────┐
│                          CLIENT (Leptos WASM)                             │
├───────────────────────────────────────────────────────────────────────────┤
│                                                                           │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────┐   │
│  │ use_components  │  │ use_query       │  │ use_mutation            │   │
│  │                 │  │                 │  │                         │   │
│  │ Always current  │  │ Cached until    │  │ Fire-and-forget with    │   │
│  │ Server pushes   │  │ server says     │  │ response handling       │   │
│  │ every change    │  │ "stale"         │  │                         │   │
│  └─────────────────┘  └─────────────────┘  └─────────────────────────┘   │
│                                                                           │
└───────────────────────────────────────────────────────────────────────────┘
```

### When to Use Each Pattern

| Pattern | Use Case | Examples |
|---------|----------|----------|
| `use_components<T>()` | Real-time state that changes frequently | Robot position, joint angles, I/O states, control status |
| `use_query<R>()` | Data fetched on-demand, cached until invalidated | Program list, configuration list, database records |
| `use_mutation<R>()` | Commands that modify server state | Create/update/delete operations, robot commands |

## Current pl3xus API Analysis

### What We Have Now

```rust
// Basic request (returns state signal)
let (send, state) = use_request::<ListRobots>();
send(ListRobots);
// state.get().is_loading(), state.get().data, state.get().error

// Request with handler (eliminates Effect boilerplate)
let send = use_request_with_handler::<LoadProgram, _>(move |result| {
    match result {
        Ok(r) if r.success => toast.success("Loaded"),
        Ok(r) => toast.error(format!("Failed: {}", r.error.as_deref().unwrap_or(""))),
        Err(e) => toast.error(format!("Error: {e}")),
    }
});
send(LoadProgram { program_id: 42 });

// Targeted request (entity-specific with authorization)
let (send, state) = use_targeted_request::<SetSpeedOverride>();
send(entity_id, SetSpeedOverride { value: 50.0 });

// Targeted request with handler
let send = use_targeted_request_with_handler::<AbortMotion, _>(move |result| { ... });
send(entity_id, AbortMotion);
```

### Current Pain Points

1. **Naming**: `use_targeted_request_with_handler` is verbose and awkward
2. **No caching**: Each request is independent, no deduplication
3. **No query keys**: Can't invalidate related queries
4. **No background refetching**: Manual refresh only
5. **Two-level success**: Transport success vs business logic success (`response.success`)

## TanStack Query Core Concepts

### Key Patterns

1. **Queries vs Mutations**: Clear separation
   - Queries: Read data, cacheable, auto-refetch
   - Mutations: Write data, side effects, invalidate queries

2. **Query Keys**: Unique identifiers for caching/invalidation
   ```typescript
   useQuery({ queryKey: ['todos', todoId], queryFn: fetchTodo })
   ```

3. **Mutation Callbacks**: `onMutate`, `onSuccess`, `onError`, `onSettled`
   ```typescript
   useMutation({
     mutationFn: addTodo,
     onSuccess: (data) => { ... },
     onError: (error) => { ... },
   })
   ```

4. **Query Invalidation**: After mutation, invalidate related queries
   ```typescript
   queryClient.invalidateQueries({ queryKey: ['todos'] })
   ```

5. **Optimistic Updates**: Update UI before server confirms

## leptos_query Patterns

```rust
// Create a query scope (defines the fetcher and options)
fn track_query() -> QueryScope<TrackId, TrackData> {
    create_query(get_track, QueryOptions::default())
}

// Use the query in a component
let QueryResult { data, .. } = track_query().use_query(move || id.clone());
```

Key features:
- `QueryScope<K, V>`: Type-safe query definition
- `QueryResult`: Reactive result with `data`, `is_loading`, `is_error`
- `QueryOptions`: Stale time, cache time, refetch intervals
- `QueryClient`: Global cache management

## Proposed pl3xus API Design

### Design Principles

1. **Embrace pl3xus's unique model**: WebSocket, real-time sync, entity-targeted
2. **Separate concerns**: Queries (read) vs Mutations (write)
3. **Ergonomic naming**: Short, clear, memorable
4. **Type-safe**: Compile-time guarantees
5. **Minimal boilerplate**: Handler pattern as default

### Proposed API

#### 1. Mutations (Write Operations)

```rust
// Simple mutation with inline handler
let abort = use_mutation::<AbortMotion>(move |result| {
    match result {
        Ok(r) if r.success => toast.warning("Motion aborted"),
        Ok(r) => toast.error(format!("Denied: {}", r.error())),
        Err(e) => toast.error(format!("Failed: {e}")),
    }
});
abort.send(AbortMotion);

// Targeted mutation (entity-specific)
let set_speed = use_mutation_targeted::<SetSpeedOverride>(move |result| { ... });
set_speed.send(entity_id, SetSpeedOverride { value: 50.0 });

// Access state if needed
if set_speed.is_loading() { ... }
```

#### 2. Queries (Read Operations)

```rust
// Simple query - auto-fetches on mount
let robots = use_query::<ListRobots>(|| ListRobots);
// robots.data(), robots.is_loading(), robots.refetch()

// Query with key for caching/invalidation
let program = use_query_keyed::<GetProgram>(
    move || program_id.get(),  // Key (reactive)
    move |id| GetProgram { id },  // Request builder
);

// Targeted query
let config = use_query_targeted::<GetRobotConfig>(entity_id, || GetRobotConfig);
```

#### 3. Mutation Builder (Advanced)

```rust
// Full control with builder pattern
let create_program = use_mutation_builder::<CreateProgram>()
    .on_success(move |response| {
        toast.success(format!("Created: {}", response.program_id));
        query_client.invalidate::<ListPrograms>();
    })
    .on_error(move |error| toast.error(error))
    .build();
```

#### 4. Query Invalidation

```rust
// Get query client from context
let query_client = use_query_client();

// After mutation, invalidate related queries
query_client.invalidate::<ListPrograms>();
query_client.invalidate_keyed::<GetProgram>(program_id);
```

### Naming Comparison

| Current API | Proposed API | Notes |
|-------------|--------------|-------|
| `use_request::<T>()` | `use_query::<T>()` | For read operations |
| `use_request_with_handler::<T, _>(h)` | `use_mutation::<T>(h)` | For write operations |
| `use_targeted_request::<T>()` | `use_query_targeted::<T>()` | Entity-specific reads |
| `use_targeted_request_with_handler::<T, _>(h)` | `use_mutation_targeted::<T>(h)` | Entity-specific writes |

### Return Types

```rust
// Mutation returns a handle with send + state
pub struct MutationHandle<R: RequestMessage> {
    send: impl Fn(R) + Clone,
    state: Signal<MutationState<R::ResponseMessage>>,
}

impl<R: RequestMessage> MutationHandle<R> {
    pub fn send(&self, request: R) { ... }
    pub fn is_loading(&self) -> bool { ... }
    pub fn is_idle(&self) -> bool { ... }
    pub fn data(&self) -> Option<&R::ResponseMessage> { ... }
    pub fn error(&self) -> Option<&str> { ... }
}

// Query returns reactive data
pub struct QueryHandle<R: RequestMessage> {
    data: Signal<Option<R::ResponseMessage>>,
    state: Signal<QueryState>,
    refetch: impl Fn() + Clone,
}

impl<R: RequestMessage> QueryHandle<R> {
    pub fn data(&self) -> Option<R::ResponseMessage> { ... }
    pub fn is_loading(&self) -> bool { ... }
    pub fn is_stale(&self) -> bool { ... }
    pub fn refetch(&self) { ... }
}
```

## Server-Side Invalidation: The pl3xus Innovation

### The Problem with Client-Side Caching

Traditional query libraries (TanStack Query, SWR, etc.) use client-side cache management:

```
┌──────────────────────────────────────────────────────────────────────┐
│                Traditional HTTP Query Pattern                         │
│                                                                       │
│  Client                                    Server                     │
│    │                                         │                        │
│    │──── GET /todos ──────────────────────>│ │                        │
│    │<─── [todos] ───────────────────────────│                         │
│    │                                         │                        │
│    │  Cache locally with TTL                 │                        │
│    │                                         │                        │
│    │  ... time passes ...                    │                        │
│    │                                         │                        │
│    │  Cache expires, refetch                 │ (data may have changed │
│    │──── GET /todos ──────────────────────>│ │  or not - we don't    │
│    │<─── [todos] ───────────────────────────│   know!)                │
│    │                                         │                        │
└──────────────────────────────────────────────────────────────────────┘

Problems:
- Stale data between refetches
- Wasted bandwidth if data hasn't changed
- Complex client-side invalidation logic
- No guarantee data is current
```

### The pl3xus Solution: Server-Pushed Invalidation

With persistent WebSocket connections, the server can tell clients when data changes:

```
┌──────────────────────────────────────────────────────────────────────┐
│                pl3xus Server-Side Invalidation                        │
│                                                                       │
│  Client                                    Server                     │
│    │                                         │                        │
│    │──── QUERY ListPrograms ───────────────>│                         │
│    │<─── [programs] ────────────────────────│                         │
│    │                                         │                        │
│    │  Cache locally (no TTL needed!)         │                        │
│    │                                         │                        │
│    │                         ┌───────────────┤                        │
│    │                         │ Another client│                        │
│    │                         │ creates a     │                        │
│    │                         │ new program   │                        │
│    │                         └───────────────┤                        │
│    │                                         │                        │
│    │<─── INVALIDATE ListPrograms ───────────│ Server knows data      │
│    │                                         │ changed, tells client  │
│    │──── QUERY ListPrograms ───────────────>│                         │
│    │<─── [programs with new one] ───────────│                         │
│    │                                         │                        │
└──────────────────────────────────────────────────────────────────────┘

Benefits:
✓ Data is always current (or explicitly marked stale)
✓ No wasted refetches - only fetch when actually stale
✓ Server is authoritative - clients trust the server
✓ Simpler client code - no TTL/cache management
✓ Perfect for industrial/robotics applications
```

### Wire Protocol Addition

Add new message type to `SyncServerMessage`:

```rust
/// Server -> client sync messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncServerMessage {
    Welcome(WelcomeMessage),
    SyncBatch(SyncBatch),
    MutationResponse(MutationResponse),
    QueryResponse(QueryResponse),
    /// NEW: Invalidate cached queries
    QueryInvalidation(QueryInvalidation),
}

/// Invalidate one or more cached queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryInvalidation {
    /// Query type names to invalidate.
    /// Empty means invalidate all queries.
    pub query_types: Vec<String>,
    /// Optional: specific keys within query types.
    /// e.g., for GetProgram, might include specific program IDs.
    pub keys: Option<Vec<String>>,
}
```

### Server-Side Invalidation Trigger

On the server, after mutations that affect query data:

```rust
// Server handler for CreateProgram
fn handle_create_program(
    mut commands: Commands,
    network: Res<Network<NP>>,
    // ... other params
) {
    // Create the program...

    // Invalidate all clients' ListPrograms cache
    network.broadcast(SyncServerMessage::QueryInvalidation(QueryInvalidation {
        query_types: vec!["ListPrograms".to_string()],
        keys: None,
    }));
}
```

### Client-Side Query Hook with Auto-Refetch

```rust
/// Query hook that auto-refetches when server invalidates.
pub fn use_query<R>() -> QueryHandle<R>
where
    R: RequestMessage + Clone + 'static,
    R::ResponseMessage: Clone + 'static,
{
    let ctx = use_sync_context();
    let (data, set_data) = signal::<Option<R::ResponseMessage>>(None);
    let (state, set_state) = signal(QueryState::Idle);
    let (request_id, set_request_id) = signal(0u64);

    // Subscribe to invalidation messages for this query type
    let query_type = std::any::type_name::<R>().to_string();
    let invalidation_subscription = ctx.subscribe_invalidations(query_type.clone());

    // Auto-refetch when invalidated
    Effect::new(move |_| {
        if invalidation_subscription.get() {
            // Server said our data is stale - refetch
            refetch();
        }
    });

    // ... rest of implementation
}
```

## Complete API Design

### Queries (Read Operations)

```rust
// Auto-fetching query - fetches on mount, auto-refetches when invalidated
let programs = use_query::<ListPrograms>(|| ListPrograms);

// Access data reactively
view! {
    <Show when=move || programs.is_loading()>
        <LoadingSpinner/>
    </Show>
    <For
        each=move || programs.data().map(|p| p.programs.clone()).unwrap_or_default()
        key=|p| p.id
        children=|program| view! { <ProgramRow program/> }
    />
}

// Keyed query - key changes trigger refetch, each key cached separately
let program = use_query_keyed::<GetProgram, _>(
    move || program_id.get(),  // Key (reactive)
    move |id| GetProgram { id },  // Request builder from key
);

// Targeted query - query specific entity
let config = use_query_targeted::<GetRobotConfig>(
    move || robot_entity_id.get(),
    || GetRobotConfig,
);
```

### QueryHandle Methods

```rust
pub struct QueryHandle<R: RequestMessage> {
    // Private internals
}

impl<R: RequestMessage> QueryHandle<R> {
    /// Get the cached data (reactive)
    pub fn data(&self) -> Option<R::ResponseMessage>;

    /// Check if currently fetching (reactive)
    pub fn is_loading(&self) -> bool;

    /// Check if data is stale (invalidated but not yet refetched)
    pub fn is_stale(&self) -> bool;

    /// Check if there's an error
    pub fn is_error(&self) -> bool;

    /// Get error message if any
    pub fn error(&self) -> Option<String>;

    /// Manually trigger refetch
    pub fn refetch(&self);
}

// Make it Copy for ergonomic use in closures
impl<R: RequestMessage> Clone for QueryHandle<R> { ... }
impl<R: RequestMessage> Copy for QueryHandle<R> { ... }
```

### Mutations with Query Invalidation

The mutation handler can manually invalidate queries:

```rust
let query_client = use_query_client();

let create_program = use_mutation::<CreateProgram>(move |result| {
    match result {
        Ok(r) if r.success => {
            toast.success("Program created");
            // Manual client-side invalidation (optional - server also pushes)
            query_client.invalidate::<ListPrograms>();
        }
        Ok(r) => toast.error(format!("Failed: {}", r.error())),
        Err(e) => toast.error(format!("Error: {e}")),
    }
});
```

But typically the **server handles invalidation** - the client doesn't need to know:

```rust
// Simpler pattern - server will push invalidation
let create_program = use_mutation::<CreateProgram>(move |result| {
    match result {
        Ok(r) if r.success => toast.success("Program created"),
        Ok(r) => toast.error(format!("Failed: {}", r.error())),
        Err(e) => toast.error(format!("Error: {e}")),
    }
});
// ListPrograms query auto-refetches when server pushes invalidation!
```

## pl3xus-Specific Considerations

### 1. Real-Time Sync vs Queries

pl3xus already has `use_components<T>()` for real-time synced state. Queries are for:
- One-time fetches (list of programs, configurations)
- Data that doesn't need real-time updates
- Expensive operations that should be cached

### 2. Entity Targeting

Unlike TanStack Query, pl3xus has entity-targeted operations with authorization:
- `use_mutation_targeted` sends to specific entity
- Server checks `EntityAccessPolicy` before processing
- Authorization failures return structured errors

### 3. Two-Level Success

pl3xus responses have transport success AND business logic success:
```rust
pub struct SomeResponse {
    pub success: bool,
    pub error: Option<String>,
    pub data: Option<SomeData>,
}
```

The handler receives `Result<&Response, &str>`:
- `Err(e)` = transport/network error
- `Ok(r)` where `!r.success` = business logic error
- `Ok(r)` where `r.success` = success

### 4. No HTTP Caching

WebSocket doesn't have HTTP caching semantics. Our "cache" is:
- In-memory query results
- Invalidated explicitly after mutations
- Optional stale-while-revalidate pattern

## Implementation Phases

### Phase 1: Core Mutation API ✅ COMPLETE
- [x] Rename `use_request_with_handler` → `use_mutation`
- [x] Rename `use_targeted_request_with_handler` → `use_mutation_targeted`
- [x] Add `MutationHandle` return type with ergonomic methods
- [x] Make handles `Copy` for ergonomic use in closures

### Phase 2: Server-Side Invalidation Protocol ✅ COMPLETE
- [x] Add `QueryInvalidation` message to `SyncServerMessage`
- [x] Add `query_invalidations` tracking to `SyncContext` on client
- [x] Add `handle_query_invalidation()` method to context
- [x] Handle `QueryInvalidation` in client message router

### Phase 3: Query API ✅ COMPLETE
- [x] Add `use_query<R>()` for auto-fetching reads
- [x] Add `QueryHandle` with reactive data and state
- [x] Add `use_query_keyed<R, F>()` for reactive keyed queries
- [x] Add `QueryState` with is_loading, is_fetching, is_stale, is_success, is_error
- [x] Make QueryHandle `Copy`

### Phase 4: Server-Side Integration ✅ COMPLETE
- [x] Add `invalidate_queries::<NP>(world, &["QueryType"])` helper
- [x] Add `invalidate_queries_with_keys::<NP>(world, types, keys)` for specific keys
- [x] Add `invalidate_all_queries::<NP>(world)` nuclear option

### Phase 5: Query Client (Future - Optional)
- [ ] Add `QueryClientProvider` context component
- [ ] Add `use_query_client()` hook
- [ ] Implement `query_client.invalidate::<R>()` for manual client-side invalidation
- [ ] Add `query_client.refetch::<R>()` for manual refetch

### Phase 6: Advanced Features (Future)
- [ ] Query deduplication (multiple `use_query` calls share one request)
- [ ] Stale-while-revalidate UI patterns (show stale data with indicator)
- [ ] Optimistic updates for mutations
- [ ] Prefetching API
- [ ] `use_query_targeted<R>()` for entity-specific queries

## Migration Path

1. **Backward compatible**: Keep old hooks as deprecated aliases
2. **Gradual migration**: Update one component at a time
3. **Remove deprecated**: After full migration, remove old hooks

## Files to Create/Modify

### New Files
- `crates/pl3xus_client/src/query.rs` - Query hooks and QueryHandle
- `crates/pl3xus_client/src/query_client.rs` - QueryClient and provider

### Modified Files
- `crates/pl3xus_sync/src/messages.rs` - Add QueryInvalidation
- `crates/pl3xus_client/src/context.rs` - Add query cache and invalidation handling
- `crates/pl3xus_client/src/provider.rs` - Handle QueryInvalidation messages
- `crates/pl3xus_client/src/lib.rs` - Export new types and hooks

## Comparison: Before and After

### Before (manual refetching)
```rust
// settings.rs - 50+ lines of Effect handling
let (fetch_configs, configs_state) = use_request::<GetRobotConfigurations>();
let (create_config, create_config_state) = use_request::<CreateConfiguration>();

// Load initial data
Effect::new(move |_| {
    if selected_robot_id.get().is_some() {
        fetch_configs(GetRobotConfigurations { robot_connection_id: robot_id });
    }
});

// Handle create response - manually refetch
Effect::new(move |_| {
    let state = create_config_state.get();
    if let Some(response) = &state.data {
        if response.success {
            // Manual refetch after mutation!
            fetch_configs(GetRobotConfigurations { robot_connection_id: robot_id });
        }
    }
});
```

### After (server-pushed invalidation)
```rust
// settings.rs - ~10 lines total
// Query configurations for selected robot - auto-refetches when robot changes
// AND when server invalidates the query type
let configs = use_query_keyed::<GetRobotConfigurations, _>(move || {
    selected_robot_id.get().map(|id| GetRobotConfigurations { robot_connection_id: id })
});

let create_config = use_mutation::<CreateConfiguration>(move |result| {
    match result {
        Ok(r) if r.success => toast.success("Configuration created"),
        Ok(r) => toast.error(format!("Failed: {}", r.error.as_deref().unwrap_or(""))),
        Err(e) => toast.error(format!("Error: {e}")),
    }
    // No manual refetch needed! Server pushes invalidation.
});

// On server side, after creating configuration:
fn handle_create_configuration<NP: NetworkProvider>(world: &mut World, ...) {
    // ... create configuration in database ...

    // Tell all clients to refetch their configurations
    invalidate_queries::<NP>(world, &["GetRobotConfigurations"]);
}
```

## Implementation Status

### ✅ Completed (Phase 1 - Core API)

1. **Mutation API** - `use_mutation<R>()` and `use_mutation_targeted<R>()`
   - Copy handles for ergonomic use in closures
   - Handler callbacks called exactly once per response
   - Migrated all fanuc_rmi_replica mutations

2. **Query API** - `use_query<R>()`, `use_query_keyed<R, F>()`, `use_query_targeted<R>()`
   - Auto-fetch on mount
   - Reactive refetch when parameters change (keyed queries)
   - Entity-specific queries with per-entity caching (targeted queries)
   - Server-side invalidation support
   - QueryHandle with .data(), .error(), .is_loading(), .refetch()

3. **Server-Side Invalidation Protocol**
   - QueryInvalidation message type
   - invalidate_queries(), invalidate_queries_with_keys(), invalidate_all_queries() helpers
   - Client automatically refetches when invalidation received

4. **Query Client** - `use_query_client()` returns `QueryClient`
   - invalidate::<R>() - Client-side invalidation
   - invalidate_all() - Invalidate all queries
   - has_cached_data::<R>() - Check cache status
   - clear_cache() - Clear all cached data

5. **Query Deduplication**
   - Multiple components using same query share one state signal
   - Reference counting for automatic cleanup
   - Prevents duplicate network requests

6. **Example Integration**
   - fanuc_rmi_replica fully migrated to new API
   - Server handlers broadcast invalidation after all CRUD operations
   - Zero manual refetch code needed

### ✅ Completed Enhancements (Phase 2)

1. **Query Client** - `use_query_client()` returns `QueryClient` with:
   - `invalidate::<R>()` - Invalidate all queries of a specific type
   - `invalidate_all()` - Invalidate all queries
   - `has_cached_data::<R>()` - Check if query type has cached data
   - `clear_cache()` - Clear all cached query data

2. **Query Deduplication** - Multiple components using the same query share one state signal:
   - Cache uses `(query_type, query_key)` as key
   - Reference counting for automatic cleanup when all subscribers unmount
   - Cache stores raw bytes, hooks deserialize to typed `QueryState<T>`
   - Prevents duplicate network requests

3. **Targeted Queries** - `use_query_targeted<R>(entity_id, request)`:
   - Similar to `use_query` but targets a specific entity
   - Cache key includes entity ID for per-entity caching
   - Supports server-side invalidation
   - Automatic cleanup on unmount

### 🔮 Future Enhancements

1. **Optimistic Updates** - Update UI before server confirms
2. **Prefetching** - Fetch data before it's needed
3. **Query Deduplication for use_query_keyed** - Currently only `use_query` uses the cache

## Conclusion

The proposed API:
- Uses familiar TanStack Query naming (`use_mutation`, `use_query`)
- Adapts patterns to pl3xus's WebSocket/entity model
- **Innovates with server-pushed invalidation** - always accurate data
- Reduces boilerplate significantly (50+ lines → ~10 lines)
- Provides clear separation of reads vs writes
- Perfect for industrial/robotics applications where accuracy is critical

The key differentiator from TanStack Query is **server-side invalidation**:
- TanStack Query: Client manages cache, uses TTL/stale-while-revalidate
- pl3xus: Server is authoritative, pushes invalidation when data changes

This is the "best-in-class" API for real-time applications.
