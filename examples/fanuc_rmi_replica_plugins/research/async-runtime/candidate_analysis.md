# Comprehensive Candidate Analysis

This document provides an in-depth analysis of ALL viable async solutions for Bevy 0.17, evaluating their suitability for pl3xus industrial/robotics applications.

---

## Candidate 1: bevy-async-ecs

**Repository**: https://github.com/dlom/bevy-async-ecs  
**Crates.io**: https://crates.io/crates/bevy-async-ecs  
**Latest Version**: 0.9.0 (Supports Bevy 0.17)  
**License**: MIT/Apache 2.0

### Architecture

`bevy-async-ecs` provides an **AsyncWorld** - a thread-safe handle to the Bevy World that wraps an MPSC channel. It does NOT provide native `async fn` systems. Instead, it serves as an **Orchestration Layer** driven by an external async runtime.

- **AsyncWorld**: Entrypoint created via `FromWorld`. Cheaply cloneable, uses MPSC channels internally.
- **System Registration**: Synchronous Bevy systems are registered, returning a handle for async triggering.
- **Imperative Control**: Async tasks "drive" logic by sequentially calling `.run().await` on registered systems.

### Usage Pattern (Real Example)

```rust
use bevy::prelude::*;
use bevy::tasks::AsyncComputeTaskPool;
use bevy_async_ecs::*;

// Vanilla Bevy synchronous system
fn print_names(query: Query<(Entity, &Name)>) {
    for (id, name) in query.iter() {
        info!("entity {:?} has name '{}'", id, name);
    }
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, AsyncEcsPlugin))
        .add_systems(Startup, |world: &mut World| {
            let async_world = AsyncWorld::from_world(world);
            
            let fut = async move {
                // Register a vanilla synchronous system
                let print_names = async_world.register_system(print_names).await;
                
                // Spawn entity asynchronously
                let entity = async_world.spawn_named("Frank").await;
                
                // Trigger the system and wait for completion
                print_names.run().await;
                
                // Cleanup
                entity.despawn().await;
            };
            
            AsyncComputeTaskPool::get().spawn(fut).detach();
        })
        .run();
}
```

### Critical Analysis

#### Strengths
1. **Linear Control Flow**: Ideal for scripting sequences ("Move Arm" → "Close Gripper" → "Wait for Sensor").
2. **WASM Support**: ✅ Confirmed working - runs examples in browser with wasm-server-runner.
3. **Safe Bridge**: Thread-safe by design using channels, no raw `&mut World` in async context.
4. **CommandQueue Support**: Can batch multiple commands together atomically.
5. **Actively Maintained**: Regular updates tracking Bevy releases.

#### Weaknesses
1. **Not "Native" Async Systems**: Cannot write `async fn my_system(...)` directly.
2. **Channel Overhead**: Every `run().await` involves channel round-trips to the main thread.
3. **Different Paradigm**: Must treat async task as "Script/Director", not "System".
4. **No Built-in Cancellation**: Task cancellation must be implemented externally.

#### Suitability Score: 8/10
Best for: Orchestration, sequencing, scripted behaviors.

---

## Candidate 2: bevy_async_task

**Repository**: https://github.com/Zeenobit/bevy_async_task  
**Crates.io**: https://crates.io/crates/bevy_async_task  
**Latest Version**: 0.11.x (Supports Bevy 0.17)  
**License**: MIT

### Architecture

`bevy_async_task` provides **minimal ergonomic abstractions** for async programming. It focuses on:
- `TaskRunner<T>` system parameter for spawning and polling single tasks
- `TaskPool<T>` system parameter for managing multiple concurrent tasks
- Full WASM support via `wasm-bindgen-futures` and `web-time`

### Usage Pattern

```rust
use bevy::prelude::*;
use bevy_async_task::{AsyncTaskPlugin, TaskRunner};

fn spawn_async_task(mut task: TaskRunner<u32>) {
    if task.is_idle() {
        task.start(async {
            // Perform async computation
            let result = fetch_some_data().await;
            result.len() as u32
        });
    }
}

fn handle_task_result(mut task: TaskRunner<u32>) {
    if let Some(result) = task.poll() {
        info!("Task completed with result: {}", result);
    }
}
```

