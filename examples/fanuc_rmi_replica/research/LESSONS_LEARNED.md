# Lessons Learned - Leptos & Reactive Patterns

## Critical Pattern: Effects and Signal Updates

### ❌ DON'T: Update signals inside Effects using reactive methods

```rust
// This WILL cause reactive graph panic!
Effect::new(move |_| {
    let data = some_signal.get();  // reactive read - creates subscription
    another_signal.set(data);       // reactive write - triggers notification
    // PANIC: reactive graph detects cycle
});
```

### ✅ DO: Use untracked methods when updating inside Effects

```rust
Effect::new(move |_| {
    let data = some_signal.get();  // This is fine - we need to track the source
    // Use update_untracked + notify to avoid reactive loop
    another_signal.update_untracked(|val| *val = data);
    another_signal.notify();  // Manually notify subscribers
});
```

### ✅ BETTER: Use get_untracked when you don't need reactivity

```rust
Effect::new(move |_| {
    let data = some_signal.get();  // Track this signal
    // Read without creating subscription
    let other = other_signal.get_untracked();
    // ...
});
```

## StoredValue vs RwSignal

### Use `StoredValue` for:
- Counters/IDs that increment but don't need reactive updates
- State tracking inside Effects (like "last processed hash")
- Values that need to persist but not trigger re-renders

```rust
let counter = StoredValue::new(0u64);
// Inside Effect:
let current = counter.get_value();  // Non-reactive read
counter.set_value(current + 1);      // Non-reactive write
```

### Use `RwSignal` for:
- UI state that should trigger re-renders when changed
- Shared state between components

```rust
let count = RwSignal::new(0);
// In view:
view! { <span>{move || count.get()}</span> }
// On click:
count.set(count.get() + 1);
```

## Common Pitfalls

### 1. Infinite Effect Loops
```rust
// BAD: Effect that modifies the signal it's tracking
Effect::new(move |_| {
    let val = signal.get();
    signal.set(val + 1);  // Triggers the effect again!
});
```

### 2. Reactive Reads Inside Closures
```rust
// BAD: Creating subscriptions in event handlers
let on_click = move |_| {
    let data = signal.get();  // This can be problematic in some contexts
};

// BETTER: Use get_untracked in event handlers
let on_click = move |_| {
    let data = signal.get_untracked();
};
```

### 3. HashMap Signal Updates
```rust
// BAD: Reading and writing same HashMap signal
if let Some(entry) = map_signal.get().get(&key) {
    // Modifying here creates issues
}

// GOOD: Use get_untracked + clone
if let Some(entry) = map_signal.get_untracked().get(&key).cloned() {
    // Now safe to modify
    entry.set(new_value);
}
```

## WebSocket Message Handling Pattern

The pattern we use for handling WebSocket messages:

```rust
// In provider.rs
Effect::new(move |_| {
    if let Some(bytes) = raw_message.get() {
        // Deserialize the message
        let packet = deserialize(bytes);
        
        // Route to appropriate handler
        // IMPORTANT: Handler must use untracked operations
        ctx.handle_incoming_message(packet);
    }
});

// In context.rs - handle_incoming_message
pub fn handle_incoming_message(&self, type_name: String, data: Vec<u8>) {
    // Use get_untracked to read the map
    if let Some(signal) = self.incoming_messages.get_untracked().get(&key).cloned() {
        // Use update_untracked + notify
        signal.update_untracked(|bytes| *bytes = data);
        signal.notify();
    } else {
        // Creating new signal is fine
        let new_signal = RwSignal::new(data);
        self.incoming_messages.update(|map| {
            map.insert(key, new_signal);
        });
    }
}
```

## WASM RefCell Already Borrowed Panic

### The Problem

When using WebSockets with rapid message delivery, you may see:
```
panicked at wasm-bindgen-futures-0.4.56/src/task/singlethread.rs:132:37:
RefCell already borrowed
```

This happens when async tasks overlap - the WASM async task queue uses a RefCell internally,
and nested operations can cause borrow conflicts.

### The Chain That Causes It

1. WebSocket receives message (inside leptos-use async machinery)
2. Provider Effect triggers `handle_incoming_message()`
3. That calls `signal.notify()` or `signal.set()`
4. Downstream Effects run synchronously
5. Those Effects also update signals or spawn async operations
6. PANIC - RefCell already borrowed

### The Fix

In ALL places where Effects update signals (including pl3xus_client internals):

```rust
// ❌ BAD - causes RefCell panic with rapid updates
Effect::new(move |_| {
    let data = source_signal.get();
    target_signal.set(data);  // Triggers downstream effects synchronously
});

// ✅ GOOD - avoids nested async conflicts
Effect::new(move |_| {
    let data = source_signal.get();
    target_signal.try_update_untracked(|val| *val = data);
    target_signal.notify();
});
```

### Files That Required This Fix

- `crates/pl3xus_client/src/context.rs` - `subscribe_message`, `subscribe_component`, `handle_incoming_message`
- `crates/pl3xus_client/src/provider.rs` - `handle_server_message`, `handle_sync_item`
- `examples/fanuc_rmi_replica/client/src/components/toast.rs` - `ToastContext::show()`
- `examples/fanuc_rmi_replica/client/src/layout/top_bar.rs` - `ConsoleLogHandler`

## Debugging Tips

1. **Check browser console** for `reactive_graph... RuntimeError: unreachable`
2. **Check for RefCell panics** - usually indicates nested async/reactive operations
3. **Add logging** before and after signal operations to trace the issue
4. **Look for Effects** that both read and write signals
5. **Search for `.get()` calls** inside Effect closures
6. **Search for `.set()` or `.update()` calls** inside Effects - should be `.try_update_untracked()` + `.notify()`

## References

- Leptos Book: https://book.leptos.dev/
- Leptos GitHub: https://github.com/leptos-rs/leptos
- reactive_graph crate: Underlying reactive system
