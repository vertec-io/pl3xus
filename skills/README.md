# pl3xus Development Skills System

**Version**: 1.0.0
**Date**: 2025-12-24

A collection of reusable AI agent skills for production-grade industrial application development using the pl3xus framework (Bevy ECS server + Leptos WASM client).

---

## Quick Start

### Invoke Skills

**Recommended Approach (Comprehensive Workflow)**:
```
"Use the pl3xus-development skill to create a new robot control application"
```

This skill guides you through all phases: Architecture → Server → Client → Testing

**Alternative Approach (Individual Skills)**:
```
"Use the pl3xus-project-structure skill to set up the crate organization"
"Use the pl3xus-server skill to implement component synchronization"
"Use the pl3xus-client skill to create reactive UI components"
"Use the pl3xus-queries skill to implement request/response patterns"
"Use the pl3xus-mutations skill to handle state mutations"
"Use the pl3xus-authorization skill to implement entity access control"
```

---

## Available Skills

### Comprehensive Workflow

| Skill | Purpose |
|-------|---------|
| [pl3xus-development](./pl3xus-development/SKILL.md) | **Complete end-to-end workflow** for building pl3xus applications |

### Framework Skills

| Skill | Purpose |
|-------|---------|
| [pl3xus-project-structure](./pl3xus-project-structure/SKILL.md) | Project organization patterns (shared types vs plugin-based) |
| [pl3xus-server](./pl3xus-server/SKILL.md) | Server-side Bevy ECS patterns, component sync, message handlers |
| [pl3xus-client](./pl3xus-client/SKILL.md) | Client-side Leptos hooks, reactive patterns, UI components |
| [pl3xus-queries](./pl3xus-queries/SKILL.md) | Request/response patterns, targeted queries |
| [pl3xus-mutations](./pl3xus-mutations/SKILL.md) | Mutations, invalidation, optimistic updates |
| [pl3xus-authorization](./pl3xus-authorization/SKILL.md) | Entity policies, control patterns, access control |

### Foundation Skills

| Skill | Purpose |
|-------|---------|
| [bevy-ecs](./bevy-ecs/SKILL.md) | Bevy ECS fundamentals for industrial applications |
| [leptos-ui](./leptos-ui/SKILL.md) | Leptos UI patterns for industrial interfaces |
| [industrial-systems](./industrial-systems/SKILL.md) | ECS vs traditional industrial frameworks |

---

## Workflow

```
┌──────────────────┐    ┌──────────────────┐    ┌──────────────────┐
│   Architecture   │───▶│  Server Setup    │───▶│  Client Setup    │
│   (pl3xus-dev)   │    │  (pl3xus-server) │    │  (pl3xus-client) │
└──────────────────┘    └──────────────────┘    └────────┬─────────┘
                                                         │
                                                         ▼
┌──────────────────┐    ┌──────────────────┐    ┌──────────────────┐
│   Authorization  │◀───│    Mutations     │◀───│     Queries      │
│   (pl3xus-auth)  │    │  (pl3xus-mut)    │    │  (pl3xus-query)  │
└──────────────────┘    └──────────────────┘    └──────────────────┘
```

---

## Configuration

See [config.json](./config.json) for:
- Path mappings to crates and examples
- Production patterns and conventions
- Anti-patterns to avoid
- Reference examples

---

## Core Principles

### 1. Production-First
All skills default to production-grade patterns. No toy examples.

### 2. Server-Authoritative
The server is the source of truth. Clients reflect server state.

### 3. Bevy 0.17 Compliance
Use current Bevy APIs: `MessageReader`, `ChildOf`, `Name`.

### 4. Progressive Disclosure
Skills are structured for efficient context usage:
- Metadata (~100 tokens): name/description
- Instructions (<5000 tokens): SKILL.md body
- Resources (as needed): reference files

---

## Reference Example

The `fanuc_rmi_replica` example demonstrates production-grade patterns:
- Multi-robot support with entity targeting
- Hierarchical entity control
- Real-time component synchronization
- Authorization and access control

---

## Related Resources

- [pl3xus Documentation](../../docs/)
- [Anthropic Agent Skills Specification](https://agentskills.io/specification)
- [Bevy 0.17 Documentation](https://bevyengine.org/)
- [Leptos Documentation](https://leptos.dev/)

