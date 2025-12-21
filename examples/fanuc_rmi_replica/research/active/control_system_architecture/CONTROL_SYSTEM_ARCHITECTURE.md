# Control System Architecture: Challenges, Solutions, and Future Directions

## Document Purpose

This document analyzes the challenges encountered while implementing exclusive control in the fanuc_rmi_replica application, evaluates different architectural patterns, and proposes future directions for the pl3xus framework.

---

## 1. The Core Problem: Finding the Controllable Entity

### Challenge

When a client wants to take or release control, it needs to know *which entity* to send the control request for. In a Bevy ECS system where entities are identified by opaque `Entity` handles (converted to `u64` bits for network transmission), the client needs a way to discover:

1. **Which entity is "the system"** - the root entity that controls the apparatus
2. **Which entity has `EntityControl`** - so it can check if it has control

### What Went Wrong

We initially used the fragile pattern:
```rust
let system_entity = control_state.get().keys().next().copied();
```

This worked *by accident* when only one entity had `EntityControl`. But when we added `EntityControl` to both System (entity `0xFFFFFFFF`) and Robot (entity `0xFFFFFFFE`) entities:

1. **HashMap ordering is non-deterministic** - `.keys().next()` could return either entity
2. **We got the Robot entity** - causing control requests to be sent for the wrong entity
3. **The UI started oscillating** - because Robot's EntityControl was being updated by position sync, triggering re-renders

The "fix" of using `.keys().max()` was also fragile - it relied on System having a higher entity ID than Robot, which is an implementation detail.

---

## 2. The SystemMarker Solution

### Approach

We introduced a `SystemMarker` component:

```rust
// In shared types crate (fanuc_replica_types)
#[cfg_attr(feature = "server", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct SystemMarker;
```

**Server-side:**
- Spawns the System entity with `SystemMarker`
- Registers it for sync: `app.sync_component::<SystemMarker>(None);`

**Client-side:**
- Subscribes to `SystemMarker` to discover the system entity
- The only entity with this marker is the system entity

### Why This Works

1. **Explicit identification** - The marker exists specifically to identify the system
2. **Sync guarantees delivery** - The component arrives before any user interaction

---

## 3. Pattern Evaluation: Context vs Direct Subscription

### Option 1: Direct Subscription in Every Component

```rust
// In each component that needs system entity
let system_marker = use_sync_component::<SystemMarker>();
let system_entity_bits = move || system_marker.get().keys().next().copied();
```

**Pros:**
- Simpler mental model - no context to understand
- Component is self-contained
- No need to understand parent component structure

**Cons:**
- Boilerplate: `.get().keys().next().copied()` repeated everywhere
- Multiple subscriptions: Each component creates its own subscription
- Pattern fragility: The `.keys().next()` pattern could still fail if multiple SystemMarkers exist

### Option 2: Context Provider (Current Implementation)

```rust
// In DesktopLayout (once)
let system_markers = use_sync_component::<SystemMarker>();
let system_entity_id = Memo::new(move |_| system_markers.get().keys().next().copied());
provide_context(SystemEntityContext::new(system_entity_id.into()));

// In child components
let system_ctx = use_system_entity();
let system_entity_bits = move || system_ctx.entity_id.get();
```

**Pros:**
- Single subscription point - better for reactivity
- Memo provides stability - only notifies when value changes
- Less boilerplate in child components
- Centralizes the "fragile" lookup pattern

**Cons:**
- Requires understanding context hierarchy
- Must be descendant of the provider
- Another abstraction layer

### Verdict: Context is Better for Boilerplate Reduction

The context pattern reduces boilerplate and aligns with pl3xus principles:
- **Eliminate boilerplate** - child components don't repeat the lookup pattern
- **Server-authoritative** - the system entity comes from synced state
- **Reactivity stability** - Memo prevents cascading updates

However, the `.get().keys().next().copied()` pattern remains ugly. We'll address this next.

---

## 4. The Fragility of `.get().keys().next().copied()`

This pattern is problematic:

```rust
system_markers.get().keys().next().copied()
```

**Issues:**
1. **Not self-documenting** - What does "first key" mean semantically?
2. **Assumes singleton** - Will break silently if multiple entities have the marker
3. **Verbose** - 4 method calls for a simple operation
4. **No error handling** - Returns `None` silently if no entity exists

### Alternative Pattern: `use_sync_component_where` with Name

If entities had a `Name` component:

```rust
// More semantic - find entity by name
let system = use_sync_component_where::<Name, _>(|_id, name| name.0 == "system");
let system_entity = move || system.get().keys().next().copied();
```

**This is more robust because:**
- Name is meaningful and documented
- Could validate name uniqueness
- Pattern works for any named entity, not just "the system"

### Even Better: A New Hook

What we really want:

```rust
// Proposed: find single entity with a component
let system_entity = use_sync_singleton::<SystemMarker>();
// Returns: Signal<Option<u64>>

// Or with semantic name
let system_entity = use_sync_entity_by_name("system");
// Returns: Signal<Option<u64>>
```

This would:
- Eliminate the boilerplate
- Make intent clear
- Handle errors (warn if multiple entities match)
- Return entity ID directly, not `HashMap`

---

## 5. Framework-Level Considerations

### Should ExclusiveControlPlugin Provide System Entity?

**Current State:**
- `ExclusiveControlPlugin` handles control requests/releases
- Applications define their own entity hierarchies
- No framework concept of "the controllable root"

**Proposal: Add `ControlRoot` marker at framework level:**

