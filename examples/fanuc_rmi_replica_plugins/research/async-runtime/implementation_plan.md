# Implementation Plan: pl3xus_async

A detailed, actionable roadmap for implementing the world-class async system for pl3xus.

---

## Summary

| Phase | Duration | Goal |
|-------|----------|------|
| **Phase 1**: Foundation | Week 1-2 | Create crate structure, basic abstractions |
| **Phase 2**: Safety Features | Week 2-3 | Cancellation, task registry |
| **Phase 3**: Supervision | Week 3-4 | Error handling, state propagation |
| **Phase 4**: Ergonomics | Week 4-5 | Macros, builders, sugar |
| **Phase 5**: Migration Pilot | Week 5-6 | Migrate fanuc handlers |
| **Phase 6**: Documentation | Week 6-7 | Skills, examples, guides |

---

## Phase 1: Foundation & Prototyping

**Goal**: Establish `crates/pl3xus_async` with core abstractions.

### 1.1 Create Crate Structure
- [ ] Create `crates/pl3xus_async/Cargo.toml`
- [ ] Add to workspace in root `Cargo.toml`
- [ ] Set up feature flags:
  - `native` (default): Tokio runtime
  - `wasm`: wasm-bindgen-futures runtime
  - `orchestration`: bevy-async-ecs integration

### 1.2 Runtime Abstraction
- [ ] Create `src/runtime.rs` with WASM/native abstraction
- [ ] Implement unified `spawn()` function
- [ ] Implement unified `sleep()` function
- [ ] Implement `timeout()` wrapper

```rust
// Target API
use pl3xus_async::runtime;

runtime::spawn(async { /* ... */ });
runtime::sleep(Duration::from_secs(1)).await;
runtime::timeout(Duration::from_secs(5), async { /* ... */ }).await;
```

### 1.3 Basic Plugin
- [ ] Create `Pl3xusAsyncPlugin`
- [ ] Initialize `AsyncWorld` from bevy-async-ecs
- [ ] Expose `TokioTasksRuntime` (re-export from bevy-tokio-tasks)

```rust
// Target API
app.add_plugins(Pl3xusAsyncPlugin::default());
```

### 1.4 Verification
- [ ] Create `examples/pl3xus_async_demo.rs` in fanuc workspace
- [ ] Test basic spawn on native
- [ ] Test basic spawn on WASM (trunk serve)

---

## Phase 2: Safety Features (Critical)

**Goal**: Implement cancellation and task tracking for safety-critical operations.

### 2.1 Cancellation Token
- [ ] Create `src/cancellation.rs`
- [ ] Implement `CancellationToken` (based on tokio_util pattern)
- [ ] Implement `CancellableTask<T>` wrapper

```rust
// Target API
let token = CancellationToken::new();
let task = spawn_cancellable(token.clone(), async {
    // Task logic
});

// Later:
token.cancel();  // Task aborts at next await point
```

### 2.2 Task Registry
- [ ] Create `src/registry.rs`
- [ ] Implement `AsyncTaskRegistry` resource
- [ ] Add task categorization (motion, io, general)
- [ ] Add entity-scoped tracking

```rust
// Target API
#[derive(Resource)]
pub struct AsyncTaskRegistry {
    motion_tasks: HashMap<Entity, Vec<CancellableHandle>>,
    io_tasks: Vec<CancellableHandle>,
}

impl AsyncTaskRegistry {
    pub fn register_motion(&mut self, entity: Entity, handle: CancellableHandle);
    pub fn cancel_entity(&mut self, entity: Entity);
    pub fn cancel_all_motion(&mut self);
    pub fn cancel_all(&mut self);
}
```

### 2.3 Safety Integration
- [ ] Create `SafetyStop` event
- [ ] Add `safety_stop_system` that cancels all motion
- [ ] Test cancellation latency (target: <100ms)

```rust
// Target usage
fn safety_stop_handler(
    mut events: MessageReader<SafetyStopEvent>,
    mut registry: ResMut<AsyncTaskRegistry>,
) {
    for _ in events.read() {
        registry.cancel_all_motion();
    }
}
```

### 2.4 Verification
- [ ] Write test: spawn 10 motion tasks, trigger stop, verify all cancelled
- [ ] Measure cancellation latency
- [ ] Test with actual robot motion sequence

---

## Phase 3: Supervision & Error Handling

**Goal**: Implement error propagation and task supervision.

