# Current Issue: Reactive Graph Panic on ControlResponse

## Symptom

When clicking the "REQUEST CONTROL" button in the top bar:
1. The toast "Control requested" appears successfully
2. The client immediately panics with:
```
panicked at /home/apino/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/reactive_graph-...
RuntimeError: unreachable
```

## Root Cause Analysis

The panic occurs in the Leptos reactive graph system. The issue is that `handle_incoming_message` in `crates/pl3xus_client/src/context.rs` is called from inside an Effect (in provider.rs) and performs reactive operations that trigger the panic.

### The Problem Code Path

1. **provider.rs** (lines 145-183) - WebSocket message handler Effect:
```rust
Effect::new(move |_| {
    if let Some(bytes) = raw_message.get() {
        // ... deserialize packet ...
        ctx.handle_incoming_message(packet.type_name, packet.data);
    }
});
```

2. **context.rs** (lines 268-281) - `handle_incoming_message`:
```rust
// BEFORE FIX (problematic):
if let Some(signal) = self.incoming_messages.get().get(&short_name) {  // reactive read
    signal.set(data);  // reactive write inside effect = PANIC
} else {
    let new_signal = RwSignal::new(data);
    self.incoming_messages.update(|map| {  // reactive write
        map.insert(short_name, new_signal);
    });
}
```

### Fix Applied (needs testing)

Changed to use untracked operations:
```rust
// AFTER FIX:
if let Some(signal) = self.incoming_messages.get_untracked().get(&short_name).cloned() {
    signal.update_untracked(|bytes| *bytes = data);
    signal.notify();  // Manually notify subscribers
} else {
    let new_signal = RwSignal::new(data);
    self.incoming_messages.update(|map| {
        map.insert(short_name, new_signal);
    });
}
```

## Why This Happens

In Leptos, calling `.set()` or `.update()` inside an Effect creates a reactive loop:
1. Effect runs → reads signal → adds subscription
2. Effect modifies signal → triggers notification
3. Notification runs during effect execution → reactive graph detects cycle → panic

## Testing Steps

1. Refresh browser at http://127.0.0.1:8084/
2. Connect to Quick Connect (simulator)
3. Click "REQUEST CONTROL" button in top bar
4. Expected: Button should change to "IN CONTROL" without panic
5. Check browser console for errors

## Related Files Modified

- `crates/pl3xus_client/src/context.rs` - Line 268-281 (handle_incoming_message)
- `crates/pl3xus_client/src/provider.rs` - Added hash-based deduplication
- `examples/fanuc_rmi_replica/client/src/layout/top_bar.rs` - ControlResponseHandler uses StoredValue
- `examples/fanuc_rmi_replica/client/src/components/toast.rs` - ToastContext uses StoredValue

## If Fix Doesn't Work

Consider alternative approaches:
1. Use `spawn_local` to defer the signal update outside the Effect context
2. Use a channel/queue pattern instead of direct signal updates
3. Restructure to use `StoredValue` instead of `RwSignal` for incoming_messages
4. Use Leptos's `queue_microtask` to schedule updates outside reactive context

## Other Issues After This Fix

Once control works, these remain:
1. Verify control is released on client disconnect
2. Test multi-client control scenarios (control denied when another has it)
3. Compare UI with original for any missing features
4. End-to-end program execution testing

