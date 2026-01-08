# Comparison to Other Frameworks

## ROS2 (Robot Operating System)

### Communication Patterns
1. **Topics** (Pub/Sub) - Fire-and-forget, streaming
2. **Services** (Request/Response) - Synchronous, blocking
3. **Actions** (Goal/Feedback/Result) - Long-running with progress

### Mapping to pl3xus
| ROS2 | pl3xus Equivalent |
|------|-------------------|
| Topic | Message / Synced Component |
| Service | Request |
| Action | Not implemented (future) |

### What ROS2 Gets Right
- Clear separation of concerns
- Actions for long-running operations
- Type-safe interfaces (IDL-defined)

### What ROS2 Gets Wrong
- Services are blocking (bad for UI)
- No built-in authorization model
- No entity/node targeting at protocol level

## gRPC / Protobuf

### Communication Patterns
1. **Unary** - Single request, single response
2. **Server Streaming** - Single request, stream of responses
3. **Client Streaming** - Stream of requests, single response
4. **Bidirectional Streaming** - Stream both ways

### Mapping to pl3xus
| gRPC | pl3xus Equivalent |
|------|-------------------|
| Unary | Request |
| Server Streaming | Subscription |
| Client Streaming | Messages (batched) |
| Bidirectional | Not implemented |

### What gRPC Gets Right
- Strong typing via protobuf
- Excellent tooling
- Built-in streaming

### What gRPC Gets Wrong
- No ECS/entity concept
- Authorization is middleware-only (no protocol support)
- Overkill for simple applications

## Phoenix LiveView (Elixir)

### Communication Patterns
1. **Pushes** - Server → Client updates (like synced components)
2. **Events** - Client → Server user actions
3. **Handle_info** - Server-side event processing

### What LiveView Gets Right
- Server-driven UI (like pl3xus)
- Automatic diff-based sync
- Clean separation of client events vs server state

### What LiveView Gets Wrong
- No explicit request/response pattern
- No entity targeting (page-level only)
- No authorization at event level

## Socket.IO

### Communication Patterns
1. **emit** - Fire-and-forget message
2. **emit with ack** - Message with callback
3. **room** - Topic-based broadcasting

### Mapping to pl3xus
| Socket.IO | pl3xus Equivalent |
|-----------|-------------------|
| emit | Message |
| emit with ack | Request |
| room | Entity-based targeting |

### What Socket.IO Gets Right
- Simple mental model
- Ack pattern for confirmations
- Room-based grouping

### What Socket.IO Gets Wrong
- No typing (JSON blobs)
- No built-in authorization
- No ECS/entity awareness

## Summary: What pl3xus Should Be

Taking the best from each:

| Feature | Source | pl3xus |
|---------|--------|--------|
| Type-safe interfaces | ROS2, gRPC | ✓ via serde/bincode |
| Request/Response | gRPC Unary | ✓ RequestMessage |
| Server-driven state | LiveView | ✓ Synced components |
| Entity targeting | ECS-native | ✓ TargetedMessage |
| Authorization | - | ✓ EntityAccessPolicy |
| Streaming | gRPC, Socket.IO | ✓ Messages |
| Actions | ROS2 | ⏳ Future |

## Unique Value of pl3xus

1. **ECS-Native** - Entity targeting is first-class
2. **Authorization Built-In** - Control semantics at protocol level
3. **Hybrid Sync** - Combines sync (LiveView) with RPC (gRPC)
4. **Type-Safe** - Rust types enforced end-to-end

