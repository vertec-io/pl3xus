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

### 1. Execution Plugin Architecture
**Status:** Design Complete, Ready for Implementation
**Location:** [`execution-plugin/`](./execution-plugin/)

The main research project defining the buffer-based toolpath execution system.

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

| Phase | Description | Status |
|-------|-------------|--------|
| 0 | Create execution_plugin crate structure | Not Started |
| 1 | Database Schema Migration | Not Started |
| 2 | ToolpathBuffer Component | Not Started |
| 3 | Import Layer | Not Started |
| 4 | Orchestrator System | Not Started |
| 5 | FANUC Driver Integration | Not Started |
| 6 | End-to-End Testing | Not Started |

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

1. **Start with:** `execution-plugin/README.md`
2. **Understand the diagrams:** `execution-plugin/diagrams.md`
3. **Implementation begins at:** Phase 0 (crate structure)
4. **Key insight:** The ECS entity hierarchy IS the configuration. We don't hard-code where components live.

