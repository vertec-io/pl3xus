# FANUC Real Robot Control Client

A Leptos WASM client for controlling a real FANUC robot using the FANUC_RMI_API driver.

## Overview

This example demonstrates:
- ✅ **pl3xus_client hooks** - Using `use_sync_component<T>()` for reactive data
- ✅ **Shared crate pattern** - Using `fanuc_real_shared` types without Bevy dependency
- ✅ **DevTools integration** - Built-in component inspector
- ✅ **Real-time updates** - Live robot position, status, and joint angles
- ✅ **Tailwind UI** - Polished, professional interface

## Prerequisites

1. **Trunk** - WASM build tool
   ```bash
   cargo install trunk
   rustup target add wasm32-unknown-unknown
   ```

2. **FANUC Real Server** - Running on `ws://127.0.0.1:8082`
   ```bash
   cargo run -p pl3xus_sync --example fanuc_real_server --features runtime
   ```

## Running the Example

```bash
cd crates/pl3xus_client/examples/fanuc_real_client
trunk serve --port 8084
```

Open `http://localhost:8084` in your browser.

## Features

### Real-Time Robot Data

The client displays:
- **Robot Status** - Servo ready, TP enabled, in motion indicators
- **Cartesian Position** - X, Y, Z, W, P, R coordinates
- **Joint Angles** - J1-J6 joint positions

### DevTools Integration

The right panel shows the built-in DevTools with:
- Entity list with component counts
- Component inspector
- Real-time value updates
- Mutation support (edit component values)

### Shared Types

Uses `fanuc_real_shared` crate with conditional compilation:
```rust
// Server builds with "server" feature (includes Bevy)
shared_types = { path = "../../../fanuc_real_shared", features = ["server"] }

// Client builds WITHOUT "server" feature (no Bevy, WASM-compatible)
shared_types = { path = "../../../fanuc_real_shared" }
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    fanuc_real_client                        │
│                    (Leptos WASM)                            │
│                                                             │
│  ┌──────────────────┐  ┌──────────────────┐               │
│  │ use_sync_component│  │    DevTools      │               │
│  │  <RobotPosition> │  │   (Inspector)    │               │
│  │  <RobotStatus>   │  │                  │               │
│  │  <JointAngles>   │  │                  │               │
│  └──────────────────┘  └──────────────────┘               │
│           │                      │                          │
│           └──────────┬───────────┘                          │
│                      │                                      │
│              ┌───────▼────────┐                            │
│              │  SyncProvider  │                            │
│              │  (WebSocket)   │                            │
│              └───────┬────────┘                            │
└──────────────────────┼─────────────────────────────────────┘
                       │
                       │ ws://127.0.0.1:8082
                       │
┌──────────────────────▼─────────────────────────────────────┐
│              fanuc_real_server                              │
│              (Bevy + FANUC_RMI_API)                        │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐ │
│  │  Pl3xusSyncPlugin                                  │ │
│  │  - Syncs RobotPosition, RobotStatus, JointAngles     │ │
│  │  - Handles mutations from client                      │ │
│  └──────────────────────────────────────────────────────┘ │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐ │
│  │  FANUC_RMI_API Driver                                │ │
│  │  - Connects to FANUC simulator                        │ │
│  │  - Reads position, status, joint angles              │ │
│  │  - Executes motion commands                           │ │
│  └──────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## Code Highlights

### Using Shared Types

```rust
use fanuc_real_shared::{RobotPosition, RobotStatus, JointAngles};
use pl3xus_client::impl_sync_component;

// Implement SyncComponent for shared types
impl_sync_component!(RobotPosition);
impl_sync_component!(RobotStatus);
impl_sync_component!(JointAngles);
```

### Reactive Component Subscription

```rust
#[component]
fn PositionDisplay() -> impl IntoView {
    // Automatically subscribes to RobotPosition components
    let robot_positions = use_sync_component::<RobotPosition>();

    // Get the first robot position
    let robot_position = move || {
        robot_positions.get()
            .values()
            .next()
            .cloned()
    };

    view! {
        <div>
            "X: " {move || format!("{:.2}", robot_position().map(|p| p.x).unwrap_or(0.0))}
        </div>
    }
}
```

### Registry Setup

```rust
let registry = ClientRegistryBuilder::new()
    .register::<RobotPosition>()
    .register::<RobotStatus>()
    .register::<JointAngles>()
    .register::<RobotInfo>()
    .build();

view! {
    <SyncProvider url="ws://127.0.0.1:8082" registry=registry>
        <RobotUI />
        <DevTools />
    </SyncProvider>
}
```

## Next Steps

- Try editing component values in DevTools
- Add motion command controls
- Implement jog controls for manual movement
- Add 3D visualization of robot position

## Related Examples

- **fanuc_real_server** - Server-side FANUC_RMI_API integration
- **basic_client** - Simpler example without real hardware
- **devtools_demo** - DevTools-focused example

