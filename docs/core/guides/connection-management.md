# Connection Management Guide

This guide covers WebSocket connection lifecycle, state handling, and reconnection patterns.

---

## Overview

Connection management in pl3xus involves:

1. **Connection state** - Tracking open/closed/connecting states
2. **Automatic reconnection** - Re-establishing connections after drops
3. **Subscription restoration** - Re-subscribing after reconnect
4. **Error handling** - Graceful degradation on failures

---

## Connection States

### ConnectionReadyState

The connection can be in one of four states:

```rust
use leptos_use::core::ConnectionReadyState;

match ready_state {
    ConnectionReadyState::Connecting => "Establishing connection...",
    ConnectionReadyState::Open => "Connected and ready",
    ConnectionReadyState::Closing => "Connection closing...",
    ConnectionReadyState::Closed => "Disconnected",
}
```

### Checking Connection State

Use the `use_sync_connection` hook:

```rust
use pl3xus_client::use_sync_connection;
use leptos::prelude::*;

#[component]
fn ConnectionIndicator() -> impl IntoView {
    let connection = use_sync_connection();

    let status_class = move || {
        match connection.ready_state.get() {
            ConnectionReadyState::Open => "bg-green-500",
            ConnectionReadyState::Connecting => "bg-yellow-500",
            _ => "bg-red-500",
        }
    };

    view! {
        <div class=move || format!("w-3 h-3 rounded-full {}", status_class())/>
    }
}
```

---

## SyncConnection API

The `use_sync_connection` hook returns a `SyncConnection`:

```rust
pub struct SyncConnection {
    /// Current connection state (reactive signal)
    pub ready_state: Signal<ConnectionReadyState>,
    
    /// Open the WebSocket connection
    pub open: Arc<dyn Fn() + Send + Sync>,
    
    /// Close the WebSocket connection
    pub close: Arc<dyn Fn() + Send + Sync>,
}
```

### Manual Connection Control

```rust
#[component]
fn ConnectionControls() -> impl IntoView {
    let connection = use_sync_connection();

    let on_connect = move |_| {
        (connection.open)();
    };

    let on_disconnect = move |_| {
        (connection.close)();
    };

    let is_connected = move || {
        connection.ready_state.get() == ConnectionReadyState::Open
    };

    view! {
        <Show
            when=is_connected
            fallback=move || view! {
                <button on:click=on_connect>"Connect"</button>
            }
        >
            <button on:click=on_disconnect>"Disconnect"</button>
        </Show>
    }
}
```

---

## SyncProvider Configuration

### Auto-Connect (Default)

```rust
<SyncProvider
    url="ws://localhost:8080"
    registry=registry
>
    // Connects automatically on mount
</SyncProvider>
```

### Manual Connect

```rust
<SyncProvider
    url="ws://localhost:8080"
    registry=registry
    auto_connect=false  // Don't connect automatically
>
    // Call connection.open() to connect
</SyncProvider>
```

---

## Error Handling

### Accessing Errors

```rust
let ctx = use_sync_context();

Effect::new(move || {
    if let Some(error) = ctx.last_error.get() {
        log::error!("Sync error: {}", error);
    }
});
```

### Error Types

```rust
pub enum SyncError {
    /// WebSocket not connected
    NotConnected,
    
    /// Failed to deserialize component
    DeserializationFailed { component_name: String, error: String },
    
    /// Component type not registered
    TypeNotRegistered { component_name: String },
    
    /// Schema hash mismatch (version mismatch)
    SchemaHashMismatch { component_name: String, expected: u64, actual: u64 },
    
    /// WebSocket error
    WebSocketError { message: String },
    
    /// Failed to serialize component
    SerializationFailed { component_name: String, error: String },
}
```

### Displaying Errors

```rust
#[component]
fn ErrorBanner() -> impl IntoView {
    let ctx = use_sync_context();

    view! {
        <Show when=move || ctx.last_error.get().is_some()>
            <div class="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded">
                {move || ctx.last_error.get().map(|e| e.to_string())}
            </div>
        </Show>
    }
}
```

---

## Reconnection Patterns

### Automatic Subscription Restoration

When the connection reopens, subscriptions are automatically restored:

```rust
// Internal behavior (for understanding)
Effect::new(move || {
    if ready_state.get() == ConnectionReadyState::Open {
        // Re-send all active subscriptions
        for (component_type, subscription_id) in active_subscriptions {
            send_subscription_request(subscription_id, component_type);
        }
    }
});
```

**You don't need to manually re-subscribe** - the `SyncContext` handles this.

### Retry with Backoff

For production, implement exponential backoff:

```rust
#[component]
fn AutoReconnect() -> impl IntoView {
    let connection = use_sync_connection();
    let (retry_count, set_retry_count) = signal(0);
    let (retry_delay, set_retry_delay) = signal(1000); // Start at 1 second

    Effect::new(move || {
        if connection.ready_state.get() == ConnectionReadyState::Closed {
            // Schedule reconnection with backoff
            let delay = retry_delay.get();
            set_timeout(
                move || {
                    (connection.open)();
                    set_retry_count.update(|c| *c += 1);
                    // Exponential backoff: 1s, 2s, 4s, 8s, max 30s
                    set_retry_delay.update(|d| (*d * 2).min(30000));
                },
                std::time::Duration::from_millis(delay),
            );
        } else if connection.ready_state.get() == ConnectionReadyState::Open {
            // Reset on successful connection
            set_retry_count.set(0);
            set_retry_delay.set(1000);
        }
    });

    view! {}
}
```

