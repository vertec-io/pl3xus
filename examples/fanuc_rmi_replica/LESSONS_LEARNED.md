# pl3xus Development Lessons Learned

This document captures important patterns and anti-patterns discovered while developing the fanuc_rmi_replica example application using the pl3xus framework.

## 1. Effect Infinite Loop Prevention ⚠️ CRITICAL

The most common and dangerous mistake when using Leptos Effects with pl3xus's `use_request` hook is creating infinite loops.

### The Problem

When an Effect calls a fetch function, and the response updates a signal that the Effect depends on, you create an infinite loop:

```rust
// ❌ BAD - Infinite loop! UI freezes, server floods with requests
Effect::new(move || {
    fetch_robots(ListRobotConnections);  // Runs on every render
});
```

### Solution 1: Guard Signal Pattern

Use a "has loaded" guard to ensure fetch only happens once:

```rust
// ✅ GOOD - Only runs once on mount
let (has_loaded, set_has_loaded) = signal(false);

Effect::new(move |_| {
    if !has_loaded.get() {
        set_has_loaded.set(true);
        fetch_robots(ListRobotConnections);
    }
});
```

### Solution 2: ID Tracking Pattern

When you need to re-fetch based on selection changes, track the last loaded ID:

```rust
// ✅ GOOD - Only fetches when selection actually changes
let (last_loaded_id, set_last_loaded_id) = signal::<Option<i64>>(None);

Effect::new(move |_| {
    let current_id = selected_robot_id.get();
    let last_id = last_loaded_id.get_untracked();  // IMPORTANT: untracked!
    
    if current_id != last_id {
        set_last_loaded_id.set(current_id);
        if let Some(id) = current_id {
            fetch_configs(GetRobotConfigurations { robot_connection_id: id });
        }
    }
});
```

### Solution 3: Version Counter Pattern

For responses that might arrive multiple times:

```rust
let (last_version, set_last_version) = signal::<usize>(0);

Effect::new(move |_| {
    let state = response_state.get();
    if let Some(response) = &state.data {
        let new_version = last_version.get_untracked() + 1;
        set_last_version.set(new_version);
        set_data.set(response.clone());
    }
});
```

## 2. Signal Tracking Rules

Understanding how Leptos tracks signal dependencies is crucial:

| Method | Creates Dependency? | Use When |
|--------|-------------------|----------|
| `signal.get()` | ✅ Yes | Reading reactive values to render UI |
| `signal.get_untracked()` | ❌ No | Reading for comparison without triggering re-runs |
| `signal.set()` | N/A | Always safe to use |

### Key Rule
**Never `.get()` a signal inside an Effect if updating that signal should NOT re-run the Effect.**

## 3. StoredValue for Closures

When you need to use closures (like fetch functions) inside `move ||` closures:

```rust
// Store the function so it can be accessed inside closures
let fetch_robots = StoredValue::new(fetch_robots);

Effect::new(move |_| {
    fetch_robots.with_value(|f| f(ListRobotConnections));
});

// In event handlers
on:click=move |_| {
    fetch_robots.with_value(|f| f(ListRobotConnections));
}
```

## 4. Request/Response Pattern with use_request

The `use_request` hook returns a tuple:
```rust
let (fetch_fn, state_signal) = use_request::<RequestType>();
```

- `fetch_fn`: Call this to make the request
- `state_signal`: Reactive signal containing `UseRequestState<ResponseType>`
- `state_signal.get().data`: Option containing the response when available
- `state_signal.get().loading`: Boolean indicating if request is in flight

### Typical Pattern

```rust
let (fetch_data, data_state) = use_request::<MyRequest>();
let fetch_data = StoredValue::new(fetch_data);

// Trigger fetch (e.g., on button click or Effect)
fetch_data.with_value(|f| f(MyRequest { id: 123 }));

// Handle response in Effect
Effect::new(move |_| {
    let state = data_state.get();
    if let Some(response) = &state.data {
        // Handle the response
        set_my_data.set(response.clone());
    }
});
```

## 5. Database CRUD Pattern

When implementing CRUD operations:

### Types (in shared types crate)
```rust
// Request
pub struct CreateThing {
    pub name: String,
    // ... fields
}

impl RequestMessage for CreateThing {
    type ResponseMessage = CreateThingResponse;
}

// Response
pub struct CreateThingResponse {
    pub success: bool,
    pub id: i64,
    pub error: Option<String>,
}
```

### Server Handler
```rust
fn handle_create_thing(
    mut requests: MessageReader<Request<CreateThing>>,
    db: Option<Res<DatabaseResource>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        
        let result = db.as_ref()
            .map(|db| db.create_thing(inner))
            .unwrap_or(Err(anyhow::anyhow!("Database not available")));
        
        let response = match result {
            Ok(id) => CreateThingResponse { success: true, id, error: None },
            Err(e) => CreateThingResponse { success: false, id: 0, error: Some(e.to_string()) },
        };
        
        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}
```

