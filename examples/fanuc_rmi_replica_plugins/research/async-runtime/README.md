# Async Runtime Research

## Background

Pl3xus applications (industrial/robotics) are IO-heavy. We interact with sensors, PLCs, and robots which requires robust async support. The default Bevy ECS is synchronous, and the Bevy maintainers have explicitly stated that first-class async is not on their roadmap:

> "Architecture of any sensible game is inherently synchronous, due to it running in real time on real hardware and, most commonly, independent of IO. Thus Bevy, being an engine to make games sensibly, is inherently synchronous as well."
> — Bevy Maintainer ([Discussion #2677](https://github.com/bevyengine/bevy/discussions/2677))

This is a fundamental architectural mismatch for industrial applications where:
- Network operations to robots/sensors are long-running
- Request/response patterns require non-blocking IO
- Safety-critical operations must be cancellable
- WASM clients need identical async patterns to native

**We need async as a first-class citizen in our software.**

---

## Current State

We currently use a fork of `bevy-tokio-tasks` called `async_bevy_web`:
- **Repository**: https://github.com/vertec-io/async_bevy_web
- **Package**: `bevy-tokio-tasks`
- **Features**: Tokio runtime, WASM support via gloo-timers

The current usage pattern involves significant boilerplate:
```rust
tokio_runtime.spawn_background_task(move |mut ctx| async move {
    // Async operations here...
    ctx.run_on_main_thread(move |ctx| {
        // ECS access here (manual queries on ctx.world)
    }).await;
});
```

**This works, but has poor ergonomics** - nested closures, manual world access, no supervision.

---

## Research Scope

This research evaluates all viable async solutions for Bevy 0.17:

| Solution | Crates.io | Bevy 0.17 | Primary Use Case |
|----------|-----------|-----------|------------------|
| bevy-async-ecs | v0.9.0 | ✅ | Orchestration, sequencing |
| bevy_async_task | v0.11.x | ✅ | Simple background tasks |
| bevy_flurx | v0.8.x | ⚠️ Check | Coroutine-style logic |
| bevy_defer | v0.14.x | ⚠️ Check | Deferred queries |
| bevy_mod_async | - | ⚠️ Check | ECS-integrated async |
| bevy-tokio-tasks | Fork | ✅ | Tokio ecosystem access |

---

## Ideal Target API

The goal is to make async code as ergonomic as synchronous ECS code:

```rust
// Target: Natural async system syntax with supervision
#[async_handler]
async fn handle_motion_request(
    ctx: AsyncContext,
    request: MotionRequest,
    robot: Entity,
) -> Result<MotionResponse, AsyncError> {
    // Sequence with automatic cancellation support
    ctx.sequence(robot, |seq| async move {
        seq.move_to(request.target).await?;
        seq.wait_for_arrival().await?;
        Ok(MotionResponse::success())
    }).await
}

// Target: Simple app integration
app.add_plugins(Pl3xusAsyncPlugin::default())
    .add_async_system(Update, handle_motion_request);
```

---

## Key Requirements for Industrial Applications

| Requirement | Priority | Current Gap |
|-------------|----------|-------------|
| **Task Cancellation** | Critical | No solution provides this |
| **Error Supervision** | Critical | Errors are swallowed silently |
| **WASM Parity** | High | Fork provides this |
| **Tokio Ecosystem** | High | Only bevy-tokio-tasks |
| **Ergonomic Syntax** | High | Current pattern is verbose |
| **Graceful Shutdown** | Medium | No clean shutdown handling |

---

## Research Documents

| File | Description |
|------|-------------|
| [start_here.md](./start_here.md) | Overview, objectives, and navigation |
| [current_state_analysis.md](./current_state_analysis.md) | Deep analysis of current bevy-tokio-tasks usage |
| [candidate_analysis.md](./candidate_analysis.md) | Comprehensive evaluation of ALL candidates |
| [decision_matrix.md](./decision_matrix.md) | Weighted scoring against requirements |
| [gap_analysis.md](./gap_analysis.md) | Critical missing features for industrial use |
| [recommendation.md](./recommendation.md) | Final recommendation: Layered Architecture |
| [implementation_plan.md](./implementation_plan.md) | Detailed roadmap for pl3xus_async crate |

---

## Recommendation Summary

**Build a layered `pl3xus_async` crate** that we own, which *composes* existing crates instead of trying to replace Bevy’s scheduler or Tokio:

1. **Orchestration Layer — `bevy-async-ecs`**  
   - Use `AsyncWorld` and registered sync systems to express *robot/plant sequences* as linear async flows.  
   - Fully supports Bevy 0.17 and WASM (docs and examples confirm this).  
   - Best suited for low- to mid-frequency orchestration (≈1–10 Hz), not 60 Hz control loops.

2. **IO / Tokio Layer — `async_bevy_web` fork of `bevy-tokio-tasks`**  
   - Provides a real Tokio `Runtime` inside Bevy plus our WASM-compatible fork (via `async_bevy_web`).  
   - Ideal for high-throughput IO (robot sockets, database, websockets) and long-lived background tasks.  
   - Exposes `TokioTasksRuntime::spawn_background_task` / `TaskContext::run_on_main_thread` but `pl3xus_async` will wrap these in a safer, more ergonomic API so application code rarely touches raw `&mut World`.

3. **Supervision & Cancellation Layer — new in `pl3xus_async`**  
   - Adds `AsyncTaskRegistry`, `CancellationToken`-style cancellation, and an `AsyncSupervisor` for error propagation.  
   - Turns today’s "fire-and-forget" tasks into "fire-and-manage": trackable, cancellable, and observable.

4. **Runtime Abstraction — unified native/WASM surface**  
   - Wraps Tokio (native) and `wasm-bindgen-futures` + `gloo-timers` (WASM) behind a tiny API: `spawn`, `sleep`, `timeout`.  
   - Application/skill code never needs `cfg(target_arch = "wasm32")` branches for basic async operations.

5. **Optional Compute Layer — `bevy_async_task` (future hook)**  
   - `bevy_async_task` offers ergonomic `TaskRunner` / `TaskPool` / `TimedTaskRunner` params built on `bevy_tasks` with good WASM support.  
   - Our conclusion after review is that it’s useful for *pure compute / data-fetch tasks that don’t need direct ECS access or Tokio-only crates*.  
   - `pl3xus_async` can expose a thin wrapper over this later, but it is not a core dependency for the initial design.

See [recommendation.md](./recommendation.md) for the full architecture and migration plan.

### Where to Use Each Layer (Performance & Ergonomics)

To keep both performance and ergonomics high, code should follow this decision tree:

1. **Plain Bevy systems (no async)**  
   Use normal synchronous systems for:
   - Tight 60 Hz control loops and simple state updates  
   - Logic that does not block on network/disk/robot IO

2. **Tokio / IO layer (through `pl3xus_async` wrappers over `bevy-tokio-tasks`)**  
   Use for:
   - Robot/PLC sockets, database access (`sqlx`, etc.), websockets  
   - High-frequency polling tasks (e.g. 60 Hz pose streaming) where channel-based `AsyncWorld` would add avoidable overhead

3. **Orchestration layer (`bevy-async-ecs` via `pl3xus_async`)**  
   Use for:
   - Multi-step robot or cell sequences: "move → wait for sensor → close gripper → move back"  
   - Flows that need to interleave ECS queries/commands with IO but do *not* run at frame rate

4. **Supervision & cancellation (always-on concern)**  
   All of the above async entry points should go through `pl3xus_async` so that:
   - Motion tasks are cancellable on safety stop within a bounded latency  
   - Errors transition entities into fault states instead of being silently logged  
   - Long-running tasks participate in graceful shutdown.

This hybrid, layered approach is the outcome of a second, more detailed review of `bevy-async-ecs`, `bevy-tokio-tasks`/`async_bevy_web`, and `bevy_async_task` against pl3xus’ industrial constraints.

---

## Next Steps

1. Review research documents for approval
2. Create `crates/pl3xus_async` skeleton
3. Begin Phase 1 implementation per [implementation_plan.md](./implementation_plan.md)

---

## Related Resources

- [Bevy Async Discussion #2677](https://github.com/bevyengine/bevy/discussions/2677)
- [bevy-async-ecs docs](https://docs.rs/bevy-async-ecs/latest/bevy_async_ecs/)
- [async_bevy_web (our fork)](https://github.com/vertec-io/async_bevy_web)
- [bevy-async-ecs repo](https://github.com/dlom/bevy-async-ecs)