### 3.1 Async Supervisor
- [ ] Create `src/supervisor.rs`
- [ ] Implement `AsyncSupervisor` resource
- [ ] Implement `AsyncError` event type
- [ ] Add error severity levels

```rust
#[derive(Message)]
pub struct AsyncError {
    pub source: Entity,
    pub operation: String,
    pub error: anyhow::Error,
    pub severity: ErrorSeverity,
}

#[derive(Clone, Copy)]
pub enum ErrorSeverity {
    Warning,   // Log only
    Error,     // Log + state change
    Critical,  // Log + state + alert
}
```

### 3.2 Supervised Task Wrapper
- [ ] Create `supervised_spawn()` function
- [ ] Implement automatic error capture
- [ ] Send errors to supervisor channel

```rust
// Target API
pub fn supervised_spawn<F, T>(
    ctx: &AsyncContext,
    entity: Entity,
    operation: &str,
    fut: F,
) -> CancellableTask<Option<T>>
where
    F: Future<Output = Result<T, anyhow::Error>>,
```

### 3.3 Error Handler System
- [ ] Create `error_handler_system`
- [ ] Propagate errors to entity states
- [ ] Add hooks for custom error handling

```rust
fn error_handler_system(
    mut errors: MessageReader<AsyncError>,
    mut robots: Query<&mut RobotConnectionState>,
) {
    for error in errors.read() {
        if error.severity >= ErrorSeverity::Error {
            if let Ok(mut state) = robots.get_mut(error.source) {
                *state = RobotConnectionState::Fault;
            }
        }
    }
}
```

### 3.4 Verification
- [ ] Test: spawn task that errors, verify state changes
- [ ] Test: spawn task that panics, verify handled gracefully
- [ ] Review all error paths in existing handlers

---

## Phase 4: Ergonomics & Sugar

**Goal**: Make async code as ergonomic as synchronous ECS code.

### 4.1 Orchestration Builder
- [ ] Create `src/orchestration.rs`
- [ ] Implement `Sequence` builder for step-by-step flows
- [ ] Add common patterns (wait_for_component, wait_for_signal)

```rust
// Target API
Sequence::new(async_world, entity)
    .step("Move to position", |ctx| async move {
        ctx.run_system(move_robot_system).await
    })
    .step("Wait for arrival", |ctx| async move {
        ctx.wait_for_component::<AtPosition>().await
    })
    .step("Close gripper", |ctx| async move {
        ctx.run_system(close_gripper_system).await
    })
    .execute()
    .await
```

### 4.2 Context Object
- [ ] Create `AsyncContext` that bundles common resources
- [ ] Include registry, supervisor, async_world
- [ ] Make it Clone + Send

```rust
#[derive(Clone)]
pub struct AsyncContext {
    registry: AsyncTaskRegistry,
    supervisor: AsyncSupervisor,
    async_world: AsyncWorld,
}

impl AsyncContext {
    pub fn spawn(&self, ...) -> CancellableTask<T>;
    pub fn spawn_supervised(&self, ...) -> CancellableTask<Option<T>>;
    pub fn run_system<S>(&self, system: S) -> impl Future<Output = ()>;
}
```

### 4.3 Macros (Optional)
- [ ] Research proc-macro for `#[async_handler]`
- [ ] Evaluate if macros add significant value
- [ ] If yes, implement basic transformation

```rust
// Possible future API
#[async_handler]
async fn handle_motion_request(
    ctx: AsyncContext,
    request: MotionRequest,
) -> Result<MotionResponse, AsyncError> {
    // Much cleaner than current pattern
}
```

### 4.4 Verification
- [ ] Create example showcasing new ergonomics
- [ ] Compare line counts: before vs after
- [ ] Get feedback on API design

---

## Phase 5: Migration Pilot

**Goal**: Migrate real fanuc handlers to validate the architecture.

### 5.1 Select Pilot Handlers
- [ ] Choose 3 handlers of varying complexity:
  - Simple: `handle_get_frame_data` (read-only)
  - Medium: `handle_write_frame_data` (write + response)
  - Complex: Motion sequence handler

### 5.2 Migration Process (per handler)
- [ ] Create new handler using pl3xus_async
- [ ] Keep old handler for A/B testing
- [ ] Validate identical behavior
- [ ] Measure any performance difference

### 5.3 Document Migration Patterns
- [ ] Create migration guide document
- [ ] Document common transformations
- [ ] List gotchas and pitfalls

