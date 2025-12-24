# DevTools Guide

This guide covers how to use the built-in DevTools for debugging and inspecting ECS entities synchronized via pl3xus_sync.

---

## Overview

DevTools provides a real-time inspector for your synchronized ECS state. It allows you to:

- **View all entities** synchronized from the server
- **Inspect component data** in JSON format
- **Navigate entity hierarchies** (parent/child relationships)
- **Edit component values** and send mutations to the server
- **Monitor connection status** and incoming messages

---

## Setup

### 1. Enable the DevTools Feature

Add the `devtools` feature to your client's `Cargo.toml`:

```toml
[dependencies]
pl3xus_client = { version = "0.1", features = ["devtools"] }
```

### 2. Configure the Registry

DevTools requires JSON conversion support in your type registry. Add `.with_devtools_support()`:

```rust
use pl3xus_client::ClientTypeRegistry;
use std::sync::Arc;

let registry = Arc::new(
    ClientTypeRegistry::builder()
        .register::<Position>()
        .register::<Velocity>()
        .with_devtools_support()  // Required for DevTools
        .build()
);
```

> **Important**: Without `.with_devtools_support()`, DevTools will show an error and component data won't display correctly.

### 3. Add the DevTools Component

```rust
use pl3xus_client::devtools::{DevTools, DevToolsMode};

#[component]
fn App() -> impl IntoView {
    let registry = Arc::new(
        ClientTypeRegistry::builder()
            .register::<Position>()
            .with_devtools_support()
            .build()
    );

    view! {
        <SyncProvider url="ws://localhost:8082" registry=registry.clone()>
            <MyAppUI />
            // Add DevTools - it manages its own connection
            <DevTools 
                ws_url="ws://localhost:8082/sync" 
                registry=registry
            />
        </SyncProvider>
    }
}
```

---

## Display Modes

DevTools supports two display modes:

### Widget Mode (Default)

A floating button in the corner that expands to a modal:

```rust
<DevTools 
    ws_url="ws://localhost:8082/sync" 
    registry=registry
    mode=DevToolsMode::Widget  // Default
/>
```

**Features:**
- Minimal footprint when collapsed (small floating button)
- Expands to full modal on click
- "Open in New Tab" button for dedicated view
- Badge shows entity count

### Embedded Mode

Full-screen DevTools view for dedicated debugging pages:

```rust
<DevTools 
    ws_url="ws://localhost:8082/sync" 
    registry=registry
    mode=DevToolsMode::Embedded
/>
```

**Features:**
- Takes up the entire viewport
- Best for dedicated DevTools windows/tabs
- Maximum visibility for debugging

---

## UI Overview

The DevTools interface has several sections:

### 1. Header Bar

- **Connection status indicator** (green = connected, red = disconnected)
- **WebSocket URL** being monitored
- **Entity count** currently synchronized

### 2. Entity List (Left Panel)

Lists all synchronized entities. Two view modes:

- **List View**: Simple flat list of entities
- **Tree View**: Hierarchical view showing parent/child relationships

Click an entity to inspect its components.

### 3. Component Inspector (Right Panel)

Displays all components on the selected entity:

- **Component type name** as header
- **Editable fields** for each component property
- Changes are sent as mutations when you press Enter

### 4. Server Messages Panel (Bottom)

Collapsible panel showing:

- Raw server messages for debugging
- Flash indicator when new messages arrive
- Expand to see full message content

---

## Entity Hierarchies

DevTools automatically detects parent/child relationships using **Bevy's built-in hierarchy components**:

- **`ChildOf`**: Bevy's relationship component that stores the parent entity
- **`Children`**: Automatically maintained by Bevy when `ChildOf` is used

When using Tree View, entities display in a hierarchical structure with expand/collapse controls.

### Setting Up Hierarchies

Use Bevy's standard entity hierarchy APIs - no special registration required:

```rust
use bevy::prelude::*;

// Spawn a parent entity
let parent = commands.spawn((
    Name::new("System"),
    SystemConfig::default(),
)).id();

// Spawn children using ChildOf
commands.spawn((
    Name::new("Robot A"),
    ChildOf(parent),  // This creates the parent-child relationship
    RobotConfig::default(),
));

// Or use the fluent API
commands.entity(parent).with_children(|parent| {
    parent.spawn((
        Name::new("Robot B"),
        RobotConfig::default(),
    ));
});
```

