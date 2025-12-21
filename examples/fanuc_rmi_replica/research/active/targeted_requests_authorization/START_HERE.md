# Targeted Requests with Authorization Middleware

## Agent Context

You are continuing research started by a previous agent session. This document provides everything you need to understand the problem space and continue the work.

## Background

The pl3xus framework provides a server-driven state synchronization pattern:
- **Bevy server** is the authoritative source of truth
- **Leptos clients** receive synced components and can request changes
- **ExclusiveControlPlugin** handles control take/release for entities
- **MutationAuthorizer** validates client mutations before applying them

### The Gap

Currently, **mutations** have authorization middleware, but **requests** do not. When a client wants to send a command that targets a specific entity (e.g., "jog Robot #3 at velocity X"), there's no framework-level way to:

1. Attach a target entity ID to the request
2. Automatically check if the client has control of that entity
3. Provide authorized request data to the system handler

Each system currently has to manually check control authorization, which is boilerplate-heavy and error-prone.

## Meteorite Reference Implementation

The meteorite project (at `/home/apino/dev/meteorite/`) solved this problem with:

### 1. TargetedMessage Wrapper
```rust
// In eventwork_common
pub struct TargetedMessage<T: EventworkMessage> {
    pub target_id: String,  // Node ID to target
    pub message: T,         // The actual message
}
```

### 2. AuthorizedNetworkData Type
```rust
// Systems receive this instead of raw NetworkData
pub struct AuthorizedNetworkData<T: EventworkMessage> {
    pub inner: T,                           // The unwrapped message
    pub authorized: bool,                   // Control check result
    pub source: ConnectionId,               // Client that sent it
    pub node_id: String,                    // Target node ID
    pub control_state: Option<NodeControl>, // Control info if authorized
}
```

### 3. Authorization Middleware System
```rust
// Transforms TargetedMessage<T> → AuthorizedNetworkData<T>
fn handle_authorized_messages<T: EventworkMessage>(
    mut network_events: MessageReader<NetworkData<TargetedMessage<T>>>,
    mut nodes: Query<(&NetworkNode, Option<&mut NodeControl>, ...)>,
    mut authorized_events: MessageWriter<AuthorizedNetworkData<T>>,
) {
    for event in network_events.read() {
        let node_id = &event.target_id;
        // Check control, emit AuthorizedNetworkData
    }
}
```

**Key Files:**
- `/home/apino/dev/meteorite/plugins/src/core/authorization/systems/message_systems.rs`
- `/home/apino/dev/meteorite/plugins/src/core/authorization/models/messages.rs`
- `/home/apino/dev/meteorite/plugins/src/core/authorization/authorization.rs`

## Current pl3xus State

### What Exists

1. **`TargetedMessage<T>`** - Already defined in `crates/pl3xus_common/src/messages.rs`
   ```rust
   pub struct TargetedMessage<T: Pl3xusMessage> {
       pub target_id: String,
       pub message: T,
   }
   ```

2. **`use_request<R>()`** - Client hook for request/response
   - Returns `(impl Fn(R), Signal<UseRequestState<R::ResponseMessage>>)`
   - No support for targeting an entity
   - No authorization at framework level

3. **`MutationAuthorizerResource`** - Server-side mutation authorization
   - Works for component mutations
   - Has `has_control_hierarchical()` helper
   - Does NOT apply to requests

4. **`ExclusiveControlPlugin`** - Control take/release handling
   - Handles `ControlRequest::Take(entity_id)` / `Release`
   - Syncs `EntityControl` component
   - Does NOT intercept other requests

### What's Missing

## Use Cases

### 1. Robot Jogging (Primary Use Case)
Client wants to jog robot at specific velocity. Must have control.
```rust
// Client
let (jog, _) = use_request_targeted::<JogCommand>(|| robot_entity_id);
jog(JogCommand { axis: Axis::X, velocity: 0.5 });

// Server system receives authorized data, no manual check needed
fn handle_jog(mut requests: MessageReader<AuthorizedRequest<JogCommand>>) {
    for req in requests.read() {
        if req.authorized {
            // Execute jog
        }
    }
}
```

### 2. Program Load/Run Commands
Must have control to load or run programs on the system.

### 3. Configuration Changes
Must have control to modify robot configuration.

## Research Tasks

1. **Evaluate API Design Options**
   - How should `use_request_targeted` differ from `use_request`?
   - Should the entity ID be passed per-request or at hook creation?
   - Should we use entity bits (u64) or String IDs like meteorite?

2. **Server-Side Middleware Design**
   - How to register a targeted request type with authorization?
   - Should it use `RequestMessage` trait or a new trait?
   - How does it integrate with `ExclusiveControlPlugin`?

3. **Evaluate Alternatives**
   - Could we extend mutations to handle command-like operations?
   - Is a generic "action" pattern better than request/response?
   - How do other frameworks (ROS2 action servers) handle this?

4. **Implementation Feasibility**
   - What changes are needed to `pl3xus_client`?
   - What changes are needed to `pl3xus_sync`?
   - Is this breaking or additive?

## Key Codebase Locations

### pl3xus Framework
- `crates/pl3xus_client/src/hooks.rs` - Client hooks including `use_request`
- `crates/pl3xus_client/src/context.rs` - `SyncContext.request()` implementation
- `crates/pl3xus_common/src/messages.rs` - `TargetedMessage`, `RequestMessage` trait
- `crates/pl3xus_sync/src/control.rs` - `ExclusiveControlPlugin`
- `crates/pl3xus_sync/src/registry.rs` - `MutationAuthorizerResource`, `has_control_hierarchical`
- `crates/pl3xus/src/managers/network_request.rs` - Request handling

### Meteorite (Reference)
- `/home/apino/dev/meteorite/plugins/src/core/authorization/` - Full authorization system
- `/home/apino/dev/meteorite/app/src/utils/app_state/websocket_provider.rs` - `send_targeted()`

### Related Research
- `examples/fanuc_rmi_replica/research/active/control_system_architecture/` - SystemMarker pattern

## Constraints & Principles

1. **Eliminate boilerplate** - Systems shouldn't manually check control
2. **Server-authoritative** - All control decisions on server
3. **Additive changes preferred** - Don't break existing `use_request`
4. **Hierarchical control support** - Control parent = control children
5. **Type-safe** - Compile-time guarantees where possible

## Expected Outputs

1. **DESIGN.md** - Proposed API and architecture
2. **ALTERNATIVES.md** - Evaluated alternatives with pros/cons
3. **IMPLEMENTATION_PLAN.md** - Step-by-step implementation guide
4. **Optional: Prototype code** - If feasible, draft implementations

## Agent Instructions

1. Read this document fully
2. Review the meteorite implementation (files listed above)
3. Review current pl3xus request/mutation handling
4. Evaluate the design space and document findings
5. Propose a concrete API design
6. Create implementation plan for next agent session
