---
name: pl3xus-queries
description: Request/response patterns for pl3xus applications. Covers targeted requests, batch registration, response handling, and query caching. Use when implementing data fetching.
allowed-tools:
  - view
  - codebase-retrieval
  - save-file
  - str-replace-editor
---

# pl3xus Queries Skill

## Purpose

This skill covers request/response patterns in pl3xus. Queries are used to fetch data from the server, while requests can also trigger actions.

## When to Use

Use this skill when:
- Implementing data fetching
- Creating request/response types
- Setting up targeted requests
- Handling query responses

## Request Types

### Defining Request/Response Types

```rust
// shared/src/requests.rs
use pl3xus_common::RequestMessage;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetRobotInfo {
    pub robot_id: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RobotInfoResponse {
    pub name: String,
    pub model: String,
    pub status: RobotStatus,
}

impl RequestMessage for GetRobotInfo {
    type ResponseMessage = RobotInfoResponse;
}
```

### Standard Response Pattern

Include success flag and optional error:

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionResponse {
    pub success: bool,
    pub error: Option<String>,
}
```

## Server Registration

### Targeted Request (Production Pattern)

```rust
app.request::<GetRobotInfo, WebSocketProvider>()
    .targeted()
    .with_default_entity_policy()
    .register();
```

### Batch Registration

Register multiple requests with same configuration:

```rust
app.requests::<(
    GetRobotInfo,
    GetRobotStatus,
    GetRobotConfig,
), WebSocketProvider>()
    .targeted()
    .with_default_entity_policy()
    .register();
```

### Non-Targeted Request

For requests that don't target a specific entity:

```rust
app.request::<ListRobots, WebSocketProvider>()
    .register();
```

## Server Handlers

### Targeted Request Handler

```rust
fn handle_get_robot_info(
    mut messages: MessageReader<NetworkData<TargetedRequest<GetRobotInfo>>>,
    robots: Query<(&Name, &RobotModel, &RobotStatus)>,
) {
    for request in messages.read() {
        let entity = Entity::from_bits(request.message.target_entity);
        
        if let Ok((name, model, status)) = robots.get(entity) {
            let _ = request.respond(RobotInfoResponse {
                name: name.to_string(),
                model: model.0.clone(),
                status: status.clone(),
            });
        } else {
            // Entity not found - framework handles error response
        }
    }
}
```

### Non-Targeted Request Handler

```rust
fn handle_list_robots(
    mut messages: MessageReader<NetworkData<NetworkRequest<ListRobots>>>,
    robots: Query<(Entity, &Name), With<RobotMarker>>,
) {
    for request in messages.read() {
        let list: Vec<_> = robots.iter()
            .map(|(e, name)| RobotListItem {
                id: e.to_bits(),
                name: name.to_string(),
            })
            .collect();
        
        let _ = request.respond(ListRobotsResponse { robots: list });
    }
}
```

## Client Usage

### Basic Request

```rust
let (fetch, state) = use_request::<ListRobots>();

Effect::new(move |_| {
    fetch(ListRobots);
});

view! {
    <Show when=move || state.get().is_loading()>
        <p>"Loading..."</p>
    </Show>
    <Show when=move || state.get().data.is_some()>
        <ul>
            {move || state.get().data.unwrap().robots.iter().map(|r| {
                view! { <li>{&r.name}</li> }
            }).collect::<Vec<_>>()}
        </ul>
    </Show>
}
```

### Request with Handler

```rust
let fetch_info = use_request_with_handler::<GetRobotInfo, _>(move |result| {
    match result {
        Ok(info) => set_robot_info.set(Some(info.clone())),
        Err(e) => log::error!("Failed to fetch: {e}"),
    }
});

// Fetch for specific entity
fetch_info(GetRobotInfo { robot_id: entity_id });
```

### Targeted Request

```rust
let (fetch, state) = use_targeted_request::<GetRobotConfig>();

// Fetch config for specific robot
fetch(robot_id, GetRobotConfig);
```

## Query State

The `RequestState<T>` provides:

```rust
pub struct RequestState<T> {
    pub data: Option<T>,
    pub error: Option<String>,
    pub is_fetching: bool,
    pub is_stale: bool,
}

impl<T> RequestState<T> {
    pub fn is_loading(&self) -> bool { self.is_fetching && self.data.is_none() }
    pub fn is_success(&self) -> bool { self.data.is_some() && self.error.is_none() }
    pub fn is_error(&self) -> bool { self.error.is_some() }
}
```

## Related Skills

- **pl3xus-mutations**: For state-changing operations
- **pl3xus-authorization**: For access control
- **pl3xus-server**: Server-side patterns

## Reference

- [Request/Response Patterns](./references/request-response.md)
- [Targeted Requests](./references/targeted-requests.md)

