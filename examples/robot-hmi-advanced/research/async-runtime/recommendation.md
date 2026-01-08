# Recommendation: Layered Async Architecture for pl3xus

## Executive Summary

After comprehensive evaluation of 6 async solutions against pl3xus industrial/robotics requirements, and a follow-up review of the latest crate docs (as of late 2025), I recommend a **Layered Architecture** approach that combines the strengths of multiple solutions:

| Layer | Solution | Purpose |
|-------|----------|---------|
| **High-Level** | `bevy-async-ecs` | Orchestration, sequencing, scripted behaviors (via `AsyncWorld`) |
| **Low-Level** | `async_bevy_web` / `bevy-tokio-tasks` | Tokio ecosystem, database, hardware IO, WASM-aware runtime |
| **Wrapper** | `pl3xus_async` (new) | Unified API, supervision, cancellation, runtime abstraction |

This is **NOT** a simple adoption of one crate. It is a **strategic architecture** that addresses the fundamental mismatch between game-engine async patterns and industrial system requirements, *after explicitly validating the capabilities and limits of* `bevy-async-ecs`, `bevy-tokio-tasks` (and our `async_bevy_web` fork), and `bevy_async_task`.

### Re‑review Notes (2025‑12‑30)

The follow-up review focused on the concern that earlier work might have underused the existing crates:

- **`bevy-async-ecs`**  
  - Confirmed: fully documented WASM support and a clear `AsyncWorld` story for orchestration, with `CommandQueue`/`start_queue` for batching world mutations.  
  - Limitation remains: no built-in cancellation or supervision; it deliberately does *not* try to turn Bevy systems into `async fn`.

- **`bevy-tokio-tasks` and `async_bevy_web`**  
  - Confirmed: simple, efficient Tokio integration, plus our fork that wires it into a web stack and extends it to WASM.  
  - Limitation remains: raw `&mut World` in callbacks and no supervision/cancellation; ergonomics issues are real but can be fixed in our own wrapper layer.

- **`bevy_async_task`**  
  - Confirmed: mature cross-platform `TaskRunner` / `TaskPool` / `TimedTaskRunner` built on `bevy_tasks` with good WASM story.  
  - Limitation: tasks cannot touch ECS directly and don’t integrate Tokio-specific crates; it complements, but does not replace, the Tokio layer.

These confirmations reinforce—not weaken—the layered design: we rely on `bevy-async-ecs` for orchestration semantics, `bevy-tokio-tasks`/`async_bevy_web` for high-performance IO, and `pl3xus_async` to close the industrial gaps (cancellation, supervision, and unified runtime surface).

---

## The Verdict

### ❌ Do NOT adopt any single solution as-is

Every evaluated solution has critical gaps for industrial use:
- `bevy-async-ecs`: No cancellation, no Tokio ecosystem
- `bevy-tokio-tasks`: Poor ergonomics, no supervision
- `bevy_async_task`: No ECS interaction
- `bevy_flurx/bevy_defer/bevy_mod_async`: Limited IO capabilities

### ✅ DO build a layered wrapper crate

Create `pl3xus_async` as a first-class citizen in the pl3xus framework that:
1. Provides multiple abstraction levels for different use cases
2. Adds industrial-grade features missing from all candidates
3. Maintains full compatibility with existing `bevy-tokio-tasks` code during migration

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                      Application Code                            │
│   (handlers.rs, motion.rs, polling.rs, connection.rs)           │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                       pl3xus_async                               │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │  Orchestration  │  │  Task Manager   │  │    Supervision  │  │
│  │  (sequences)    │  │  (spawn/cancel) │  │    (errors)     │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘  │
│                              │                                   │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │               Runtime Abstraction (WASM/Native)             ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┴───────────────┐
              ▼                               ▼
┌──────────────────────────┐    ┌──────────────────────────┐
│     bevy-async-ecs       │    │    bevy-tokio-tasks      │
│   (orchestration layer)  │    │    (Tokio runtime)       │
└──────────────────────────┘    └──────────────────────────┘
```

---

## Justification by Use Case

### Use Case 1: Robot Motion Sequences
**Current Pain**: 50+ lines of nested callbacks per sequence  
**Proposed Solution**: Orchestration layer with bevy-async-ecs

```rust
// BEFORE: bevy-tokio-tasks
tokio_runtime.spawn_background_task(move |mut ctx| async move {
    // Move to position A
    send_motion_command(driver.clone(), pos_a).await;
    ctx.run_on_main_thread(move |ctx| {
        // Wait for motion complete...
    }).await;
    
    // Close gripper
    send_gripper_command(driver.clone(), close).await;
    ctx.run_on_main_thread(move |ctx| {
        // Wait for gripper...
    }).await;
    
    // Move to position B
    // ... more nesting ...
});