```rust
// In pl3xus_sync
#[derive(Component, Serialize, Deserialize, Clone, Default)]
pub struct ControlRoot;

// Server auto-syncs this component
// Client can query: use_sync_component::<ControlRoot>()
```

**However, this may be premature abstraction.**

### Multi-System Scenarios

Not all applications have a single system:

1. **Multi-robot workcells** - Multiple independent robots, each controllable
2. **Hierarchical control** - Different clients control different hierarchy levels
3. **Zone-based control** - Control authority based on physical zones
4. **Role-based control** - Operators vs supervisors have different control scopes

The current `ExclusiveControlPlugin` already supports:
- Control per entity (not global)
- Hierarchical propagation (`propagate_to_children`)
- Custom control components

**We should NOT hardcode a single "system" concept at the framework level.**

---

## 6. Inspiration from Other Robotics Platforms

### ROS2 Lifecycle Management

ROS2 uses **lifecycle nodes** (managed nodes) with explicit state machines:
- States: `Unconfigured → Inactive → Active → Finalized`
- Transitions are explicit service calls
- Multiple nodes can be in `Active` state simultaneously

**Insight:** ROS2 doesn't have exclusive control at the node level - coordination is handled by higher-level constructs like behavior trees or task planners.

### ROS2 Control / MoveIt

For robot arm control specifically:
- **Controller Manager** arbitrates access to hardware interfaces
- Only one controller can "claim" a joint at a time
- **Action servers** handle goal preemption (new goal replaces old)

**Insight:** Control is scoped to *resources* (joints, hardware interfaces), not entities.

### UAV Ground Control Station Handover

Drone systems have sophisticated control authority transfer:
- **Primary/Secondary GCS roles** - one controls, others monitor
- **Explicit handover protocol** - requires acknowledgment from both sides
- **Automatic failover** - if primary loses link, secondary can take over

**Insight:** Control transfer is a *protocol*, not just state.

### Industrial Robot Controllers (FANUC, KUKA)

- **Teach Pendant has priority** - physical device always overrides remote
- **Remote mode must be explicitly enabled** - operator consent required
- **No multi-client simultaneous control** - strictly exclusive

**Insight:** Safety considerations drive strict exclusivity.

---

## 7. Alternative Architectures to Consider

### Option A: Named Entity Discovery

Instead of marker components, use a standard `Name` component:

```rust
// Server
commands.spawn((
    Name::new("system"),
    EntityControl::default(),
    // ...
));
app.sync_component::<Name>(None);

// Client - find by name
let system = use_sync_component_where::<Name, _>(|_, n| n.as_str() == "system");
```

**Pros:** Semantic, debuggable, works for any entity
**Cons:** String comparison, requires Name to be synced

### Option B: Entity Registry with IDs

Server provides a registry of "well-known" entities:

```rust
// Server broadcasts this once
#[derive(Message)]
struct EntityRegistry {
    system: u64,
    robots: Vec<u64>,
}

// Client receives and provides via context
let registry = use_sync_message::<EntityRegistry>();
```

**Pros:** Explicit, single message, all IDs available
**Cons:** New message type, manual maintenance

### Option C: Control Context at Framework Level

Framework provides a control context automatically:

```rust
// In pl3xus_client
pub fn use_control_context() -> ControlContext {
    // Automatically tracks EntityControl components
    // Provides: controlled_entities, has_control_of(entity), request_control(entity)
}
```

**Pros:** Zero boilerplate, framework handles complexity
**Cons:** Framework makes assumptions about control patterns

---

## 8. Recommendations

### For fanuc_rmi_replica (Now)

1. **Keep the SystemMarker + Context pattern** - it works and is explicit
2. **Consider adding a `use_sync_singleton` hook** - reduces boilerplate
3. **Document the pattern** - make it a project convention

### For pl3xus Framework (Future)

1. **Add `use_sync_singleton<T>()` hook** - returns `Signal<Option<u64>>` for marker components
2. **Add `use_sync_entity_by_name(name)` hook** - if Name component is synced
3. **Do NOT add ControlRoot** - let applications define their own control hierarchies
4. **Consider control context** - but as optional utility, not core framework

### Evaluation Summary

| Pattern | Boilerplate | Flexibility | Robustness | Recommendation |
|---------|-------------|-------------|------------|----------------|
| `.keys().next()` | High | Low | Low | ❌ Avoid |
| `SystemMarker` + Context | Medium | Medium | High | ✅ Use now |
| `use_sync_singleton<T>()` | Low | Medium | High | ⭐ Propose for framework |
| Named entity lookup | Low | High | Medium | Consider for multi-entity |
| Framework ControlRoot | Very Low | Very Low | High | ❌ Too limiting |

---

## 9. Conclusion

The SystemMarker pattern successfully resolved the entity identification problem, but revealed a gap in the pl3xus client API: **there's no clean way to find a singleton entity by component type**.

The `.get().keys().next().copied()` pattern, while functional, is:
- Not self-documenting
- Prone to silent failures
- Repeated boilerplate

A `use_sync_singleton<T>()` hook would address this elegantly while maintaining the framework's flexibility for multi-entity and hierarchical control scenarios.

The context pattern (`SystemEntityContext`) remains the right architectural choice for this application because:
1. Single subscription point prevents reactivity issues
2. Memo provides stable entity ID
3. Child components get clean API via `use_system_entity()`

For applications with multiple controllable roots, the same pattern scales - just provide multiple context values or use a registry pattern.
