# Critical Decisions: Component Mutation Handlers

## Decision Log

### D1: Handler Execution Model
**Decision**: Handlers are normal Bevy systems that read `ComponentMutation<T>` events
**Rationale**: Provides full World access and follows familiar Bevy patterns
**Date**: 2025-12-23

### D2: Mutation Application Timing
**Decision**: Mutations are applied AFTER handler processes them (handler-controlled)
**Rationale**: Prevents invalid states from being visible to clients before validation
**Date**: 2025-12-23

### D3: Response Types
**Decision**: Handlers can respond with custom data types, similar to request/response pattern
**Rationale**: Provides flexibility for complex validation feedback
**Date**: 2025-12-23

### D4: Authorization
**Decision**: Reuse existing `MutationAuthorizer` hook; handlers are for business logic only
**Rationale**: Separation of concerns; authorization is a cross-cutting concern
**Date**: 2025-12-23

### D5: Partial Updates
**Decision**: Full component replacement only (partial updates added to roadmap)
**Rationale**: Simpler initial implementation; partial updates add significant complexity
**Date**: 2025-12-23

### D6: Registration API
**Decision**: Builder pattern: `app.sync_component::<T>().with_handler(system, config)`
**Rationale**: Consistent with Bevy app builder patterns, extensible
**Date**: 2025-12-23

### D7: Client Hooks Naming
**Decision**: 
- Rename `use_entity_component` to `use_component`
- Add `use_mut_component` for mutations with handlers
- Add `use_component_store` for nested struct reactivity
**Rationale**: Clearer naming, separation of read-only vs mutable access
**Date**: 2025-12-23

### D8: Entity Parameter Type
**Decision**: Accept `impl Into<Signal<Option<EntityIdBits>>>` or similar for entity parameter
**Rationale**: Allows both raw values and reactive signals, matching Leptos 0.8 patterns
**Date**: 2025-12-23

### D9: Targeted vs Non-Targeted
**Decision**: Component mutations follow existing entity targeting patterns from authorization
**Rationale**: Consistency with existing targeted request patterns
**Date**: 2025-12-23

## Roadmap Items (Not Implementing Now)

1. **Partial/Field-Level Updates**: Allow mutating individual fields instead of whole component
2. **Optimistic Merge**: Handler can merge client changes with current server state

