# pl3xus Client Hooks Reference

## Component Hooks

### use_entity_component<T>(entity_id) - PREFERRED

Syncs a specific component for a specific entity. **Use this for multi-entity scenarios.**

```rust
#[component]
fn RobotPosition(robot_id: u64) -> impl IntoView {
    let position = use_entity_component::<Position>(robot_id);
    
    view! {
        <Show when=move || position.get().is_some()>
            {move || {
                let pos = position.get().unwrap();
                format!("({:.2}, {:.2}, {:.2})", pos.x, pos.y, pos.z)
            }}
        </Show>
    }
}
```

**Returns:** `Signal<Option<T>>`

### use_components<T>()

Returns all entities with component T. Use for listing entities.

```rust
#[component]
fn RobotList() -> impl IntoView {
    let robots = use_components::<RobotInfo>();
    
    view! {
        <For
            each=move || robots.get().into_iter()
            key=|(id, _)| *id
            children=|(id, info)| view! {
                <RobotCard id=id info=info />
            }
        />
    }
}
```

**Returns:** `Signal<HashMap<u64, T>>`

**⚠️ Anti-pattern:** Don't use `.values().next()` to get a single entity - no guarantee of which entity you get.

## Request Hooks

### use_request<R>()

Basic request hook. Returns fetch function and state.

```rust
let (fetch, state) = use_request::<ListRobots>();

Effect::new(move |_| {
    fetch(ListRobots);
});

view! {
    <Show when=move || state.get().is_loading()>
        <Spinner />
    </Show>
    <Show when=move || state.get().data.is_some()>
        {move || format!("{:?}", state.get().data)}
    </Show>
}
```

**Returns:** `(impl Fn(R), Signal<RequestState<R::ResponseMessage>>)`

### use_request_with_handler<R, F>(handler)

Request with callback on response.

```rust
let fetch = use_request_with_handler::<GetRobotInfo, _>(move |result| {
    match result {
        Ok(info) => set_info.set(Some(info.clone())),
        Err(e) => log::error!("Failed: {e}"),
    }
});

fetch(GetRobotInfo { robot_id: 42 });
```

**Returns:** `impl Fn(R)`

### use_targeted_request<R>()

Request targeting a specific entity.

```rust
let (fetch, state) = use_targeted_request::<GetRobotConfig>();

// Fetch config for specific robot
fetch(robot_id, GetRobotConfig);
```

**Returns:** `(impl Fn(u64, R), Signal<RequestState<R::ResponseMessage>>)`

## Mutation Hooks

### use_mutation_targeted<R>(handler)

Targeted mutation with response handler. **Production pattern for mutations.**

```rust
let update = use_mutation_targeted::<UpdatePosition>(move |result| {
    match result {
        Ok(r) if r.success => log::info!("Updated"),
        Ok(r) => log::error!("Failed: {:?}", r.error),
        Err(e) => log::error!("Error: {e}"),
    }
});

// Send mutation to specific entity
update.send(robot_id, UpdatePosition { x: 1.0, y: 2.0, z: 3.0 });

// Check pending state
view! {
    <button disabled=move || update.is_pending()>
        {move || if update.is_pending() { "Saving..." } else { "Save" }}
    </button>
}
```

**Returns:** `MutationHandle<R>` with:
- `.send(entity_id, request)` - Send mutation
- `.is_pending()` - Check if mutation is in flight

## RequestState<T>

State object returned by request hooks:

```rust
pub struct RequestState<T> {
    pub data: Option<T>,
    pub error: Option<String>,
    pub is_fetching: bool,
    pub is_stale: bool,
}

impl<T> RequestState<T> {
    pub fn is_loading(&self) -> bool {
        self.is_fetching && self.data.is_none()
    }
    
    pub fn is_success(&self) -> bool {
        self.data.is_some() && self.error.is_none()
    }
    
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }
}
```

## Hook Selection Guide

| Scenario | Hook |
|----------|------|
| Read component for specific entity | `use_entity_component` |
| List all entities with component | `use_components` |
| Fetch data (no entity target) | `use_request` |
| Fetch data with callback | `use_request_with_handler` |
| Fetch data for specific entity | `use_targeted_request` |
| Mutate specific entity | `use_mutation_targeted` |

## Anti-Patterns

```rust
// ❌ Wrong - no guarantee of correct entity
let robots = use_components::<RobotInfo>();
let robot = robots.get().values().next();

// ✅ Correct - explicit entity targeting
let robot = use_entity_component::<RobotInfo>(robot_id);
```