### 5.4 Verification
- [ ] Full test cycle with migrated handlers
- [ ] Performance benchmarks
- [ ] Code review for patterns

---

## Phase 6: Documentation & Integration

**Goal**: Integrate with pl3xus skills system and document thoroughly.

### 6.1 Crate Documentation
- [ ] Comprehensive `README.md` for pl3xus_async
- [ ] API docs with examples on all public items
- [ ] Architecture overview document

### 6.2 Skills Integration
- [ ] Create `skills/pl3xus-async/SKILL.md`
- [ ] Add reference patterns:
  - Simple background task
  - Supervised task
  - Cancellable sequence
  - Error handling
- [ ] Update `skills/pl3xus-development/SKILL.md` to reference async skill

### 6.3 Examples
- [ ] `examples/async_basic.rs` - Simple spawn and wait
- [ ] `examples/async_sequence.rs` - Orchestrated sequence
- [ ] `examples/async_cancellation.rs` - Safety stop demo
- [ ] `examples/async_supervision.rs` - Error handling demo

### 6.4 Verification
- [ ] Review all documentation for accuracy
- [ ] Test all examples compile and run
- [ ] Get user feedback on clarity

---

## Definition of Done

The `pl3xus_async` crate is complete when:

### Functional Requirements ✅
- [ ] All async operations are cancellable
- [ ] Errors propagate to entity state automatically
- [ ] WASM and native work identically
- [ ] Sequences can be expressed linearly

### Non-Functional Requirements ✅
- [ ] No measurable performance regression on 60Hz loops
- [ ] Cancellation latency < 100ms
- [ ] All existing handlers can be migrated

### Documentation Requirements ✅
- [ ] API documentation complete
- [ ] Migration guide available
- [ ] Skills integration complete
- [ ] At least 4 working examples

### Test Requirements ✅
- [ ] Unit tests for core abstractions
- [ ] Integration tests for cancellation
- [ ] WASM smoke tests

---

## Dependencies

```toml
# crates/pl3xus_async/Cargo.toml
[dependencies]
bevy = { version = "0.17", default-features = false }
bevy-async-ecs = "0.9"
bevy-tokio-tasks = { git = "https://github.com/vertec-io/async_bevy_web.git", package = "bevy-tokio-tasks" }
anyhow = "1.0"
tracing = "0.1"

[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "time", "sync"] }
tokio-util = "0.7"

[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen-futures = "0.4"
gloo-timers = "0.3"
web-time = "1.0"

[features]
default = ["native"]
native = []
wasm = []
```

---

## Risk Mitigations

| Risk | Mitigation |
|------|------------|
| bevy-async-ecs API changes | Fork into crates/pl3xus_async_ecs, maintain internally |
| WASM differences cause bugs | Comprehensive WASM testing early in Phase 1 |
| Migration breaks existing handlers | Keep old handlers during transition, A/B test |
| Performance regression | Benchmark critical paths before/after each phase |

---

## Success Metrics

Track these throughout implementation:

1. **Lines of code per handler** (target: 50% reduction)
2. **Cancellation latency** (target: <100ms)
3. **Error handling coverage** (target: 100% of async operations)
4. **Test coverage** (target: >80%)
5. **Documentation completeness** (target: all public API documented)

---

## Timeline Summary

```
Week 1:  [ Phase 1: Foundation -------- ]
Week 2:  [ Phase 1 ][ Phase 2: Safety -- ]
Week 3:  [ Phase 2 --------- ][ Phase 3: ]
Week 4:  [ Phase 3: Supervision -------- ]
Week 5:  [ Phase 4: Ergonomics ][ P5 --- ]
Week 6:  [ Phase 5: Migration Pilot ---- ]
Week 7:  [ Phase 6: Documentation ----- ]
```

**Total Estimated Duration**: 6-7 weeks for full implementation

---

## Conclusion

This implementation plan provides a clear path to a world-class async system for pl3xus. By building on proven foundations and focusing on industrial requirements, we can deliver a solution that treats async as a true first-class citizen in the framework.

The key principles guiding this implementation:
1. **Safety first**: Cancellation and supervision are non-negotiable
2. **Ergonomics matter**: Developer productivity directly impacts product quality
3. **Incremental adoption**: Existing code continues working during migration
4. **Platform parity**: WASM and native behave identically

Execute this plan phase by phase, validating at each checkpoint before proceeding.
