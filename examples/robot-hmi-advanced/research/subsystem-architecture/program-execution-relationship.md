# What Is a Program? How Should Execution Be Tracked?

## The Core Question

Is "Execution" the execution of a **program**, or the execution of a **buffer**?
Is a "program" something you execute, or something that **feeds** execution?

## Current Mental Models

### Model A: Program-Centric Execution
```
┌──────────────────────────────────────────────────────────┐
│                        Program                            │
│  ┌─────────────────────────────────────────────────────┐ │
│  │ Line 1: Move to (100, 200, 50)                      │ │
│  │ Line 2: Extrude at rate 5                           │ │
│  │ Line 3: Move to (150, 200, 50)         ◄── current  │ │
│  │ Line 4: ...                                         │ │
│  └─────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────┘
                           │
                           │ Execution IS the program
                           ▼
                    ┌──────────────┐
                    │   Executor   │
                    └──────────────┘
```

- Execution state = where are we in the program
- Progress = line 3 of 1000
- Pause/Resume = pause at current line

### Model B: Buffer-Centric Execution
```
┌─────────────────┐      ┌─────────────────┐
│ Static Program  │──┐   │ Dynamic Generator│
│ (CSV file)      │  │   │ (algorithm)      │
└─────────────────┘  │   └────────┬─────────┘
                     │            │
                     ▼            ▼
              ┌──────────────────────────────┐
              │          BUFFER              │ ◄── This is what we execute
              │  ┌─────────────────────────┐ │
              │  │ Point 1  Point 2  ...   │ │
              │  └─────────────────────────┘ │
              └──────────────────────────────┘
                           │
                           ▼
                    ┌──────────────┐
                    │   Executor   │
                    └──────────────┘
```

- Execution state = buffer state (points remaining, executing, etc.)
- Progress = points executed, time elapsed
- Pause/Resume = pause buffer consumption
- "Program" is just one of many possible point sources

## What Could Feed the Buffer?

1. **Static Program (CSV/Database)**
   - Pre-computed points loaded into memory
   - Known total, can show "line 50 of 1000"

2. **Dynamic Generator (Procedural)**
   - Algorithmic generation (spirals, patterns)
   - May or may not know total upfront

3. **Real-time Slicer**
   - Slicing happens during execution
   - Points generated on-demand

4. **Vision-Guided Path**
   - Camera/sensor input determines next points
   - Truly dynamic, no predetermined total

5. **Manual Teaching**
   - Operator provides points in real-time
   - Recording mode

6. **Hybrid Sources**
   - Base program + dynamic modifications
   - Program with conditional branches

## The "Current Line" Problem

With Model A (program-centric), we can show:
- "Line 50 of 1000"
- "Sequence 2: Infill"
- "25% complete"

With Model B (buffer-centric) + dynamic sources:
- Points executed: 50
- Points remaining: ??? (unknown for dynamic)
- Time elapsed: 5:32

**Question**: Is "current line" a program concept or an execution concept?

I think it's a PROGRAM concept. The execution system shouldn't care about "lines" - 
it cares about "points to execute." The program (or source) knows about lines.

## Proposed Conceptual Split

