# pl3xus Client Cheatsheet

Quick reference for pl3xus client hooks and patterns. The pl3xus framework provides a reactive synchronization layer between a Bevy server and Leptos clients.

**Last Updated**: December 2025

## Core Philosophy

1. **Server is the SINGLE SOURCE OF TRUTH** for all system state
2. **Client is a PURE REFLECTION** of server state - it does NOT own or maintain state
3. **UI-Local State Only**: Modal visibility, accordion states, selected tabs, console messages
4. **Server-Owned State**: Execution state, robot position, connection status, active frame/tool

## Quick Reference Table

### Primary Hooks (Use These)

| Hook | Returns | Use Case |
|------|---------|----------|
| `use_entity_component::<T, _>(entity_id_fn)` | `(Signal<T>, Signal<bool>)` | Subscribe to specific entity's component |
| `use_components::<T>()` | `ReadSignal<HashMap<u64, T>>` | Read all entities with component T |
| `use_query::<R>()` | `QueryHandle<R>` | Cached query, auto-refetches on invalidation |
| `use_query_keyed::<R, K>(key_fn)` | `QueryHandle<R>` | Keyed query (null key skips fetch) |
| `use_mutation::<R>(callback)` | `MutationHandle<R>` | Fire-and-forget with response handler |
| `use_mutation_targeted::<R>(callback)` | `TargetedMutationHandle<R>` | Entity-targeted mutation with auth |
| `use_send_targeted::<M>()` | `impl Fn(u64, M)` | Send targeted message (fire-and-forget) |
| `use_request::<R>()` | `(impl Fn(R), UseRequestState<R>)` | Low-level request (prefer use_query/use_mutation) |

### Secondary Hooks

| Hook | Returns | Use Case |
|------|---------|----------|
| `use_component_store::<T>()` | `Store<HashMap<u64, T>>` | Fine-grained reactivity for component T |
| `use_sync_context()` | `SyncContext` | Raw access to send/mutate methods |
| `use_sync_connection()` | `SyncConnection` | WebSocket connection control |

## Reading Server State

### Read Specific Entity's Component (Preferred Pattern)
```rust
let system_ctx = use_system_entity();

// Subscribe to the active robot's position
let (position, robot_exists) = use_entity_component::<RobotPosition, _>(
    move || system_ctx.robot_entity_id.get()
);

// Always check robot_exists before accessing component data
let robot_ready = Memo::new(move |_| {
    robot_exists.get() && connection_state.get().robot_connected
});

view! {
    <Show when=move || robot_exists.get()>
        <p>"X: " {move || position.get().x}</p>
    </Show>
}
```

### Read All Entities of a Type
```rust
let positions = use_components::<RobotPosition>();

view! {
    <For
        each=move || positions.get().into_iter()
        key=|(id, _)| *id
        children=|(id, pos)| view! { <li>{format!("Entity {}: {:?}", id, pos)}</li> }
    />
}
```

### Fine-Grained Reactivity (Store)
```rust
let positions = use_component_store::<RobotPosition>();

// Only re-renders when THIS entity's position changes
view! { <p>{move || positions.read().get(&entity_id).map(|p| p.x)}</p> }
```

## Queries (Cached Data with Server Invalidation)

### Simple Query (Auto-fetches on mount)
```rust
let programs = use_query::<ListPrograms>();

view! {
    <Show when=move || programs.is_loading()>
        <p>"Loading..."</p>
    </Show>
    {move || programs.data().map(|data| view! {
        <ul>
            {data.programs.iter().map(|p| view! { <li>{&p.name}</li> }).collect_view()}
        </ul>
    })}
    {move || programs.error().map(|e| view! { <p class="text-red-500">{e}</p> })}
}
```

### Keyed Query (Conditional fetching)
```rust
// Only fetches when key returns Some
let program = use_query_keyed::<GetProgram, _>(move || {
    selected_id.get().map(|id| GetProgram { id })
});

// Refetches automatically when selected_id changes
// Refetches when server sends QueryInvalidation for GetProgram with matching key
```

