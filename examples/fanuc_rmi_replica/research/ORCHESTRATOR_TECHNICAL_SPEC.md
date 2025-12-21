# Program Orchestrator Technical Specification

## Overview

This document specifies the refactored program execution architecture, replacing the singleton `ProgramExecutor` resource with a component-based `ProgramOrchestrator` on the System entity and per-robot `RobotExecutionState` components.

---

## 1. Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           SYSTEM ENTITY                                     │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │  Components:                                                           │ │
│  │  - SystemMarker (marker)                                               │ │
│  │  - EntityControl (pl3xus exclusive control)                            │ │
│  │  - ProgramOrchestrator (execution logic)                               │ │
│  │  - OrchestratorStatus (synced to clients)                              │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│  ┌─────────────────────────────┐    ┌─────────────────────────────┐        │
│  │  ROBOT A (Child Entity)     │    │  ROBOT B (Child Entity)     │        │
│  │  - FanucRobot (marker)      │    │  - FanucRobot (marker)      │        │
│  │  - ActiveRobot (optional)   │    │                             │        │
│  │  - RobotConnectionState     │    │  - RobotConnectionState     │        │
│  │  - RobotConnectionDetails   │    │  - RobotConnectionDetails   │        │
│  │  - RmiDriver (when conn)    │    │  - RmiDriver (when conn)    │        │
│  │  - RobotExecutionState      │    │  - RobotExecutionState      │        │
│  │    (synced)                 │    │    (synced)                 │        │
│  │  - RobotPosition (synced)   │    │  - RobotPosition (synced)   │        │
│  │  - JointAngles (synced)     │    │  - JointAngles (synced)     │        │
│  │  - ... other components     │    │  - ... other components     │        │
│  └─────────────────────────────┘    └─────────────────────────────┘        │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Component Definitions

### 2.1 System-Level Components

```rust
/// Marker for the root System entity
#[derive(Component, Default)]
pub struct SystemMarker;

/// Program orchestrator - manages program flow and step advancement
/// This is a COMPONENT on System entity, not a Resource
#[derive(Component, Default)]
pub struct ProgramOrchestrator {
    /// The loaded program
    pub program: Option<LoadedProgram>,
    
    /// Current step index in the program
    pub current_step: usize,
    
    /// Orchestrator state machine
    pub state: OrchestratorState,
    
    /// Error message if in Error state
    pub error: Option<String>,
}

/// Orchestrator state machine
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OrchestratorState {
    #[default]
    Idle,       // No program running
    Running,    // Actively executing program
    Paused,     // Program paused, can resume
    Completed,  // Program finished successfully
    Error,      // Program stopped due to error
}

/// Synced component for UI to display orchestrator status
#[derive(Component, Clone, Default, Serialize, Deserialize, SyncComponent)]
pub struct OrchestratorStatus {
    pub state: String,  // "idle", "running", "paused", "completed", "error"
    pub program_name: Option<String>,
    pub current_step: usize,
    pub total_steps: usize,
    pub error: Option<String>,
}
```

### 2.2 Robot-Level Components

```rust
/// Per-robot execution state - what is this robot currently doing?
#[derive(Component, Clone, Default, Serialize, Deserialize, SyncComponent)]
pub struct RobotExecutionState {
    /// Current state of the robot's execution
    pub state: RobotExecState,
    
    /// Which instruction (step index) this robot is executing
    pub executing_step: Option<usize>,
    
    /// Description of current action for UI
    pub current_action: Option<String>,
    
    /// Error if in error state
    pub error: Option<String>,
}

/// Robot execution state machine
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RobotExecState {
    #[default]
    Idle,       // Robot is not executing anything
    Busy,       // Robot is executing a motion/command
    Waiting,    // Robot completed, waiting for orchestrator
    Error,      // Robot encountered an error
}

/// Marker component for the currently active robot
/// Only one robot should have this at a time
#[derive(Component)]
pub struct ActiveRobot;

/// Component added when robot disconnects, tracks time for cleanup
#[derive(Component)]
pub struct DisconnectedSince(pub std::time::Instant);
```

---

## 3. State Machines

### 3.1 Orchestrator State Machine

