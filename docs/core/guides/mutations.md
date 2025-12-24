# Mutations

Mutations let clients request changes to server state. pl3xus provides a TanStack Query-inspired API for ergonomic mutation handling with loading states, error handling, and response callbacks.

---

## Quick Start

### Client: Send a Mutation

```rust
use pl3xus_client::use_mutation;

#[component]
fn UpdateButton() -> impl IntoView {
    // Create a mutation handle with a response handler
    let mutation = use_mutation::<UpdatePosition>(|result| {
        match result {
            Ok(response) => log!("Updated: {:?}", response),
            Err(e) => log!("Error: {}", e),
        }
    });

    view! {
        <button
            on:click=move |_| mutation.send(UpdatePosition { x: 10.0, y: 20.0 })
            disabled=move || mutation.is_loading()
        >
            {move || if mutation.is_loading() { "Saving..." } else { "Update" }}
        </button>
    }
}
```

### Server: Handle the Mutation

```rust
use pl3xus_sync::AppRequestRegistrationExt;

// Register the request handler
app.request::<UpdatePosition, NP>().register();

// Handle the request
fn handle_update_position(
    mut requests: MessageReader<Request<UpdatePosition>>,
    mut query: Query<&mut Position>,
    net: Res<Network<NP>>,
) {
    for request in requests.read() {
        // Apply the update
        if let Ok(mut pos) = query.get_single_mut() {
            pos.x = request.x;
            pos.y = request.y;
        }

        // Send response
        net.send(request.source(), UpdatePositionResponse { success: true });
    }
}
```

---

## Mutation Types

### Non-Targeted Mutations

For operations that don't target a specific entity:

```rust
// Client
let mutation = use_mutation::<CreateRobot>(|result| {
    if let Ok(response) = result {
        log!("Created robot: {}", response.robot_id);
    }
});

mutation.send(CreateRobot { name: "Robot-1".into() });
```

### Targeted Mutations

For operations on a specific entity (with authorization):

```rust
// Client
let mutation = use_mutation_targeted::<SetSpeed>(|result| {
    match result {
        Ok(_) => toast.success("Speed updated"),
        Err(e) => toast.error(format!("Failed: {e}")),
    }
});

// Send to a specific entity
mutation.send(entity_id, SetSpeed { value: 100.0 });
```

```rust
// Server - register with authorization
app.request::<SetSpeed, NP>()
    .targeted()
    .with_default_entity_policy()  // Requires EntityControl
    .register();

// Handler receives AuthorizedRequest
fn handle_set_speed(
    mut requests: MessageReader<AuthorizedRequest<SetSpeed>>,
    mut query: Query<&mut Speed>,
) {
    for request in requests.read() {
        let entity = request.entity();  // Already authorized!
        if let Ok(mut speed) = query.get_mut(entity) {
            speed.value = request.value;
        }
    }
}
```

---

## MutationHandle API

The `use_mutation` hook returns a `MutationHandle` with these methods:

| Method | Description |
|--------|-------------|
| `send(request)` | Send the mutation request |
| `is_loading()` | Returns `true` while waiting for response |
| `is_success()` | Returns `true` if last mutation succeeded |
| `is_error()` | Returns `true` if last mutation failed |
| `data()` | Returns `Option<&Response>` for successful response |
| `error()` | Returns `Option<&str>` for error message |
| `reset()` | Reset state to idle |

### Example: Full State Handling

```rust
let mutation = use_mutation::<SaveSettings>(|_| {});

view! {
    <button
        on:click=move |_| mutation.send(settings.get())
        disabled=move || mutation.is_loading()
    >
        {move || match () {
            _ if mutation.is_loading() => "Saving...",
            _ if mutation.is_success() => "Saved ✓",
            _ if mutation.is_error() => "Failed ✗",
            _ => "Save",
        }}
    </button>

    <Show when=move || mutation.is_error()>
        <p class="error">{move || mutation.error().unwrap_or_default()}</p>
    </Show>
}
```

---

## Server Authorization

### Default Entity Policy

The most common pattern - requires the client to have `EntityControl`:

```rust
app.request::<WriteValue, NP>()
    .targeted()
    .with_default_entity_policy()
    .register();
```

This checks:
1. Does the target entity exist?
2. Does the client have `EntityControl` of this entity (or a parent)?

### Custom Authorization

For custom authorization logic:

```rust
use pl3xus_sync::{EntityAccessPolicy, AuthResult};

app.request::<AdminCommand, NP>()
    .targeted()
    .with_entity_policy(EntityAccessPolicy::from_fn(|world, ctx, entity| {
        // Check if user is admin
        if is_admin(world, ctx.connection_id) {
            AuthResult::Authorized
        } else {
            AuthResult::Denied("Admin access required".into())
        }
    }))
    .register();
```

---

## Component Mutations

For mutating synced components directly (not via request/response):

### Server: Register with Handler

```rust
app.sync_component_builder::<JogSettings>()
    .with_handler::<NP>(handle_jog_settings_mutation)
    .targeted()
    .with_default_entity_policy()
    .build();

fn handle_jog_settings_mutation(
    mut mutations: MessageReader<AuthorizedComponentMutation<JogSettings>>,
    mut query: Query<&mut JogSettings>,
) {
    for mutation in mutations.read() {
        if let Ok(mut settings) = query.get_mut(mutation.entity()) {
            *settings = mutation.into_inner();
        }
    }
}
```

### Client: Use `use_mut_component`

```rust
let (settings, mutate) = use_mut_component::<JogSettings>(entity_id);

// Read current value
let current = settings.get();

// Mutate with new value
mutate(JogSettings { speed: 50.0, ..current });
```

---

## Best Practices

### 1. Always Handle Errors

```rust
let mutation = use_mutation::<SaveData>(|result| {
    match result {
        Ok(_) => { /* success */ },
        Err(e) => {
            // Log, show toast, update UI state
            toast.error(format!("Save failed: {e}"));
        }
    }
});
```

### 2. Disable UI During Loading

```rust
<button disabled=move || mutation.is_loading()>
    {move || if mutation.is_loading() { "Saving..." } else { "Save" }}
</button>
```

### 3. Use Targeted Mutations for Entity Operations

```rust
// ❌ Don't pass entity_id in the request body
struct UpdateRobot { entity_id: u64, speed: f32 }

// ✅ Use targeted mutations
struct UpdateRobotSpeed { speed: f32 }
mutation.send(entity_id, UpdateRobotSpeed { speed: 100.0 });
```

### 4. Server-Side Validation

Always validate on the server, even if you validate on the client:

```rust
fn handle_set_speed(mut requests: MessageReader<AuthorizedRequest<SetSpeed>>) {
    for request in requests.read() {
        // Validate
        if request.value < 0.0 || request.value > 1000.0 {
            // Send error response
            continue;
        }
        // Apply...
    }
}
```

---

## Related

- [Requests & Queries](./requests.md) - Read-only request patterns
- [Authorization](./authorization.md) - Deep dive into authorization
- [Entity Control](./entity-control.md) - Control handoff patterns


