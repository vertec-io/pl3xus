# Execution Plugin Diagrams

This file contains Mermaid diagram source code for the execution plugin architecture.

## 1. Plugin Dependency Graph

```mermaid
flowchart TB
    subgraph app["Application Layer"]
        App["Application\n(composes plugins,\nconfigures hierarchy)"]
    end
    
    subgraph exec["Execution Layer"]
        ExecPlugin["execution_plugin\n━━━━━━━━━━━━━━\nToolpathBuffer\nOrchestrator\nDevice Traits"]
    end
    
    subgraph devices["Device Layer"]
        Fanuc["fanuc_plugin\nimpl MotionDevice"]
        ABB["abb_plugin\nimpl MotionDevice"]
        Extruder["extruder_plugin\nimpl AuxiliaryDevice"]
        Sensor["sensor_plugin\nimpl FeedbackSource"]
    end
    
    subgraph base["Base Layer"]
        Core["core\n━━━━━━━━━━━━━━\nECS, Events\nCoordinate Types\nCommon Utils"]
    end
    
    App --> ExecPlugin
    App --> Fanuc
    App --> ABB
    App --> Extruder
    App --> Sensor
    
    ExecPlugin --> Core
    Fanuc --> ExecPlugin
    Fanuc --> Core
    ABB --> ExecPlugin
    ABB --> Core
    Extruder --> ExecPlugin
    Extruder --> Core
    Sensor --> ExecPlugin
    Sensor --> Core
    
    style app fill:#e8f5e9,stroke:#2e7d32
    style exec fill:#e3f2fd,stroke:#1565c0
    style devices fill:#fff3e0,stroke:#ef6c00
    style base fill:#f3e5f5,stroke:#7b1fa2
```

## 2. Entity Hierarchy Examples

```mermaid
flowchart TB
    subgraph single["Single Robot"]
        S1["System"]
        R1["Robot\n[ExecutionCoordinator]\n[ToolpathBuffer]"]
        E1["Extruder\n[AuxiliaryDevice]"]
        Sen1["Sensor\n[FeedbackSource]"]
        S1 --> R1
        R1 --> E1
        R1 --> Sen1
    end
    
    subgraph independent["Multi-Robot Independent"]
        S2["System"]
        R2a["Robot1\n[ExecutionCoordinator]\n[ToolpathBuffer]"]
        R2b["Robot2\n[ExecutionCoordinator]\n[ToolpathBuffer]"]
        Sen2a["Sensor1"]
        Sen2b["Sensor2"]
        S2 --> R2a
        S2 --> R2b
        R2a --> Sen2a
        R2b --> Sen2b
    end
    
    subgraph coordinated["Multi-Robot Coordinated"]
        S3["System\n[ExecutionCoordinator]\n[ToolpathBuffer]"]
        R3a["Robot1\n[MotionDevice]"]
        R3b["Robot2\n[MotionDevice]"]
        Cam["CellCamera\n[FeedbackSource]"]
        S3 --> R3a
        S3 --> R3b
        S3 --> Cam
    end
    
    style single fill:#e8f5e9,stroke:#2e7d32
    style independent fill:#e3f2fd,stroke:#1565c0
    style coordinated fill:#fff3e0,stroke:#ef6c00
```

## 3. Data Flow with Feedback Loop

```mermaid
flowchart LR
    subgraph producers["Path Producers"]
        Static["Static Loader"]
        GCode["G-Code Parser"]
        Realtime["Real-time\nGenerator"]
    end
    
    Buffer["ToolpathBuffer\n━━━━━━━━━━━\npoints[]\ncurrent_index\ncompleted_count\nstate"]
    
    Orch["Orchestrator\n━━━━━━━━━\nConsumes buffer\nManages timing\nCoordinates devices"]
    
    subgraph devices["Devices"]
        Motion["MotionDevice\n(Robot)"]
        Aux["AuxiliaryDevice\n(Extruder/IO)"]
    end
    
    subgraph feedback["Feedback"]
        Sensors["FeedbackSource\n(Sensors)"]
    end
    
    Static --> Buffer
    GCode --> Buffer
    Realtime --> Buffer
    
    Buffer --> Orch
    Orch --> Motion
    Orch --> Aux
    
    Sensors --> Orch
    Sensors -.->|"modifies upcoming"| Buffer
    
    style producers fill:#c8e6c9,stroke:#2e7d32
    style Buffer fill:#bbdefb,stroke:#1565c0
    style Orch fill:#ffe0b2,stroke:#ef6c00
    style devices fill:#f3e5f5,stroke:#7b1fa2
    style feedback fill:#ffcdd2,stroke:#c62828
```

## 4. Buffer-Based Toolpath Architecture (Full System)