```
                    ┌──────────────────────────────────┐
                    │                                  │
                    ▼                                  │
    ┌──────┐    LoadProgram    ┌─────────┐         ┌──────────┐
    │ Idle │ ─────────────────►│ Running │────────►│Completed │
    └──────┘                   └─────────┘         └──────────┘
        ▲                          │ │                  │
        │                    Pause │ │ Error            │
        │                          ▼ ▼                  │
        │                      ┌────────┐  ┌───────┐    │
        │                      │ Paused │  │ Error │    │
        │                      └────────┘  └───────┘    │
        │                          │           │        │
        │         Resume           │           │        │
        │◄─────────────────────────┘           │        │
        │                                      │        │
        └──────────────── Stop ────────────────┴────────┘
```

### 3.2 Robot Execution State Machine

```
    ┌──────┐  Receive Command   ┌──────┐  Command Complete  ┌─────────┐
    │ Idle │ ──────────────────►│ Busy │ ──────────────────►│ Waiting │
    └──────┘                    └──────┘                    └─────────┘
        ▲                           │                            │
        │                     Error │                            │
        │                           ▼                            │
        │                       ┌───────┐                        │
        │                       │ Error │                        │
        │                       └───────┘                        │
        │                           │                            │
        └────── Orchestrator ───────┴────────────────────────────┘
                acknowledges
```

---

## 4. Execution Flow

### 4.1 Single Robot Execution (Current Use Case)

```
1. Client sends LoadProgram { program_id }
   └── Server loads program into ProgramOrchestrator.program
   └── OrchestratorStatus syncs: program_name, total_steps

2. Client sends StartProgram
   └── Server sets OrchestratorState::Running
   └── orchestrator_step_system runs:
       a. Get current instruction from program[current_step]
       b. Find ActiveRobot entity
       c. Send command to robot's RmiDriver
       d. Set robot's RobotExecutionState::Busy { executing_step: current_step }

3. Robot completes motion
   └── Driver response received
   └── Robot's RobotExecutionState::Waiting

4. orchestrator_advance_system runs:
   └── Sees ActiveRobot is Waiting
   └── Sets robot to Idle
   └── Increments orchestrator.current_step
   └── If more steps: goto step 2b
   └── If complete: OrchestratorState::Completed

5. Client sees OrchestratorStatus { state: "completed" }
```

### 4.2 Pause/Resume Flow

```
1. Client sends PauseProgram
   └── OrchestratorState::Paused
   └── Robot continues current motion (Busy)
   └── When robot completes: Waiting (not Idle - orchestrator paused)

2. Client sends ResumeProgram
   └── OrchestratorState::Running
   └── Robot in Waiting → Idle
   └── Orchestrator advances to next step
```

### 4.3 Stop Flow

```
1. Client sends StopProgram
   └── OrchestratorState::Idle
   └── If robot Busy: send abort to driver (if supported)
   └── Robot → Idle
   └── Clear program or reset current_step (TBD)
```

---

## 5. System Implementations

### 5.1 Core Systems

```rust
/// Advance orchestrator when active robot completes
fn orchestrator_advance_system(
    mut orchestrators: Query<&mut ProgramOrchestrator, With<SystemMarker>>,
    mut robots: Query<&mut RobotExecutionState, With<ActiveRobot>>,
    mut status: Query<&mut OrchestratorStatus, With<SystemMarker>>,
) {
    let Ok(mut orchestrator) = orchestrators.get_single_mut() else { return };
    let Ok(mut robot_state) = robots.get_single_mut() else { return };

    // Only advance if orchestrator is running and robot is waiting
    if orchestrator.state != OrchestratorState::Running {
        return;
    }
    if robot_state.state != RobotExecState::Waiting {
        return;
    }

    // Robot completed - acknowledge and advance
    robot_state.state = RobotExecState::Idle;
    robot_state.executing_step = None;
    robot_state.current_action = None;

    // Advance to next step
    orchestrator.current_step += 1;

    // Check if program complete
    if let Some(ref program) = orchestrator.program {
        if orchestrator.current_step >= program.instructions.len() {
            orchestrator.state = OrchestratorState::Completed;
            // Update synced status
            if let Ok(mut s) = status.get_single_mut() {
                s.state = "completed".to_string();
            }
        }
    }
}

/// Execute current step on active robot
fn orchestrator_step_system(
    orchestrators: Query<&ProgramOrchestrator, With<SystemMarker>>,
    mut robots: Query<(&RmiDriver, &mut RobotExecutionState), With<ActiveRobot>>,
) {
    let Ok(orchestrator) = orchestrators.get_single() else { return };
    let Ok((driver, mut robot_state)) = robots.get_single_mut() else { return };

    // Only execute if orchestrator running and robot idle
    if orchestrator.state != OrchestratorState::Running {
        return;
    }
    if robot_state.state != RobotExecState::Idle {
        return;
    }

    // Get current instruction
    let Some(ref program) = orchestrator.program else { return };
    let Some(instruction) = program.instructions.get(orchestrator.current_step) else {
        return;
    };

    // Send to driver
    // (actual implementation depends on instruction type)
    robot_state.state = RobotExecState::Busy;
    robot_state.executing_step = Some(orchestrator.current_step);
    robot_state.current_action = Some(format!("{:?}", instruction));
}
```

