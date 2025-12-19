# Related Repositories Reference

## 1. Original Application: /home/apino/dev/Fanuc_RMI_API

**THE SOURCE OF TRUTH** - This is what we're replicating exactly.

### Structure
```
Fanuc_RMI_API/
├── src/
│   ├── main.rs           # Bevy app with axum router
│   ├── robot/            # Robot driver, state machine, commands
│   ├── api/              # WebSocket handlers, messages
│   └── db/               # SQLite database operations
├── web_app/              # Leptos client application
│   ├── src/
│   │   ├── main.rs
│   │   ├── app.rs
│   │   ├── components/
│   │   │   ├── layout/
│   │   │   │   ├── top_bar.rs        # ~500 lines - WS status, connection, control
│   │   │   │   ├── left_navbar.rs    # Navigation links
│   │   │   │   └── workspace/
│   │   │   │       ├── dashboard/
│   │   │   │       │   ├── control/  # Quick commands, jog, command input
│   │   │   │       │   ├── info/     # Config, frames, tools
│   │   │   │       │   └── right_panel.rs  # Position display
│   │   │   │       ├── programs.rs   # 1525 lines - program management
│   │   │   │       └── settings.rs   # Robot management, system settings
│   │   │   └── common/               # Shared components (modals, etc.)
│   │   ├── hooks/                    # Custom Leptos hooks
│   │   ├── state/                    # Global state management
│   │   └── types/                    # Client-side types
│   └── web_common/       # Shared types (RobotConnection, etc.)
└── sim/                  # FANUC simulator for testing
    └── src/main.rs       # TCP server simulating FANUC controller
```

### Key Patterns to Study
- `web_app/src/state/` - How state is managed (signals, context)
- `web_app/src/hooks/` - Custom hooks for WebSocket, database
- `src/robot/state.rs` - Robot state machine (OperationalMode, etc.)
- `src/api/messages.rs` - All WebSocket message types

### Running the Original
```bash
cd /home/apino/dev/Fanuc_RMI_API
# Build simulator
cargo build -p sim --release
# Run simulator
./target/release/sim
# In another terminal, run server
cargo run --release
# Open http://localhost:8081
```

## 2. Async Bevy Web: /home/apino/dev/async_bevy_web

Reference for Bevy + async Rust patterns.

### Key Patterns
- `bevy_tokio_tasks` - Running async code from Bevy systems
- `TokioTasksRuntime` - Spawning async tasks that can interact with ECS
- Bridging sync Bevy world with async I/O operations

### Usage in Our Project
```rust
use bevy_tokio_tasks::{TokioTasksPlugin, TokioTasksRuntime};

// Spawn async task from system
fn connect_robot(runtime: Res<TokioTasksRuntime>) {
    runtime.spawn_background_task(|mut ctx| async move {
        let driver = FanucDriver::connect("127.0.0.1:16001").await?;
        ctx.run_on_main_thread(move |world| {
            // Update ECS from async context
            world.insert_resource(driver);
        }).await;
        Ok(())
    });
}
```

## 3. Leptos: /home/apino/dev/leptos

Reference for Leptos framework patterns and best practices.

### Key Documentation
- `examples/` - Official examples showing patterns
- `leptos_reactive/` - Understanding signals, effects, memos
- `leptos_dom/` - How DOM updates work

### Critical Patterns for Our Project
- **StoredValue** - Non-reactive storage (use for counters, IDs inside Effects)
- **get_untracked()** - Read signal without creating subscription
- **update_untracked()** - Modify signal without notifying (then call .notify())
- **Effect::new()** - Side effects that run on signal changes

## 4. Leptos-Use: /home/apino/dev/leptos-use

Collection of useful Leptos hooks.

### Relevant Hooks
- `use_websocket` - WebSocket connection management
- `use_debounce` - Debouncing rapid updates
- `use_throttle` - Throttling updates
- `use_local_storage` - Persisting state

### Patterns Used in Our Project
```rust
// Debouncing jog commands
use leptos_use::use_debounce_fn;

let debounced_jog = use_debounce_fn(
    move |direction| send_jog_command(direction),
    100.0 // ms
);
```

## Quick Reference: File Locations

| Feature | Original Location | Replica Location |
|---------|-------------------|------------------|
| Top Bar | `web_app/src/components/layout/top_bar.rs` | `client/src/layout/top_bar.rs` |
| Dashboard | `web_app/src/components/layout/workspace/dashboard/` | `client/src/layout/workspace/dashboard/` |
| Programs | `web_app/src/components/layout/workspace/programs.rs` | `client/src/pages/programs.rs` |
| Settings | `web_app/src/components/layout/workspace/settings.rs` | `client/src/pages/settings.rs` |
| Robot Driver | `src/robot/driver.rs` | `server/src/driver/` |
| Messages | `src/api/messages.rs` | `types/src/messages.rs` |
| State Types | `web_common/src/` | `types/src/lib.rs` |

