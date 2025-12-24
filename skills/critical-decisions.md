# Critical Decisions Log

This document tracks key technical decisions made during the pl3xus skills implementation.

---

## Decision 1: Skill Organization Strategy

**Date**: 2025-12-24
**Status**: Approved

### Context
Need to decide between a single comprehensive skill vs. multiple focused skills.

### Decision
Use **multiple focused skills** with a comprehensive `pl3xus-development` skill as the entry point.

### Rationale
1. Progressive disclosure - agents load only what they need
2. Easier maintenance - each skill is self-contained
3. Follows odoo-skills pattern which has proven effective
4. Allows granular invocation: "Use the pl3xus-queries skill to..."

### Alternatives Considered
- Single monolithic skill: Too large, violates progressive disclosure
- Only granular skills: No unified workflow guidance

---

## Decision 2: Production-First API Patterns

**Date**: 2025-12-24
**Status**: Approved

### Context
pl3xus has multiple ways to accomplish the same task (e.g., `use_components` vs `use_entity_component`).

### Decision
Skills will **only document production-grade patterns**:
- `use_entity_component` over `use_components` for multi-entity scenarios
- Builder pattern registration over simple `register_message`
- Targeted requests with entity policies over non-targeted
- Batch registration for related requests

### Rationale
1. Prevents anti-patterns from being learned
2. Ensures generated code is production-ready
3. Reduces cognitive load - one right way to do things

### Implications
- Some simpler APIs will not be documented in skills
- Skills may reference docs for "alternative approaches" but not teach them

---

## Decision 3: Bevy 0.17 API Compliance

**Date**: 2025-12-24
**Status**: Approved

### Context
Bevy 0.17 deprecated `EventReader`/`EventWriter` in favor of `MessageReader`/`MessageWriter`.

### Decision
All skills will use **Bevy 0.17 APIs exclusively**:
- `MessageReader<T>` / `MessageWriter<T>` for messages
- `ChildOf(Entity)` / `Children` for hierarchies
- `Name` component (built-in) for entity names

### Rationale
1. Current Bevy version is 0.17
2. Deprecated APIs may be removed in future versions
3. Consistency with existing pl3xus codebase

---

## Decision 4: Config File Pattern

**Date**: 2025-12-24
**Status**: Approved

### Context
Need to decide how to structure the skills configuration.

### Decision
Follow the **odoo-skills config.json pattern** with pl3xus-specific adaptations:
- Version information
- Path mappings to crates and examples
- Framework-specific settings

### Rationale
1. Proven pattern from odoo-skills
2. Allows skills to reference paths dynamically
3. Single source of truth for configuration

---

## Decision 5: Reference File Strategy

**Date**: 2025-12-24
**Status**: Approved

### Context
Need to decide what goes in SKILL.md vs. reference files.

### Decision
- **SKILL.md**: Workflow, when to use, key patterns (<500 lines)
- **references/**: Detailed API docs, templates, edge cases

### Rationale
1. Follows Agent Skills specification for progressive disclosure
2. Keeps main skill file focused and scannable
3. Allows deep-dives without bloating main file

---

## Decision 6: Industrial Systems Comparison Scope

**Date**: 2025-12-24
**Status**: Approved

### Context
User requested comparison with industrial frameworks (Ignition, Thingworx, SCADA, PLC).

### Decision
Create `industrial-systems` skill that:
1. Explains ECS architecture benefits for industrial applications
2. Compares with traditional approaches (not as criticism, but as context)
3. Maps industrial concepts to ECS patterns

### Rationale
1. Helps developers from industrial backgrounds understand pl3xus
2. Provides vocabulary translation between domains
3. Justifies architectural decisions

---

## Future Decisions (Pending)

### Pending: DevTools Skill
Should we create a dedicated skill for DevTools usage and debugging?

### Pending: Testing Skill
Should we create a skill for testing pl3xus applications?

### Pending: Deployment Skill
Should we create a skill for deploying pl3xus applications?