// AFTER: pl3xus_async orchestration
async fn pick_and_place(ctx: AsyncContext, entity: Entity) -> Result<()> {
    ctx.sequence(entity, |seq| async move {
        seq.move_to(Position::A).await?;
        seq.close_gripper().await?;
        seq.move_to(Position::B).await?;
        seq.open_gripper().await?;
        Ok(())
    }).await
}
```

### Use Case 2: Database Operations
**Current State**: Works well with bevy-tokio-tasks  
**Proposed Solution**: Keep using Tokio layer, add supervision

```rust
// BEFORE: bevy-tokio-tasks (works but unsupervised)
tokio_runtime.spawn_background_task(move |_ctx| async move {
    let rows = sqlx::query_as!(Config, "SELECT * FROM configs")
        .fetch_all(&pool)
        .await?;
    // Error goes nowhere if this fails
});

// AFTER: pl3xus_async with supervision
ctx.spawn_supervised(entity, "load_configs", async move {
    let rows = sqlx::query_as!(Config, "SELECT * FROM configs")
        .fetch_all(&pool)
        .await?;  // Error automatically reported
    Ok(rows)
}).await
```

### Use Case 3: High-Frequency Polling
**Current State**: Dedicate Tokio task for polling  
**Proposed Solution**: Keep as-is, this pattern works well

```rust
// This pattern remains unchanged - it's efficient
tokio_runtime.spawn_background_task(|mut ctx| async move {
    loop {
        let position = driver.get_position().await;
        ctx.run_on_main_thread(move |ctx| {
            // Update position component
        }).await;
        tokio::time::sleep(Duration::from_millis(16)).await; // 60Hz
    }
});
```

### Use Case 4: Safety-Critical Operations
**Current Gap**: No cancellation  
**Proposed Solution**: Cancellable task pattern

```rust
// AFTER: pl3xus_async with cancellation
let task = ctx.spawn_cancellable(entity, "motion_task", async move {
    ctx.sequence(entity, |seq| async move {
        seq.move_to(target).await?;  // Checks cancellation token internally
        Ok(())
    }).await
}).await;

// Safety stop handler
fn safety_stop_system(mut registry: ResMut<AsyncTaskRegistry>) {
    registry.cancel_all_motion();  // All motion tasks abort immediately
}
```

---

## Alternatives Rejected

### Option A: Pure bevy-async-ecs
**Why Rejected**: 
- Cannot access Tokio ecosystem (sqlx, reqwest, tokio-tungstenite)
- Would require rewriting all database code
- Channel overhead for high-frequency operations

### Option B: Pure bevy-tokio-tasks (status quo)
**Why Rejected**:
- Ergonomics are unacceptable for sequences
- Developer productivity impact
- Error-prone patterns lead to bugs

### Option C: Custom from-scratch solution
**Why Rejected**:
- Unnecessary when excellent foundations exist
- Would take 3-6 months to reach feature parity
- Risk of introducing new bugs in critical infrastructure

### Option D: Wait for Bevy native async
**Why Rejected**:
- Bevy maintainers have explicitly stated async is not on roadmap
- Quote: "Architecture of any sensible game is inherently synchronous"
- Cannot wait indefinitely for uncertain feature

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| `bevy-async-ecs` breaking changes | Medium | Medium | Fork and internalize |
| Complexity of layered approach | Medium | Low | Clear documentation, good examples |
| Migration disrupts existing code | Low | Medium | Incremental adoption, parallel APIs |
| Performance overhead | Low | Low | Benchmark before/after, optimize hot paths |

---

## Success Criteria

The `pl3xus_async` implementation will be considered successful when:

1. **Ergonomics**: New async handlers require ≤50% of current line count
2. **Safety**: All motion tasks cancellable within 100ms of safety stop
3. **Reliability**: Async errors automatically propagate to entity state
4. **Performance**: No measurable impact on 60Hz control loops
5. **Compatibility**: WASM client works identically to native

---

## Recommended Next Steps

1. **Prototype** (1 week):
   - Create minimal `pl3xus_async` crate structure
   - Implement basic orchestration wrapper around bevy-async-ecs
   - Test with simple robot sequence

2. **Safety Features** (2 weeks):
   - Implement CancellationToken integration
   - Create AsyncTaskRegistry
   - Add SafetyStop → cancel_all_motion integration

3. **Supervision** (1 week):
   - Implement AsyncSupervisor
   - Add error → state propagation
   - Create supervised task wrappers

4. **Migration** (2 weeks):
   - Migrate 2-3 handlers from bevy-tokio-tasks to pl3xus_async
   - Document patterns for remaining handlers
   - Create migration guide

5. **Documentation** (ongoing):
   - API documentation for pl3xus_async
   - Examples for common patterns
   - Integration with pl3xus skills system

See `implementation_plan.md` for detailed task breakdown.

---

## Conclusion

The recommendation is **not** to simply adopt an existing crate, but to **build a purpose-designed async layer** that addresses the unique requirements of industrial/robotics applications.

**Key Principle**: Industrial systems need "fire and manage" semantics, not "fire and forget". Every async operation must be trackable, cancellable, supervised, and observable.

By building on proven foundations (bevy-async-ecs for orchestration, bevy-tokio-tasks for IO), we minimize risk while maximizing value. The layered architecture allows incremental migration and maintains compatibility with existing code.

This is the path to a **world-class async implementation** that treats async as a first-class citizen in the pl3xus framework.
