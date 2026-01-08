# Developer Experience Analysis

How should pl3xus feel to an engineer encountering it for the first time?

## The Mental Model

An engineer should think:

> "I have an ECS world on the server. I have a UI in the browser. 
> Some state syncs automatically. Some actions require explicit requests.
> The server controls who can do what."

## The Decision Tree

When adding a new operation, the engineer should ask:

```
┌─ Is this just reading/displaying state?
│  └─ YES → Use synced components (already done)
│
├─ Is this a continuous stream (jogging, position, etc)?
│  └─ YES → Use targeted message
│           - No response (would create backpressure)
│           - Rely on synced components for feedback
│
├─ Is this a one-shot command that needs confirmation?
│  └─ YES → Does it operate on a specific entity?
│           ├─ YES → Use targeted request
│           └─ NO  → Use regular request
│
└─ Is this a query/fetch?
   └─ YES → Use regular request (no entity target)
```

## What "Intuitive" Looks Like

### Server Registration

```rust
// All message types use the same builder pattern
app.message::<T, NP>().register();                                    // Simple message
app.message::<T, NP>().targeted().register();                         // Targeted, no auth
app.message::<T, NP>().targeted().with_default_entity_access().register(); // Targeted + default policy

app.request::<R, NP>().register();                                    // Simple request
app.request::<R, NP>().targeted().register();                         // Targeted request, no auth
app.request::<R, NP>().targeted().with_default_entity_access().register(); // Targeted + default policy

// Custom policies when needed
app.request::<R, NP>()
   .targeted()
   .with_entity_access(EntityAccessPolicy::from_fn(custom_check))
   .register();
```

Pattern: **message/request** → optional **targeting** → optional **policy**

### Client Usage

```rust
// Messages (fire-and-forget)
ctx.send(MyMessage { ... });                    // Broadcast
ctx.send_targeted(entity, MyMessage { ... });   // Targeted

// Requests (response expected)
let (send, state) = use_request::<MyRequest>();
send(MyRequest { ... });                        // Non-targeted

let (send, state) = use_targeted_request::<MyRequest>();
send(entity, MyRequest { ... });                // Targeted
```

Pattern: **send** for messages, **use_request hook** for requests

### Handler Types

```rust
// The type tells you everything
MessageReader<T>                      // Plain message
MessageReader<TargetedMessage<T>>     // Targeted, no auth (rare)
MessageReader<AuthorizedTargetedMessage<T>>  // Targeted + auth

MessageReader<Request<R>>             // Plain request
MessageReader<AuthorizedRequest<R>>   // Targeted + auth request
```

Pattern: Wrapper type indicates capabilities

## Common Mistakes to Prevent

### Mistake 1: Using message for command that needs response
```rust
// ❌ Bad: Message for one-shot command
app.message::<SetSpeedOverride, NP>().targeted().with_auth().register();
// Client has no idea if it worked!

// ✅ Good: Request for one-shot command
app.request::<SetSpeedOverride, NP>().targeted().with_auth().register();
// Client gets Response with success/error
```

### Mistake 2: Using request for high-frequency stream
```rust
// ❌ Bad: Request for streaming
app.request::<JogCommand, NP>().targeted().with_auth().register();
// Response overhead at 50Hz = bad performance

// ✅ Good: Message for streaming
app.message::<JogCommand, NP>().targeted().with_auth().register();
// Fire-and-forget, observe results via synced position
```

### Mistake 3: Forgetting authorization
```rust
// ❌ Bad: Targeted without policy
app.request::<SetSpeedOverride, NP>().targeted().register();
// Anyone can set speed on any robot!

// ✅ Good: Targeted with entity access policy
app.request::<SetSpeedOverride, NP>()
   .targeted()
   .with_default_entity_access()  // Uses EntityControl-based policy
   .register();
// Only client with control can set speed
```

## Documentation Structure

The docs should be structured as:

1. **Synced Components** - Automatic state synchronization
2. **Messages** - Fire-and-forget operations
3. **Requests** - Operations requiring response
4. **Targeting** - Entity-specific operations
5. **Authorization** - Control-based access

Each section builds on the previous, with clear guidance on when to use each.

## API Consistency Checklist

| Feature | Message API | Request API | Symmetric? |
|---------|-------------|-------------|------------|
| Basic registration | `app.message::<T>().register()` | `app.request::<R>().register()` | ✅ |
| Targeting | `.targeted()` | `.targeted()` | ✅ |
| Default entity policy | `.with_default_entity_access()` | `.with_default_entity_access()` | ✅ |
| Custom entity policy | `.with_entity_access(policy)` | `.with_entity_access(policy)` | ✅ |
| Default message policy | `.with_default_message_access()` | `.with_default_message_access()` | ✅ |
| Custom message policy | `.with_message_access(policy)` | `.with_message_access(policy)` | ✅ |
| Client send | `ctx.send_targeted()` | `send(entity, req)` | ✅ |
| Handler type | `AuthorizedTargetedMessage<T>` | `AuthorizedRequest<R>` | ✅ |

