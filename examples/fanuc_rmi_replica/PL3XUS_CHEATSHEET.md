# pl3xus Client Cheatsheet

Quick reference for pl3xus client hooks and patterns. The pl3xus framework provides a reactive synchronization layer between a Bevy server and Leptos clients.

## Core Philosophy

1. **Server is the SINGLE SOURCE OF TRUTH** for all system state
2. **Client is a PURE REFLECTION** of server state - it does NOT own or maintain state
3. **UI-Local State Only**: Modal visibility, accordion states, selected tabs, console messages
4. **Server-Owned State**: Execution state, robot position, connection status, active frame/tool

## Quick Reference Table

| Hook | Returns | Use Case |
|------|---------|----------|
| `use_sync_component::<T>()` | `ReadSignal<HashMap<u64, T>>` | Read all entities with component T |
| `use_sync_component_store::<T>()` | `Store<HashMap<u64, T>>` | Fine-grained reactivity for component T |
| `use_sync_component_where::<T, F>(filter)` | `Signal<HashMap<u64, T>>` | Filtered subset of entities |
| `use_sync_entity::<T>(id)` | `Signal<Option<T>>` | Read single entity's component |
| `use_sync_message::<T>()` | `ReadSignal<T>` | Broadcast messages from server |
| `use_sync_message_store::<T>()` | `Store<T>` | Fine-grained broadcast message access |
| `use_request::<R>()` | `(impl Fn(R), Signal<UseRequestState<R::Response>>)` | Request/response with loading state |
| `use_sync_context()` | `SyncContext` | Raw access to send/mutate methods |
| `use_sync_field_editor(...)` | `(NodeRef, RwSignal, String, Fn, Fn)` | Editable fields with focus retention |
| `use_sync_connection()` | `SyncConnection` | WebSocket connection control |
| `use_sync_mutations()` | `ReadSignal<HashMap<u64, MutationState>>` | Track mutation statuses |
| `use_sync_untracked(append_fn)` | `(Signal<TFull>, Signal<Option<TIncremental>>)` | Log-style incremental appending |

## Reading Server State

### Read All Entities (Atomic Reactivity)
```rust
let positions = use_sync_component::<RobotPosition>();

view! {
    <For
        each=move || positions.get().into_iter()
        key=|(id, _)| *id
        children=|(id, pos)| view! { <li>{format!("Entity {}: {:?}", id, pos)}</li> }
    />
}
```

### Read All Entities (Fine-Grained Reactivity)
```rust
let positions = use_sync_component_store::<RobotPosition>();

// Only re-renders when THIS entity's position changes
view! { <p>{move || positions.read().get(&entity_id).map(|p| p.x)}</p> }
```

### Read Single Entity
```rust
let config = use_sync_entity::<MicrowaveConfig>(entity_id);

view! { <p>{move || config.get().map(|c| c.power_enabled.to_string())}</p> }
```

### Filtered Entities
```rust
let active_positions = use_sync_component_where::<Position, _>(|pos| pos.x > 100.0);
```

### Derive Memos from Synced State (Common Pattern)
```rust
let exec_state = use_sync_component::<ExecutionState>();

let is_running = Memo::new(move |_| {
    exec_state.get().values().next().map(|s| s.running).unwrap_or(false)
});

let loaded_program = Memo::new(move |_| {
    exec_state.get().values().next().and_then(|s| s.loaded_program_name.clone())
});
```

## Sending Messages to Server

### Fire-and-Forget (One-Way)
```rust
let ctx = use_sync_context();

let init = move |_| {
    ctx.send(InitializeRobot { group_mask: Some(1) });
};

view! { <button on:click=init>"Initialize"</button> }
```

### Mutate Component (Optimistic Update)
```rust
let ctx = use_sync_context();

let toggle_power = move |_| {
    if let Some(config) = server_config.get_untracked() {
        ctx.mutate(entity_id, MicrowaveConfig {
            power_enabled: !config.power_enabled,
            ..config
        });
    }
};
```

### Request/Response Pattern
```rust
let (fetch, state) = use_request::<ListPrograms>();

Effect::new(move |_| {
    fetch(ListPrograms); // Fetch on mount
});

view! {
    <Show when=move || state.get().is_loading()>
        <p>"Loading..."</p>
    </Show>
    <Show when=move || state.get().data.is_some()>
        <ul>
            {move || state.get().data.unwrap_or_default().programs.iter().map(|p| {
                view! { <li>{&p.name}</li> }
            }).collect::<Vec<_>>()}
        </ul>
    </Show>
}
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
let position = Memo::new(move |_| {
    synced_positions.get().values().next().cloned()
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

### Broadcasting a Synced Component
```rust
fn update_robot_position(
    mut robot_query: Query<&mut RobotPosition>,
    net: Res<Network<FanucChannel>>,
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

