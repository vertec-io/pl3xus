# Leptos Reactivity Fix Documentation

## Problem Summary

The Leptos WebSocket chat client had critical reactivity bugs where messages weren't appearing in the UI immediately:

1. **Optimistic messages** (sent by the user) didn't render immediately
2. **Incoming messages** from the server didn't render immediately
3. **Connection state messages** didn't render immediately

The console logs showed that messages were being added to the `chat_messages` signal, but the UI wasn't updating.

## Root Cause Analysis

### Issue 1: Non-Reactive Zone in leptos-use

The `leptos-use` library's `use_websocket_with_options` function intentionally wraps callbacks (`on_open`, `on_close`, `on_error`, `on_message`) in a `SpecialNonReactiveZone` (see `leptos-use/src/use_websocket.rs` lines 564-586).

This prevents signal updates inside these callbacks from triggering reactivity, which is by design to avoid infinite loops and unintended side effects.

**The correct pattern** (shown in leptos-use examples, lines 68-74) is to use `Effect::new()` to watch the returned signals (`ready_state`, `message`), not to update signals in the callbacks.

### Issue 2: `<For>` Component Not Detecting Changes

The original code used:
```rust
<For
    each=move || chat_messages.get()
    key=|msg| msg.id
    children=move |msg: ChatMessage| { ... }
/>
```

This pattern didn't properly trigger re-renders when the Vec was mutated. The `<For>` component needs to detect that the collection has changed, but the way we were using it wasn't reactive enough.

## Solution

### Fix 1: Use `Effect` to Watch Signals

**Before:**
```rust
.on_open(move |_| {
    log::info!("WebSocket connected!");
    set_is_connected.set(true);
    set_chat_messages.update(|msgs| {
        msgs.push(ChatMessage::system("Connected to server!"));
    });
})
```

**After:**
```rust
.on_open(move |_| {
    log::info!("WebSocket connected!");
    set_is_connected.set(true);
})

// Separate Effect to watch ready_state changes
Effect::new(move |prev_state: Option<ConnectionReadyState>| {
    let current_state = ready_state.get();
    
    if let Some(prev) = prev_state {
        if prev != current_state {
            match current_state {
                ConnectionReadyState::Open => {
                    chat_messages.update(|msgs| {
                        msgs.push(ChatMessage::system("Connected to server!"));
                    });
                }
                ConnectionReadyState::Closed => {
                    chat_messages.update(|msgs| {
                        msgs.push(ChatMessage::system("Disconnected from server!"));
                    });
                }
                _ => {}
            }
        }
    }
    
    current_state
});
```

### Fix 2: Replace `<For>` with Direct Iteration

**Before:**
```rust
<For
    each=move || chat_messages.get()
    key=|msg| msg.id
    children=move |msg: ChatMessage| { ... }
/>
```

**After:**
```rust
{move || {
    chat_messages.with(|msgs| {
        msgs.iter().map(|msg| {
            let is_system = msg.is_system;
            let author = msg.author.clone();
            let text = msg.text.clone();
            view! {
                <div class="chat-message" class:system-message=is_system>
                    {if is_system {
                        view! { <span class="message-text">{text}</span> }.into_any()
                    } else {
                        view! {
                            <span class="message-author">{author + ":"}</span>
                            <span class="message-text">{text}</span>
                        }.into_any()
                    }}
                </div>
            }
        }).collect_view()
    })
}}
```

This pattern uses `chat_messages.with()` inside a reactive closure, which properly tracks the signal and re-renders when it changes.

### Fix 3: Use `RwSignal` Instead of `signal()`

**Before:**
```rust
let (chat_messages, set_chat_messages) = signal(Vec::<ChatMessage>::new());
```

**After:**
```rust
let chat_messages = RwSignal::new(Vec::<ChatMessage>::new());
```

This simplifies the code and ensures consistent reactivity patterns.

## Testing Results

After implementing these fixes, the chat application works perfectly:

✅ **Connection messages** appear immediately when connecting/disconnecting
✅ **Optimistic messages** (sent by the user) appear immediately with "You:" prefix
✅ **Incoming messages** from other clients appear immediately
✅ **No duplicate messages** for the sender (thanks to `broadcast_except`)
✅ **Multi-client communication** works correctly between browser tabs

## Key Takeaways

1. **Never update signals in leptos-use callbacks** - Use `Effect` to watch the returned signals instead
2. **Use `chat_messages.with()` inside reactive closures** for proper reactivity tracking
3. **Prefer `RwSignal` for mutable state** that needs to be both read and written
4. **Test with multiple clients** to ensure proper message routing and no duplicates

