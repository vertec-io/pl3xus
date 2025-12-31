# Decision Matrix - Comprehensive Evaluation

This document provides a rigorous scoring of all async candidates against pl3xus industrial/robotics requirements.

---

## Evaluation Criteria

| Criteria | Weight | Description | Why It Matters |
|----------|--------|-------------|----------------|
| **Ergonomics** | Critical | How easy to write, read, and maintain async logic | Developer productivity, code quality, bug reduction |
| **Performance** | Critical | Must NOT block main thread. 60+ FPS required | Robot control loops, UI responsiveness |
| **Safety** | Critical | Prevention of race conditions, deadlocks, data corruption | Industrial system reliability |
| **WASM Support** | High | Client runs in browser for control panel UI | Core architecture requirement |
| **Tokio Ecosystem** | High | Access to tokio::time, tokio::sync, sqlx, etc. | Database access, timeouts, network libraries |
| **Cancellation** | High | Ability to abort running tasks for safety stops | Safety-critical robot operations |
| **ECS Integration** | Medium | How naturally it works with Bevy's ECS patterns | Consistency with rest of codebase |
| **Maturity** | Medium | Stability, bug-tested, community adoption | Production reliability |
| **Maintenance** | Medium | Ongoing effort to keep working with Bevy updates | Long-term sustainability |

---

## Detailed Scoring (1-10 Scale)

### Legend
- **10**: Exceptional, best-in-class
- **7-9**: Strong, minor limitations
- **4-6**: Adequate, notable trade-offs
- **1-3**: Weak, significant issues

### Candidate Scores

| Criteria | bevy-async-ecs | bevy_async_task | bevy_flurx | bevy_defer | bevy_mod_async | bevy-tokio-tasks |
|----------|----------------|-----------------|------------|------------|----------------|------------------|
| **Ergonomics** | **8** | 6 | 7 | 7 | 7 | 3 |
| **Performance** | 6 | **9** | 5 | 5 | 6 | **9** |
| **Safety** | **8** | 7 | 7 | 8 | 7 | 7 |
| **WASM Support** | **9** | **9** | 4 | 4 | 5 | 8 |
| **Tokio Ecosystem** | 5 | 3 | 6 | 3 | 3 | **10** |
| **Cancellation** | 4 | 3 | 5 | 4 | 3 | 5 |
| **ECS Integration** | **8** | 4 | 7 | 7 | **8** | 4 |
| **Maturity** | 7 | 6 | 5 | 5 | 4 | **8** |
| **Maintenance** | **8** | 7 | 5 | 5 | 4 | 4 |

### Weighted Totals

Using weights: Critical=3x, High=2x, Medium=1x

| Candidate | Raw Total | Weighted Score | Rank |
|-----------|-----------|----------------|------|
| **bevy-async-ecs** | 63 | **108** | **1** |
| **bevy-tokio-tasks** | 58 | **101** | **2** |
| bevy_async_task | 54 | 90 | 3 |
| bevy_flurx | 51 | 82 | 4 |
| bevy_defer | 48 | 79 | 5 |
| bevy_mod_async | 47 | 78 | 6 |

---

## Detailed Analysis by Criterion

### Ergonomics (Critical)

| Solution | Score | Justification |
|----------|-------|---------------|
| bevy-async-ecs | **8** | Linear control flow, register_system pattern, familiar async/await |
| bevy_flurx | 7 | Reactor/Action model is clean but requires learning new paradigm |
| bevy_defer | 7 | Familiar query syntax, but deferred semantics can be confusing |
| bevy_mod_async | 7 | with_world is intuitive, but limited to simple patterns |
| bevy_async_task | 6 | Minimal but lacks ECS interaction ergonomics |
| **bevy-tokio-tasks** | **3** | Nested closures, raw World access, verbose boilerplate |

Current code complexity example:
```rust
// bevy-tokio-tasks: 15+ lines of boilerplate
tokio_runtime.spawn_background_task(move |mut ctx| async move {
    // ... async work ...
    ctx.run_on_main_thread(move |ctx| {
        let mut query = ctx.world.query_filtered::<...>();
        // ... manual query iteration ...
    }).await;
});

// bevy-async-ecs: 5 lines
let system = async_world.register_system(my_system).await;
system.run().await;
```

### Performance (Critical)

| Solution | Score | Justification |
|----------|-------|---------------|
| bevy-tokio-tasks | **9** | Direct Tokio execution, minimal overhead |
| bevy_async_task | **9** | Minimal abstraction, efficient polling |
| bevy-async-ecs | 6 | Channel round-trips add overhead per operation |
| bevy_mod_async | 6 | Exclusive access may delay operations |
| bevy_flurx | 5 | Main thread execution limits throughput |
| bevy_defer | 5 | Frame delay inherent in design |

