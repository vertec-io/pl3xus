# Bevy Pl3xus Examples

This directory contains all examples for the `pl3xus` project, organized by use case.

## Structure

```
examples/
├── shared/                # Shared types used by multiple examples
│   ├── basic_types/       # Types for basic example
│   ├── demo_types/        # Types for demo
│   ├── fanuc_types/       # Types for FANUC demo
│   ├── fanuc_real_types/  # Real FANUC RMI API types
│   └── control_demo_types/# Types for control demo
├── basic/                 # Basic client-server example
│   ├── server/            # Bevy ECS server
│   └── client/            # Leptos WASM client
├── fanuc/                 # FANUC robot control example
│   ├── server/            # Bevy ECS server with FANUC simulation
│   └── client/            # Leptos WASM client
├── control-demo/          # Exclusive control demonstration
│   └── server/            # Server demonstrating ExclusiveControlPlugin
└── devtools-demo/         # DevTools demonstration
    └── server/            # Server for DevTools testing
```

## Running Examples

### Basic Example

**Server:**
```bash
cargo run -p basic_server
```

**Client:**
```bash
cd examples/basic/client
trunk serve --port 8081
```

Then open http://127.0.0.1:8081/

### FANUC Example

**Server:**
```bash
cargo run -p fanuc_server
```

**Client:**
```bash
cd examples/fanuc/client
trunk serve --port 8082
```

Then open http://127.0.0.1:8082/

### Control Demo

**Server:**
```bash
cargo run -p control_demo_server
```

This example demonstrates the `ExclusiveControlPlugin` for managing exclusive control of entities. See [control-demo/README.md](control-demo/README.md) for details.

### DevTools Demo

**Server:**
```bash
cargo run -p devtools_demo_server
```

Then connect with any client using the DevTools widget.

## Example Descriptions

### Basic Example
Demonstrates the core functionality of `pl3xus_client`:
- WebSocket connection to Bevy ECS server
- Component synchronization (Position, Velocity, EntityName)
- Real-time updates
- DevTools integration

### FANUC Example
Shows how to use `pl3xus_client` for industrial robot control:
- Real FANUC RMI API types
- Robot position and status monitoring
- Joint angle visualization
- Mutation support for robot commands

### Control Demo
Demonstrates the `ExclusiveControlPlugin` for exclusive control transfer:
- Exclusive control semantics (only one client can control an entity)
- Control request/release messages
- Automatic timeout for inactive clients
- Hierarchy propagation (control parent = control children)
- State synchronization (all clients see who has control)

### DevTools Demo
Demonstrates the DevTools widget capabilities:
- Entity inspection
- Component viewing
- Subscription management
- Mutation testing

## Shared Types

All shared type crates follow the same pattern:
- `#[derive(Serialize, Deserialize)]` for network serialization
- `#[cfg_attr(feature = "server", derive(Component))]` for conditional Bevy integration
- Feature flag `server` enables Bevy dependency

This pattern enables type sharing between server (with Bevy) and client (WASM, without Bevy).
