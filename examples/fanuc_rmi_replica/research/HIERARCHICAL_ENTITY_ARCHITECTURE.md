# Hierarchical Entity Architecture

## Executive Summary

This document outlines the architectural vision for a scalable, hierarchical entity system where:
- A **System/Apparatus entity** serves as the hierarchy root
- **Robot entities** are spawned as children of the System on connection
- **EntityControl** is managed at the System level with hierarchical propagation
- **Components are filtered by entity** on the client using `where` clauses
- **Executors are entity components**, not resources, enabling multi-system scalability

---

## 1. Current Architecture (Problems)

### 1.1 Entity Spawning
- Robot entity is spawned on `ConnectToRobot` request
- No entity exists before connection → **clients cannot request control**
- All synced components are attached to the single robot entity

### 1.2 Control Model
- `EntityControl` is per-entity, but no entity exists to control before connection
- No hierarchical propagation of control
- Client checks `robot_entity_bits()` which fails if no robot connected

### 1.3 Executor as Resource
- `ProgramExecutor` is a Bevy Resource (singleton)
- Cannot scale to multiple systems/apparatuses
- Tightly coupled to "the one robot"

### 1.4 Client Component Subscription
- `use_sync_component::<T>()` returns `HashMap<u64, T>` (all entities)
- No filtering by entity hierarchy
- No concept of "currently selected robot" in context

---

## 2. Proposed Hierarchical Architecture

### 2.1 Entity Hierarchy

```
System/Apparatus (Root)
├── EntityControl (granted to controlling client)
├── ProgramExecutor (component, not resource)
├── SystemSettings (execution defaults, etc.)
│
├── Robot_1 (child, spawned on connection)
│   ├── RobotConnectionState
│   ├── RobotConnectionDetails
│   ├── RmiDriver (when connected)
│   ├── RobotPosition
│   ├── JointAngles
│   ├── RobotStatus
│   ├── IoStatus
│   ├── ConnectionState (synced)
│   ├── JogSettingsState (synced)
│   └── ...other robot-specific components
│
├── Robot_2 (another child, if multi-robot)
│   └── ...
│
└── (Future: PLC connections, other controllers)
```

### 2.2 Control Flow

1. **Server Startup**: System entity is spawned with `EntityControl` component
2. **Client Connects**: Sees System entity via synced `EntityControl`
3. **Request Control**: Client sends `ControlRequest::Take(system_entity_bits)`
4. **Hierarchical Propagation**: pl3xus propagates control to all children
5. **Connection Request**: Only controller can issue `ConnectToRobot`
6. **Robot Spawned**: As child of System, inherits control from parent

### 2.3 Connection Lifecycle

```
[Client requests ConnectToRobot]
         │
         ▼
[Server validates control at System level]
         │
         ▼
[Spawn Robot entity as child of System]
  - Insert RobotConnectionDetails from DB/message
  - Insert ConnectionState { robot_connecting: true }
  - Set RobotConnectionState::Connecting
         │
         ▼
[handle_connecting_state system]
  - Async connect to FANUC controller
  - On success: insert RmiDriver, set Connected
  - On failure: set Disconnected with error
         │
         ▼
[Robot is now Connected]
  - Polling systems read RmiDriver
  - UI sees ConnectionState { robot_connected: true }
```

### 2.4 Disconnection Lifecycle

```
[Client requests DisconnectRobot(robot_entity_id)]
         │
         ▼
[Server validates control]
         │
         ▼
[Set RobotConnectionState::Disconnecting]
         │
         ▼
[handle_disconnecting_state system]
  - Call driver.disconnect()
  - Wait for acknowledgment
  - Remove RmiDriver, RmiResponseChannel
  - Set RobotConnectionState::Disconnected
         │
         ▼
[Optional: Despawn robot entity or keep for reconnect]
```

---

## 3. Component Placement

### 3.1 System Entity Components
| Component | Purpose |
|-----------|---------|
| `SystemMarker` | Marker struct for queries |
| `EntityControl` | Who controls the apparatus |
| `ProgramExecutor` | Execution state (was resource) |
| `SystemSettings` | Global settings |

### 3.2 Robot Entity Components
| Component | Purpose |
|-----------|---------|
| `FanucRobot` | Marker struct |
| `RobotConnectionState` | State machine (enum) |
| `RobotConnectionDetails` | addr, port, name |
| `RmiDriver` | Arc<FanucDriver> (when connected) |
| `RobotPosition` | Cartesian position (synced) |
| `JointAngles` | Joint values (synced) |
| `RobotStatus` | Alarms, mode, etc. (synced) |
| `IoStatus` | Digital/analog I/O (synced) |
| `ConnectionState` | UI-facing connection info (synced) |
| `JogSettingsState` | Jog defaults (synced) |
| `ExecutionState` | Current execution progress (synced) |

