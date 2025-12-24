# Query Invalidation API Design Research

## Executive Summary

This document researches industry patterns for automatic query invalidation after mutations, comparing implicit (framework-handled) vs explicit (developer-controlled) approaches. The goal is to determine the best API design for pl3xus.

**Recommendation**: Implement **automatic invalidation by default** with an **opt-out escape hatch**. This follows the "convention over configuration" philosophy while preserving flexibility.

---

## Current Implementation Analysis

### Current Pattern (Explicit)
```rust
// Server: Register invalidation rules
app.invalidation_rules()
    .on_success::<CreateProgram>().invalidate("ListPrograms")
    .on_success::<DeleteProgram>().invalidate("ListPrograms");

// Handler: Explicit broadcast call
fn handle_create_program(...) {
    // ... handle request ...
    if success {
        broadcast_invalidations::<CreateProgram, _>(&net, &rules, None);
    }
}
```

**Problems:**
1. Boilerplate: Every handler needs the explicit `broadcast_invalidations` call
2. Easy to forget: Missing the call = stale data bugs
3. Inconsistent: Some handlers may call it, others may not
4. Verbose: Rules defined separately from where they're used

---

## Industry Research

### 1. TanStack Query (React Query)

**Approach: Explicit with Global Hooks**

TanStack Query deliberately provides NO built-in automatic invalidation. Instead, it offers:

```javascript
// Global MutationCache callback pattern
const queryClient = new QueryClient({
  mutationCache: new MutationCache({
    onSuccess: (_data, _variables, _context, mutation) => {
      // Invalidate everything by default
      queryClient.invalidateQueries();
      
      // Or use meta for fine-grained control
      if (mutation.meta?.invalidates) {
        queryClient.invalidateQueries({ queryKey: mutation.meta.invalidates });
      }
    },
  }),
});
```

**Key Insight from TkDodo (React Query maintainer):**
> "I prefer fetching some data more often than strictly necessary over missing a refetch."

**Takeaway**: Even the explicit approach benefits from a global hook that runs automatically.

### 2. SWR (Vercel)

**Approach: Bound Mutate + Global Mutate**

```javascript
// Bound mutate - automatically revalidates the same key
const { data, mutate } = useSWR('/api/user', fetcher);
await mutate(); // Revalidates '/api/user'

// Global mutate - explicit key targeting
import { mutate } from 'swr';
mutate('/api/user'); // Revalidates specific key
mutate(key => key.startsWith('/api/')); // Pattern matching
```

**Takeaway**: SWR provides automatic revalidation for the same key, but cross-key invalidation is explicit.

### 3. Relay (Facebook GraphQL)

**Approach: Declarative + Automatic Store Updates**

```graphql
mutation CreateTodo($input: CreateTodoInput!) {
  createTodo(input: $input) {
    todoEdge {
      node {
        id
        text
      }
    }
  }
}
```

Relay automatically updates the normalized store when mutation responses include the affected data. Uses `@appendEdge`, `@deleteEdge` directives for list updates.

**Takeaway**: Relay is highly automatic but requires GraphQL schema conventions.

### 4. Apollo GraphQL

**Approach: Automatic + Manual Options**

```javascript
// Automatic: refetchQueries option
const [createTodo] = useMutation(CREATE_TODO, {
  refetchQueries: ['GetTodos'], // Automatic refetch
});

// Manual: cache.modify for fine-grained control
cache.modify({
  fields: {
    todos(existingTodos = []) {
      return [...existingTodos, newTodoRef];
    }
  }
});
```

**Takeaway**: Apollo defaults to automatic with `refetchQueries`, provides escape hatches.

### 5. Rails (Convention over Configuration)

**Philosophy**: The framework should do the right thing by default.

```ruby
# ActiveRecord callbacks - automatic, implicit
class Post < ApplicationRecord
  after_save :invalidate_cache
  
  private
  def invalidate_cache
    Rails.cache.delete("posts_list")
  end
end
```

**Takeaway**: Rails favors implicit behavior with explicit overrides.

---

## Implicit vs Explicit Tradeoffs

| Aspect | Implicit (Automatic) | Explicit (Manual) |
|--------|---------------------|-------------------|
| **Boilerplate** | Minimal | High |
| **Correctness** | Guaranteed (can't forget) | Error-prone |
| **Flexibility** | May over-invalidate | Full control |
| **Debugging** | Magic can be confusing | Clear causality |
| **Learning Curve** | Lower | Higher |
| **Performance** | May refetch unnecessarily | Optimized |

---

## Proposed Design for pl3xus

### Principle: Automatic by Default, Explicit Escape Hatch

Following Rails' "convention over configuration" and TkDodo's advice to "prefer over-fetching over missing refetches":

### Option A: Trait-Based Automatic Invalidation (Recommended)

```rust
// In shared_types crate (where request types are defined)
use pl3xus_sync::Invalidates;

#[derive(Serialize, Deserialize)]
pub struct CreateProgram { ... }

impl Invalidates for CreateProgram {
    fn invalidates() -> &'static [&'static str] {
        &["ListPrograms"]
    }
}
```

**Server-side (automatic):**
```rust
// Framework automatically calls invalidation after successful response
// No explicit broadcast_invalidations() needed!
```

**Escape hatch (when needed):**
```rust
// For complex cases, use explicit broadcast
broadcast_invalidations::<CreateProgram, _>(&net, &rules, Some(entity_id));
```

### Option B: Registration-Time Declaration (Current + Auto-Broadcast)

Keep current registration pattern but make broadcast automatic:

```rust
// Registration (same as current)
app.invalidation_rules()
    .on_success::<CreateProgram>().invalidate("ListPrograms");

// Handler: NO explicit call needed - framework handles it
fn handle_create_program(...) {
    // ... handle request ...
    // Invalidation happens automatically on success
}
```

---

## Implementation Considerations

### Making pl3xus_sync a Dependency of shared_types

The user suggested implementing the trait in shared_types. This is feasible:

```toml
# shared_types/Cargo.toml
[dependencies]
pl3xus_sync = { path = "...", optional = true }

[features]
default = []
server = ["pl3xus_sync"]  # Only on server, not WASM
```

```rust
// shared_types/src/lib.rs
#[cfg(feature = "server")]
use pl3xus_sync::Invalidates;

#[cfg_attr(feature = "server", derive(Invalidates))]
pub struct CreateProgram { ... }

// Or manual impl
#[cfg(feature = "server")]
impl Invalidates for CreateProgram {
    fn invalidates() -> &'static [&'static str] {
        &["ListPrograms"]
    }
}
```

This avoids the orphan rule issue since the trait is implemented where the type is defined.

---

## Recommendation

**Implement Option A: Trait-Based Automatic Invalidation**

1. **Define `Invalidates` trait in pl3xus_sync**
2. **Implement trait on request types in shared_types** (feature-gated for server)
3. **Framework automatically broadcasts invalidations** after successful responses
4. **Keep `broadcast_invalidations` as escape hatch** for edge cases

**Benefits:**
- Zero boilerplate for common cases
- Impossible to forget invalidation
- Collocated: invalidation rules live with the type definition
- Flexible: escape hatch available when needed
- Follows industry best practices (Rails, Apollo)

**Migration Path:**
1. Add `Invalidates` trait to pl3xus_sync
2. Add pl3xus_sync as optional dependency to shared_types
3. Implement trait on existing request types
4. Remove explicit `broadcast_invalidations` calls from handlers
5. Keep `broadcast_invalidations` for complex cases (entity-specific, conditional)