```
┌─────────────────────────────────────────────────────────────────┐
│                         SOURCES                                  │
│                                                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐           │
│  │ LoadedProgram│  │  Generator   │  │  Manual Mode │           │
│  │ (static)     │  │  (dynamic)   │  │  (real-time) │           │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘           │
│         │                 │                 │                    │
│         │  Each source may have its own     │                    │
│         │  "progress" concept               │                    │
│         └─────────────────┼─────────────────┘                    │
└───────────────────────────┼──────────────────────────────────────┘
                            │
                            │ All sources produce ExecutionPoints
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│                         EXECUTION                                │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                    ExecutionBuffer                        │   │
│  │  Points waiting: 50   Points executed: 150                │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                    ExecutionState                         │   │
│  │  state: Running   can_pause: true   can_stop: true        │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

## What Each Layer Knows

### Execution Layer (execution plugin)
- Buffer state (points waiting, executing, etc.)
- Execution state (Running, Paused, Stopped)
- Points executed count
- Available actions
- **Does NOT know**: what's feeding it, "lines", "sequences", "program name"

### Source Layer (programs plugin, generator plugins, etc.)
- What is feeding the buffer
- Source-specific progress (line 50 of 1000, or "generating...")
- Source-specific display data (program lines for UI)
- **Does NOT know**: execution state, available actions

## Implications for UI

The UI needs BOTH:
1. Execution controls (Start, Pause, Stop) - from execution layer
2. Source display (program lines, current line highlight) - from source layer

```
┌─────────────────────────────────────────────────────────┐
│ Program: my_print.csv                    ← Source layer │
│ Line 50 of 1000  [===========----------]               │
├─────────────────────────────────────────────────────────┤
│ Status: Running                          ← Exec layer   │
│ Points executed: 150                                    │
│                                                         │
│ [Pause]  [Stop]                         ← Exec layer   │
└─────────────────────────────────────────────────────────┘
```

## Revisiting the Components

### ExecutionState (execution plugin, synced)
```rust
pub struct ExecutionState {
    pub state: ExecutionStatus,  // Running, Paused, Idle, etc.
    pub points_executed: usize,
    pub points_buffered: usize,
    pub can_start: bool,
    pub can_pause: bool,
    pub can_resume: bool,
    pub can_stop: bool,
}
```
Note: NO program info, NO "current line"

### SourceInfo (synced, owned by whoever is feeding)
```rust
// For static programs (programs plugin):
pub struct ProgramSourceInfo {
    pub program_id: i64,
    pub program_name: String,
    pub current_line: usize,
    pub total_lines: usize,
    pub program_lines: Vec<ProgramLineInfo>,
}

// For dynamic generators (hypothetical generator plugin):
pub struct GeneratorSourceInfo {
    pub generator_type: String,  // "spiral", "procedural", etc.
    pub points_generated: usize,
    pub estimated_remaining: Option<usize>,
}
```

## Key Questions to Answer

### 1. Should there be an "ActiveSource" abstraction?

If we have multiple source types (programs, generators, manual), should execution
know about a unified "source" concept?

**Option A**: No abstraction - execution just consumes points, doesn't care about source
- Pro: Simple, execution is completely decoupled
- Con: No way to show "what's feeding me" in execution state

**Option B**: Trait-based abstraction
```rust
trait ExecutionSource {
    fn next_points(&mut self, count: usize) -> Vec<ExecutionPoint>;
    fn is_complete(&self) -> bool;
    fn progress_hint(&self) -> Option<(usize, usize)>;  // (current, total)
}
```
- Pro: Unified interface
- Con: Trait objects, complexity

**Option C**: Enum-based source type in execution state
```rust
pub enum ActiveSourceType {
    None,
    Program { id: i64, name: String },
    Generator { name: String },
    Manual,
}
```
- Pro: Simple, informational only
- Con: Execution knows about sources (coupling)

### 2. Who owns "current line" during execution?

If a program is loaded and executing, who tracks "we're on line 50"?

**Option A**: Programs plugin
- Programs plugin has `current_line` field
- Execution plugin updates it via event or direct write
- Pro: Program concept stays in programs plugin
- Con: Execution plugin needs to notify programs plugin of progress

**Option B**: Execution plugin (as source metadata)
- Execution stores "source progress" generically
- Pro: Centralized tracking
- Con: Execution knows about program-specific concepts

**Option C**: Execution plugin sends events, programs plugin reacts
```rust
// Execution plugin emits:
pub struct PointExecuted { pub index: usize }