### Query Handle API
```rust
let query = use_query::<ListPrograms>();

query.is_loading()      // bool - request in flight
query.data()            // Option<&Response> - cached data
query.error()           // Option<String> - error message
query.refetch()         // Force refetch
query.is_stale()        // bool - data marked stale by server
```

## Mutations (Commands with Response Handling)

### Simple Mutation
```rust
let toast = use_toast();

let create_program = use_mutation::<CreateProgram>(move |result| {
    match result {
        Ok(r) if r.success => toast.success("Program created!"),
        Ok(r) => toast.error(format!("Failed: {}", r.error.unwrap_or_default())),
        Err(e) => toast.error(format!("Error: {e}")),
    }
});

// Call the mutation
create_program.send(CreateProgram { name: "MyProgram".into() });
```

### Targeted Mutation (Entity-specific with authorization)
```rust
let system_ctx = use_system_entity();
let toast = use_toast();

let set_speed = use_mutation_targeted::<SetSpeedOverride>(move |result| {
    match result {
        Ok(r) if r.success => toast.success("Speed updated"),
        Ok(r) => toast.error(r.error.unwrap_or_default()),
        Err(e) => toast.error(format!("{e}")),
    }
});

// Send to specific entity
if let Some(robot_id) = system_ctx.robot_entity_id.get() {
    set_speed.send(robot_id, SetSpeedOverride { value: 50.0 });
}
```

## Targeted Messages (Fire-and-Forget)

### Send to Specific Entity
```rust
let system_ctx = use_system_entity();
let send_jog = use_send_targeted::<JogCommand>();

let on_jog = move |direction| {
    if let Some(robot_id) = system_ctx.robot_entity_id.get() {
        send_jog(robot_id, JogCommand { direction, speed: 10.0 });
    }
};
```

## Low-Level Request (Prefer use_query/use_mutation)

### use_request (for imperative polling patterns)
```rust
// Only use for imperative triggers like "refresh I/O data"
let (fetch_io, io_state) = use_request::<GetIOData>();

// Trigger on button click (not auto-fetching)
let refresh = move |_| fetch_io(GetIOData { robot_id });
```
```

## Receiving Broadcast Messages

### Toast Notifications Pattern
```rust
let notification = use_sync_message::<ProgramNotification>();
let last_seen = RwSignal::new(0u64);

Effect::new(move |_| {
    let notif = notification.get();
    if notif.sequence > last_seen.get_untracked() {
        last_seen.set(notif.sequence);
        match &notif.kind {
            ProgramNotificationKind::Completed { program_name, .. } => {
                show_toast(&format!("Program '{}' completed!", program_name));
            }
            _ => {}
        }
    }
});
```

### Console Log Handler Pattern
For server-broadcast log messages that should be appended to a local console:
```rust
#[component]
pub fn ConsoleLogHandler() -> impl IntoView {
    let ctx = use_context::<WorkspaceContext>();
    let console_entry = use_sync_message::<ConsoleLogEntry>();
    let last_seen = RwSignal::new(0u64);

    Effect::new(move |_| {
        let entry = console_entry.get();
        // Skip default/empty entries and already-seen entries
        if entry.timestamp > last_seen.get_untracked() {
            last_seen.set(entry.timestamp);
            if let Some(ctx) = ctx.as_ref() {
                ctx.console_messages.update(|msgs| {
                    msgs.push(entry.clone());
                });
            }
        }
    });

    view! {} // Invisible component
}
```

## Editable Fields with Focus Retention

### Using the Hook
```rust
let (input_ref, is_focused, initial_value, on_keydown, on_blur_handler) =
    use_sync_field_editor(
        entity_id,
        |pos: &Position| pos.x,                           // accessor
        |pos: &Position, new_x: f32| Position { x: new_x, y: pos.y }, // mutator
    );