### Critical Analysis

#### Strengths
1. **Minimal API**: Very small learning curve, focused abstractions.
2. **WASM Native**: First-class browser support with `wasm-bindgen-futures`.
3. **System Param Integration**: `TaskRunner<T>` feels like natural Bevy code.
4. **Task Lifecycle**: Built-in polling for task completion.

#### Weaknesses
1. **No World Access in Tasks**: Tasks cannot interact with ECS directly.
2. **Limited Orchestration**: Not designed for complex sequencing.
3. **Communication Required**: Must use channels to pass data to/from tasks.
4. **No Cancellation**: Tasks run to completion once started.

#### Suitability Score: 6/10
Best for: Simple background computations, data fetching without ECS interaction.

---

## Candidate 3: bevy_flurx

**Repository**: https://github.com/not-elm/bevy_flurx  
**Crates.io**: https://crates.io/crates/bevy_flurx  
**Latest Version**: 0.8.x  
**License**: MIT

### Architecture

`bevy_flurx` provides **coroutine-like behavior** using "Reactors" and "Actions". Designed for:
- Sequential delays and user input waiting
- Character movement sequences
- State machine logic
- Tokio runtime integration via `side_effects`

### Usage Pattern

```rust
use bevy::prelude::*;
use bevy_flurx::prelude::*;

fn setup_reactor(mut commands: Commands) {
    commands.spawn(Reactor::schedule(|task| async move {
        // Wait for specific duration
        task.will(Update, wait::delay::frames(60)).await;
        
        // Move entity over time
        task.will(Update, {
            wait::until(|time: Res<Time>| time.elapsed_seconds() > 5.0)
        }).await;
        
        // Trigger another action
        task.will(Update, once::run(|mut state: ResMut<GameState>| {
            state.phase = Phase::Active;
        })).await;
    }));
}
```

### Critical Analysis

#### Strengths
1. **Coroutine Semantics**: Excellent for sequential game logic.
2. **Tokio Integration**: Can use Tokio via `side_effects` for actual async IO.
3. **Incremental Adoption**: Can be added to existing projects gradually.
4. **Rich Action Library**: Built-in actions for common patterns.

#### Weaknesses
1. **Main Thread Execution**: Reactor runs on main thread, must offload heavy work.
2. **Learning Curve**: Reactor/Action model is unique and takes time to learn.
3. **Not IO-Focused**: Designed for game logic, not industrial IO patterns.
4. **Overhead for Simple Cases**: Overkill for simple async operations.

#### Suitability Score: 5/10
Best for: Game-like sequential logic, animations, user input flows.

---

## Candidate 4: bevy_defer

**Repository**: https://github.com/mintlu8/bevy_defer  
**Crates.io**: https://crates.io/crates/bevy_defer  
**Latest Version**: 0.14.x  
**License**: MIT/Apache 2.0

### Architecture

`bevy_defer` provides **deferred queries with async semantics**. Features:
- `AsyncWorld` for deferred world access
- `Signals` for inter-entity communication
- Single-threaded runtime on main thread (NOT for CPU-heavy tasks)
- `AsyncWorld::unblock` for offloading to `AsyncComputeTaskPool`

### Usage Pattern

```rust
use bevy::prelude::*;
use bevy_defer::prelude::*;

fn spawn_async_entity(world: &mut World) {
    let async_world = AsyncWorld::from_world(world);
    
    spawn(async move {
        // Deferred query - waits for next frame
        let position = async_world.query::<&Transform>()
            .filter::<With<Player>>()
            .get_first()
            .await;
        
        // Signals for communication
        async_world.send_signal(PlayerMoved(position));
    });
}
```

### Critical Analysis

#### Strengths
1. **Deferred Queries**: Familiar query syntax with async semantics.
2. **Signals**: Robust inter-entity communication.
3. **Consistency Focused**: Prioritizes data consistency over parallelism.
4. **Familiar API**: Query syntax mirrors standard Bevy.

#### Weaknesses
1. **Single-Threaded**: Main thread execution, must explicitly offload CPU work.
2. **Frame Delay**: Deferred operations execute on next frame.
3. **Not for IO**: Designed for wait-heavy logic, not network/hardware IO.
4. **Parallelism Trade-off**: Consistency comes at cost of parallel execution.

