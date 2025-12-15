---
title: Type Registry Guide
---
# Type Registry Guide

This guide covers the `ClientTypeRegistry` for client-side type deserialization and DevTools support.

---

## Overview

The `ClientTypeRegistry` is the client-side counterpart to the server's `SyncRegistry`. It provides:

1. **Type deserialization** - Convert bincode bytes to concrete Rust types
2. **DevTools support** - JSON conversion for debugging UI (optional)
3. **Type enumeration** - List registered types for tooling

---

## Quick Start

### Basic Registry

```rust
use pl3xus_client::ClientTypeRegistry;
use shared_types::{Position, Velocity, EntityName};

let registry = ClientTypeRegistry::builder()
    .register::<Position>()
    .register::<Velocity>()
    .register::<EntityName>()
    .build();
```

### With DevTools Support

```rust
let registry = ClientTypeRegistry::builder()
    .register::<Position>()
    .register::<Velocity>()
    .register::<EntityName>()
    .with_devtools_support()  // Enable JSON conversion
    .build();
```

---

## Using with SyncProvider

Pass the registry to `SyncProvider`:

```rust
use leptos::prelude::*;
use pl3xus_client::{SyncProvider, ClientTypeRegistry};
use shared_types::{Position, Velocity};

#[component]
pub fn App() -> impl IntoView {
    let registry = ClientTypeRegistry::builder()
        .register::<Position>()
        .register::<Velocity>()
        .with_devtools_support()
        .build();

    view! {
        <SyncProvider
            url="ws://localhost:8080"
            registry=registry
        >
            <MyAppUI />
        </SyncProvider>
    }
}
```

---

## Type Requirements

Registered types must implement:

```rust
use serde::{Serialize, Deserialize};

// Required traits for sync components
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}
```

**Required traits:**
- `Clone` - For reactive signals
- `Serialize + Deserialize` - For wire format
- `Send + Sync + 'static` - For thread safety

**Recommended:**
- `Debug` - For error messages
- `PartialEq` - For change detection in UI

---

## SyncComponent Trait

Types must implement `SyncComponent` for name resolution:

```rust
use pl3xus_client::SyncComponent;

impl SyncComponent for Position {
    fn component_name() -> &'static str {
        "Position"  // Must match server-side type name
    }
}
```

### Automatic Implementation

For shared crates, use the blanket implementation:

```rust
// In shared_types/src/lib.rs
use pl3xus_common::SyncComponent;

// Blanket impl provided by pl3xus_common
// Just derive the required traits:
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}
// SyncComponent is automatically implemented!
```

---

## DevTools Support

### Enabling DevTools

Call `.with_devtools_support()` on the builder:

```rust
let registry = ClientTypeRegistry::builder()
    .register::<Position>()
    .register::<Velocity>()
    .with_devtools_support()  // Enables JSON conversion
    .build();
```

**What this enables:**
- `deserialize_to_json()` - Convert bincode to JSON for display
- `serialize_from_json()` - Convert JSON edits back to bincode
- `has_json_support()` - Check if a type supports JSON

### Zero Overhead

If `.with_devtools_support()` is **not** called:
- JSON converters are dropped during `.build()`
- No runtime overhead for production builds
- `deserialize_to_json()` returns `TypeNotRegistered` error

### Checking DevTools Status

```rust
if registry.is_devtools_support_enabled() {
    // DevTools features available
} else {
    log::warn!("DevTools support not enabled");
}
```

---

## API Reference

### ClientTypeRegistry

```rust
impl ClientTypeRegistry {
    /// Create a new builder
    fn builder() -> ClientTypeRegistryBuilder;

    /// Deserialize to concrete type (for hooks)
    fn deserialize<T: 'static>(&self, name: &str, data: &[u8]) -> Result<T, SyncError>;

    /// Check if type is registered
    fn is_registered(&self, name: &str) -> bool;

    /// Get TypeId for a registered type
    fn get_type_id(&self, name: &str) -> Option<TypeId>;

    /// List all registered type names
    fn registered_types(&self) -> Vec<String>;

    /// Deserialize to JSON (DevTools)
    fn deserialize_to_json(&self, name: &str, data: &[u8]) -> Result<serde_json::Value, SyncError>;

    /// Serialize from JSON (DevTools mutations)
    fn serialize_from_json(&self, name: &str, json: &serde_json::Value) -> Result<Vec<u8>, SyncError>;

    /// Check if type has JSON support
    fn has_json_support(&self, name: &str) -> bool;

    /// Check if DevTools support is enabled
    fn is_devtools_support_enabled(&self) -> bool;
}
```