### Client Usage
```rust
let (create_thing, create_state) = use_request::<CreateThing>();

// Handle response - refresh list after create
Effect::new(move |_| {
    let state = create_state.get();
    if let Some(response) = &state.data {
        if response.success {
            // Refresh the list
            fetch_things.with_value(|f| f(ListThings));
        } else {
            set_error_message.set(response.error.clone());
        }
    }
});
```

## 6. Common Debugging Tips

### UI Freezes
If the UI freezes:
1. Check for infinite loop Effects
2. Look at server logs - rapid repeated requests indicate a loop
3. Add guards to all Effects that call fetch functions

### Server SendError
If you see `Failed to send response: SendError` in server logs:
1. Usually caused by client reconnecting or infinite loops
2. The client connection changed before response could be sent
3. Fix the root cause (usually an infinite loop)

### Request Not Arriving
If requests don't seem to reach the server:
1. Check that the request type is registered in the plugin:
   ```rust
   app.listen_for_request_message::<MyRequest, WebSocketProvider>();
   ```
2. Check that the handler system is added:
   ```rust
   app.add_systems(Update, handle_my_request);
   ```

## 7. Code Organization

### Settings Page Structure
- Keep related signals together (e.g., form fields in one section)
- Use `StoredValue` for all fetch functions at the top
- Group Effects by purpose (fetch triggers, response handlers)
- Keep modals at the bottom of the view

### Component Props
For complex components, consider using a props struct:
```rust
#[component]
fn MyComponent(
    // Read signals
    data: ReadSignal<Vec<Item>>,
    selected_id: ReadSignal<Option<i64>>,
    // Write signals
    set_selected_id: WriteSignal<Option<i64>>,
    // Callbacks
    on_save: impl Fn() + Clone + 'static,
) -> impl IntoView { ... }
```

## 8. Server Handler Pattern

### Standard Request Handler Structure

```rust
fn handle_my_request(
    mut requests: MessageReader<Request<MyRequest>>,
    db: Option<Res<DatabaseResource>>,
) {
    for request in requests.read() {
        let inner = request.get_request();

        // Do the work
        let result = match db.as_ref() {
            Some(db) => db.do_something(inner),
            None => Err(anyhow::anyhow!("Database not available")),
        };

        // Build response
        let response = match result {
            Ok(data) => MyResponse { success: true, data, error: None },
            Err(e) => MyResponse { success: false, data: Default::default(), error: Some(e.to_string()) },
        };

        // Send response
        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}
```

### Register Handler in Plugin

```rust
impl Plugin for RequestsPlugin {
    fn build(&self, app: &mut App) {
        // Register message type
        app.listen_for_request_message::<MyRequest, WebSocketProvider>();

        // Add handler system
        app.add_systems(Update, handle_my_request);
    }
}
```

## 9. Component Sync Patterns

### Server-Side: Marking Components for Sync

```rust
// Automatically syncs to all connected clients
commands.entity(entity).insert(MyComponent { ... });

// With specific sync config
commands.entity(entity).insert((
    MyComponent { ... },
    SyncConfig::new().with_rate(60.0),  // 60 Hz max
));
```

### Client-Side: Subscribing to Components

```rust
// Get reactive signal for a specific entity's component
let my_comp = use_sync_component::<MyComponent>(entity_id);

// Use in view
view! {
    {move || my_comp.get().map(|c| view! { <div>{c.value}</div> })}
}
```

## 10. Feature Implementation Checklist

When adding a new feature that requires client-server communication:

1. **Define Types** (`fanuc_replica_types/src/lib.rs`)
   - [ ] Request struct with fields
   - [ ] Response struct with success/error fields
   - [ ] `impl RequestMessage for Request { type ResponseMessage = Response; }`

2. **Implement Server Handler** (`server/src/plugins/requests.rs`)
   - [ ] Handler function with `MessageReader<Request<T>>`
   - [ ] Business logic / database operations
   - [ ] Response construction and sending

3. **Register Handler** (`server/src/plugins/requests.rs` or plugin)
   - [ ] `app.listen_for_request_message::<Request, WebSocketProvider>()`
   - [ ] `app.add_systems(Update, handler_fn)`

4. **Client Integration** (page/component)
   - [ ] `use_request::<Request>()` hook
   - [ ] Effect to trigger fetch
   - [ ] Effect to handle response
   - [ ] UI update on state change

## 11. Conditional Compilation for WASM Logging

When adding logging that should only run in WASM builds:

```rust
// Correct pattern
#[cfg(target_arch = "wasm32")]
leptos::logging::log!("Message: {:?}", value);

// For closures with conditional logging
.map_err(|_e| {
    #[cfg(target_arch = "wasm32")]
    leptos::logging::warn!("Error: {:?}", _e);
    MyError::SomeVariant
})
```

Note: Prefix variables with `_` if only used in conditional blocks to avoid warnings on native builds.
