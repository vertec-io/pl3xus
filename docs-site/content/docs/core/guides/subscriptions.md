---
title: Subscriptions Guide
---
# Subscriptions Guide

This guide explains how component subscriptions work between pl3xus_client and pl3xus_sync.

---

## Overview

Subscriptions are the core mechanism for synchronizing ECS component data from server to client. The flow is:

1. **Client subscribes** to a component type (e.g., `Position`)
2. **Server sends snapshot** of all current entities with that component
3. **Server streams updates** whenever components change
4. **Client unsubscribes** when the data is no longer needed

This model provides:
- **Lazy loading**: Only fetch data when needed
- **Automatic updates**: No polling required
- **Cleanup**: Server stops sending when client unsubscribes

---

## Client-Side Subscriptions

### Basic Usage

Use hooks to subscribe to component data:

```rust
use pl3xus_client::use_sync_component;

#[component]
fn PositionList() -> impl IntoView {
    // Subscribe to all Position components
    let positions = use_sync_component::<Position>();

    view! {
        <For
            each=move || positions.get().into_iter()
            key=|(id, _)| *id
            children=|(id, pos)| view! {
                <div>{format!("Entity {}: ({}, {})", id, pos.x, pos.y)}</div>
            }
        />
    }
}
```

### Subscription Lifecycle

```
Component Mounts                    Component Unmounts
      │                                    │
      ▼                                    ▼
┌─────────────┐                    ┌─────────────┐
│  increment  │                    │  decrement  │
│  ref count  │                    │  ref count  │
└──────┬──────┘                    └──────┬──────┘
       │                                  │
       ▼                                  ▼
┌──────────────┐                   ┌──────────────┐
│ First sub?   │──Yes──►Send       │ Last sub?    │──Yes──►Send
│ (count == 1) │      Subscribe    │ (count == 0) │      Unsubscribe
└──────────────┘                   └──────────────┘
```

### Deduplication

Multiple components subscribing to the same type share one subscription:

```rust
#[component]
fn Dashboard() -> impl IntoView {
    view! {
        <PositionList/>     // Subscribes to Position
        <PositionMap/>      // Shares the same subscription
        <PositionStats/>    // Shares the same subscription
    }
}
```

**Internally:**
- First call to `use_sync_component::<Position>()` sends `SubscriptionRequest`
- Subsequent calls increment ref count and return cached signal
- When last component unmounts, sends `UnsubscribeRequest`

---

## Server-Side Handling

### Subscription Flow

```
┌─────────────────────────────────────────────────────────────┐
│                          SERVER                              │
└─────────────────────────────────────────────────────────────┘
          │                           │
          ▼                           ▼
   ┌─────────────┐            ┌──────────────────┐
   │ Subscription│            │ Component Change │
   │   Request   │            │    Detection     │
   └──────┬──────┘            └────────┬─────────┘
          │                            │
          ▼                            ▼
   ┌─────────────┐            ┌──────────────────┐
   │   Add to    │            │   Match against  │
   │  Manager    │            │   subscriptions  │
   └──────┬──────┘            └────────┬─────────┘
          │                            │
          ▼                            ▼
   ┌─────────────┐            ┌──────────────────┐
   │ Queue Full  │            │  Queue SyncItem  │
   │  Snapshot   │            │   for matching   │
   └──────┬──────┘            │   subscribers    │
          │                   └────────┬─────────┘
          ▼                            │
   ┌─────────────┐                     │
   │  Send to    │◄────────────────────┘
   │   Client    │
   └─────────────┘
```

### Subscription Manager

The `SubscriptionManager` tracks active subscriptions:

```rust
// Internal structure (for understanding)
pub struct SubscriptionManager {
    pub subscriptions: Vec<SubscriptionEntry>,
}

pub struct SubscriptionEntry {
    pub connection_id: ConnectionId,
    pub subscription_id: u64,
    pub component_type: String,
    pub entity: Option<SerializableEntity>,  // None = all entities
}
```

**Automatic cleanup**: When a client disconnects, all their subscriptions are removed.

---

## Initial Snapshots

When a subscription is created, the server sends a snapshot of current state:

```
Client: Subscribe to Position
          │
          ▼
Server: Query all (Entity, Position) pairs
          │
          ▼
Server: Serialize each to SyncItem
          │
          ▼
Server: Send SyncBatch with items
          │
          ▼
Client: Populate signal with received data
```

### Why Snapshots Matter

- Client immediately receives current state (not just future changes)
- No separate "initialization" request needed
- Works correctly for late-joining clients

---

## Update Streaming

After the initial snapshot, the server streams updates:

### Component Added/Changed

```rust
// When Position component changes on an entity...

// 1. Change detection fires ComponentChangeEvent
// 2. broadcast_component_changes matches against subscriptions
// 3. SyncItem::ComponentUpdate sent to matching clients
```

