# Alternatives Analysis: Targeted Requests Authorization

This document evaluates alternative approaches considered for the targeted requests problem.

## Alternative 1: Extend Mutations for Commands

**Concept:** Instead of new request types, use mutations with special "command components."

```rust
// Server side
#[derive(Component, Serialize, Deserialize)]
struct JogCommand {
    axis: Axis,
    velocity: f32,
    executed: bool,
}

// Client mutates the command component, server sees it via MutationAuthorizer
ctx.mutate(robot_entity, JogCommand { axis: X, velocity: 0.5, executed: false });

// Server system watches for changes
fn handle_jog_mutations(
    query: Query<(Entity, &JogCommand), Changed<JogCommand>>,
) {
    for (entity, cmd) in query.iter() {
        if !cmd.executed { /* execute jog */ }
    }
}
```

**Pros:**
- Uses existing mutation authorization infrastructure
- No new framework code needed
- Already has hierarchical control support

**Cons:**
- ❌ **Semantically wrong** - Commands are not state, they're actions
- ❌ **No response mechanism** - Mutations don't have response payloads
- ❌ **Polling required** - Client must watch for `executed: true` 
- ❌ **State accumulation** - Commands need cleanup after execution
- ❌ **Race conditions** - Multiple commands could overwrite each other

**Verdict:** Rejected - Misuse of ECS components for ephemeral operations.

---

## Alternative 2: Flag-Based Authorization (Meteorite Pattern)

**Concept:** Middleware passes all requests through with `authorized: bool` flag.

```rust
pub struct AuthorizedRequest<R> {
    pub request: R,
    pub authorized: bool,  // System must check this
    pub entity: Entity,
    pub source: ConnectionId,
}

// System must check the flag
fn handle_jog(mut requests: MessageReader<AuthorizedRequest<JogCommand>>) {
    for req in requests.read() {
        if req.authorized {
            // execute
        } else {
            // handle rejection (log? custom response?)
        }
    }
}
```

**Pros:**
- ✅ Flexible - systems can handle unauthorized differently
- ✅ Matches meteorite pattern exactly
- ✅ Good for logging all attempts (including failures)

**Cons:**
- ❌ Boilerplate still exists (checking flag in every system)
- ❌ Easy to forget the check (runtime bug)
- ❌ Inconsistent with mutation behavior (mutations are silently rejected)

**Verdict:** Rejected - Doesn't eliminate boilerplate, which is the primary goal.

---

## Alternative 3: Separate Queues for Authorized/Unauthorized

**Concept:** Emit to different message types based on authorization.

```rust
// Authorized requests go here
fn handle_jog(mut requests: MessageReader<AuthorizedRequest<JogCommand>>) { ... }

// Unauthorized requests go here (optional subscription)
fn log_failed_jog(mut requests: MessageReader<UnauthorizedRequest<JogCommand>>) { ... }
```

**Pros:**
- ✅ Type-level separation (can't accidentally ignore auth)
- ✅ Optional handling of failures
- ✅ Clear semantics

**Cons:**
- ❌ More types to manage
- ❌ Registration is more complex
- ❌ Responses still need unified handling for correlation IDs

**Verdict:** Possible - But adds complexity for marginal benefit over auto-rejection.

---

## Alternative 4: ROS2 Action Server Pattern

**Concept:** Long-running actions with goal/cancel/feedback/result lifecycle.

```rust
// Action definition
trait ActionMessage {
    type Goal;
    type Feedback;
    type Result;
}

// Server handles lifecycle
fn accept_goal(&mut self, goal: Goal) -> GoalResponse { ... }
fn execute(&mut self) -> Feedback { ... }  // Called periodically
fn on_cancel(&mut self) -> CancelResponse { ... }
fn on_complete(&mut self) -> Result { ... }
```

**Pros:**
- ✅ Handles long-running operations properly
- ✅ Supports cancellation and progress feedback
- ✅ Well-established pattern (ROS2)

**Cons:**
- ❌ **Overkill for simple commands** - Jog is immediate, not long-running
- ❌ Significant framework complexity
- ❌ Requires client-side state machine
- ❌ Doesn't directly address authorization (orthogonal concern)

**Verdict:** Out of scope - Could be a future addition for program execution, but not needed for jog/config commands.

---

## Alternative 5: Extend `use_request` with Optional Entity

**Concept:** Add entity parameter to existing hook instead of new hook.

```rust
// Option A: Parameter to existing hook
let (send, state) = use_request::<JogCommand>();
send(JogCommand { ... }, Some(robot_entity));  // Optional entity

// Option B: Builder pattern
let (send, state) = use_request::<JogCommand>()
    .targeted(|| robot_entity)
    .build();
```

**Pros:**
- ✅ Single hook to learn
- ✅ Backward compatible if entity is optional

**Cons:**
- ❌ Mixes two concepts (targeted vs untargeted) in one API
- ❌ Runtime errors if entity expected but not provided
- ❌ Server registration still needs to differentiate

**Verdict:** Rejected - Cleaner to have separate hooks with distinct semantics.

---

## Alternative 6: Trait-Based Request Types

**Concept:** Marker trait for requests that require authorization.

```rust
/// Marker trait for requests requiring entity targeting and control authorization
trait TargetedRequestMessage: RequestMessage {
    // Optional: configurable authorization behavior
    fn requires_exclusive_control() -> bool { true }
    fn allow_uncontrolled_entities() -> bool { false }
}

impl TargetedRequestMessage for JogCommand {}
```

**Pros:**
- ✅ Type-level opt-in to targeted behavior
- ✅ Configurable per request type
- ✅ Could auto-derive from derive macro

**Cons:**
- ❌ Another trait to implement
- ❌ Doesn't change wire format or server handling
- ❌ Registration API still needed

**Verdict:** Could complement chosen design - Worth considering for v2.

---

## Recommendation Summary

| Alternative | Verdict | Reason |
|------------|---------|--------|
| 1. Mutations for commands | ❌ Reject | Semantic mismatch |
| 2. Flag-based (Meteorite) | ❌ Reject | Doesn't eliminate boilerplate |
| 3. Separate queues | ⚠️ Possible | Added complexity |
| 4. ROS2 Action pattern | ⏳ Future | Out of scope for simple commands |
| 5. Extend use_request | ❌ Reject | Mixes concerns |
| 6. Trait-based | ⏳ Future | Good v2 enhancement |

**Selected Approach: New `use_request_targeted` hook with middleware rejection**

This provides the cleanest API, eliminates boilerplate, and matches the established mutation pattern.

