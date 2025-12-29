# Program & Execution Migration Plan

This document outlines the steps to migrate program management from `fanuc_replica_fanuc` to `fanuc_replica_programs` and eliminate legacy execution code from the fanuc crate.

## Current Architecture Overview

### Crate Responsibilities

| Crate | Current Responsibility |
|-------|----------------------|
| `fanuc_replica_programs` | New crate - types defined but handlers are placeholders |
| `fanuc_replica_fanuc` | Program CRUD handlers + legacy execution state + motion handlers |
| `fanuc_replica_execution` | Orchestrator, ToolpathBuffer, BufferState, DeviceStatus |

### Key Files to Modify

**In `fanuc_replica_fanuc`:**
- `src/handlers.rs` - Contains program CRUD + execution handlers (3124 lines)
- `src/types.rs` - Contains program types (2098 lines)
- `src/program.rs` - Legacy Program component, ProgramState, ProgramDefaults (144 lines)
- `src/motion.rs` - Motion handler + legacy helpers (682 lines)
- `src/database/schema.rs` - Programs table schema
- `src/database/queries.rs` - Program CRUD queries

**In `fanuc_replica_programs`:**
- `src/handlers.rs` - Placeholder (12 lines)
- `src/database.rs` - Placeholder (13 lines)
- `src/types.rs` - New sequence-based types (314 lines)

## Migration Tasks

### Phase 1: Database Schema (New Fresh Start)

**Goal:** Replace flat program structure with sequence-based structure.

**Database Changes:**
1. Delete existing database file (user will do this manually)
2. Create new schema in `fanuc_replica_programs/src/database.rs`:

```sql
-- Programs table (simplified - no start/end positions)
CREATE TABLE IF NOT EXISTS programs (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    default_speed REAL,
    default_term_type TEXT DEFAULT 'CNT',
    default_term_value INTEGER DEFAULT 100,
    move_speed REAL NOT NULL DEFAULT 100.0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Sequences table (approach, main, retreat)
CREATE TABLE IF NOT EXISTS program_sequences (
    id INTEGER PRIMARY KEY,
    program_id INTEGER NOT NULL,
    sequence_type TEXT NOT NULL, -- 'approach', 'main', 'retreat'
    name TEXT,
    order_index INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (program_id) REFERENCES programs(id) ON DELETE CASCADE
);

-- Instructions table (references sequence, not program directly)
CREATE TABLE IF NOT EXISTS sequence_instructions (
    id INTEGER PRIMARY KEY,
    sequence_id INTEGER NOT NULL,
    line_number INTEGER NOT NULL,
    x REAL NOT NULL,
    y REAL NOT NULL,
    z REAL NOT NULL,
    w REAL,
    p REAL,
    r REAL,
    ext1 REAL,
    ext2 REAL,
    ext3 REAL,
    speed REAL,
    term_type TEXT,
    term_value INTEGER,
    FOREIGN KEY (sequence_id) REFERENCES program_sequences(id) ON DELETE CASCADE
);
```

**Files to Create/Modify:**
- `programs/src/database.rs` - Implement `ProgramsDatabaseInit` with schema above
- `programs/Cargo.toml` - Add `rusqlite` dependency (already done, just needs feature gate)

### Phase 2: Program CRUD Handlers Migration

**Goal:** Move all program CRUD from fanuc to programs crate.

**Handlers to Move (from `fanuc/src/handlers.rs`):**

| Handler | Lines | Notes |
|---------|-------|-------|
| `handle_list_programs` | 706-753 | Convert to return `Vec<ProgramInfo>` |
| `handle_get_program` | 757-777 | Load sequences from new schema |
| `handle_create_program` | 781-821 | Create program + main sequence |
| `handle_delete_program` | 825-863 | Cascade deletes sequences |
| `handle_update_program_settings` | 867-931 | Simplified - no start/end positions |
| `handle_upload_csv` | 935-975 | Upload to specified sequence |
| `parse_and_insert_csv` | 979-1054 | Move to programs crate |

**New Handlers to Add:**
- `handle_add_sequence` - Add approach/retreat sequence to program
- `handle_remove_sequence` - Remove a sequence from program

**Files to Modify:**
- `programs/src/handlers.rs` - Implement all handlers
- `programs/src/lib.rs` - Register handlers in ProgramsPlugin
- `fanuc/src/handlers.rs` - Remove CRUD handlers (keep execution handlers only)

### Phase 3: Execution State Cleanup

