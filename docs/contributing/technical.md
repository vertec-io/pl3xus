# Technical Reference

Technical deep dives for pl3xus contributors.

---

## Design Philosophy

### Industrial Applications First

pl3xus is designed for:

1. **Robotics Control Systems** - Real-time robot state synchronization
2. **Industrial Automation** - Factory floor monitoring and control
3. **Digital Twins** - Physical-digital synchronization
4. **SCADA Systems** - Supervisory control and data acquisition

### Performance Requirements

- **High frame rates** (30-60+ FPS updates)
- **High throughput** (thousands of updates/second)
- **Low latency** (sub-frame response times)
- **Deterministic behavior** (predictable under load)

---

## Two-Stage Relay Pattern

### The Pattern

```rust
// Application logic systems write events (no network access needed)
fn robot_control_system(mut commands: Commands) {
    commands.trigger(OutboundMessage::<JointPosition>::new(entity, position));
}

// Relay system in late stage reads events and broadcasts
fn relay_outbound(
    mut events: EventReader<OutboundMessage<T>>,
    network: Res<NetworkProvider>,
) {
    for event in events.read() {
        network.broadcast(&event.message);
    }
}
```

### Rationale

- **Separation of concerns**: Logic systems don't need network access
- **Deterministic timing**: All sends happen in specific system set
- **Batching opportunity**: Process all queued messages together
- **Testability**: Logic systems can be tested without network

---

## Type Name Resolution

### How It Works

Both client and server extract short type names:

```rust
let full_name = std::any::type_name::<T>();
// "my_app::components::Position"

let short_name = full_name.rsplit("::").next().unwrap_or(full_name);
// "Position"
```

### Three-Tier Matching

1. **type_name** - Fast path, exact match
2. **schema_hash** - Fallback for version differences
3. **hash-to-name mapping** - Resolution for mismatches

### Ensuring Consistency

Use shared types crate:

```rust
// shared_types/src/lib.rs
pub struct Position { pub x: f32, pub y: f32 }

// Both server and client use shared_types::Position
// Both resolve to "Position"
```

---

## Async Task Management

### Task Detachment Pattern

Long-running async tasks use weak references:

```rust
let connection_task_weak = Arc::downgrade(&self.connection_tasks);

Box::new(run_async(
    async move {
        // ... async work ...
        
        // Safe cleanup even if parent dropped
        connection_task_weak
            .upgrade()
            .expect("Network dropped")
            .remove(&task_count);
    },
    runtime,
))
```

### Why This Matters

- Prevents dangling references
- Allows graceful shutdown
- Handles disconnection cleanly

---

## Change Detection Internals

### Bevy's Change Detection

Components are marked changed when:
- `DerefMut` is called on `Mut<T>`
- Component is inserted
- Component is replaced

### pl3xus_sync Detection

```rust
for (entity, component) in query.iter() {
    if component.is_changed() {
        // Include in next SyncBatch
    }
}
```

### Gotchas

- `is_changed()` clears after each frame
- Order matters: detection must run after mutations
- Removed components detected via `RemovedComponents<T>`

---

## Subscription Lifecycle

### Server-Side

```rust
pub struct SubscriptionEntry {
    pub connection_id: ConnectionId,
    pub subscription_id: u64,
    pub component_type: String,
    pub entity_filter: Option<u64>,  // None = all entities
}
```

### Client-Side Deduplication

Multiple hooks sharing subscription:

```rust
// First hook: creates subscription, ref_count = 1
let data1 = use_sync_component::<Position>();

// Second hook: reuses subscription, ref_count = 2
let data2 = use_sync_component::<Position>();

// When both unmount: ref_count = 0, unsubscribe sent
```

---

## Mutation Authorization

### Default Behavior

All mutations rejected unless authorizer configured:

```rust
pub trait MutationAuthorizer: Send + Sync + 'static {
    fn authorize(
        &self,
        connection_id: ConnectionId,
        entity: Entity,
        component_type: &str,
        world: &World,
    ) -> bool;
}
```

### Hierarchy-Aware Authorization

```rust
impl MutationAuthorizer for HierarchyAuthorizer {
    fn authorize(&self, conn, entity, _, world) -> bool {
        // Check if connection controls entity or any ancestor
        has_control_recursive(world, entity, conn)
    }
}
```

---

## WebSocket Codec

### Pl3xusBincodeCodec

Custom codec for leptos-use WebSocket:

```rust
impl<T> Decoder<T> for Pl3xusBincodeCodec
where
    T: DeserializeOwned,
{
    fn decode(val: &[u8]) -> Result<T, Self::Error> {
        // Skip 4-byte length prefix
        let data = &val[4..];
        bincode::serde::decode_from_slice(data, config)
    }
}
```

### Wire Format

```
[4 bytes: length][bincode data]
```

---

## Error Handling Patterns

### SyncError Types

```rust
pub enum SyncError {
    NotConnected,
    DeserializationFailed { component_name, error },
    TypeNotRegistered { component_name },
    SchemaHashMismatch { component_name, expected, actual },
    WebSocketError { message },
    SerializationFailed { component_name, error },
}
```

### Client-Side Error Signal

```rust
let ctx = use_sync_context();

Effect::new(move || {
    if let Some(error) = ctx.last_error.get() {
        log::error!("Sync error: {}", error);
    }
});
```

---

## Related Documentation

- [Architecture Reference](./architecture.md)
- [Performance Reference](./performance.md)
- [Research Process](./research-process.md)

---

**Last Updated**: 2025-12-07

