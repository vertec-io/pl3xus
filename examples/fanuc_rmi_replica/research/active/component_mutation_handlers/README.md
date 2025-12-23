# Component Mutation Handlers Research

## Problem Statement

Currently, pl3xus has two separate patterns for modifying server state:

1. **Synced Components with Direct Mutations**: Components can be mutated directly by clients (if `allow_client_mutations` is true), but there's no hook to run custom logic when a mutation occurs.

2. **Request/Response Pattern**: Clients send typed requests (`use_mutation<SetActiveFrameTool>`), server has handlers that process them. Separate from the sync system.

### The Gap

When a component represents state bound to an external system (like robot frame/tool), we currently must:

1. Define a separate request type (`SetActiveFrameTool`)
2. Register a request handler 
3. Have the handler update the synced component
4. Client uses `use_mutation` (request) + `use_entity_component` (sync) - two separate things

**What we want**: The ergonomics of component mutations with the power of request handlers.

## Current State Analysis

### How SetActiveFrameTool Works Today

**Client** (`frame_panel.rs`):
```rust
// Sync - read-only view of server state
let (frame_tool_state, _) = use_entity_component::<FrameToolDataState, _>(...);

// Mutation - separate request pattern
let set_frame_tool = use_mutation::<SetActiveFrameTool>(move |result| {...});

// Apply button sends request, NOT a component mutation
set_frame_tool.send(SetActiveFrameTool { uframe: 5, utool: 1 });
```

**Server** (`requests.rs`):
```rust
fn handle_set_active_frame_tool(
    mut requests: MessageReader<Request<SetActiveFrameTool>>,
    mut robots: Query<&mut FrameToolDataState, With<FanucRobot>>,
) {
    for request in requests.read() {
        // Update synced component
        for mut ft_state in robots.iter_mut() {
            ft_state.active_frame = inner.uframe;
            ft_state.active_tool = inner.utool;
        }
        // TODO: Actually call robot driver to set frame/tool!
        request.respond(SetActiveFrameToolResponse { success: true, ... });
    }
}
```

### Issues with Current Pattern

1. **Duplication**: Must define request type + response type + handler, even when just updating a synced component field
2. **Disconnect**: Request payload (`SetActiveFrameTool`) is different from component (`FrameToolDataState`)
3. **Missing propagation**: Handler updates component but doesn't call robot driver (TODO in code)
4. **Boilerplate**: Client needs both `use_entity_component` (read) + `use_mutation` (write)

## Proposed Pattern: Component Mutation Handlers

### Concept

Allow registering a **handler function** that intercepts component mutations before they're applied:

```rust
// Server registration
app.sync_component_with_handler::<FrameToolDataState, _, _>(
    handler_system,
    ComponentMutationConfig {
        // Mutation triggers handler, doesn't apply directly
        apply_mode: ApplyMode::HandlerControlled,
        // Handler can return response
        response_type: Some::<FrameToolMutationResponse>(),
    }
);

fn handler_system(
    mut mutations: MessageReader<ComponentMutation<FrameToolDataState>>,
    mut robots: Query<(&mut FrameToolDataState, &RobotDriver)>,
) {
    for mutation in mutations.read() {
        let entity = mutation.entity();
        let new_value = mutation.new_value();
        
        // Validate
        if new_value.active_frame < 0 || new_value.active_frame > 9 {
            mutation.respond(Err("Invalid frame number"));
            continue;
        }
        
        // Call external system
        if let Ok((mut state, driver)) = robots.get_mut(entity) {
            if let Err(e) = driver.set_frame_tool(new_value.active_frame, new_value.active_tool) {
                mutation.respond(Err(e.to_string()));
                continue;
            }
            
            // Apply the mutation now that external system confirmed
            *state = new_value.clone();
            mutation.respond(Ok(()));
        }
    }
}
```

### Client Usage

```rust
// Single hook for both reading AND writing with response handling
let frame_tool = use_synced_mutation::<FrameToolDataState>(
    move || robot_entity_id.get(),
    move |result| {
        match result {
            Ok(()) => {} // Success - component auto-updated from sync
            Err(e) => toast.error(format!("Failed: {e}")),
        }
    }
);

// Read current value (reactive)
let current_frame = frame_tool.state.active_frame;

// Mutate with handler processing
frame_tool.mutate(|state| {
    state.active_frame = 5;
});
```

## Research Questions

See `RESEARCH_QUESTIONS.md` for detailed analysis.

## Comparison

See `COMPARISON.md` for comparison with current patterns.

## API Design Options

See `API_OPTIONS.md` for different implementation approaches.

