# Comparison: Current vs Proposed Patterns

## Use Case: Set Active Frame/Tool

### Current Implementation (Request Pattern)

**Types Needed** (`fanuc_replica_types/src/lib.rs`):
```rust
// Synced component (separate from request)
pub struct FrameToolDataState {
    pub active_frame: i32,
    pub active_tool: i32,
    pub frames: HashMap<i32, FrameToolData>,
    pub tools: HashMap<i32, FrameToolData>,
}

// Request type (duplicates fields from component)
pub struct SetActiveFrameTool {
    pub uframe: i32,
    pub utool: i32,
}

// Response type
pub struct SetActiveFrameToolResponse {
    pub success: bool,
    pub error: Option<String>,
}

impl RequestMessage for SetActiveFrameTool {
    type ResponseMessage = SetActiveFrameToolResponse;
}
```

**Server Handler** (`requests.rs`):
```rust
app.listen_for_request_message::<SetActiveFrameTool, WebSocketProvider>();

fn handle_set_active_frame_tool(
    mut requests: MessageReader<Request<SetActiveFrameTool>>,
    mut robots: Query<&mut FrameToolDataState, With<FanucRobot>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        
        // Update synced component
        for mut ft_state in robots.iter_mut() {
            ft_state.active_frame = inner.uframe;
            ft_state.active_tool = inner.utool;
        }
        
        // TODO: Actually call robot driver!
        
        request.respond(SetActiveFrameToolResponse { 
            success: true, 
            error: None 
        });
    }
}
```

**Client** (`frame_panel.rs`):
```rust
// Two separate hooks
let (frame_tool_state, _) = use_entity_component::<FrameToolDataState, _>(
    move || system_ctx.robot_entity_id.get()
);
let set_frame_tool = use_mutation::<SetActiveFrameTool>(move |result| {...});

// UI reads from synced component
let active_frame = Memo::new(move |_| frame_tool_state.get().active_frame as usize);

// But writes go through request
set_frame_tool.send(SetActiveFrameTool {
    uframe: new_frame as i32,
    utool: active_tool.get() as i32,
});
```

**Total Lines of Code**: ~50+ across 3 files

---

### Proposed Implementation (Component Mutation Handler)

**Types Needed** (`fanuc_replica_types/src/lib.rs`):
```rust
// Only the synced component - no separate request/response types!
pub struct FrameToolDataState {
    pub active_frame: i32,
    pub active_tool: i32,
    pub frames: HashMap<i32, FrameToolData>,
    pub tools: HashMap<i32, FrameToolData>,
}
```

**Server Registration + Handler** (`sync.rs`):
```rust
// Registration with handler
app.sync_component_with_handler::<FrameToolDataState>(
    handle_frame_tool_mutation.in_set(PluginSchedule::ClientRequests)
);

// Handler is a Bevy system
fn handle_frame_tool_mutation(
    mut mutations: MessageReader<ComponentMutation<FrameToolDataState>>,
    mut robots: Query<(&mut FrameToolDataState, &FanucDriver)>,
) {
    for mutation in mutations.read() {
        let new_state = mutation.new_value();
        
        if let Ok((mut state, driver)) = robots.get_mut(mutation.entity()) {
            // Call robot driver
            match driver.set_frame_tool(new_state.active_frame, new_state.active_tool) {
                Ok(()) => {
                    // Apply mutation only after external system confirms
                    *state = new_state.clone();
                    mutation.respond(Ok(()));
                }
                Err(e) => mutation.reject(&e.to_string()),
            }
        }
    }
}
```

**Client** (`frame_panel.rs`):
```rust
// Single hook for reading AND writing
let frame_tool = use_synced_mutation::<FrameToolDataState>(
    move || system_ctx.robot_entity_id.get(),
    move |result| {
        if let Err(e) = result {
            toast.error(format!("Frame change failed: {e}"));
        }
    }
);

// UI reads from handle
let active_frame = Memo::new(move |_| frame_tool.get().active_frame);

// Writes use same handle
frame_tool.mutate(|state| {
    state.active_frame = new_frame;
});
```

**Total Lines of Code**: ~30 across 2 files

---

## Summary

| Aspect | Current (Request) | Proposed (Component Handler) |
|--------|-------------------|------------------------------|
| Types to define | 3 (component, request, response) | 1 (component only) |
| Server registration | 2 calls (sync + listen) | 1 call (sync_with_handler) |
| Handler complexity | Same | Same |
| Client hooks | 2 (read + write) | 1 (combined) |
| Lines of code | ~50+ | ~30 |
| Type duplication | Yes (request duplicates component fields) | No |
| Response flexibility | Full custom response | Ok/Err or custom |

## When to Use Each

### Use Request Pattern When:
- Operation involves multiple entities
- Operation doesn't map to a single component
- Response needs complex data structure
- Operation is not idempotent
- Examples: `ExecuteProgram`, `CreateRobotConnection`

### Use Component Mutation Handler When:
- Updating a synced component that reflects external state
- Client conceptually "edits" a value the server controls
- Success means "apply the change to component and external system"
- Examples: `FrameToolDataState`, `JogSettingsState`, `SpeedOverride`