### Component Removed

```rust
// When Position is removed from an entity...

// SyncItem::ComponentRemove sent with:
// - entity: which entity
// - component_type: "Position"
```

### Entity Despawned

```rust
// When entity is despawned...

// SyncItem::EntityRemove sent with:
// - entity: which entity was removed
```

---

## Message Conflation

For high-frequency updates, conflation prevents network flooding:

```rust
// Server configuration
app.insert_resource(SyncSettings {
    enable_message_conflation: true,
    max_update_rate_hz: Some(30.0),  // 30 updates per second max
    ..default()
});
```

### How Conflation Works

```
Time 0ms: Position changes to (1, 1)  → Queue
Time 5ms: Position changes to (2, 2)  → Replace queued
Time 10ms: Position changes to (3, 3) → Replace queued
Time 33ms: Flush timer fires         → Send (3, 3) only
```

**Result**: 3 changes become 1 network message.

**Non-conflatable items** (entity removals, component removals) are never conflated and sent in order.

---

## Entity-Specific Subscriptions

Subscribe to a single entity instead of all entities with a component:

```rust
// Client-side: subscribe to specific entity
let robot = use_sync_entity::<Robot>(entity_id);
```

**Server behavior:**
- Only sends updates for the specified entity
- More efficient when you only need one entity's data

---

## Subscription Messages

### Client → Server

```rust
// Subscribe to a component type
pub struct SubscriptionRequest {
    pub subscription_id: u64,      // Client-assigned ID
    pub component_type: String,    // Type name
    pub entity: Option<SerializableEntity>,  // None = all entities
}

// Unsubscribe from a component type
pub struct UnsubscribeRequest {
    pub subscription_id: u64,
}
```

### Server → Client

```rust
pub struct SyncBatch {
    pub items: Vec<SyncItem>,
}

pub enum SyncItem {
    ComponentUpdate {
        entity: SerializableEntity,
        component_type: String,
        data: Vec<u8>,  // Bincode-encoded component
    },
    ComponentRemove {
        entity: SerializableEntity,
        component_type: String,
    },
    EntityRemove {
        entity: SerializableEntity,
    },
}
```

---

## Best Practices

### 1. Subscribe at the Right Level

```rust
// ❌ Bad: Subscribing in every list item
#[component]
fn RobotListItem(id: u64) -> impl IntoView {
    let robot = use_sync_entity::<Robot>(id);  // N subscriptions!
    // ...
}

// ✅ Good: Subscribe once, pass data down
#[component]
fn RobotList() -> impl IntoView {
    let robots = use_sync_component::<Robot>();  // 1 subscription

    view! {
        <For
            each=move || robots.get().into_iter()
            key=|(id, _)| *id
            children=|(id, robot)| view! {
                <RobotListItem robot=robot/>  // Pass data, not ID
            }
        />
    }
}
```

### 2. Use Filtered Hooks for Subsets

```rust
// ❌ Filtering in view
let all = use_sync_component::<Robot>();
view! {
    <For each=move || all.get().into_iter().filter(|(_, r)| r.active)>
}

// ✅ Use the filtering hook
let active = use_sync_component_where::<Robot, _>(|r| r.active);
view! {
    <For each=move || active.get().into_iter()>
}
```

### 3. Handle Loading States

```rust
#[component]
fn RobotDetail(id: u64) -> impl IntoView {
    let robot = use_sync_entity::<Robot>(id);
    let connection = use_sync_connection();

    view! {
        <Show
            when=move || robot.get().is_some()
            fallback=move || {
                if connection.ready_state.get() == ConnectionReadyState::Open {
                    view! { <p>"Robot not found"</p> }
                } else {
                    view! { <p>"Loading..."</p> }
                }
            }
        >
            // Render robot details
        </Show>
    }
}
```

### 4. Clean Up on Navigation

Subscriptions auto-cleanup when components unmount. When using client-side routing, unmounted views automatically unsubscribe.

---

## Debugging Subscriptions

### Enable Tracing

On the server:
```rust
// Set RUST_LOG=pl3xus_sync=debug
```

You'll see:
```
[pl3xus_sync] New subscription: conn=0, sub_id=0, component_type=Position, entity=None
[pl3xus_sync] Sending snapshot: 10 items for Position
```

### Check Connection State

```rust
let connection = use_sync_connection();

Effect::new(move || {
    leptos::logging::log!("Connection state: {:?}", connection.ready_state.get());
});
```

---

## Related Documentation

- [Hooks](./hooks.md) - All subscription hooks
- [Server Setup](./server-setup.md) - Configuring sync on the server
- [WebSocket Patterns](./websocket-patterns.md) - Connection handling
- [API Reference](https://docs.rs/pl3xus_sync) - Full API documentation

---

**Last Updated**: 2025-12-07
**pl3xus_sync Version**: 0.1


