# Fanuc RMI Replica

A complete replica of the Fanuc_RMI_API web application, rebuilt using the **pl3xus** real-time sync framework.

## Overview

This example demonstrates how to build a full-featured industrial robotics control application using pl3xus. It replicates all functionality of the original Fanuc_RMI_API application while significantly reducing code complexity and improving maintainability.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Client (WASM)                            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │   Leptos    │  │  pl3xus     │  │     Components          │  │
│  │   Signals   │◄─┤  Client     │◄─┤  - Dashboard            │  │
│  │             │  │  Hooks      │  │  - Programs             │  │
│  └─────────────┘  └──────┬──────┘  │  - Settings             │  │
│                          │         └─────────────────────────┘  │
└──────────────────────────┼──────────────────────────────────────┘
                           │ WebSocket (Binary)
┌──────────────────────────┼──────────────────────────────────────┐
│                          │         Server (Bevy ECS)            │
│  ┌─────────────┐  ┌──────▼──────┐  ┌─────────────────────────┐  │
│  │   SQLite    │◄─┤  Request    │◄─┤     Plugins             │  │
│  │   Database  │  │  Handlers   │  │  - Connection           │  │
│  └─────────────┘  └──────┬──────┘  │  - Sync                 │  │
│                          │         │  - Requests             │  │
│  ┌─────────────┐  ┌──────▼──────┐  └─────────────────────────┘  │
│  │  fanuc_rmi  │◄─┤   Driver    │                               │
│  │   Driver    │  │   Sync      │                               │
│  └─────────────┘  └─────────────┘                               │
└─────────────────────────────────────────────────────────────────┘
```

## Features

### Robot Management
- **Connection CRUD** - Create, read, update, delete robot connections
- **Configuration CRUD** - Manage robot configurations with default settings
- **Real-time Connection** - Connect/disconnect to physical robots via `fanuc_rmi`

### Robot Control
- **Initialization** - Initialize robot with configuration
- **Jogging** - Manual robot movement (joint/Cartesian)
- **Motion Commands** - Linear and joint motion execution
- **Speed Override** - Real-time speed adjustment
- **Abort/Reset** - Emergency stop and fault recovery

### Programs
- **Program CRUD** - Create, edit, delete robot programs
- **Instruction Management** - Add/remove motion instructions
- **Program Execution** - Load, start, pause, resume, stop programs

### I/O Operations
- **Digital I/O** - Read digital inputs, write digital outputs
- **Analog I/O** - Read analog inputs, write analog outputs
- **Group I/O** - Read/write group I/O

### Settings
- **Robot Settings** - Default speed, frames, tools
- **I/O Display Config** - Customize I/O panel display
- **Database Reset** - Full database reset capability

## Running the Application

### Prerequisites
- Rust toolchain with `wasm32-unknown-unknown` target
- `trunk` for WASM builds: `cargo install trunk`
- `cargo-make` for task running: `cargo install cargo-make`

### Development

```bash
# From the repository root
cd examples/fanuc_rmi_replica

# Run both client and server in development mode
cargo make dev

# Or run separately:
# Terminal 1: Server
cargo run -p fanuc_replica_server

# Terminal 2: Client (with hot reload)
cd client && trunk serve --open
```

### Production Build

```bash
cd examples/fanuc_rmi_replica
cargo make build
```

## Project Structure

```
fanuc_rmi_replica/
├── client/                 # WASM client application
│   ├── src/
│   │   ├── app.rs         # Main app component & routing
│   │   ├── components/    # Reusable UI components
│   │   ├── layout/        # Layout components (top bar, etc.)
│   │   └── pages/         # Page components
│   │       ├── dashboard/ # Robot control dashboard
│   │       ├── programs/  # Program management
│   │       └── settings.rs # Settings page
│   └── assets/            # Static assets
├── server/                 # Native server application
│   └── src/
│       ├── main.rs        # Server entry point
│       ├── database.rs    # SQLite database layer
│       ├── driver_sync.rs # Robot driver state sync
│       ├── jogging.rs     # Jog command handling
│       └── plugins/       # Bevy plugins
│           ├── connection.rs  # Robot connection management
│           ├── requests.rs    # Request handlers
│           └── sync.rs        # State synchronization
└── shared/                 # Shared types (symlink to fanuc_replica_types)
```

## Key Files

| File | Purpose |
|------|---------|
| `fanuc_replica_types/src/lib.rs` | All request/response message types |
| `server/src/plugins/requests.rs` | Server-side request handlers |
| `server/src/database.rs` | SQLite database operations |
| `client/src/pages/dashboard/` | Main robot control interface |

## Documentation

- [LESSONS_LEARNED.md](./LESSONS_LEARNED.md) - Development patterns and anti-patterns
- [PL3XUS_BENEFITS.md](./PL3XUS_BENEFITS.md) - Benefits over original implementation

