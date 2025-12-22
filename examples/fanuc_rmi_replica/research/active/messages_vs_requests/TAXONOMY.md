# Network Operations Taxonomy

This document classifies all network operations and their ideal patterns.

## Classification Matrix

| Axis | Options |
|------|---------|
| **Direction** | Client→Server, Server→Client, Bidirectional |
| **Targeting** | None (broadcast), Entity-targeted, Client-targeted |
| **Response** | None (fire-forget), Ack only, Full response |
| **Authorization** | None, Message-level, Entity-level (control) |
| **Frequency** | One-shot, Periodic, Streaming |

## Pattern Definitions

### Pattern 1: Simple Message
```
Direction: Any
Targeting: None
Response: None
Authorization: Optional (message-level)
Frequency: Any
```
Use for: Broadcasts, notifications, keepalive

### Pattern 2: Targeted Message  
```
Direction: Client→Server
Targeting: Entity
Response: None
Authorization: Entity-level (control)
Frequency: High (streaming ok)
```
Use for: Continuous control (jogging), real-time data

### Pattern 3: Request/Response
```
Direction: Client→Server→Client
Targeting: None
Response: Full
Authorization: Message-level
Frequency: Low (one-shot)
```
Use for: Queries, CRUD operations, config changes

### Pattern 4: Targeted Request/Response ⭐ NEW
```
Direction: Client→Server→Client
Targeting: Entity
Response: Full
Authorization: Entity-level (control)
Frequency: Low (one-shot)
```
Use for: Robot commands, entity-specific operations

### Pattern 5: Subscription (Pub/Sub)
```
Direction: Server→Clients
Targeting: Topic/Entity
Response: None (or ack)
Authorization: Optional
Frequency: Continuous
```
Use for: State sync, event streams

## Current Commands Classification

### Already Correct ✓

| Command | Current | Ideal | Notes |
|---------|---------|-------|-------|
| `JogCommand` | Targeted Message | Targeted Message | High-freq, observable via sync |
| `ListPrograms` | Request | Request | Query, no entity |
| `GetSettings` | Request | Request | Query, no entity |

### Need Response ⚠️

| Command | Current | Ideal | Why |
|---------|---------|-------|-----|
| `SetSpeedOverride` | Targeted Message | Targeted Request | Client needs success confirmation |
| `InitializeRobot` | Targeted Message | Targeted Request | Critical, must confirm success |
| `ResetRobot` | Targeted Message | Targeted Request | Must confirm reset completed |
| `AbortMotion` | Targeted Message | Targeted Request | Safety-critical, must confirm |

### Need Targeting ⚠️

| Command | Current | Ideal | Why |
|---------|---------|-------|-----|
| `StartProgram` | Request | Targeted Request | Operates on system entity |
| `PauseProgram` | Request | Targeted Request | Operates on system entity |
| `StopProgram` | Request | Targeted Request | Operates on system entity |
| `LoadProgram` | Request | Targeted Request | Operates on system entity |
| `ConnectToRobot` | Targeted Message | Targeted Request | Must confirm connection |

### Ambiguous Cases

| Command | Current | Options | Analysis |
|---------|---------|---------|----------|
| `LinearMotionCommand` | Targeted Message | Either | One-shot but part of streaming sequence |
| `JointMotionCommand` | Targeted Message | Either | Same as above |

## Design Decision Framework

```
Is this a continuous/high-frequency stream?
├── YES → Targeted Message (no response overhead)
└── NO → Does the client need to know success/failure?
          ├── YES → Targeted Request
          └── NO → Is it visible via synced components?
                    ├── YES → Targeted Message (sync is implicit response)
                    └── NO → Targeted Request (explicit confirmation)
```

## Framework Implications

To support all patterns cleanly, pl3xus needs:

1. **Targeted Requests** - Entity ID + correlation ID + response
2. **Request Authorization Middleware** - Apply EntityAccessPolicy to requests
3. **Unified Builder Pattern** - Similar to message builder

```rust
// Proposed API
app.request::<SetSpeedOverride, NP>()
   .targeted()                       // Expects TargetedRequest<T> wire format
   .with_default_entity_access()     // Uses ExclusiveControlPlugin policy
   .register();

// Or for non-targeted
app.request::<ListPrograms, NP>()
   .register();  // Current behavior
```

