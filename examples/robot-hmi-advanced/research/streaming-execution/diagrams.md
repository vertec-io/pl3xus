# Streaming Execution Diagrams

This document contains all diagrams for the streaming execution architecture.

## 1. Execution Modes Comparison

```mermaid
stateDiagram-v2
    direction LR
    
    state "Static Program" as Static {
        [*] --> Loaded: Load all points
        Loaded --> Executing: Start
        note right of Loaded: sealed=true, expected_total=Some(N)
        Executing --> Complete: buffer empty + confirmed >= total
    }
    
    state "Streaming Execution" as Streaming {
        [*] --> Generating: Start with initial points
        Generating --> AwaitingPoints: buffer low/empty
        note right of Generating: sealed=false, expected_total=None
        AwaitingPoints --> Generating: new points arrive
        Generating --> Draining: producer.seal()
        note right of Draining: sealed=true, expected_total=total_added
        Draining --> Complete: buffer empty + confirmed >= total
    }
```

## 2. BufferState State Machine (Full)

```mermaid
stateDiagram-v2
    direction TB
    
    [*] --> Idle
    
    Idle --> Buffering: points received
    Idle --> Ready: LoadProgram (static)
    
    Buffering --> Ready: min threshold reached
    
    Ready --> Executing: StartProgram
    
    Executing --> AwaitingPoints: buffer empty, NOT sealed
    note right of AwaitingPoints: Streaming only
    
    AwaitingPoints --> Executing: points added
    
    Executing --> Complete: buffer empty + sealed + confirmed
    note right of Complete: Normal completion
    
    Executing --> Stopped: StopProgram (user)
    AwaitingPoints --> Stopped: StopProgram (user)
    Executing --> Paused: PauseProgram
    note right of Stopped: User-initiated stop
    
    Paused --> Executing: ResumeProgram
    Paused --> Stopped: StopProgram
    
    Executing --> WaitingForFeedback: wait condition
    WaitingForFeedback --> Executing: condition met
    WaitingForFeedback --> Stopped: StopProgram
    
    Executing --> Error: device error
    AwaitingPoints --> Error: timeout / producer error
    note right of Error: Error state
    
    Complete --> Idle: Reset / Disconnect
    Stopped --> Idle: Reset / Disconnect
    Error --> Idle: Reset / Disconnect
```

## 3. Sealed Buffer Lifecycle

```mermaid
flowchart TB
    subgraph static["Static Program Flow"]
        S1["new_static(100)"] --> S2["sealed=true\nexpected=Some(100)"]
        S2 --> S3["push(points...)"]
        S3 --> S4["Executing"]
        S4 --> S5["drain buffer"]
        S5 --> S6["Complete\n(when confirmed=100)"]
    end
    
    subgraph streaming["Streaming Flow"]
        T1["new_streaming()"] --> T2["sealed=false\nexpected=None"]
        T2 --> T3["push(batch1)"]
        T3 --> T4["Executing"]
        T4 --> T5["push(batch2)"]
        T5 --> T6["...more batches..."]
        T6 --> T7["seal()"]
        T7 --> T8["sealed=true\nexpected=Some(total_added)"]
        T8 --> T9["drain remaining"]
        T9 --> T10["Complete"]
    end
    
    style static fill:#e8f5e9,stroke:#2e7d32
    style streaming fill:#e3f2fd,stroke:#1565c0
```

## 4. Completion Logic Decision Tree

```mermaid
flowchart TD
    Start["Check Completion"]
    
    Sealed{"buffer.is_sealed()?"}
    Empty{"buffer.is_empty()?"}
    Confirmed{"completed >= total_added?"}
    
    Complete["→ BufferState::Complete"]
    Streaming{"Was streaming?"}
    Await["→ BufferState::AwaitingPoints"]
    Continue["Continue Executing"]
    
    Start --> Sealed
    Sealed -->|No| Streaming
    Sealed -->|Yes| Empty
    
    Streaming -->|Yes| Empty
    Streaming -->|No| Continue
    
    Empty -->|No| Continue
    Empty -->|Yes| Confirmed
    
    Confirmed -->|Yes| Complete
    Confirmed -->|No| Await
    
    style Complete fill:#c8e6c9,stroke:#2e7d32
    style Await fill:#fff3e0,stroke:#ef6c00
    style Continue fill:#e3f2fd,stroke:#1565c0
```