// Programs plugin handles:
fn update_current_line(
    mut events: EventReader<PointExecuted>,
    mut program_info: Query<&mut ProgramSourceInfo>,
) {
    // Map execution index → program line
}
```
- Pro: Decoupled via events
- Con: More complexity

### 3. What happens if the source changes mid-execution?

Can you:
- Switch programs while paused?
- Add a generator while running?
- Have multiple sources feeding simultaneously?

For now, probably: **No, one source at a time, must stop to switch.**

### 4. Does "Idle" mean "has source but not running" or "no source"?

Current model: Idle = program loaded but not running
Buffer-centric model: Could have sources connected but not feeding

**Proposed**:
- `NoSource` - nothing to execute
- `Ready` - source connected, ready to start
- `Running` - actively consuming points
- `Paused` - temporarily stopped, can resume
- `Completed` - source exhausted, execution done
- `Error` - something went wrong

## Revised State Model

```
                    ┌──────────────┐
                    │   NoSource   │◄──────────────────────────┐
                    └──────┬───────┘                           │
                           │ AttachSource (Load/Connect)       │ DetachSource
                           ▼                                   │
                    ┌──────────────┐                           │
         ┌─────────►│    Ready     │───────────────────────────┤
         │          └──────┬───────┘                           │
         │                 │ Start                             │
         │                 ▼                                   │
         │          ┌──────────────┐                           │
         │          │  Validating  │◄──────┐                   │
         │          └──────┬───────┘       │                   │
         │                 │               │ Resume            │
         │     ┌───────────┼───────────┐   │                   │
         │     │           │           │   │                   │
         │     ▼           ▼           ▼   │                   │
         │  ┌─────┐  ┌──────────┐  ┌───────┴───┐               │
         │  │Error│  │ Running  │  │  Paused   │               │
         │  └──┬──┘  └────┬─────┘  └─────┬─────┘               │
         │     │          │              │                     │
         │     │          │ Complete     │ Stop                │
         │     │          ▼              │                     │
         │     │   ┌──────────────┐      │                     │
         │     │   │  Completed   │      │                     │
         │     │   └──────┬───────┘      │                     │
         │     │          │              │                     │
         └─────┴──────────┴──────────────┘
                     (all return to Ready or can Detach)
```

Note: "Load" becomes "AttachSource" conceptually. A program is just one type of source.

## Connection to Streaming Execution Research

The `streaming-execution` research already introduces key concepts:

### From streaming-execution/README.md:

**Producer Types (already defined):**
1. **StaticLoader**: Loads all points from DB, seals immediately
2. **StreamingImporter**: Imports from external source, seals when source ends
3. **RealtimeGenerator**: Generates from sensor feedback, seals when algorithm decides

**Sealed Buffer Pattern:**
- Buffer tracks `sealed: bool` - whether more points can be added
- `expected_total: Option<u32>` - known for static, unknown until sealed for streaming
- Completion = sealed + buffer empty + all confirmed

This aligns perfectly with Model B (buffer-centric execution)!

### Connecting the Concepts

```
┌─────────────────────────────────────────────────────────────────┐
│                     PRODUCERS (Sources)                          │
│                                                                  │
│  ┌──────────────┐  ┌──────────────────┐  ┌──────────────────┐   │
│  │ StaticLoader │  │ StreamingImporter│  │ RealtimeGenerator│   │
│  │ (programs    │  │ (csv import,     │  │ (sensor-based,   │   │
│  │  plugin)     │  │  network stream) │  │  algorithm)      │   │
│  └──────┬───────┘  └────────┬─────────┘  └────────┬─────────┘   │
│         │                   │                     │              │
│         │ Each producer has its own               │              │
│         │ SourceInfo synced to client             │              │
│         └───────────────────┼─────────────────────┘              │
└─────────────────────────────┼────────────────────────────────────┘
                              │
                              │ All producers call buffer.push_points()
                              │ and eventually buffer.seal()
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    EXECUTION (buffer-centric)                    │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                    ToolpathBuffer                         │   │
│  │  sealed: bool     expected_total: Option<u32>             │   │
│  │  total_added: u32  points: VecDeque<ExecutionPoint>       │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                    ExecutionState                         │   │
│  │  state: Executing/Paused/AwaitingPoints/Complete/etc.     │   │
│  │  points_executed: u32   can_pause: bool   can_stop: bool  │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  Execution plugin does NOT know what producer is feeding it     │
└─────────────────────────────────────────────────────────────────┘
```

### What "Programs Plugin" Actually Is

Given this model, the "programs" plugin is really a **StaticLoader producer**.
It knows how to:
1. Load a program from the database
2. Convert it to ExecutionPoints
3. Push all points to the buffer and seal immediately
4. Track its own progress (current line, total lines)

Other producers would be their own plugins:
- "streaming" plugin for network/file streaming
- "generator" plugin for algorithmic generation

### Revised Component Ownership

| Component | Owner | Synced? | Notes |
|-----------|-------|---------|-------|
| `ToolpathBuffer` | execution | No | Internal buffer with sealed pattern |
| `ExecutionState` | execution | Yes | Running/Paused/etc, can_* actions |
| `ProgramSourceInfo` | programs | Yes | For static programs only |
| `StreamSourceInfo` | streaming | Yes | For streaming sources (future) |
| `GeneratorSourceInfo` | generator | Yes | For generators (future) |

### The ActiveSource Question Revisited

Given that multiple producer types exist, how does the UI know what's feeding execution?

**Option C seems best**: Execution knows the source TYPE but not details
```rust
pub struct ExecutionState {
    pub state: ExecutionStatus,
    pub source_type: SourceType,  // None, StaticProgram, Stream, Generator
    pub points_executed: u32,
    // ...
}