view! {
    <input
        node_ref=input_ref
        type="text"
        value=initial_value
        on:focus=move |_| is_focused.set(true)
        on:blur=move |_| {
            is_focused.set(false);
            on_blur_handler();
        }
        on:keydown=on_keydown
    />
}
```

### Using the Component
```rust
view! {
    <SyncFieldInput
        entity_id=entity_id
        field_accessor=|pos: &Position| pos.x
        field_mutator=|pos: &Position, new_x: f32| Position { x: new_x, y: pos.y }
        input_type="text"
        class="number-input"
    />
}
```

**Behavior:**
- **Enter key**: Apply mutation to server
- **Click away (blur)**: Revert to server value
- **Server updates**: Value updates in DOM only when NOT focused

## Connection Control

```rust
let connection = use_sync_connection();

let status_text = move || match connection.ready_state.get() {
    ConnectionReadyState::Connecting => "Connecting...",
    ConnectionReadyState::Open => "Connected",
    ConnectionReadyState::Closing => "Closing...",
    ConnectionReadyState::Closed => "Disconnected",
};

view! {
    <p>"Status: " {status_text}</p>
    <button on:click=move |_| (connection.open)()>"Connect"</button>
    <button on:click=move |_| (connection.close)()>"Disconnect"</button>
}
```

## Common Anti-Patterns to Avoid

### ❌ WRONG: Subscribing to wrong entity
```rust
// DON'T subscribe to ConnectionState on the system entity
// ConnectionState lives on the ROBOT entity!
let (connection_state, _) = use_entity_component::<ConnectionState, _>(
    move || system_ctx.system_entity_id.get()  // ❌ WRONG!
);
```

### ✅ CORRECT: Subscribe to the right entity
```rust
// ConnectionState, RobotStatus, RobotPosition all live on the robot entity
let (connection_state, robot_exists) = use_entity_component::<ConnectionState, _>(
    move || system_ctx.robot_entity_id.get()  // ✅ Correct
);

// Always check robot_exists before accessing state
let robot_connected = Memo::new(move |_| {
    robot_exists.get() && connection_state.get().robot_connected
});
```

### ❌ WRONG: Copying server state to local signals
```rust
// DON'T DO THIS - duplicates server state
let (local_position, set_local_position) = signal(Position::default());

Effect::new(move |_| {
    if let Some(pos) = synced_positions.get().values().next() {
        set_local_position.set(pos.clone()); // ❌ Anti-pattern!
    }
});
```

### ✅ CORRECT: Derive from synced state
```rust
// DO THIS - create derived signals/memos
let (position, _) = use_entity_component::<RobotPosition, _>(
    move || system_ctx.robot_entity_id.get()
);
```

### ❌ WRONG: Effect runs on mount with stale data
```rust
// DON'T process stale response data on mount
Effect::new(move |_| {
    if let Some(response) = state.data() {
        close_popup();  // ❌ Closes immediately on mount!
    }
});
```

### ✅ CORRECT: Guard against stale data
```rust
// DO guard with a "waiting" signal
Effect::new(move |_| {
    // Only process when we're actively waiting for a response
    if waiting_for_response.get().is_none() {
        return;
    }
    if let Some(response) = state.data() {
        close_popup();  // ✅ Only runs when expected
    }
});
```

### ❌ WRONG: Client-side state detection
```rust
// DON'T detect state changes client-side to trigger notifications
Effect::new(move |_| {
    let was_running = prev_running.get_untracked();
    let is_running = exec_state.get().running;
    if was_running && !is_running {
        show_toast("Completed!"); // ❌ Race condition prone!
    }
});
```

### ✅ CORRECT: Server broadcasts notifications
```rust
// Server sends explicit notification, clients react
let notification = use_sync_message::<ProgramNotification>();
Effect::new(move |_| {
    if notification.get().kind == ProgramNotificationKind::Completed { ... }
});
```

## Type Definitions (Shared Types)

### Synced Components (Server → Client)
Types that implement `Component` on server and are synced to clients:
- `RobotPosition` - Cartesian position (x, y, z, w, p, r)
- `JointAngles` - Joint angles (j1-j9)
- `RobotStatus` - servo_ready, in_motion, speed_override, etc.
- `ExecutionState` - running, paused, current_line, program_lines
- `ConnectionState` - robot_connected, robot_addr, robot_name
- `ActiveConfigState` - frame/tool numbers, arm configuration
- `JogSettingsState` - jog speeds and step sizes
- `FrameToolDataState` - all frame (1-9) and tool (1-10) data
- `IoStatus` - digital/analog/group I/O values
- `IoConfigState` - I/O display names and visibility

### Broadcast Messages (Server → All Clients)
One-way messages for notifications:
- `ProgramNotification` - program completed/stopped/error
- `ConsoleLogEntry` - console log messages

### Request/Response Messages
For correlated request/response patterns:
```rust
impl RequestMessage for ListPrograms {
    type ResponseMessage = ListProgramsResponse;
}
```

Examples: `ListPrograms`, `LoadProgram`, `StartProgram`, `GetFrameData`, etc.

### Fire-and-Forget Messages
One-way client → server messages:
- `InitializeRobot`, `ResetRobot`, `AbortMotion`
- `SetSpeedOverride`, `JogCommand`, `LinearMotionCommand`
- `UpdateJogSettings`, `UpdateActiveConfig`

## Server-Side Patterns (Bevy)

### Query Invalidation (Trigger Client Refetch)
```rust
use pl3xus_sync::{invalidate_queries, invalidate_queries_with_keys};