DevTools will automatically detect and display the hierarchy. The `Children` component is managed by Bevy and doesn't need to be manually added.

---

## Editing Components

DevTools provides inline editing for component values:

1. **Click** on a field value to enter edit mode
2. **Modify** the value in the input field
3. **Press Enter** to send the mutation to the server
4. **Click away** to cancel and revert to server value

The server will process the mutation through your `MutationAuthorizer` (see [Mutations Guide](./mutations.md)).

---

## Standalone DevTools Page

For complex debugging, open DevTools in a dedicated browser tab:

1. Click the **"Open in New Tab"** button in Widget mode
2. Or navigate to your app URL with `?devtools=1` query parameter

This gives you a full-screen debugging experience.

---

## Best Practices

### 1. Enable DevTools Only in Development

Use feature flags to exclude DevTools from production builds:

```rust
#[cfg(feature = "dev")]
view! {
    <DevTools
        ws_url="ws://localhost:8082/sync"
        registry=registry
    />
}
```

### 2. Register All Synced Components

Ensure the client registry includes all component types the server syncs:

```rust
// Server syncs these:
app.sync_component::<Position>(None);
app.sync_component::<Velocity>(None);

// Client must register the same types:
let registry = ClientTypeRegistry::builder()
    .register::<Position>()
    .register::<Velocity>()
    .with_devtools_support()
    .build();
```

Unregistered types will show as raw bytes in DevTools.

### 3. Use Bevy's Name Component for Better Labels

DevTools automatically looks for Bevy's built-in `Name` component to display friendly labels:

```rust
use bevy::prelude::*;

commands.spawn((
    Name::new("Robot A"),  // Bevy's built-in Name component
    RobotConfig::default(),
));

// In DevTools, this entity shows as "Robot A · #12345" instead of just "#12345"
```

DevTools looks for any component with a `name` or `label` field, so Bevy's `Name` component works automatically.

### 4. Check Registry Configuration

If DevTools shows warnings or blank panels, verify:

- `.with_devtools_support()` was called on the registry builder
- The registry is the same `Arc<ClientTypeRegistry>` passed to both `SyncProvider` and `DevTools`
- All component types are registered before building

---

## Troubleshooting

### "DevTools will not function correctly" Error

**Cause**: Registry wasn't built with `.with_devtools_support()`.

**Fix**:
```rust
let registry = ClientTypeRegistry::builder()
    .register::<MyComponent>()
    .with_devtools_support()  // Add this!
    .build();
```

### Components Show as Raw Bytes

**Cause**: Component type not registered in client registry.

**Fix**: Register the component type:
```rust
.register::<MissingComponent>()
```

### Entity Hierarchy Not Showing

**Cause**: Entities not spawned with parent-child relationships using Bevy's `ChildOf` component.

**Fix**: Use Bevy's built-in hierarchy APIs:
```rust
use bevy::prelude::*;

// Use ChildOf to establish parent-child relationships
let parent = commands.spawn(Name::new("Parent")).id();
commands.spawn((
    Name::new("Child"),
    ChildOf(parent),  // This creates the hierarchy
));
```

DevTools automatically detects `ChildOf` relationships and displays them in Tree View.

### Mutations Failing

**Cause**: Server's `MutationAuthorizer` rejecting requests.

**Fix**: Check server logs and authorization configuration. See [Mutations Guide](./mutations.md).

---

## Complete Example

Run the basic example with DevTools:

```bash
# Terminal 1: Start the server
cargo run -p basic_server

# Terminal 2: Start the client with DevTools
cd examples/basic/client && trunk serve --open
```

The client includes DevTools that you can expand to inspect synchronized entities.

---

## Related Documentation

- [Mutations](./mutations.md) - Control what clients can edit
- [Shared Types](./shared-types.md) - Setting up shared component types
- [Getting Started: pl3xus_client](../../client/index.md) - Full client setup
- [API Reference](https://docs.rs/pl3xus_client) - Full API documentation

---

**Last Updated**: 2025-12-07
**pl3xus_client Version**: 0.1


