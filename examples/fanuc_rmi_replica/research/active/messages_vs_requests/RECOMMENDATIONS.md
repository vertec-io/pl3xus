# Recommendations

## Executive Summary

After analyzing the current implementation, other frameworks, and use cases, here are the recommendations:

## 1. Unify Request API with Message API ✅ DONE

**Recommendation**: Create `app.request()` builder API parallel to `app.message()`

```rust
// Parallel APIs - now using with_entity_policy/with_message_policy
app.message::<JogCommand, NP>().targeted().with_default_entity_policy().register();
app.request::<SetSpeedOverride, NP>().targeted().with_default_entity_policy().register();
```

**Rationale**:
- Engineers learn one pattern, apply to both
- Same mental model for targeting and authorization
- Progressive disclosure (simple → complex)

## 2. Add Targeted Requests ✅ DONE

**Recommendation**: Implement `TargetedRequest<R>` wire format and `AuthorizedRequest<R>` handler type

**Client API**:
```rust
let (send, state) = use_targeted_request::<SetSpeedOverride>();
send(entity_id, SetSpeedOverride { speed: 50 });
```

**Server API**:
```rust
fn handle(mut reqs: MessageReader<AuthorizedRequest<SetSpeedOverride>>) {
    for req in reqs.read() {
        // req.entity(), req.source(), req.get_request()
        req.respond(Response { success: true });
    }
}
```

**Rationale**:
- Eliminates the message-for-commands-that-need-responses anti-pattern
- Authorization middleware handles control checks
- Clean separation of concerns

## 3. Migrate Commands to Targeted Requests ✅ DONE (High/Medium Priority)

**High Priority** (need response immediately) - ✅ ALL DONE:
- `SetSpeedOverride` - UI needs success confirmation ✅
- `InitializeRobot` - Critical operation, must confirm ✅
- `AbortMotion` - Safety-critical ✅
- `ResetRobot` - Must confirm completion ✅
- `ConnectToRobot` - ⚠️ Future improvement (complex async handler)

**Medium Priority** (can observe via sync) - ✅ ALL DONE:
- `StartProgram`, `PauseProgram`, `ResumeProgram`, `StopProgram`, `UnloadProgram` ✅
  - Now targeted requests with authorization middleware
  - Response is useful but ExecutionState sync provides visibility

**Keep as Messages** (high-frequency/streaming):
- `JogCommand` - 50Hz, response would create backpressure ✅ Kept as message
- `JogRobot` - Similar, streaming control ✅ Kept as message
- Motion commands during program execution - Buffered streaming

## 4. Consider JogCommand Exception 🤔

`JogCommand` is unique:
- High frequency (50Hz)
- Should NOT have per-message response
- But client might want to know "is jogging working?"

**Options**:
a) Keep as message, rely on position sync for feedback
b) Add separate "JogStatus" synced component
c) Add rate-limited acknowledgment (every 100ms)

**Recommendation**: Option (a) for now, with clear documentation

## 5. Future: Action Pattern for Long-Running Ops

For program execution:
```rust
// Future API
let action = use_action::<ExecuteProgram>();
let handle = action.send_goal(program_id);

// Track progress
handle.on_feedback(|progress| { /* update UI */ });
handle.on_complete(|result| { /* show result */ });
handle.cancel(); // If needed
```

**Not recommended for v1** - Current request pattern is sufficient

## Decision Matrix

| Command | Previous | Current | Status |
|---------|----------|---------|--------|
| `SetSpeedOverride` | Targeted Message | Targeted Request | ✅ Done |
| `InitializeRobot` | Targeted Message | Targeted Request | ✅ Done |
| `AbortMotion` | Targeted Message | Targeted Request | ✅ Done |
| `ResetRobot` | Targeted Message | Targeted Request | ✅ Done |
| `ConnectToRobot` | Message | Targeted Request | ⚠️ Future |
| `StartProgram` | Request | Targeted Request | ✅ Done |
| `PauseProgram` | Request | Targeted Request | ✅ Done |
| `ResumeProgram` | Request | Targeted Request | ✅ Done |
| `StopProgram` | Request | Targeted Request | ✅ Done |
| `UnloadProgram` | Request | Targeted Request | ✅ Done |
| `JogCommand` | Targeted Message | Targeted Message | ✅ Kept |
| `JogRobot` | Targeted Message | Targeted Message | ✅ Kept |

## Implementation Order

1. **Phase 1**: Framework changes ✅ DONE
   - Add `app.request().targeted()` builder API
   - Add `TargetedRequest<R>` wire format
   - Add `AuthorizedRequest<R>` handler type
   - Add `use_targeted_request()` client hook
   - Renamed to `with_entity_policy` and `with_message_policy`

2. **Phase 2**: High-priority migrations ✅ DONE
   - `SetSpeedOverride` ✅
   - `InitializeRobot` ✅
   - `AbortMotion` ✅
   - `ResetRobot` ✅
   - `ConnectToRobot` - ⚠️ Future (complex async handler)

3. **Phase 3**: Medium-priority migrations ✅ DONE
   - `StartProgram` ✅
   - `PauseProgram` ✅
   - `ResumeProgram` ✅
   - `StopProgram` ✅
   - `UnloadProgram` ✅

4. **Phase 4**: Documentation + cleanup ✅ DONE
   - Updated examples
   - Client uses `use_targeted_request()` hook
   - Server uses `AuthorizedRequest<T>` handler type

