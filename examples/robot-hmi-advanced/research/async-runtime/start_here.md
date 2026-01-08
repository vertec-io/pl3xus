# Async Runtime Research - Start Here

> **Status**: COMPREHENSIVE REVIEW COMPLETE
> **Goal**: Evaluate and recommend a first-class async runtime solution for pl3xus.
> **Last Updated**: 2024-12-30

## Quick Context

Pl3xus applications (industrial/robotics) are IO-heavy. We interact with sensors, PLCs, and robots which requires robust async support. The default Bevy ECS is synchronous. Improving the ergonomics and capabilities of async operations is critical for code maintainability and system responsiveness.

**The Core Challenge**: Bevy is designed for games, which run synchronously at 60+ FPS. Industrial applications have fundamentally different requirements:
- Long-running network operations (robot connections, sensor polling)
- Request/response patterns with external hardware
- Graceful cancellation for safety-critical operations
- WASM support for web-based control panels

## Research Objectives

1. **Evaluate Current State**: Analyze the pros/cons of our current `bevy-tokio-tasks` fork (async_bevy_web).
2. **Comprehensive Candidate Evaluation**: Deep dive into ALL viable async solutions:
   - `bevy-async-ecs` - Async World interface (v0.9.0 supports Bevy 0.17)
   - `bevy_async_task` - Minimal ergonomic abstractions (v0.9-0.11 support Bevy 0.17)
   - `bevy_flurx` - Coroutine-like sequential logic
   - `bevy_defer` - Deferred queries with signals
   - `bevy_mod_async` - ECS-integrated async
3. **Recommendation**: Select the best path forward (adopt, fork, extend, or build).
4. **Specification**: Design a first-class API for async systems in pl3xus.

## Files in This Research Folder

| File | Description |
|------|-------------|
| `start_here.md` | This file - overview and index |
| `README.md` | Full background and context for this research |
| `current_state_analysis.md` | Analysis of `bevy-tokio-tasks` usage and limitations |
| `candidate_analysis.md` | Deep dive into ALL candidates with updated analysis |
| `decision_matrix.md` | Scoring matrix and trade-off analysis |
| `gap_analysis.md` | Missing features and risks for industrial use |
| `recommendation.md` | Final recommendation and justification |
| `implementation_plan.md` | Roadmap for implementing the chosen solution |

## Key Concepts

- **Sync ECS**: Bevy's default mode. Systems run once per tick at ~60Hz.
- **Async Runtime**: Tokio (native) or wasm-bindgen-futures (web) handling IO/Tasks.
- **Bridging**: The mechanism to communicate between Sync ECS and Async Runtime.
- **Ergonomics**: How easy it is to write async code that interacts with the World.
- **Orchestration**: Using async for sequencing ("move arm", then "close gripper").
- **Task Supervision**: Managing task lifecycle, cancellation, and error propagation.

## The Core Questions

1. **Can we achieve a "native" async system experience in Bevy (e.g. `async fn system(...)`) without compromising safety or performance?**
   - **Answer**: No, not directly. Bevy's scheduler is fundamentally synchronous. All solutions are wrappers/bridges.

2. **What is the ideal pattern for industrial robotics async code?**
   - **Answer**: A layered approach:
     - High-frequency control loops (60Hz+) → Standard ECS systems
     - Orchestration/sequencing → Async coroutine-style logic
     - Heavy IO (database, network) → Background tasks with main-thread callbacks

## Critical Insights from Review

### What the Original Research Got Right
- Correctly identified `bevy-tokio-tasks` ergonomic limitations
- Recognized `bevy-async-ecs` as a strong candidate
- Identified task cancellation as a critical safety gap

### What the Original Research Missed
1. **Incomplete Candidate Analysis**: Didn't evaluate `bevy_flurx`, `bevy_defer`, `bevy_async_task`, `bevy_mod_async`
2. **Paradigm Mismatch**: `bevy-async-ecs` is for *orchestration*, not replacing systems
3. **WASM Verification**: Claimed uncertainty about `bevy-async-ecs` WASM support - it's confirmed to work
4. **Performance Overhead**: Channel-based approaches have measurable overhead for high-frequency operations
5. **Hybrid Strategy**: The ideal solution is a *layered* approach, not a single replacement

## Next Steps

1. Review `current_state_analysis.md` to understand the pain points.
2. Review `candidate_analysis.md` for comprehensive evaluation of ALL options.
3. Check `decision_matrix.md` for the updated scoring with all candidates.
4. Read `recommendation.md` for the revised verdict.
5. Execute `implementation_plan.md` to build `pl3xus_async`.