```mermaid
flowchart TB
    subgraph sources["Data Sources"]
        direction LR
        GCode["G-Code\n(Euler: A,B,C)"]
        Slicer["Slicer Output\n(Euler: W,P,R)"]
        Native["Native Generator\n(Quaternion)"]
        Sensor["Sensor Feedback\n(Runtime)"]
    end
    
    subgraph import["Import Layer"]
        Convert["euler_to_quaternion()"]
    end
    
    subgraph storage["Database (Quaternion Storage)"]
        DB[("toolpath_points\ntx,ty,tz,qw,qx,qy,qz")]
    end
    
    subgraph runtime["Runtime (ECS)"]
        Buffer["ToolpathBuffer\nVecDeque<ExecutionPoint>"]
        Orch["Orchestrator System\n(consumer)"]
        
        subgraph prods["Producer Systems"]
            Loader["Static Program Loader"]
            Stream["Streaming Importer"]
            RealTime["Real-time Generator"]
        end
    end
    
    subgraph driver["Driver Layer (Conversion Boundary)"]
        direction LR
        Fanuc["FanucDriver\nquat→WPR"]
        ABB["ABBDriver\nquat→quat"]
        UR["URDriver\nquat→axis-angle"]
    end
    
    Robot["Robot Hardware"]
    
    GCode --> Convert
    Slicer --> Convert
    Convert --> DB
    Native --> DB
    
    DB --> Loader
    Sensor --> RealTime
    
    Loader --> Buffer
    Stream --> Buffer
    RealTime --> Buffer
    
    Buffer --> Orch
    Orch --> driver
    driver --> Robot
    
    style sources fill:#e1f5fe,stroke:#01579b
    style import fill:#fff3e0,stroke:#e65100
    style storage fill:#f3e5f5,stroke:#7b1fa2
    style runtime fill:#e8f5e9,stroke:#2e7d32
    style driver fill:#ffebee,stroke:#c62828
```

## 5. Execution States

```mermaid
stateDiagram-v2
    [*] --> Idle
    Idle --> Buffering : points received
    Buffering --> Ready : min buffer reached
    Ready --> Executing : start command
    Executing --> Paused : pause command
    Paused --> Executing : resume command
    Executing --> WaitingForFeedback : wait condition
    WaitingForFeedback --> Executing : condition met
    Executing --> Complete : buffer empty + expected_total reached
    Executing --> Error : device error
    Paused --> Idle : abort command
    Error --> Idle : reset
    Complete --> Idle : reset
```

## 6. Component Relationships (ECS)

```mermaid
erDiagram
    Entity ||--o{ ExecutionCoordinator : has
    Entity ||--o{ ToolpathBuffer : has
    Entity ||--o{ BufferState : has
    Entity ||--o{ ExecutionTarget : has
    Entity ||--o{ FeedbackProvider : has
    Entity ||--o{ PrimaryMotion : has

    ExecutionCoordinator ||--|| ToolpathBuffer : "requires"
    ExecutionCoordinator ||--|| BufferState : "requires"

    ToolpathBuffer ||--|{ ExecutionPoint : contains

    ExecutionPoint ||--o| MotionCommand : has
    ExecutionPoint ||--|{ AuxiliaryCommand : has

    MotionCommand ||--|| RobotPose : contains

    ExecutionTarget }|--|| MotionDevice : "impl trait"
    ExecutionTarget }|--|| AuxiliaryDevice : "impl trait"
    FeedbackProvider }|--|| FeedbackSource : "impl trait"
```

## 7. Orchestrator System Flow

```mermaid
flowchart TD
    Start["System Tick"]

    Query["Query entities with\nExecutionCoordinator +\nToolpathBuffer + BufferState"]

    CheckState{"BufferState\n== Executing?"}

    ReadyCheck{"Device\nready_for_next()?"}

    PopBuffer["Pop ExecutionPoint\nfrom ToolpathBuffer"]

    UpdateIndex["Set BufferState.current_index"]

    QueryTargets["Query child entities with\nExecutionTarget component"]

    SendMotion["Call MotionDevice::send_motion()\non PrimaryMotion entity"]

    SendAux["Call AuxiliaryDevice::send_command()\non other ExecutionTarget entities"]

    UpdateCompleted["Increment completed_count\nwhen motion confirms done"]

    CheckComplete{"Buffer empty &&\ncompleted == expected?"}

    SetComplete["Set state = Complete"]

    End["End Tick"]

    Start --> Query
    Query --> CheckState
    CheckState -->|No| End
    CheckState -->|Yes| ReadyCheck
    ReadyCheck -->|No| End
    ReadyCheck -->|Yes| PopBuffer
    PopBuffer --> UpdateIndex
    UpdateIndex --> QueryTargets
    QueryTargets --> SendMotion
    SendMotion --> SendAux
    SendAux --> UpdateCompleted
    UpdateCompleted --> CheckComplete
    CheckComplete -->|No| End
    CheckComplete -->|Yes| SetComplete
    SetComplete --> End
```

