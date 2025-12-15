# Leptos WebSocket Client Example

This example demonstrates how to build a **non-Bevy web client** that communicates with a pl3xus WebSocket server using the `Pl3xusBincodeCodec`.

## Features

- ✅ **Leptos web framework** for reactive UI
- ✅ **leptos_use WebSocket** integration
- ✅ **Pl3xusBincodeCodec** for binary serialization (same as Bevy clients)
- ✅ **Real-time chat** with the pl3xus server
- ✅ **Beautiful, responsive UI** with modern CSS

## Architecture

This example shows that pl3xus servers can communicate with **any WebSocket client**, not just Bevy applications. The key is using the same binary codec (`Pl3xusBincodeCodec`) for message serialization.

```
┌─────────────────────┐         WebSocket + Bincode        ┌──────────────────────┐
│  Leptos Web Client  │ ◄──────────────────────────────► │  Bevy Server         │
│  (This Example)     │                                    │  (pl3xus)         │
└─────────────────────┘                                    └──────────────────────┘
```

## Prerequisites

Install [Trunk](https://trunkrs.dev/) for building the WASM application:

```bash
cargo install trunk
rustup target add wasm32-unknown-unknown
```

## Running the Example

### 1. Start the WebSocket Server

In one terminal, run the pl3xus WebSocket server:

```bash
cd crates/pl3xus_websockets
cargo run --example server
```

The server will listen on `ws://127.0.0.1:8081`

### 2. Start the Leptos Client

In another terminal, run the Leptos web client:

```bash
cd crates/pl3xus_websockets/leptos_client_example
trunk serve --open
```

The web app will open at `http://127.0.0.1:8080`

### 3. Connect and Chat

1. Click the **"Connect"** button to establish a WebSocket connection
2. Type a message in the input field
3. Click **"Send"** or press Enter
4. Your message will be sent to the server and broadcast to all connected clients

## How It Works

### Message Types

The client and server share the same message types:

```rust
// Client → Server
struct UserChatMessage {
    message: String,
}

// Server → Client
struct NewChatMessage {
    name: String,
    message: String,
}
```

### Binary Codec

Both client and server use `Pl3xusBincodeCodec` which:
- Serializes messages using **bincode** (compact binary format)
- Does **NOT** add length prefixes (WebSocket frames provide message boundaries)
- Is compatible with the `codee` crate used by `leptos_use`

### WebSocket Integration

The Leptos client uses `leptos_use::use_websocket_with_options` with a custom codec:

```rust
use_websocket_with_options::<NewChatMessage, UserChatMessage, Pl3xusBincodeCodec, _, _>(
    "ws://127.0.0.1:8081",
    UseWebSocketOptions::default()
        .on_message(|msg: NewChatMessage| {
            // Handle incoming messages
        })
)
```

## Testing with Multiple Clients

You can run multiple clients simultaneously:

1. **Bevy WASM client**: `cd crates/pl3xus_websockets/wasm_client_example && trunk serve`
2. **Leptos web client**: `cd crates/pl3xus_websockets/leptos_client_example && trunk serve --port 8082`
3. **Bevy native client**: `cargo run --example client --package pl3xus` (if you create a WebSocket version)

All clients will see messages from each other in real-time! 🎉

## Code Structure

```
leptos_client_example/
├── src/
│   ├── main.rs       # Leptos app component and UI
│   ├── codec.rs      # Pl3xusBincodeCodec wrapper
│   └── shared.rs     # Shared message types
├── index.html        # HTML template
├── style.css         # Styling
├── Cargo.toml        # Dependencies
├── Trunk.toml        # Trunk configuration
└── README.md         # This file
```

## Key Differences from Bevy Clients

| Aspect | Bevy Client | Leptos Client |
|--------|-------------|---------------|
| **Framework** | Bevy application engine | Leptos web framework |
| **UI** | Bevy UI components | HTML/CSS |
| **WebSocket** | pl3xus Network resource | leptos_use::use_websocket |
| **Codec** | Pl3xusBincodeCodec | Same! |
| **Messages** | Same types | Same types |
| **Target** | Native or WASM | WASM only |

## Troubleshooting

### Connection Refused

Make sure the server is running on `ws://127.0.0.1:8081`:

```bash
cd crates/pl3xus_websockets
cargo run --example server
```

### WASM Build Errors

Ensure you have the WASM target installed:

```bash
rustup target add wasm32-unknown-unknown
```

### Codec Errors

If you see serialization errors, ensure both client and server are using the same version of the message types and bincode.

## Next Steps

- Add authentication
- Implement private messages
- Add user presence indicators
- Store chat history
- Add emoji support 🎨

## License

MIT