fn handle_create_program(
    mut requests: MessageReader<Request<CreateProgram>>,
    mut sync_state: ResMut<SyncState<WebSocketProvider>>,
    db: Option<Res<DatabaseResource>>,
) {
    for request in requests.read() {
        // ... create program in database ...

        // Invalidate ALL ListPrograms queries on all clients
        invalidate_queries::<ListPrograms>(&mut sync_state);

        // Invalidate specific keyed queries
        invalidate_queries_with_keys::<GetProgram, _>(&mut sync_state, &[program_id]);

        request.clone().respond(response).ok();
    }
}
```

### Broadcasting a Synced Component
```rust
fn update_robot_position(
    mut robot_query: Query<&mut RobotPosition>,
) {
    for mut pos in robot_query.iter_mut() {
        pos.x += 1.0;
        // pl3xus automatically syncs changed components to clients
    }
}
```

### Broadcasting a Message
```rust
fn send_notification(net: Res<Network<FanucChannel>>) {
    net.broadcast(ProgramNotification {
        sequence: next_sequence(),
        kind: ProgramNotificationKind::Completed {
            program_name: "MyProgram".into(),
            total_instructions: 42,
        },
    });
}
```

### Broadcasting Console Log Entries
Helper function for creating timestamped console entries:
```rust
pub fn console_entry(
    content: impl Into<String>,
    direction: ConsoleDirection,
    msg_type: ConsoleMsgType,
) -> ConsoleLogEntry {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    ConsoleLogEntry {
        timestamp: now.as_millis() as u64,
        direction,
        msg_type,
        content: content.into(),
    }
}

// Usage in a system:
fn on_program_start(net: Res<Network<FanucChannel>>, executor: Res<ProgramExecutor>) {
    let msg = console_entry(
        format!("Program started ({} instructions)", executor.total_instructions),
        ConsoleDirection::System,
        ConsoleMsgType::Status,
    );
    net.broadcast(msg);
}
```

### Handling Incoming Messages (Bevy 0.17+)
```rust
use bevy::ecs::message::MessageReader;

fn handle_initialize(
    mut reader: MessageReader<pl3xus::NetworkData<InitializeRobot>>,
    mut robot_query: Query<&mut RobotStatus>,
) {
    for event in reader.read() {
        let client_id = *event.source();
        let msg: &InitializeRobot = &*event;
        // Handle the message
    }
}
```

### Handling Request/Response Messages (Bevy 0.17+)
```rust
use bevy::ecs::message::MessageReader;
use pl3xus::managers::network_request::Request;

fn handle_list_programs(
    mut requests: MessageReader<Request<ListPrograms>>,
    db: Option<Res<DatabaseResource>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        // Process request...

        // Send response back to requesting client
        if let Err(e) = request.clone().respond(ListProgramsResponse { programs }) {
            error!("Failed to send response: {:?}", e);
        }
    }
}
```