pub enum SourceType {
    None,
    StaticProgram,  // Look at ProgramSourceInfo for details
    Stream,         // Look at StreamSourceInfo for details
    Generator,      // Look at GeneratorSourceInfo for details
}
```

The UI can then:
1. Check `ExecutionState.source_type`
2. Query the appropriate *SourceInfo component for display details

## Revised Design Summary

### Execution Plugin Owns:
- `ToolpathBuffer` - internal, with sealed pattern
- `BufferState` - internal state machine
- `ExecutionState` - synced, with source_type hint
- Handlers: `Start`, `Pause`, `Resume`, `Stop`
- Subsystem infrastructure

### Programs Plugin Owns:
- `LoadedProgram` - internal execution data
- `ProgramSourceInfo` - synced display data for static programs
- Handlers: `Load`, `Unload`
- StaticLoader logic (convert program → points, push to buffer, seal)
- Programs subsystem validation

### Future Streaming Plugin Would Own:
- `StreamSource` - internal connection/import state
- `StreamSourceInfo` - synced display data
- Handlers: `StartStream`, `StopStream`
- StreamingImporter logic

## RESOLVED DECISIONS

### 1. Source type in ExecutionState
**RESOLVED**: Yes, `source_type` in ExecutionState is the right approach.

### 2. Programs as static loader
**RESOLVED**: Yes, implement now. Programs plugin is a StaticLoader that:
- Loads program from DB
- Converts to ExecutionPoints
- Pushes all points to buffer
- Seals immediately

### 3. Current line tracking
**RESOLVED**: Option B - Execution tracks generic `current_index`, UI maps to line.

Key insight: **The "program table" in the UI IS the execution buffer table.**
- UI syncs with execution buffer to show points
- `current_index` from ExecutionState determines highlighted row
- Programs plugin doesn't need to track "current line" at all!

**Benefits for future streaming/generators:**
- New points are added to buffer → automatically appear in UI table
- current_index advances → highlighting moves
- Same UI works for static programs, streams, and generators

## Final Component Design

### ExecutionState (execution plugin, synced to clients)
```rust
#[derive(Component, Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "stores", derive(Store))]
pub struct ExecutionState {
    /// Current execution state
    pub state: SystemState,

    /// What type of source is feeding the buffer
    pub source_type: SourceType,

    /// Source name for display (e.g., "my_print.csv")
    pub source_name: Option<String>,

    /// Current execution index (0-based)
    /// UI uses this to highlight the current row in the buffer table
    pub current_index: usize,

    /// Total points in buffer (if known)
    /// For static: known upfront. For streaming: grows as points added.
    pub total_points: Option<usize>,

    /// Points confirmed executed
    pub points_executed: usize,