#### Suitability Score: 4/10
Best for: Turn-based games, UI reactivity, state machines.

---

## Candidate 5: bevy_mod_async

**Repository**: https://github.com/mintlu8/bevy_mod_async  
**Crates.io**: https://crates.io/crates/bevy_mod_async  
**License**: MIT

### Architecture

`bevy_mod_async` builds on `bevy_tasks` executor to provide:
- `commands.spawn_task()` for spawning async tasks
- `TaskContext::with_world` for exclusive async World access
- `WithWorld` futures that apply modifications on await

### Usage Pattern

```rust
use bevy::prelude::*;
use bevy_mod_async::prelude::*;

fn spawn_task(mut commands: Commands) {
    commands.spawn_task(|ctx| async move {
        // Request exclusive world access
        ctx.with_world(|world| {
            // Modify world synchronously
            world.spawn(SomeComponent);
        }).await;
    });
}
```

### Critical Analysis

#### Strengths
1. **Ergonomic API**: `with_world` is intuitive for ECS interaction.
2. **Built on bevy_tasks**: Uses Bevy's native task infrastructure.

#### Weaknesses
1. **Tied to Bevy Runtime**: Difficult to integrate with Tokio.
2. **Frame Delay**: World accesses wait until exclusive system runs.
3. **Limited Parallelism**: World access blocks other async operations.
4. **Maturity**: Less battle-tested than alternatives.

#### Suitability Score: 5/10
Best for: Simple async patterns within Bevy's ecosystem.

---

## Candidate 6: Current Solution (bevy-tokio-tasks fork)

See `current_state_analysis.md` for detailed analysis.

### Summary
- **Strengths**: Proven, Tokio ecosystem, WASM support, runtime context access
- **Weaknesses**: Poor ergonomics, raw World access, no supervision, maintenance burden

#### Suitability Score: 7/10
Best for: Low-level IO operations where Tokio integration is required.

---

## Summary Comparison Table

| Feature | bevy-async-ecs | bevy_async_task | bevy_flurx | bevy_defer | bevy_mod_async | bevy-tokio-tasks |
|---------|----------------|-----------------|------------|------------|----------------|------------------|
| Bevy 0.17 | ✅ v0.9.0 | ✅ v0.11.x | ⚠️ Check | ⚠️ Check | ⚠️ Check | ✅ Fork |
| WASM Support | ✅ Native | ✅ Native | ⚠️ Limited | ⚠️ Limited | ⚠️ Limited | ✅ Fork |
| ECS Access | ✅ Via systems | ❌ Channels | ✅ Actions | ✅ Queries | ✅ with_world | ✅ Callback |
| Tokio | ⚠️ External | ❌ | ⚠️ side_fx | ❌ | ❌ | ✅ Native |
| Orchestration | ✅ Excellent | ❌ | ✅ Excellent | ⚠️ Good | ⚠️ Fair | ⚠️ Manual |
| Cancellation | ⚠️ External | ❌ | ⚠️ Manual | ⚠️ Manual | ⚠️ Manual | ⚠️ AbortHandle |
| Performance | ⚠️ Channel | ✅ Minimal | ⚠️ Main thread | ⚠️ Frame delay | ⚠️ Exclusive | ✅ Direct Tokio |
| Ergonomics | 8/10 | 6/10 | 7/10 | 7/10 | 7/10 | 4/10 |

---

## Conclusion

**No single solution addresses all pl3xus requirements.** The ideal approach is a **layered architecture**:

1. **Low-level IO Layer**: Continue using `bevy-tokio-tasks` (or similar) for raw Tokio access, network connections, and hardware communication.

2. **Orchestration Layer**: Adopt `bevy-async-ecs` for high-level scripting and sequencing of robot operations.

3. **Wrapper Layer**: Build `pl3xus_async` to provide:
   - Unified API hiding the complexity of both layers
   - Task supervision and cancellation
   - WASM/native runtime abstraction
   - Ergonomic macros for common patterns

See `recommendation.md` for the detailed strategy.
