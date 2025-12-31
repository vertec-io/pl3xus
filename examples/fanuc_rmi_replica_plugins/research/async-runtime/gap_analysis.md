# Gap Analysis - Industrial/Robotics Requirements

This document identifies features **missing from ALL candidates** that are critical for industrial/robotics applications, along with mitigation strategies for pl3xus.

---

## Overview

Industrial robotics systems have fundamentally different requirements than game development:
- **Safety-critical operations** that must be cancellable
- **Deterministic timing** for control loops
- **Graceful degradation** under failure conditions
- **Regulatory compliance** for certain industries
- **Long-running processes** (hours/days of operation)

None of the evaluated async solutions were designed with these requirements in mind.

---

## Critical Gap 1: Task Cancellation (Safety Critical) ⚠️

### Requirement
If a "Safety Stop" button is pressed, **ALL active async motion tasks must be cancelled immediately**.

### Current State by Solution

| Solution | Cancellation Support | Implementation |
|----------|---------------------|----------------|
| bevy-tokio-tasks | ⚠️ Partial | Tokio `AbortHandle` available but not integrated |
| bevy-async-ecs | ❌ None | No cancellation mechanism exposed |
| bevy_async_task | ❌ None | Tasks run to completion |
| bevy_flurx | ⚠️ Partial | Reactor despawn cancels, but not granular |
| bevy_defer | ❌ None | No cancellation |
| bevy_mod_async | ❌ None | No cancellation |

### Risk Scenario
```
1. User initiates "Move Robot to Position A" (async task starts)
2. User presses EMERGENCY STOP
3. Async task continues executing...
4. Robot continues moving for 2-5 seconds until task hits next await point
5. ❌ SAFETY VIOLATION
```

### Required Behavior
```
1. User initiates "Move Robot to Position A"
2. System registers task with CancellationToken
3. User presses EMERGENCY STOP
4. System triggers ALL active motion task cancellations
5. Within 50ms, all motion commands are aborted
6. ✅ Robot stops safely
```

### Mitigation Strategy

**Implement in pl3xus_async:**

```rust
/// CancellableTask wraps any async operation with abort capability
pub struct CancellableTask<T> {
    handle: JoinHandle<T>,
    cancellation_token: CancellationToken,
}

impl<T> CancellableTask<T> {
    pub fn cancel(&self) {
        self.cancellation_token.cancel();
    }
}

/// TaskRegistry tracks all active tasks for bulk cancellation
#[derive(Resource, Default)]
pub struct AsyncTaskRegistry {
    motion_tasks: HashMap<Entity, Vec<CancellableTask<()>>>,
    io_tasks: Vec<CancellableTask<()>>,
}

impl AsyncTaskRegistry {
    /// Called on safety stop - cancels all motion tasks
    pub fn cancel_all_motion(&mut self) {
        for (_, tasks) in self.motion_tasks.drain() {
            for task in tasks {
                task.cancel();
            }
        }
    }
}
```

**Integration with ECS:**

```rust
fn safety_stop_system(
    mut registry: ResMut<AsyncTaskRegistry>,
    safety_events: MessageReader<SafetyStopEvent>,
) {
    for _ in safety_events.read() {
        registry.cancel_all_motion();
        info!("🛑 All motion tasks cancelled");
    }
}
```

---

## Critical Gap 2: Error Propagation & Supervision ⚠️

### Requirement
If an async operation fails (sensor read error, network timeout, hardware fault), the system should:
1. Log the error with context
2. Transition affected entities to Fault state
3. Optionally notify operators
4. NOT crash the application

### Current State

| Behavior | Current Implementation |
|----------|----------------------|
| Task panics | Swallowed silently, logged to console |
| Result error | Must manually handle, often forgotten |
| State transition | Must be explicitly coded per-handler |
| Operator notification | Not standardized |

### Risk Scenario
```
1. Sensor read task panics due to network disconnect
2. Panic is caught by Tokio, logged as error
3. Robot entity stays in "Connected" state
4. UI shows robot as connected
5. User attempts motion command
6. ❌ Command silently fails
```

### Mitigation Strategy

**Implement Async Supervisor pattern:**

```rust
/// AsyncSupervisor monitors task outcomes and propagates state changes
#[derive(Resource)]
pub struct AsyncSupervisor {
    error_tx: mpsc::Sender<AsyncError>,
}

#[derive(Message)]
pub struct AsyncError {
    pub source: Entity,
    pub operation: String,
    pub error: anyhow::Error,
    pub severity: ErrorSeverity,
}

#[derive(Clone, Copy)]
pub enum ErrorSeverity {
    Warning,    // Log only
    Error,      // Log + state change
    Critical,   // Log + state change + alert
}

/// Supervised task wrapper that reports errors
pub async fn supervised_task<F, T>(
    supervisor: AsyncSupervisor,
    entity: Entity,
    operation: &str,
    fut: F,
) -> Option<T>
where
    F: Future<Output = Result<T, anyhow::Error>>,
{
    match fut.await {
        Ok(value) => Some(value),
        Err(e) => {
            let _ = supervisor.error_tx.send(AsyncError {
                source: entity,
                operation: operation.to_string(),
                error: e,
                severity: ErrorSeverity::Error,
            });
            None
        }
    }
}
```

**Error handler system:**

```rust
fn handle_async_errors(
    mut errors: MessageReader<AsyncError>,
    mut robots: Query<&mut RobotConnectionState>,
) {
    for error in errors.read() {
        error!("Async error on {:?}: {} - {}", error.source, error.operation, error.error);
        
        if error.severity >= ErrorSeverity::Error {
            if let Ok(mut state) = robots.get_mut(error.source) {
                *state = RobotConnectionState::Fault;
            }
        }
    }
}
```