**Goal:** Remove legacy execution state from fanuc crate. Confirm these are no longer being registered/used in the fanuc_rmi_replica_plugins actual compiled application ad anything that is still being used migrate to the execution plugin (should be complete but need to confirm)

**Components to Remove from `fanuc/src/program.rs`:**
- `Program` component - Legacy, replaced by ToolpathBuffer
- `ProgramState` enum - Legacy, replaced by BufferState
- `ProgramDefaults` - Move to programs crate if needed

**Components to Remove from `fanuc/src/types.rs`:**
- `ProgramInfo` (lines 598-606) - Use programs crate version
- `ProgramDetail` (lines 609-641) - Use programs crate version
- `Instruction` (lines 644-658) - Use programs crate version

**Files to Modify:**
- `fanuc/src/program.rs` - Delete file entirely
- `fanuc/src/types.rs` - Remove program types
- `fanuc/src/lib.rs` - Remove `mod program`
- `fanuc/src/motion.rs` - Update to use programs crate types

### Phase 4: Plugin Registration Updates

**Goal:** Ensure proper plugin loading order and dependencies.

**Current Plugin Order (in main.rs):**
```rust
app.add_plugins((
    ExecutionPlugin,  // Orchestrator, BufferState
    FanucPlugin,      // Motion handlers, connection
    DuetPlugin,       // Duet device handlers
));
```

**Required Changes:**
1. Add `ProgramsPlugin` to the plugin list
2. Register `ProgramsDatabaseInit` in the database registry
3. Ensure programs plugin loads before execution plugin (for type availability)

**New Plugin Order:**
```rust
app.add_plugins((
    ProgramsPlugin,   // Program CRUD, database schema
    ExecutionPlugin,  // Orchestrator, BufferState
    FanucPlugin,      // Motion handlers, connection
    DuetPlugin,       // Duet device handlers
));
```

### Phase 5: Type Mapping for Execution

**Goal:** Map programs crate types to execution crate types.

**Key Mappings:**
- `programs::Instruction` → `execution::ToolpathInstruction`
- `programs::ProgramDetail` → Load into `execution::ToolpathBuffer`
- `programs::SequenceType` → Determines instruction ordering

**Conversion Function (in execution crate):**
```rust
impl From<programs::Instruction> for ToolpathInstruction {
    fn from(instr: programs::Instruction) -> Self {
        ToolpathInstruction {
            x: instr.x,
            y: instr.y,
            z: instr.z,
            w: instr.w.unwrap_or(0.0),
            p: instr.p.unwrap_or(0.0),
            r: instr.r.unwrap_or(0.0),
            speed: instr.speed,
            term_type: instr.term_type,
            term_value: instr.term_value,
        }
    }
}
```

## Dependency Graph

```
fanuc_replica_programs
    ├── types (Instruction, ProgramDetail, SequenceType)
    ├── database (schema, queries)
    └── handlers (CRUD operations)
           │
           ▼
fanuc_replica_execution
    ├── types (ToolpathBuffer, BufferState, DeviceStatus)
    ├── orchestrator (command dispatch)
    └── systems (state management)
           │
           ▼
fanuc_replica_fanuc
    ├── motion (FANUC-specific motion handlers)
    ├── connection (robot connection state)
    └── polling (position/status polling)
```

## Files Summary

### Files to Create
- `programs/src/database.rs` - Full implementation with schema + queries

### Files to Modify
- `programs/src/handlers.rs` - Implement all CRUD handlers
- `programs/src/lib.rs` - Register handlers, database init
- `programs/Cargo.toml` - Add dependencies (rusqlite, etc.)
- `fanuc/src/handlers.rs` - Remove program CRUD handlers
- `fanuc/src/types.rs` - Remove program types
- `fanuc/src/lib.rs` - Remove program module
- `execution/src/types.rs` - Add From impl for Instruction
- `main.rs` - Add ProgramsPlugin

### Files to Delete
- `fanuc/src/program.rs` - Legacy execution state

## Testing Strategy

1. **Unit Tests:** Add tests for new database queries in programs crate
2. **Integration Tests:** Test program CRUD through WebSocket API
3. **Manual Testing:**
   - Create program with approach/main/retreat sequences
   - Upload CSV to specific sequence
   - Execute program and verify instruction ordering

## Migration Checklist

- [ ] Delete existing database file
- [ ] Implement programs database schema
- [ ] Implement program CRUD handlers
- [ ] Register ProgramsPlugin
- [ ] Remove legacy types from fanuc crate
- [ ] Delete fanuc/src/program.rs
- [ ] Update execution crate with type conversions
- [ ] Test full program lifecycle

