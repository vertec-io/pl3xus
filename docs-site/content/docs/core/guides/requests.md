---
title: Requests & Queries
---
# Requests & Queries

Request/response patterns for fetching data and performing operations.

---

## Overview

pl3xus provides three request patterns:

| Pattern | Use Case | Client Hook |
|---------|----------|-------------|
| **Request** | One-off operations | `use_request` |
| **Query** | Cached data fetching | `use_query`, `use_query_keyed` |
| **Targeted Request** | Entity-specific operations | `use_targeted_request` |

---

## Basic Requests

### Client: Send a Request

```rust
use pl3xus_client::use_request;

#[component]
fn DataLoader() -> impl IntoView {
    let (data, set_data) = signal(None);
    
    let request = use_request::<ListPrograms>(move |result| {
        if let Ok(response) = result {
            set_data.set(Some(response.programs));
        }
    });

    Effect::new(move || {
        request.send(ListPrograms {});
    });

    view! {
        <For
            each=move || data.get().unwrap_or_default()
            key=|p| p.id
            children=|program| view! { <div>{program.name}</div> }
        />
    }
}
```

### Server: Handle the Request

```rust
use pl3xus_sync::AppRequestRegistrationExt;

// Registration
app.request::<ListPrograms, NP>().register();

// Handler
fn handle_list_programs(
    mut requests: MessageReader<Request<ListPrograms>>,
    programs: Res<ProgramDatabase>,
    net: Res<Network<NP>>,
) {
    for request in requests.read() {
        let programs = programs.list_all();
        net.send(request.source(), ListProgramsResponse { programs });
    }
}
```

---

## Queries (Cached)

Queries provide caching and state management similar to TanStack Query.

### Basic Query

```rust
use pl3xus_client::use_query;

#[component]
fn ProgramList() -> impl IntoView {
    let query = use_query::<ListPrograms>();

    Effect::new(move || {
        query.fetch(ListPrograms {});
    });

    view! {
        <Show when=move || query.is_loading()>
            <p>"Loading..."</p>
        </Show>
        <Show when=move || query.is_success()>
            <For
                each=move || query.data().map(|d| d.programs.clone()).unwrap_or_default()
                key=|p| p.id
                children=|program| view! { <div>{program.name}</div> }
            />
        </Show>
        <Show when=move || query.is_error()>
            <p class="error">{move || query.error().unwrap_or_default()}</p>
        </Show>
    }
}
```

### Keyed Query

For parameterized queries where the key determines cache identity:

```rust
use pl3xus_client::use_query_keyed;

#[component]
fn ProgramDetail(program_id: Signal<u64>) -> impl IntoView {
    let query = use_query_keyed::<GetProgram, u64>();

    Effect::new(move || {
        let id = program_id.get();
        query.fetch(id, GetProgram { id });
    });

    view! {
        <Show when=move || query.is_success()>
            {move || query.data().map(|p| p.name.clone()).unwrap_or_default()}
        </Show>
    }
}
```

---

## Targeted Requests

For entity-specific operations:

### Client

```rust
use pl3xus_client::use_targeted_request;

#[component]
fn FrameDataLoader(entity_id: u64) -> impl IntoView {
    let (frame_data, set_frame_data) = signal(None);
    
    let request = use_targeted_request::<GetFrameData>(move |result| {
        if let Ok(data) = result {
            set_frame_data.set(Some(data));
        }
    });

    Effect::new(move || {
        request.send(entity_id, GetFrameData { frame_number: 1 });
    });

    view! { /* ... */ }
}
```

### Server

```rust
// Registration
app.request::<GetFrameData, NP>()
    .targeted()
    .register();

// Handler
fn handle_get_frame_data(
    mut requests: MessageReader<Request<TargetedRequest<GetFrameData>>>,
    query: Query<&FrameData>,
    net: Res<Network<NP>>,
) {
    for request in requests.read() {
        let entity = Entity::from_bits(request.target_id.parse::<u64>().unwrap());
        
        if let Ok(frame_data) = query.get(entity) {
            let frame = frame_data.get_frame(request.frame_number);
            net.send(request.source(), GetFrameDataResponse { frame });
        }
    }
}
```

---

## Query State

All query hooks provide these state methods:

| Method | Description |
|--------|-------------|
| `is_loading()` | True while request is pending |
| `is_success()` | True if last request succeeded |
| `is_error()` | True if last request failed |
| `data()` | Returns `Option<&Response>` |
| `error()` | Returns `Option<&str>` |
| `refetch()` | Re-send the last request |

---

## Related

- [Mutations](./mutations.md) - For write operations
- [Hooks Reference](./hooks.md) - Complete hook documentation