### ClientTypeRegistryBuilder

```rust
impl ClientTypeRegistryBuilder {
    /// Create new builder
    fn new() -> Self;

    /// Register a component type
    fn register<T: SyncComponent + Serialize + Deserialize>(self) -> Self;

    /// Enable DevTools support (keeps JSON converters)
    fn with_devtools_support(self) -> Self;

    /// Build the registry (wrapped in Arc)
    fn build(self) -> Arc<ClientTypeRegistry>;
}
```

---

## Type Name Matching

**Critical**: Client and server must use the same type names.

### How Names Are Resolved

Both client and server extract the short type name:

```rust
// Full type name: "my_app::components::Position"
// Short name: "Position"

let full_name = std::any::type_name::<T>();
let short_name = full_name.rsplit("::").next().unwrap_or(full_name);
```

### Ensuring Consistency

**Option 1: Shared crate (recommended)**

```rust
// shared_types/src/lib.rs
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Position { pub x: f32, pub y: f32 }

// Server uses: shared_types::Position
// Client uses: shared_types::Position
// Both resolve to: "Position"
```

**Option 2: Matching definitions**

```rust
// Server: server/src/components.rs
pub struct Position { pub x: f32, pub y: f32 }

// Client: client/src/types.rs
pub struct Position { pub x: f32, pub y: f32 }

// Both resolve to: "Position" ✓
```

**Option 3: Custom SyncComponent impl**

```rust
// If names don't match naturally
impl SyncComponent for MyClientPosition {
    fn component_name() -> &'static str {
        "Position"  // Override to match server
    }
}
```

---

## Best Practices

### 1. Use a Shared Types Crate

```
my_project/
├── shared_types/
│   ├── Cargo.toml
│   └── src/lib.rs      # Position, Velocity, etc.
├── server/
│   └── Cargo.toml      # depends on shared_types
└── client/
    └── Cargo.toml      # depends on shared_types
```

### 2. Register All Synced Types

```rust
// ❌ Missing registration causes runtime errors
let registry = ClientTypeRegistry::builder()
    .register::<Position>()
    // Forgot Velocity!
    .build();

// ✅ Register everything the server syncs
let registry = ClientTypeRegistry::builder()
    .register::<Position>()
    .register::<Velocity>()
    .register::<EntityName>()
    .build();
```

### 3. Enable DevTools Only When Needed

```rust
// Production build - no DevTools overhead
#[cfg(not(feature = "devtools"))]
let registry = ClientTypeRegistry::builder()
    .register::<Position>()
    .build();

// Development build - with DevTools
#[cfg(feature = "devtools")]
let registry = ClientTypeRegistry::builder()
    .register::<Position>()
    .with_devtools_support()
    .build();
```

### 4. Handle Registration Errors

```rust
// Check if type is registered before use
if !registry.is_registered("Position") {
    log::error!("Position type not registered!");
}

// Handle deserialization errors
match registry.deserialize::<Position>("Position", &data) {
    Ok(pos) => { /* use position */ }
    Err(SyncError::TypeNotRegistered { .. }) => {
        log::error!("Type not registered");
    }
    Err(SyncError::DeserializationFailed { error, .. }) => {
        log::error!("Deserialization failed: {}", error);
    }
    _ => {}
}
```

---

## Troubleshooting

### "TypeNotRegistered" Error

**Cause**: Component type not registered in client registry.

**Solution**: Add `.register::<T>()` for the missing type.

### "Type mismatch" Error

**Cause**: Type name matches but structure differs.

**Solution**: Ensure client and server use identical type definitions (use shared crate).

### DevTools Shows Raw Bytes

**Cause**: `.with_devtools_support()` not called.

**Solution**: Add `.with_devtools_support()` to builder.

### Type Name Mismatch

**Cause**: Client and server have different module paths.

**Solution**:
- Use shared types crate, or
- Implement custom `SyncComponent::component_name()`

---

## Related Documentation

- [Shared Types](./shared-types.md) - Setting up shared type crates
- [DevTools](./devtools.md) - Using the DevTools UI
- [Hooks](./hooks.md) - Using sync hooks with registered types
- [Server Setup](./server-setup.md) - Server-side type registration

---

**Last Updated**: 2025-12-07
**pl3xus_client Version**: 0.1
```