---

## Server-Side Connection Events

On the server, handle connection events with `NetworkEvent`:

```rust
use pl3xus::{NetworkEvent, MessageReader};

fn handle_connections(mut events: MessageReader<NetworkEvent>) {
    for event in events.read() {
        match event {
            NetworkEvent::Connected(conn_id) => {
                info!("Client connected: {:?}", conn_id);
            }
            NetworkEvent::Disconnected(conn_id) => {
                info!("Client disconnected: {:?}", conn_id);
                // Cleanup is automatic for subscriptions
            }
            NetworkEvent::Error(err) => {
                error!("Network error: {:?}", err);
            }
        }
    }
}
```

### Automatic Cleanup

When a client disconnects, pl3xus_sync automatically:
- Removes all subscriptions for that connection
- Cancels pending mutations from that connection
- Releases any exclusive control held by that connection

---

## Connection Lifecycle Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                      CLIENT LIFECYCLE                        │
└─────────────────────────────────────────────────────────────┘

  SyncProvider Mounts
         │
         ▼
  ┌─────────────┐
  │  CLOSED     │◄──────────────────────────────────┐
  └──────┬──────┘                                   │
         │ auto_connect=true                        │
         │ or open() called                         │
         ▼                                          │
  ┌─────────────┐                                   │
  │ CONNECTING  │                                   │
  └──────┬──────┘                                   │
         │                                          │
    ┌────┴────┐                                     │
    │         │                                     │
    ▼         ▼                                     │
 Success    Failure                                 │
    │         │                                     │
    ▼         └─────────────────────────────────────┤
  ┌─────────────┐                                   │
  │    OPEN     │                                   │
  └──────┬──────┘                                   │
         │                                          │
         │ Subscriptions sent                       │
         │ Data flows                               │
         │                                          │
         │ Connection lost                          │
         │ or close() called                        │
         ▼                                          │
  ┌─────────────┐                                   │
  │  CLOSING    │───────────────────────────────────┘
  └─────────────┘
```

---

## Best Practices

### 1. Show Connection Status

Always show users the connection state:

```rust
#[component]
fn StatusBar() -> impl IntoView {
    let connection = use_sync_connection();

    view! {
        <div class="flex items-center gap-2">
            <ConnectionIndicator/>
            <span class="text-sm text-gray-600">
                {move || match connection.ready_state.get() {
                    ConnectionReadyState::Open => "Connected",
                    ConnectionReadyState::Connecting => "Connecting...",
                    ConnectionReadyState::Closing => "Disconnecting...",
                    ConnectionReadyState::Closed => "Offline",
                }}
            </span>
        </div>
    }
}
```

### 2. Handle Offline Gracefully

```rust
#[component]
fn DataView() -> impl IntoView {
    let connection = use_sync_connection();
    let data = use_sync_component::<MyData>();

    view! {
        <Show
            when=move || connection.ready_state.get() == ConnectionReadyState::Open
            fallback=|| view! {
                <div class="text-gray-500">
                    "Waiting for connection..."
                </div>
            }
        >
            // Render data
        </Show>
    }
}
```

### 3. Disable Mutations When Offline

```rust
#[component]
fn EditButton(entity_id: u64) -> impl IntoView {
    let connection = use_sync_connection();
    let is_connected = move || connection.ready_state.get() == ConnectionReadyState::Open;

    view! {
        <button
            disabled=move || !is_connected()
            class=move || if is_connected() { "btn-primary" } else { "btn-disabled" }
        >
            "Edit"
        </button>
    }
}
```

### 4. Log Connection Events

```rust
Effect::new(move || {
    let state = connection.ready_state.get();
    log::info!("Connection state changed: {:?}", state);
});
```

---

## Troubleshooting

### Connection Immediately Closes

**Possible causes:**
- Server not running
- Wrong URL/port
- CORS issues (for web clients)
- Firewall blocking WebSocket

**Debug:**
```rust
.on_error(move |e| {
    log::error!("WebSocket error: {:?}", e);
})
```

### Subscriptions Not Restored

**Check:**
- Connection actually reached `Open` state
- Subscription requests are being sent (check network tab)
- Server is receiving and processing subscriptions

### High Reconnection Frequency

**Solutions:**
- Implement exponential backoff
- Check for server-side issues causing disconnects
- Monitor network stability

---

## Related Documentation

- [Hooks](./hooks.md) - `use_sync_connection` and other hooks
- [Subscriptions](./subscriptions.md) - How subscriptions work
- [WebSocket Patterns](./websocket-patterns.md) - Advanced WebSocket usage
- [Server Setup](./server-setup.md) - Server-side connection handling

---

**Last Updated**: 2025-12-07
**pl3xus_client Version**: 0.1
```