## 5. Notification Flow

```mermaid
sequenceDiagram
    participant User
    participant Handler
    participant BufferState
    participant NotificationSystem
    participant Client
    
    Note over User,Client: Completion Flow
    BufferState->>BufferState: buffer empty + sealed + confirmed
    BufferState->>NotificationSystem: Transition to Complete
    NotificationSystem->>Client: ProgramNotification::Completed
    NotificationSystem->>Client: ConsoleLogEntry (status)
    
    Note over User,Client: Stop Flow
    User->>Handler: StopProgram request
    Handler->>BufferState: Set Stopped state
    BufferState->>NotificationSystem: Transition to Stopped
    NotificationSystem->>Client: ProgramNotification::Stopped
    NotificationSystem->>Client: ConsoleLogEntry (info)
    
    Note over User,Client: Error Flow
    BufferState->>BufferState: device error detected
    BufferState->>NotificationSystem: Transition to Error
    NotificationSystem->>Client: ProgramNotification::Error
    NotificationSystem->>Client: ConsoleLogEntry (error)
```

## 6. Progress Display Logic

```mermaid
flowchart TD
    Start["Get Progress"]

    HasExpected{"expected_total.is_some()?"}

    Determinate["Determinate Progress\n━━━━━━━━━━━━━━━\ncompleted: N\ntotal: M\npercent: N/M * 100"]

    Indeterminate["Indeterminate Progress\n━━━━━━━━━━━━━━━━━\ncompleted: N\n(no percentage)"]

    UIDetBar["UI: Progress Bar\n42/100 (42%)"]
    UIIndetBar["UI: Spinner/Pulse\n42 processed..."]

    Start --> HasExpected
    HasExpected -->|Yes| Determinate
    HasExpected -->|No| Indeterminate

    Determinate --> UIDetBar
    Indeterminate --> UIIndetBar

    style Determinate fill:#c8e6c9,stroke:#2e7d32
    style Indeterminate fill:#fff3e0,stroke:#ef6c00
```

## 7. System Architecture Overview

```mermaid
flowchart TB
    subgraph producers["Path Producers"]
        Static["Static Loader\n(from DB)"]
        Stream["Streaming Importer\n(from file/network)"]
        Realtime["Realtime Generator\n(from sensors)"]
    end

    subgraph buffer["ToolpathBuffer"]
        direction TB
        Points["VecDeque<ExecutionPoint>"]
        State["BufferState"]
        Sealed["sealed: bool"]
        Expected["expected_total: Option<u32>"]
    end

    subgraph orchestrator["Orchestrator"]
        Consume["Consume from buffer"]
        Emit["Emit MotionCommandEvent"]
    end

    subgraph motion["FANUC Motion Chain"]
        Handler["fanuc_motion_handler"]
        Sent["fanuc_sent_instruction"]
        Response["fanuc_motion_response"]
        Sync["sync_device_status_to_buffer"]
    end

    subgraph notifications["Notifications"]
        Complete["Complete"]
        Stopped["Stopped"]
        Error["Error"]
    end

    Static --> Points
    Stream --> Points
    Realtime --> Points

    Stream -.->|"seal()"| Sealed
    Realtime -.->|"seal()"| Sealed

    Points --> Consume
    State --> Consume
    Consume --> Emit

    Emit --> Handler
    Handler --> Sent
    Sent --> Response
    Response --> Sync

    Sync --> State
    State -.->|"on transition"| notifications

    style producers fill:#e1f5fe,stroke:#01579b
    style buffer fill:#e8f5e9,stroke:#2e7d32
    style orchestrator fill:#fff3e0,stroke:#ef6c00
    style motion fill:#f3e5f5,stroke:#7b1fa2
    style notifications fill:#ffcdd2,stroke:#c62828
```

## 8. AwaitingPoints State Detail

```mermaid
stateDiagram-v2
    direction LR

    Executing --> AwaitingPoints: buffer.is_empty() && !buffer.is_sealed()

    state AwaitingPoints {
        [*] --> Waiting
        Waiting --> Timeout: timeout_seconds elapsed
        Waiting --> PointsReceived: push(points)
    }

    AwaitingPoints --> Executing: points added
    AwaitingPoints --> Error: timeout (optional)
    AwaitingPoints --> Stopped: user stop
```

---

*End of Diagrams Document*

