# Current Codebase Analysis: Configuration Panel

## Files Analyzed

Location: `examples/fanuc_rmi_replica/client/src/pages/dashboard/info/`

## TODOs Found

### 1. `jog_defaults.rs` Line 149
```rust
on:click=move |_| {
    // TODO: Send UpdateJogSettings message
    set_has_changes.set(false);
}
```

**Problem**: The Apply button doesn't actually send any message to the server. It just clears the `has_changes` flag.

**Expected Behavior**: Should send a mutation/request to update `JogSettingsState` on the server, which should then propagate to the robot.

**Impact**: ⚠️ **Critical** - Feature is completely non-functional

### 2. `active_config.rs` Line 178
```rust
<p class="text-xs text-[#888888] mb-4">"Save modal - to be implemented"</p>
```

**Problem**: Save configuration modal is a placeholder.

**Impact**: ⚠️ **Medium** - Save config functionality incomplete

## Pattern Issues Identified

### Issue 1: Frame/Tool Mutation Pattern

**Files**: `frame_panel.rs`, `tool_panel.rs`

The current implementation uses `use_mutation::<SetActiveFrameTool>()` which is correct as a pattern, but:

1. **Server doesn't call robot driver**: The handler in `requests.rs` only updates the ECS component, doesn't send command to actual robot
2. **Disconnect between sync and mutation**: Client subscribes to `FrameToolDataState` for reading, but sends `SetActiveFrameTool` for writing - these are different types

### Issue 2: JogSettingsState Has No Write Path

**File**: `jog_defaults.rs`

- Subscribes to `JogSettingsState` via `use_entity_component`
- Has local edit state with "Apply" button
- But no request type exists to update jog settings
- TODO comment indicates this was never implemented

**Missing Types** (should exist in `fanuc_replica_types`):
```rust
pub struct UpdateJogSettings {
    pub cartesian_jog_speed: f64,
    pub cartesian_jog_step: f64,
    pub joint_jog_speed: f64,
    pub joint_jog_step: f64,
}
```

### Issue 3: ActiveConfigState Has Limited Write Path

**File**: `active_config.rs`

- Uses `use_mutation::<LoadConfiguration>()` for loading saved configs
- Uses `use_query_keyed::<GetRobotConfigurations>()` for config list
- But changing UFrame/UTool from this panel relies on separate panels

**This is actually fine** - it's a read-focused panel that delegates writes.

## Server-Side Analysis

### `requests.rs` - SetActiveFrameTool Handler

```rust
fn handle_set_active_frame_tool(
    mut requests: MessageReader<Request<SetActiveFrameTool>>,
    mut robots: Query<&mut FrameToolDataState, With<FanucRobot>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling SetActiveFrameTool: uframe={}, utool={}", inner.uframe, inner.utool);

        // Update synced component so all clients get the active frame/tool
        for mut ft_state in robots.iter_mut() {
            ft_state.active_frame = inner.uframe;
            ft_state.active_tool = inner.utool;
        }
        
        // NOTE: No robot driver call! This just updates ECS state
        
        let response = SetActiveFrameToolResponse { success: true, error: None };
        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}
```

**Problem**: Handler updates component but doesn't send command to robot.

## Recommendations

### Short-Term Fixes (Use Existing Patterns)

1. **Fix JogDefaults**: Add `UpdateJogSettings` request type and handler
2. **Fix Frame/Tool handler**: Add robot driver call

### Long-Term (Component Mutation Handlers)

If we implement the proposed pattern, these would become:

1. **JogSettingsState**: Component mutation handler that validates + updates robot config
2. **FrameToolDataState**: Component mutation handler that sends robot command + applies

## Summary Table

| Component | Read Hook | Write Mechanism | Status |
|-----------|-----------|-----------------|--------|
| `FrameToolDataState` | `use_entity_component` | `use_mutation::<SetActiveFrameTool>` | ⚠️ Handler doesn't call robot |
| `JogSettingsState` | `use_entity_component` | None (TODO) | ❌ Broken |
| `ActiveConfigState` | `use_entity_component` | `use_mutation::<LoadConfiguration>` | ✅ Works |
| Configs List | `use_query_keyed::<GetRobotConfigurations>` | Various mutations | ✅ Works |

