# Robot HMI Advanced

A sophisticated industrial robot control application demonstrating the pl3xus **Multi-Crate Plugin Architecture** (Pattern 3).

## Architecture

This example showcases the **Multi-Crate Plugin Architecture** - the most sophisticated pattern for large pl3xus applications. Each plugin is its own crate with independent versioning, clear boundaries, and framework/application separation.

```
plugins/src/
├── lib.rs              # Exports build() function for server, types for client
├── core/               # Core plugin (networking, database, ActiveSystem)
│   ├── plugin.rs       # CorePlugin - pl3xus networking + database setup
│   ├── database.rs     # DatabaseResource + init_database
│   └── types/          # Core types (ActiveSystem)
│
└── robot/              # Robot plugin (all robot functionality)
    ├── plugin.rs       # RobotPlugin - registers components, requests, systems
    ├── handlers.rs     # Request handlers
    ├── database.rs     # Robot-specific database operations
    ├── systems.rs      # Polling, jog, motion systems
    └── types/          # Robot types (connection, program, io, requests)
```

## Prerequisites

- Rust toolchain (1.75+)
- [Trunk](https://trunkrs.dev/) for WASM builds: `cargo install trunk`
- FANUC RMI Simulator (in the Fanuc_RMI_API repository)

## Quick Start

This is a **standalone workspace** that can be copied and used independently.

### 1. Start the FANUC Simulator

The simulator emulates a FANUC robot controller. Open a terminal:

```bash
# Navigate to the Fanuc_RMI_API repository
cd path/to/Fanuc_RMI_API

# Run in realtime mode (recommended - simulates actual robot timing)
cargo run -p sim -- --realtime

# Or run in immediate mode (instant responses, good for rapid testing)
cargo run -p sim
```

You should see:
```
🤖 Starting FANUC Simulator in REALTIME mode
   (Simulates actual robot timing, return packets sent after execution)

🤖 FANUC Simulator started on 0.0.0.0:16001
   Waiting for connections...
```

### 2. Start the Server

In a new terminal, from this workspace root:

```bash
# Run the server
cargo run -p robot_hmi_plugins_server
```

You should see:
```
INFO robot_hmi_plugins::robot::plugin: 🤖 RobotPlugin initialized
INFO robot_hmi_plugins::core::database: ✅ Database opened at: fanuc_replica.db
INFO robot_hmi_plugins::robot::database: ✅ Robot database schema initialized
INFO robot_hmi_plugins::core::plugin: ✅ FANUC Replica Server listening on 127.0.0.1:8083
```

### 3. Start the Client App

In a new terminal, from this workspace root:

```bash
cd app

# Build and serve with trunk
trunk serve --port 8084 --open
```

The app will open in your browser at `http://127.0.0.1:8084/`.

## Feature Flags

### Plugins Crate

| Feature  | Description                                  |
|----------|----------------------------------------------|
| `ecs`    | Bevy Component derives (server)              |
| `server` | Server-only code (database, driver, tokio)   |
| `stores` | reactive_stores derives (client)             |

### App Crate

| Feature    | Description                    |
|------------|--------------------------------|
| `devtools` | Enable pl3xus DevTools panel   |

## Default Ports

| Service   | Port  | Description                  |
|-----------|-------|------------------------------|
| Simulator | 16001 | FANUC RMI protocol           |
| Server    | 8083  | pl3xus WebSocket server      |
| Client    | 8084  | Trunk dev server             |

## Database

The server uses SQLite for persistent storage:
- **Location**: `fanuc_replica.db` (in working directory)
- **Schema**: Auto-initialized on first run
- **Contents**: Robot connections, configurations, programs

## Extending with New Plugins

To add a new domain plugin (e.g., PLC I/O):

1. Create a new directory: `plugins/src/plc/`
2. Add the standard structure:
   - `mod.rs` - Module exports
   - `plugin.rs` - Plugin registration
   - `handlers.rs` - Request handlers
   - `database.rs` - PLC-specific DB operations
   - `types/` - Type definitions
3. Register the plugin in `lib.rs`

## Related Examples

- `examples/robot-hmi-advanced/` - Simpler shared-types architecture (Pattern 1)
- `examples/fanuc/` - Basic FANUC example without full UI

