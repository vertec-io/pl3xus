# Messages vs Requests: Summary

## The Answer

**Messages** and **Requests** serve different purposes:

| Aspect | Message | Request |
|--------|---------|---------|
| **Response** | None | Required |
| **Use case** | Streaming, continuous | One-shot, transactional |
| **Error handling** | Observable via sync | Explicit error response |
| **UI feedback** | Sync-based | Loading states, error toasts |
| **Frequency** | High OK | Low preferred |

Both can be **targeted** (entity-specific) and **authorized** (control-checked).

## The Gap

Currently pl3xus has:
- ✅ Messages with targeting + authorization
- ✅ Requests with response
- ❌ Requests with targeting + authorization

This forces developers to use messages for commands that need responses.

## The Solution

Add **Targeted Requests** with the same builder pattern:

```rust
// Server
app.request::<SetSpeedOverride, NP>()
   .targeted()
   .with_default_entity_access()
   .register();

// Client  
let (send, state) = use_targeted_request::<SetSpeedOverride>();
send(entity_id, SetSpeedOverride { speed: 50 });
// state shows: Pending → Success(response) | Error(msg)
```

## Impact on Current Commands

### Convert to Targeted Requests
- `SetSpeedOverride` - needs response for UI
- `InitializeRobot` - critical, needs confirmation
- `AbortMotion` - safety-critical
- `ResetRobot` - needs completion confirmation
- `ConnectToRobot` - needs connection confirmation

### Add Targeting to Existing Requests
- `StartProgram`, `PauseProgram`, `ResumeProgram`, `StopProgram`
- `LoadProgram`, `UnloadProgram`
- (These target the System entity)

### Keep as Messages
- `JogCommand`, `JogRobot` - high frequency, no response needed
- Real-time motion streaming

## Implementation Priority

1. **Framework**: Add `app.request().targeted()` and `AuthorizedRequest<R>`
2. **Client**: Add `use_targeted_request()` hook
3. **Migrate**: High-priority commands (speed, init, abort, reset, connect)
4. **Migrate**: Program commands (add targeting)

## Files in This Research

- `START_HERE.md` - Problem statement and core question
- `TAXONOMY.md` - Classification of all operation types
- `PROPOSED_API.md` - Detailed API proposal
- `COMPARISON_TO_OTHER_FRAMEWORKS.md` - ROS2, gRPC, LiveView analysis
- `RECOMMENDATIONS.md` - Prioritized action items
- `DEVELOPER_EXPERIENCE.md` - UX for engineers
- `SUMMARY.md` - This file

## Next Steps

1. Review this research
2. Approve or modify the proposed API
3. Implement in pl3xus core
4. Migrate fanuc_rmi_replica commands
5. Update documentation