### 5.2 Request Handlers

```rust
fn handle_start_program(
    mut orchestrators: Query<&mut ProgramOrchestrator, With<SystemMarker>>,
    robots: Query<Entity, With<ActiveRobot>>,
    // ... events
) {
    // Validate: program loaded, active robot exists and connected
    let Ok(mut orchestrator) = orchestrators.get_single_mut() else {
        // Error: no system entity
        return;
    };
    if orchestrator.program.is_none() {
        // Error: no program loaded
        return;
    }
    if robots.is_empty() {
        // Error: no active robot
        return;
    }

    orchestrator.current_step = 0;
    orchestrator.state = OrchestratorState::Running;
}

fn handle_set_active_robot(
    mut commands: Commands,
    orchestrators: Query<&ProgramOrchestrator, With<SystemMarker>>,
    current_active: Query<Entity, With<ActiveRobot>>,
    robots: Query<Entity, With<FanucRobot>>,
    // ... events with target robot entity
) {
    let Ok(orchestrator) = orchestrators.get_single() else { return };

    // Don't allow changing while running
    if orchestrator.state == OrchestratorState::Running {
        // Error: cannot change active robot while running
        return;
    }

    // Remove ActiveRobot from current
    for entity in current_active.iter() {
        commands.entity(entity).remove::<ActiveRobot>();
    }

    // Add to target (validate target_entity is in robots)
    // commands.entity(target_entity).insert(ActiveRobot);
}
```

---

## 6. Migration Plan

### Phase 4a: Create New Components
1. Add `SystemMarker` component
2. Add `ProgramOrchestrator` component (copy logic from `ProgramExecutor`)
3. Add `OrchestratorStatus` synced component
4. Add `RobotExecutionState` synced component
5. Add `ActiveRobot` marker component

### Phase 4b: Update System Entity
1. In `spawn_system_entity`, add `ProgramOrchestrator` + `OrchestratorStatus`
2. Remove `ProgramExecutor` resource insertion

### Phase 4c: Migrate Execution Systems
1. Update `handle_load_program` to query `ProgramOrchestrator` component
2. Update `handle_start_program` to query component
3. Update `handle_pause_program`, `handle_resume_program`, `handle_stop_program`
4. Create `orchestrator_step_system` and `orchestrator_advance_system`
5. Add `ActiveRobot` to first connected robot

### Phase 4d: Update Client
1. Subscribe to `OrchestratorStatus` for program state
2. Subscribe to `RobotExecutionState` for per-robot state
3. Add UI to set active robot

### Phase 4e: Cleanup
1. Remove `ProgramExecutor` resource
2. Remove `ExecutionState` synced component (replaced by `OrchestratorStatus`)
3. Update all references

---

## 7. Open Items

### 7.1 Instruction Execution
- How does the orchestrator know when a motion completes?
- Current: Response channel subscription
- Proposed: Robot system updates `RobotExecutionState` when response received

### 7.2 Error Handling
- Robot error → `RobotExecState::Error`
- Orchestrator sees robot error → `OrchestratorState::Error`
- User must acknowledge/clear error before continuing

### 7.3 Multi-Robot Coordination (Future)
- Instructions may target specific robot by entity ID
- Parallel instructions: orchestrator waits for all targeted robots
- Sequence within parallel: per-robot step tracking

---

## 8. Component Sync Registration

```rust
// In server plugin setup
app.sync_component::<OrchestratorStatus, WebSocketProvider>();
app.sync_component::<RobotExecutionState, WebSocketProvider>();
```