    /// Available actions
    pub can_start: bool,
    pub can_pause: bool,
    pub can_resume: bool,
    pub can_stop: bool,
    pub can_load: bool,
    pub can_unload: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceType {
    #[default]
    None,
    StaticProgram,
    Stream,      // Future
    Generator,   // Future
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SystemState {
    #[default]
    NoSource,     // Nothing to execute
    Ready,        // Source loaded, ready to start
    Validating,   // Checking subsystems
    Running,      // Actively executing
    Paused,       // Paused by user
    AwaitingPoints, // Buffer empty, waiting for more (streaming)
    Completed,    // All points executed
    Stopped,      // Stopped by user
    Error,        // Error occurred
}
```

### BufferDisplayData (execution plugin, synced to clients)
```rust
/// Display data for the execution buffer table in UI
/// For static programs: synced once on load
/// For streaming: incrementally updated as points added
#[derive(Component, Clone, Debug, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "stores", derive(Store))]
pub struct BufferDisplayData {
    /// The lines/points to show in the UI table
    /// Each entry has display-friendly data (not full ExecutionPoint)
    pub lines: Vec<BufferLineDisplay>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BufferLineDisplay {
    pub index: usize,
    pub line_type: String,      // "Move", "Extrude", "Wait", etc.
    pub description: String,    // "Move to (100, 200, 50)"
    pub sequence_name: Option<String>,  // "Infill", "Perimeter", etc.
}
```

The UI then:
1. Renders `BufferDisplayData.lines` as the table
2. Uses `ExecutionState.current_index` to highlight the current row
3. Grays out rows where `index < points_executed`

### LoadedProgram (programs plugin, NOT synced)
```rust
/// Internal program data for execution - NOT synced to clients
/// Client sees BufferDisplayData instead
#[derive(Component, Clone, Debug)]
pub struct LoadedProgram {
    pub program_id: i64,
    pub program_name: String,
    pub sequences: Vec<LoadedSequence>,
}
```

### What Programs Plugin Does on Load

```rust
fn handle_load(
    mut commands: Commands,
    mut requests: MessageReader<Request<Load>>,
    system: Query<Entity, With<ActiveSystem>>,
    mut execution_state: Query<&mut ExecutionState, With<ActiveSystem>>,
    mut buffer_display: Query<&mut BufferDisplayData, With<ActiveSystem>>,
    mut toolpath_buffer: Query<&mut ToolpathBuffer, With<ActiveSystem>>,
    db: Res<DatabaseResource>,
) {
    for request in requests.read() {
        // 1. Load program from database
        let program = db.get_program(request.message.program_id)?;

        // 2. Convert to execution points
        let points: Vec<ExecutionPoint> = program.to_execution_points();
        let display_lines: Vec<BufferLineDisplay> = program.to_display_lines();

        // 3. Push ALL points to buffer and seal (static loader pattern)
        let mut buffer = toolpath_buffer.single_mut()?;
        buffer.clear();
        for point in &points {
            buffer.push(point.clone());
        }
        buffer.seal();  // Static program - all points known

        // 4. Update BufferDisplayData (synced to clients)
        let mut display = buffer_display.single_mut()?;
        display.lines = display_lines;

        // 5. Update ExecutionState (synced to clients)
        let mut exec = execution_state.single_mut()?;
        exec.source_type = SourceType::StaticProgram;
        exec.source_name = Some(program.name.clone());
        exec.total_points = Some(points.len());
        exec.state = SystemState::Ready;
        exec.current_index = 0;
        exec.points_executed = 0;
        exec.update_available_actions();

        // 6. Store internal LoadedProgram
        commands.entity(system_entity).insert(LoadedProgram { ... });
    }
}
```

## Updated Plugin Responsibilities

### Execution Plugin
- **Owns**: ToolpathBuffer, BufferState, ExecutionState, BufferDisplayData, Subsystems
- **Handlers**: Start, Pause, Resume, Stop
- **Systems**: Validation coordinator, state sync
- **Does NOT know**: What loaded the points, program database structure

### Programs Plugin (Static Loader)
- **Owns**: LoadedProgram (internal), database queries
- **Handlers**: Load, Unload
- **On Load**: Converts program → points + display lines, pushes to buffer, seals
- **On Unload**: Clears buffer, clears display data
- **Subsystem**: Validates "is program loaded" during Validating state

### Future Streaming Plugin
- **Owns**: StreamConnection, streaming logic
- **Handlers**: StartStream, StopStream
- **On data**: Pushes points to buffer, updates BufferDisplayData incrementally
- **On complete**: Calls buffer.seal()

## Next Steps

1. Update types.md with final component designs
2. Update implementation-plan.md to reflect:
   - Programs as static loader
   - BufferDisplayData component
   - Execution owning current_index
3. Update handler-ownership.md with Load handler logic