**Note**: For high-frequency operations (60Hz+ control loops), channel overhead matters. For low-frequency orchestration (1-10Hz sequences), it's negligible.

### WASM Support (High)

| Solution | Score | Justification |
|----------|-------|---------------|
| bevy-async-ecs | **9** | ✅ Native WASM support, browser examples included |
| bevy_async_task | **9** | ✅ Built specifically for WASM parity |
| bevy-tokio-tasks | 8 | ✅ Fork includes gloo-timers/wasm-bindgen-futures |
| bevy_mod_async | 5 | Untested, may work but not documented |
| bevy_flurx | 4 | Tokio features won't work in WASM |
| bevy_defer | 4 | Main thread focus helps, but IO features don't translate |

### Tokio Ecosystem (High)

| Solution | Score | Justification |
|----------|-------|---------------|
| bevy-tokio-tasks | **10** | Full Tokio runtime, all ecosystem crates work |
| bevy_flurx | 6 | Tokio integration via side_effects feature |
| bevy-async-ecs | 5 | Can use Tokio as executor, but not native |
| bevy_async_task | 3 | Uses smol/bevy_tasks, not Tokio |
| bevy_defer | 3 | Own runtime, no Tokio |
| bevy_mod_async | 3 | Tied to bevy_tasks |

**Why This Matters**: Libraries like `sqlx`, `reqwest`, `tokio-tungstenite` require Tokio runtime context.

### Cancellation (High)

| Solution | Score | Justification |
|----------|-------|---------------|
| bevy_flurx | 5 | Reactor can be despawned to cancel |
| bevy-tokio-tasks | 5 | Tokio AbortHandle available but not integrated |
| bevy-async-ecs | 4 | No built-in, must implement CancellationToken |
| bevy_defer | 4 | No built-in cancellation |
| bevy_async_task | 3 | Tasks run to completion |
| bevy_mod_async | 3 | No cancellation support |

**Gap Identified**: ALL solutions lack first-class cancellation. `pl3xus_async` must implement this.

---

## Use Case Fit Analysis

### Use Case 1: Robot Command Sequences
"Move to position A → Wait for sensor → Close gripper → Move to position B"

| Solution | Fit | Notes |
|----------|-----|-------|
| **bevy-async-ecs** | ⭐⭐⭐⭐⭐ | Designed for exactly this pattern |
| bevy_flurx | ⭐⭐⭐⭐ | Good fit, Reactor model works well |
| bevy-tokio-tasks | ⭐⭐ | Awkward with nested callbacks |

### Use Case 2: High-Frequency Sensor Polling
"Poll robot position at 60Hz, update ECS components"

| Solution | Fit | Notes |
|----------|-----|-------|
| **bevy-tokio-tasks** | ⭐⭐⭐⭐⭐ | Direct Tokio, minimal overhead |
| Standard ECS system | ⭐⭐⭐⭐ | May not need async at all |
| bevy-async-ecs | ⭐⭐ | Channel overhead too high |

### Use Case 3: Database Operations
"Load/save robot configurations to SQLite"

| Solution | Fit | Notes |
|----------|-----|-------|
| **bevy-tokio-tasks** | ⭐⭐⭐⭐⭐ | sqlx works natively |
| bevy-async-ecs | ⭐⭐⭐ | Can use Tokio executor |
| Others | ⭐⭐ | Require bridging to Tokio |

### Use Case 4: WASM Control Panel
"Web UI for robot control with real-time updates"

| Solution | Fit | Notes |
|----------|-----|-------|
| **bevy-async-ecs** | ⭐⭐⭐⭐⭐ | First-class WASM support |
| bevy_async_task | ⭐⭐⭐⭐⭐ | Built for WASM parity |
| bevy-tokio-tasks | ⭐⭐⭐⭐ | Fork has WASM support |

---

## Conclusion

### Clear Winner: Hybrid Approach

No single solution wins across all criteria. The data supports a **layered architecture**:

1. **Keep bevy-tokio-tasks** for:
   - Database operations (sqlx)
   - Low-level hardware IO
   - High-frequency background tasks
   - Tokio ecosystem integration

2. **Add bevy-async-ecs** for:
   - Orchestration and sequencing
   - Improved ergonomics for scripted behaviors
   - WASM-compatible async patterns

3. **Build pl3xus_async wrapper** to:
   - Unify both approaches behind ergonomic API
   - Add task supervision and cancellation
   - Provide macros for common patterns
   - Abstract away WASM/native differences

### Next Steps

1. Prototype the hybrid approach in a test module
2. Benchmark channel overhead vs direct Tokio for critical paths
3. Design cancellation token integration
4. Implement pl3xus_async crate

See `recommendation.md` for detailed strategy and `implementation_plan.md` for execution roadmap.
