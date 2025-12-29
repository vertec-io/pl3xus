# Research Projects

This folder contains research and architectural design documents for the pl3xus robotics platform.

## Active Research Projects

### 🔴 PRIORITY: Multi-Crate Plugin Refactor
**Status:** Half-Implemented, Needs Completion
**Location:** [`multi-crate-plugins/`](./multi-crate-plugins/)

Refactoring to Pattern 3 (Multi-Crate Plugins) per the pl3xus-project-structure skill.

**⚠️ CRITICAL:** See `multi-crate-plugins/HANDOFF.md` for:
- Current state (what's working vs dead code)
- What needs to be done
- Verification checklist

**Key issue:** New plugin crates were created and code was copied, but the code is NOT WIRED UP (89 dead code warnings). The working system is still in `plugins/src/core/` and `plugins/src/robot/`.

---

### 1. Streaming Execution & Sealed Buffer Pattern (NEW)
**Status:** Design Complete, Ready for Implementation
**Location:** [`streaming-execution/`](./streaming-execution/)

Extends the execution plugin to support **streaming/realtime execution** alongside static programs.

**Key Documents:**
- [`README.md`](./streaming-execution/README.md) - Complete architecture
- [`diagrams.md`](./streaming-execution/diagrams.md) - State machines and flow diagrams
- [`implementation_spec.md`](./streaming-execution/implementation_spec.md) - Detailed code changes
- [`current_state.md`](./streaming-execution/current_state.md) - What's done vs pending

**Key Innovation: Sealed Buffer Pattern**
- Static programs: `sealed=true` from start, known total, progress bar with %
- Streaming: `sealed=false`, producer calls `seal()` when done
- Unified completion logic for both modes

**New States:**
- `BufferState::Stopped` - User-initiated stop (distinct from error/complete)
- `BufferState::AwaitingPoints` - Buffer empty but expecting more (streaming)

---

### 2. Execution Plugin Architecture
**Status:** Partially Implemented
**Location:** [`execution-plugin/`](./execution-plugin/)

The foundational research project defining the buffer-based toolpath execution system.

**Key Documents:**
- [`README.md`](./execution-plugin/README.md) - Complete specification with:
  - Problem statement and vision
  - Architectural decisions (with rationale)
  - Component designs (ECS-native)
  - Plugin architecture (trait-based, no circular dependencies)
  - Database schema (quaternion storage)
  - Implementation phases (0-6)
  - Open questions
- [`diagrams.md`](./execution-plugin/diagrams.md) - Mermaid diagram sources

**Summary:**
- Store toolpaths in quaternion format (Isometry3) for mathematical consistency
- Use ToolpathBuffer component for flexible execution (static, streaming, real-time)
- ECS hierarchy IS the configuration for multi-robot/multi-device coordination
- Trait-based device abstraction (MotionDevice, AuxiliaryDevice, FeedbackSource)
- Device plugins never depend on each other

---

### 2. Coordinate Abstraction (Earlier Research)
**Status:** Superseded by Execution Plugin research  
**Location:** [`coordinate-abstraction/`](./coordinate-abstraction/)

Earlier exploration that led to the Execution Plugin architecture.

**Key Documents:**
- [`start_here.md`](./coordinate-abstraction/start_here.md) - Entry point
- [`current_state_analysis.md`](./coordinate-abstraction/current_state_analysis.md) - Codebase analysis
- [`industry_comparison.md`](./coordinate-abstraction/industry_comparison.md) - ABB, UR, KUKA comparison
- [`quaternion_to_euler.md`](./coordinate-abstraction/quaternion_to_euler.md) - Mathematical details
- [`buffer_architecture.md`](./coordinate-abstraction/buffer_architecture.md) - Early buffer design
- [`execution_architecture.md`](./coordinate-abstraction/execution_architecture.md) - Plugin exploration

**Note:** These documents contain valuable background but the final design is in `execution-plugin/`.

---

## Implementation Status

### Execution Plugin (from execution-plugin/ research)

| Phase | Description | Status |
|-------|-------------|--------|
| 0 | Create execution_plugin crate structure | ✅ Done |
| 1 | Database Schema Migration | ⏸️ Deferred |
| 2 | ToolpathBuffer Component | ✅ Basic impl done |
| 3 | Import Layer | ⏸️ Deferred |
| 4 | Orchestrator System | ✅ Done |
| 5 | FANUC Driver Integration | ✅ Done |
| 6 | End-to-End Testing | 🔄 In Progress |

### Streaming Execution (from streaming-execution/ research)

| Phase | Description | Status |
|-------|-------------|--------|
| 1 | Add Stopped state & notification | ✅ Done |
| 2 | Sealed buffer pattern | 🔴 TODO |
| 3 | AwaitingPoints state | 🔴 TODO |
| 4 | Update completion logic | 🔴 TODO |
| 5 | ExecutionProgress type | 🔴 TODO |
| 6 | UI updates for streaming | 🔴 TODO |

---

## Quick Reference: Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Internal coordinates | Quaternion (Isometry3) | No gimbal lock, interpolation-friendly |
| Conversion timing | Import + Driver output | One-time cost, not per-execution |
| Component placement | Flexible (any entity) | ECS hierarchy = configuration |
| Device references | ECS marker components | Not Vec<Entity> fields |
| Orchestrator location | In execution_plugin | Core of execution, no artificial separation |
| Plugin dependencies | Device→Execution→Core | No inter-device dependencies |

---

## For New Contributors

**Current Priority:** Streaming Execution implementation (Phase 1-6)

1. **Start with:** `streaming-execution/README.md` for current work
2. **Background:** `execution-plugin/README.md` for foundational architecture
3. **Implementation details:** `streaming-execution/implementation_spec.md`
4. **What's done:** `streaming-execution/current_state.md`

**Key Insights:**
- The ECS entity hierarchy IS the configuration
- "Sealed" buffer pattern enables both static and streaming execution
- Completion detection uses `is_execution_complete()` not manual checks