---

## Critical Gap 3: WASM/Native Runtime Abstraction ⚠️

### Requirement
Pl3xus client runs in the browser. Code should work identically on WASM and native without feature-flag soup throughout the codebase.

### Current State

The `bevy-tokio-tasks` fork handles this, but inconsistently:
- Uses `gloo-timers` for WASM timing
- Uses `wasm-bindgen-futures` for spawning
- Some Tokio features (multi-threading) unavailable on WASM

### Risk Scenario
```rust
// Works on native, PANICS on WASM
tokio::spawn(async { /* ... */ });  // WASM has no multi-threading!

// Works on native, HANGS on WASM
tokio::time::sleep(Duration::from_secs(1)).await;  // Wrong timer source!
```

### Mitigation Strategy

**Unified runtime abstraction in pl3xus_async:**

```rust
// pl3xus_async/src/runtime.rs

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use tokio::runtime::Runtime;
    
    pub fn spawn<F>(fut: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send,
    {
        tokio::spawn(fut)
    }
    
    pub async fn sleep(duration: Duration) {
        tokio::time::sleep(duration).await
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use wasm_bindgen_futures::spawn_local;
    use gloo_timers::future::TimeoutFuture;
    
    pub fn spawn<F>(fut: F)
    where
        F: Future<Output = ()> + 'static,
    {
        spawn_local(fut)
    }
    
    pub async fn sleep(duration: Duration) {
        TimeoutFuture::new(duration.as_millis() as u32).await
    }
}

// Public API - same interface everywhere
pub use self::platform::*;

#[cfg(not(target_arch = "wasm32"))]
use native as platform;

#[cfg(target_arch = "wasm32")]
use wasm as platform;
```

---

## High Gap 4: Deterministic Replay (Medium Priority)

### Requirement
For debugging crashes, we want to replay inputs and understand task execution order.

### Current State
Async runtimes are inherently non-deterministic regarding execution order. Tasks may complete in different orders between runs.

### Mitigation Strategy

Rather than true replay, implement **Observability**:

```rust
/// Instrument all async tasks with tracing spans
#[instrument(skip_all, fields(entity = ?entity, operation = %operation))]
pub async fn traced_task<F, T>(
    entity: Entity,
    operation: &str,
    fut: F,
) -> T
where
    F: Future<Output = T>,
{
    let span = tracing::info_span!("async_task", entity = ?entity, op = %operation);
    fut.instrument(span).await
}
```

This enables:
- Task start/complete timestamps in logs
- Correlation of entity operations
- Duration tracking for performance analysis
- Integration with `tracing-subscriber` for structured logging

---

## Medium Gap 5: Graceful Shutdown

### Requirement
When application shuts down (user closes window, server restart), all async tasks should:
1. Be notified of impending shutdown
2. Have opportunity to complete gracefully (with timeout)
3. Clean up resources (close connections, save state)

### Current State
Tasks are abruptly terminated when Bevy app exits.

### Mitigation Strategy

**Shutdown coordination:**

```rust
#[derive(Resource)]
pub struct ShutdownToken {
    token: CancellationToken,
}

impl ShutdownToken {
    /// Signal all tasks to begin graceful shutdown
    pub fn initiate_shutdown(&self) {
        self.token.cancel();
    }
    
    /// Check if shutdown has been requested
    pub fn is_shutting_down(&self) -> bool {
        self.token.is_cancelled()
    }
}

// In async tasks:
pub async fn long_running_task(shutdown: ShutdownToken) {
    loop {
        tokio::select! {
            _ = shutdown.token.cancelled() => {
                info!("Received shutdown signal, cleaning up...");
                cleanup().await;
                break;
            }
            _ = do_work() => {
                // Continue working
            }
        }
    }
}
```

---

## Gap Priority Matrix

| Gap | Priority | Effort | Risk if Unaddressed |
|-----|----------|--------|---------------------|
| Task Cancellation | **Critical** | Medium | Safety violation, regulatory issues |
| Error Propagation | **Critical** | Low | Silent failures, data corruption |
| WASM Abstraction | **High** | Low | Broken web client |
| Graceful Shutdown | **High** | Low | Resource leaks, incomplete saves |
| Deterministic Replay | Medium | High | Difficult debugging |

---

## Implementation Recommendations

### Phase 1: Safety Foundation (Week 1-2)
1. Implement `CancellationToken` integration
2. Create `AsyncTaskRegistry` resource
3. Add `SafetyStopEvent` handler

### Phase 2: Error Handling (Week 2-3)
1. Implement `AsyncSupervisor` pattern
2. Create supervised task wrappers
3. Add error → state transition logic

### Phase 3: Runtime Abstraction (Week 3-4)
1. Create unified `pl3xus_async::runtime` module
2. Abstract spawn/sleep/timeout
3. Test on both WASM and native

### Phase 4: Observability (Week 4-5)
1. Add tracing spans to all async operations
2. Integrate with existing logging infrastructure
3. Add duration metrics for performance monitoring

---

## Conclusion

The largest gaps are **Task Cancellation** and **Error Propagation**. These are non-negotiable for industrial/robotics applications.

**Key Insight**: We cannot rely on "fire and forget" patterns. Industrial systems require **"fire and manage"** - every async operation must be:
- Trackable (who started it, for which entity)
- Cancellable (safety stops, graceful shutdown)
- Supervised (errors propagate to system state)
- Observable (logs, metrics, debugging)

The `pl3xus_async` crate must provide these guarantees as core features, not optional add-ons.
