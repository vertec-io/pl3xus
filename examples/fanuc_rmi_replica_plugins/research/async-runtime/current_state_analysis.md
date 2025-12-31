# Current State Analysis: bevy-tokio-tasks (Vertec Fork - async_bevy_web)

## Overview

Currently, pl3xus relies on a custom fork of `bevy-tokio-tasks` called `async_bevy_web` to handle asynchronous operations. This library provides a bridge between Bevy's synchronous ECS and Tokio's asynchronous runtime.

**Repository**: https://github.com/vertec-io/async_bevy_web  
**Dependency**: `bevy-tokio-tasks = { git = "https://github.com/vertec-io/async_bevy_web.git", package = "bevy-tokio-tasks"}`

## Architecture

- **Runtime**: A standard multi-threaded Tokio runtime is spawned in a background thread.
- **Plugin**: `TokioTasksPlugin` handles the setup and cleanup.
- **Resource**: `TokioTasksRuntime` is exposed as a Bevy Resource to spawn tasks.
- **Bridging**: Tasks are spawned with a `TaskContext`. This context allows the async task to dispatch closures back to the main Bevy thread.

## Current Usage in fanuc_rmi_replica

Based on analysis of the codebase, `spawn_background_task` is used extensively:
- **12+ call sites** across handlers.rs, connection.rs, jogging.rs, motion.rs, polling.rs
- **Pattern**: All async robot communication (sensor reads, frame/tool data, motion commands)

### Typical Usage Pattern

```rust
fn handle_get_frame_data(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut requests: MessageReader<Request<TargetedRequest<GetFrameData>>>,
    robots: Query<(&FrameToolDataState, Option<&RmiDriver>, &RobotConnectionState), With<FanucRobot>>,
) {
    // Enter the Tokio runtime context
    let _guard = tokio_runtime.runtime().enter();

    for request in requests.read() {
        // ... extract data ...
        
        if let Some((_, Some(driver), _)) = robot_info {
            let driver = driver.0.clone();
            let request = request.clone();

            // --- SPAWN ASYNC TASK ---
            tokio_runtime.spawn_background_task(move |mut ctx| async move {
                // Send packet to robot
                if let Err(e) = driver.send_packet(packet, PacketPriority::Standard) {
                    // Handle error
                    return;
                }

                // Wait for response with timeout
                let result = tokio::time::timeout(Duration::from_secs(5), async {
                    while let Ok(response) = response_rx.recv().await {
                        if let ResponsePacket::CommandResponse(CommandResponse::FrcReadUFrameData(resp)) = response {
                            return Some(resp);
                        }
                    }
                    None
                }).await;

                match result {
                    Ok(Some(resp)) => {
                        // Update ECS on main thread
                        ctx.run_on_main_thread(move |ctx| {
                            let mut query = ctx.world.query_filtered::<&mut FrameToolDataState, With<FanucRobot>>();
                            for mut ft_state in query.iter_mut(ctx.world) {
                                ft_state.frames.insert(frame_num, frame_data_clone.clone());
                            }
                        }).await;

                        let _ = request.respond(response);
                    }
                    // ... error handling ...
                }
            });
        }
    }
}
```

## Critical Analysis

### Pros

1. **Non-Blocking**: Heavy IO operations correctly run off the main thread, preserving frame rates.
2. **Standard Tokio**: Full access to the rich Tokio ecosystem (tokio::time, tokio::sync, etc.).
3. **Production Proven**: We have used this in production for fanuc_rmi_replica with 12+ async endpoints.
4. **WASM Support**: The fork includes `gloo-timers` / `wasm-bindgen-futures` support for web targets.
5. **Runtime Context Access**: `runtime().enter()` allows using Tokio-dependent code in sync systems.

### Cons

1. **Poor Ergonomics**:
   - Nested closures ("callback hell") make control flow hard to follow.
   - Distinct "Async Context" vs "Main Thread Context" confuses ownership and variable scope.
   - `move` keywords are required everywhere, often requiring explicit `.clone()` before the closure.
   
2. **Raw World Access**:
   - The callback gives `&mut World`.
   - To use Queries/Resources comfortably, we must manually construct `query` on ctx.world.
   - This boilerplate is error-prone and verbose compared to normal System params.

3. **No Task Supervision**:
   - Tasks are "fire and forget" - no built-in way to track, cancel, or supervise them.
   - Safety-critical operations (e.g., robot motion) have no graceful cancellation.

4. **Maintenance Burden**:
   - We are maintaining a fork.
   - Updates to Bevy (e.g. 0.17) require us to manually update the fork.
   - No community support or bug fixes from upstream.

5. **Response Handling Complexity**:
   - Pattern of `request.clone().respond(response)` is repetitive and easy to forget.
   - Error paths often duplicate significant code.

## Quantitative Analysis

| Metric | Current State |
|--------|---------------|
| Lines of async code per handler | 50-100 lines |
| Callback nesting depth | 2-3 levels |
| Explicit `.clone()` calls per handler | 3-5 |
| Time to write new async handler | ~30 min |
| Bug risk areas | World access, error propagation, cancellation |

## Pain Points by Category

### 1. Boilerplate (High Impact)
Every async handler requires:
- `let _guard = tokio_runtime.runtime().enter();`
- `let driver = driver.0.clone();`
- `let request = request.clone();`
- `tokio_runtime.spawn_background_task(move |mut ctx| async move { ... });`
- `ctx.run_on_main_thread(move |ctx| { ... }).await;`

### 2. Error Handling (Medium Impact)
- Errors in async tasks need explicit logging - panics are swallowed.
- Response errors need handling in multiple code paths.
- No standardized error propagation to ECS state.

### 3. Cancellation (Critical for Safety)
- No way to abort running tasks if a safety stop is triggered.
- Robot motion tasks continue executing even after disconnect.
- `AbortHandle` exists in Tokio but isn't integrated.

## Conclusion

While `bevy-tokio-tasks` works functionally, it acts as a **"second-class citizen"**. The developer experience is significantly degraded compared to standard synchronous systems. Writing async code feels like "escaping" the ECS rather than working within it.

**Key Insight**: The current solution is a *low-level bridge*. What we need is a *high-level orchestration layer* that:
1. Provides ergonomic syntax for common patterns
2. Integrates task lifecycle with ECS resources
3. Handles WASM/native runtime differences transparently
4. Offers cancellation and supervision for safety-critical operations

We need a solution that brings async code up to the same ergonomic standard as the rest of Bevy while maintaining the performance characteristics we depend on.
