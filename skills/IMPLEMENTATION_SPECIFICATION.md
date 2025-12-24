# pl3xus Skills Implementation Specification

**Version**: 1.0.0
**Date**: 2025-12-24

## Overview

This specification defines the comprehensive agent skills system for the pl3xus framework. These skills enable AI agents to develop production-grade industrial applications using the pl3xus Bevy ECS server + Leptos WASM client architecture.

## Design Principles

### 1. Production-First
All skills default to the most production-ready patterns. No toy examples or anti-patterns.

### 2. Progressive Disclosure
Following the Agent Skills specification:
- **Level 1 (Metadata)**: ~100 tokens - name/description in YAML frontmatter
- **Level 2 (Instructions)**: <5000 tokens - SKILL.md body
- **Level 3 (Resources)**: As needed - reference files, scripts, templates

### 3. Server-Authoritative
pl3xus is designed for industrial/robotics applications where the server is the source of truth. All skills reinforce this pattern.

### 4. Bevy 0.17 Compliance
Use current Bevy APIs: `MessageReader`/`MessageWriter`, `ChildOf`/`Children`, `Name` component.

---

## Skills Inventory

### Core Framework Skills

| Skill | Purpose | Priority |
|-------|---------|----------|
| `pl3xus-development` | End-to-end development workflow | P0 |
| `pl3xus-project-structure` | Project organization patterns | P0 |
| `pl3xus-server` | Server-side Bevy ECS patterns | P0 |
| `pl3xus-client` | Client-side Leptos hooks | P0 |
| `pl3xus-queries` | Request/response patterns | P0 |
| `pl3xus-mutations` | Mutations and invalidation | P0 |
| `pl3xus-authorization` | Entity policies and control | P1 |

### Foundation Skills

| Skill | Purpose | Priority |
|-------|---------|----------|
| `bevy-ecs` | Bevy ECS fundamentals | P1 |
| `leptos-ui` | Leptos UI patterns | P1 |
| `industrial-systems` | ECS vs traditional frameworks | P2 |

---

## Directory Structure

```
pl3xus-skills/
├── config.json                    # Global configuration
├── README.md                      # Skills overview and usage
├── IMPLEMENTATION_SPECIFICATION.md
├── critical-decisions.md
│
├── pl3xus-development/           # Comprehensive workflow skill
│   ├── SKILL.md
│   └── references/
│       ├── project-structure.md
│       └── common-patterns.md
│
├── pl3xus-project-structure/     # Project organization patterns
│   ├── SKILL.md
│   └── references/
│       ├── shared-types-structure.md
│       └── plugin-structure.md
│
├── pl3xus-server/                # Server-side patterns
│   ├── SKILL.md
│   └── references/
│       ├── component-sync.md
│       ├── message-handlers.md
│       └── plugin-organization.md
│
├── pl3xus-client/                # Client-side patterns
│   ├── SKILL.md
│   └── references/
│       ├── hooks-reference.md
│       ├── context-patterns.md
│       └── component-patterns.md
│
├── pl3xus-queries/               # Query patterns
│   ├── SKILL.md
│   └── references/
│       ├── request-response.md
│       └── targeted-requests.md
│
├── pl3xus-mutations/             # Mutation patterns
│   ├── SKILL.md
│   └── references/
│       ├── mutation-handlers.md
│       └── invalidation.md
│
├── pl3xus-authorization/         # Authorization patterns
│   ├── SKILL.md
│   └── references/
│       ├── entity-policies.md
│       └── control-patterns.md
│
├── bevy-ecs/                     # Bevy fundamentals
│   ├── SKILL.md
│   └── references/
│       ├── ecs-concepts.md
│       └── bevy-0.17-patterns.md
│
├── leptos-ui/                    # Leptos patterns
│   ├── SKILL.md
│   └── references/
│       ├── reactive-patterns.md
│       └── industrial-ui.md
│
└── industrial-systems/           # Industry comparison
    ├── SKILL.md
    └── references/
        ├── scada-comparison.md
        └── plc-comparison.md
```

---

## Key API Patterns

### Server Registration (Production Pattern)

```rust
// Component sync
app.sync_component::<Position>(None);

// Targeted request with authorization
app.request::<UpdatePosition, NP>()
   .targeted()
   .with_default_entity_policy()
   .register();

// Batch registration
app.requests::<(SetSpeed, ResetRobot, InitializeRobot), NP>()
   .targeted()
   .with_default_entity_policy()
   .with_error_response();
```

### Client Hooks (Production Pattern)

```rust
// Entity-specific component (preferred for multi-entity)
let position = use_entity_component::<Position>(robot_id);

// Targeted mutation with handler
let update = use_mutation_targeted::<UpdatePosition>(|result| {
    match result {
        Ok(r) if r.success => log::info!("Updated"),
        Ok(r) => log::error!("Failed: {:?}", r.error),
        Err(e) => log::error!("Error: {e}"),
    }
});
```

---

## Implementation Status

### ✅ Phase 1: Core Infrastructure (COMPLETE)
- [x] `config.json` - Global configuration with paths, conventions, anti-patterns
- [x] `critical-decisions.md` - Key technical decisions
- [x] `README.md` - Skills overview and usage guide

### ✅ Phase 2: Primary Skills (COMPLETE)
- [x] `pl3xus-development/SKILL.md` - Comprehensive workflow
- [x] `pl3xus-project-structure/SKILL.md` - Project organization patterns
- [x] `pl3xus-server/SKILL.md` - Server patterns
- [x] `pl3xus-client/SKILL.md` - Client patterns
- [x] `pl3xus-queries/SKILL.md` - Query patterns
- [x] `pl3xus-mutations/SKILL.md` - Mutation patterns

### ✅ Phase 3: Secondary Skills (COMPLETE)
- [x] `pl3xus-authorization/SKILL.md` - Authorization patterns
- [x] `bevy-ecs/SKILL.md` - Bevy fundamentals
- [x] `leptos-ui/SKILL.md` - Leptos patterns

### ✅ Phase 4: Tertiary Skills (COMPLETE)
- [x] `industrial-systems/SKILL.md` - Industry comparison

### ✅ Reference Files (COMPLETE)
- [x] `pl3xus-development/references/project-structure.md`
- [x] `pl3xus-development/references/common-patterns.md`
- [x] `pl3xus-project-structure/references/shared-types-structure.md`
- [x] `pl3xus-project-structure/references/plugin-structure.md`
- [x] `pl3xus-server/references/component-sync.md`
- [x] `pl3xus-server/references/message-handlers.md`
- [x] `pl3xus-client/references/hooks-reference.md`
- [x] `pl3xus-authorization/references/entity-policies.md`
- [x] `pl3xus-mutations/references/invalidation.md`

---

## Usage

### Invoke Skills

```
"Use the pl3xus-development skill to create a new robot control application"
"Use the pl3xus-server skill to implement component synchronization"
"Use the pl3xus-client skill to create reactive UI components"
```

### Skill Hierarchy

```
pl3xus-development (comprehensive)
├── pl3xus-project-structure (organization)
├── pl3xus-server (server-side)
├── pl3xus-client (client-side)
├── pl3xus-queries (data fetching)
├── pl3xus-mutations (state changes)
└── pl3xus-authorization (access control)

Foundation:
├── bevy-ecs (ECS concepts)
├── leptos-ui (UI patterns)
└── industrial-systems (industry context)
```

