---
name: industrial-systems
description: ECS architecture compared to traditional industrial frameworks. Covers SCADA, PLC, Ignition, Thingworx, and why ECS patterns excel for industrial applications.
allowed-tools:
  - view
  - codebase-retrieval
  - web-search
  - web-fetch
---

# Industrial Systems Skill

## Purpose

This skill explains why ECS architecture (as used in pl3xus) is well-suited for industrial applications, and how it compares to traditional industrial frameworks.

## When to Use

Use this skill when:
- Explaining pl3xus to industrial engineers
- Comparing to existing systems
- Justifying architecture decisions
- Understanding industrial requirements

## Traditional Industrial Architectures

### SCADA (Supervisory Control and Data Acquisition)

```
┌─────────────────────────────────────────────────────────────┐
│                    SCADA Architecture                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   HMI       │  │  Historian  │  │   Alarm Server      │  │
│  │ (Displays)  │  │ (Time-series│  │   (Event-based)     │  │
│  └──────┬──────┘  └──────┬──────┘  └──────────┬──────────┘  │
│         │                │                     │             │
│         └────────────────┼─────────────────────┘             │
│                          │                                   │
│                    ┌─────┴─────┐                             │
│                    │   OPC UA  │                             │
│                    │  Server   │                             │
│                    └─────┬─────┘                             │
│                          │                                   │
│         ┌────────────────┼────────────────┐                  │
│         │                │                │                  │
│    ┌────┴────┐     ┌────┴────┐     ┌────┴────┐              │
│    │  PLC 1  │     │  PLC 2  │     │  PLC 3  │              │
│    └─────────┘     └─────────┘     └─────────┘              │
└─────────────────────────────────────────────────────────────┘
```

**Characteristics:**
- Tag-based data model (flat namespace)
- Polling-based updates
- Separate systems for different concerns
- Vendor-specific protocols

### PLC Programming (IEC 61131-3)

```
Ladder Logic Example:
─┤ Start ├──┤ NOT Stop ├──┤ Sensor ├──( Motor )─

Function Block Diagram:
┌─────────┐     ┌─────────┐
│  Timer  │────▶│   AND   │────▶ Output
│  TON    │     │         │
└─────────┘     └─────────┘
```

**Characteristics:**
- Scan-cycle execution
- Real-time deterministic
- Limited data structures
- Difficult to scale

### Ignition (Inductive Automation)

**Characteristics:**
- Tag-based with hierarchical UDTs
- Python scripting
- SQL database backend
- Web-based HMI

### Thingworx (PTC)

**Characteristics:**
- Thing-based model (similar to entities)
- Property/Service/Event model
- REST API driven
- Cloud-native

## ECS vs Traditional: Comparison

| Aspect | Traditional SCADA | PLC | ECS (pl3xus) |
|--------|-------------------|-----|--------------|
| Data Model | Tags (flat) | Variables | Entities + Components |
| Relationships | Manual | None | Native hierarchies |
| Extensibility | Vendor-specific | Limited | Composable |
| Real-time | Polling | Scan cycle | Event-driven |
| Scalability | Limited | Fixed | Horizontal |
| State Management | Distributed | Local | Centralized |

## Why ECS for Industrial

### 1. Natural Entity Modeling

Industrial systems have natural entities:
- Robots, PLCs, sensors, actuators
- Production lines, cells, stations
- Products, batches, orders

```rust
// ECS models this naturally
commands.spawn((
    Name::new("Robot-1"),
    Position::default(),
    RobotConfig { model: "FANUC M-20iD".into() },
    ConnectionStatus::Disconnected,
));
```

### 2. Composable Components

Add capabilities without modifying existing code:

```rust
// Add new capability to existing entity
commands.entity(robot).insert(VisionSystem::default());

// Query entities with specific capabilities
Query<&Position, With<VisionSystem>>
```

### 3. Hierarchical Relationships

Model plant hierarchies naturally:

```
Plant
├── Line-1
│   ├── Cell-1
│   │   ├── Robot-1
│   │   └── Robot-2
│   └── Cell-2
│       └── Robot-3
└── Line-2
    └── Cell-3
        └── Robot-4
```

```rust
// Bevy hierarchy
let plant = commands.spawn(Name::new("Plant")).id();
let line = commands.spawn((Name::new("Line-1"), ChildOf(plant))).id();
let cell = commands.spawn((Name::new("Cell-1"), ChildOf(line))).id();
commands.spawn((Name::new("Robot-1"), ChildOf(cell)));
```

### 4. Real-time Synchronization

pl3xus provides:
- WebSocket-based push updates
- Component-level change detection
- Efficient binary serialization (bincode)

### 5. Authorization Model

Industrial systems need access control:
- Operator vs Engineer vs Admin
- Machine-level control
- Hierarchical permissions

```rust
// pl3xus EntityControl
app.request::<StartRobot, _>()
    .targeted()
    .with_default_entity_policy()  // Requires control
    .register();
```

## Migration Patterns

### From Tag-Based to ECS

```
SCADA Tags:                    ECS Components:
Robot1.Position.X    ──▶      Entity(Robot1) + Position { x, y, z }
Robot1.Position.Y
Robot1.Position.Z
Robot1.Speed         ──▶      Entity(Robot1) + Velocity { ... }
Robot1.Status        ──▶      Entity(Robot1) + RobotStatus { ... }
```

### From Polling to Push

```
SCADA: Client polls every 100ms
ECS: Server pushes on change

// pl3xus automatically syncs on component change
app.sync_component::<Position>(None);
```

## Industrial Requirements Mapping

| Requirement | pl3xus Solution |
|-------------|-----------------|
| Real-time updates | WebSocket push, component sync |
| Access control | EntityControl, policies |
| Audit trail | Message logging, state history |
| Scalability | ECS architecture, horizontal scaling |
| Reliability | Server-authoritative, reconnection |
| Integration | Bevy plugins, async drivers |

## Related Skills

- **pl3xus-development**: Complete workflow
- **bevy-ecs**: ECS fundamentals
- **pl3xus-authorization**: Access control

## Reference

- [Industrial Comparison](./references/industrial-comparison.md)
- [Migration Guide](./references/migration-guide.md)

