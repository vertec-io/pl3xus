# Coordinate Abstraction Architecture

## Current State (NOT using abstractions)

The orchestrator currently bypasses the abstraction layer:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           CURRENT ARCHITECTURE                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────┐     ┌──────────────────┐     ┌─────────────────────────┐  │
│  │   Database   │────▶│   Instruction    │────▶│  build_motion_packet()  │  │
│  │  (x,y,z,w,p,r)│     │  (x,y,z,w,p,r)   │     │  Creates FANUC Position │  │
│  └──────────────┘     └──────────────────┘     └───────────┬─────────────┘  │
│                                                            │                 │
│                                                            ▼                 │
│                                               ┌─────────────────────────┐   │
│                                               │   FrcLinearMotion       │   │
│                                               │   (FANUC SendPacket)    │   │
│                                               └───────────┬─────────────┘   │
│                                                           │                  │
│                                                           ▼                  │
│                                               ┌─────────────────────────┐   │
│                                               │     FANUC Driver        │   │
│                                               └─────────────────────────┘   │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Problems with current approach:**
1. FANUC-specific types are used throughout the entire pipeline
2. No robot abstraction - can't swap to ABB, UR, etc.
3. Euler angles stored everywhere (gimbal lock risk for toolpath interpolation)
4. Database schema is FANUC-specific

---

## Target Architecture (Multi-Robot Support)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           TARGET ARCHITECTURE                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │                      ROBOT-AGNOSTIC LAYER                               │ │
│  │                                                                         │ │
│  │  ┌──────────────┐     ┌──────────────────┐     ┌───────────────────┐   │ │
│  │  │   Database   │────▶│  ToolpathPoint   │────▶│   Orchestrator    │   │ │
│  │  │ (Isometry3)  │     │ (RobotPose+speed)│     │ (interpolation,   │   │ │
│  │  │              │     │                  │     │  frame transforms)│   │ │
│  │  └──────────────┘     └──────────────────┘     └─────────┬─────────┘   │ │
│  │                                                          │              │ │
│  └──────────────────────────────────────────────────────────┼──────────────┘ │
│                                                             │                │
│  ═══════════════════════ CONVERSION BOUNDARY ═══════════════╪════════════   │
│                                                             │                │
│  ┌──────────────────────────────────────────────────────────┼──────────────┐ │
│  │                      DRIVER LAYER                        │              │ │
│  │                                                          ▼              │ │
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────┐ │ │
│  │  │  FanucDriver    │  │   ABBDriver     │  │     URDriver            │ │ │
│  │  │                 │  │                 │  │                         │ │ │
│  │  │ to_fanuc_pos()  │  │ to_rapid_pos()  │  │ to_ur_script_pos()      │ │ │
│  │  │ Euler ZYX (WPR) │  │ Quaternion      │  │ Axis-Angle              │ │ │
│  │  └────────┬────────┘  └────────┬────────┘  └───────────┬─────────────┘ │ │
│  │           │                    │                       │               │ │
│  └───────────┼────────────────────┼───────────────────────┼───────────────┘ │
│              ▼                    ▼                       ▼                 │
│         ┌─────────┐          ┌─────────┐            ┌─────────┐            │
│         │  FANUC  │          │   ABB   │            │   UR    │            │
│         │  Robot  │          │  Robot  │            │  Robot  │            │
│         └─────────┘          └─────────┘            └─────────┘            │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Conversion Boundary

**The key principle: Convert at the driver layer, not before.**

| Layer | Type Used | Rationale |
|-------|-----------|-----------|
| Database | `Isometry3` (serialized) | Robot-agnostic, no precision loss |
| Orchestrator | `RobotPose`, `ToolpathPoint` | Enables interpolation, frame transforms |
| Driver | Vendor-specific | Only the driver knows vendor format |

---

## Database Storage Options

### Option A: Store Isometry3 (Recommended for new projects)

```sql
CREATE TABLE toolpath_points (
    id INTEGER PRIMARY KEY,
    program_id INTEGER,
    line_number INTEGER,
    -- Isometry3 stored as quaternion + translation (7 values)
    tx REAL, ty REAL, tz REAL,           -- Translation
    qx REAL, qy REAL, qz REAL, qw REAL,  -- Quaternion (normalized)
    -- Motion parameters
    speed REAL,
    term_type TEXT,
    term_value INTEGER,
    frame_id TEXT,  -- "world", "uframe_1", etc.
    -- Optional vendor hints (for arm config, etc.)
    vendor_config JSON
);
```

**Pros:**
- Robot-agnostic from the start
- No Euler angle ambiguity
- Easy to add new robot types

**Cons:**
- Requires migration of existing data
- UI needs to display/edit as Euler (user expectation)

### Option B: Store Euler + Convert on Load (Pragmatic for migration)

```sql
-- Keep existing schema
CREATE TABLE instructions (
    id INTEGER PRIMARY KEY,
    program_id INTEGER,
    line_number INTEGER,
    x REAL, y REAL, z REAL,
    w REAL, p REAL, r REAL,  -- Euler angles (degrees)
    ...
);
```

Then convert when loading:
```rust
fn load_program(program_id: i64) -> Vec<ToolpathPoint> {
    let instructions = db.query_instructions(program_id);
    instructions.into_iter()
        .map(|instr| instr.to_robot_pose(FrameId::UserFrame(instr.uframe)))
        .collect()
}
```

**Pros:**
- No database migration needed
- Existing programs keep working
- UI stays the same

**Cons:**
- Still storing vendor-specific format
- Conversion on every load

---

## Recommended Migration Path

### Phase 1: Current (Done)
- Created `fanuc_replica_robotics` crate with types
- Added conversion utilities to `types.rs`
- Orchestrator still uses old path (no breaking changes)

### Phase 2: Orchestrator Refactor (TODO)
- Refactor `build_motion_packet()` to use `RobotPose`
- Add `RobotDriver` trait with vendor implementations
- Keep database schema unchanged (Option B)

### Phase 3: Multi-Robot Support (Future)
- Add `RobotDriver` implementations for ABB, UR, etc.
- Add robot type selection in UI
- Consider database schema migration (Option A)

---

## Open Questions

1. **Should we refactor the orchestrator now?** 
   - Pro: Cleaner architecture, enables future multi-robot
   - Con: More work, no immediate benefit for single-robot use

2. **When to migrate database schema?**
   - When we actually add a second robot type
   - Or: Never, if Euler storage is acceptable

3. **How to handle vendor-specific features?**
   - Arm configuration (FANUC-specific)
   - External axes (varies by robot)
   - Speed units (mm/sec, %, etc.)

