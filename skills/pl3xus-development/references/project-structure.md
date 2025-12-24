# pl3xus Project Structure Reference

## Standard Three-Crate Structure

```
my_app/
├── Cargo.toml                    # Workspace manifest
├── server/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs               # App setup, plugin registration
│       └── plugins/
│           ├── mod.rs
│           ├── robot.rs          # Robot-related systems
│           ├── control.rs        # Control logic
│           └── program.rs        # Program execution
├── client/
│   ├── Cargo.toml
│   ├── index.html                # Trunk entry point
│   ├── Trunk.toml                # Trunk configuration
│   └── src/
│       ├── main.rs               # Mount point
│       ├── app.rs                # Root component, SyncProvider
│       ├── pages/
│       │   ├── mod.rs
│       │   ├── dashboard.rs
│       │   └── settings.rs
│       ├── components/
│       │   ├── mod.rs
│       │   ├── robot_card.rs
│       │   └── status_badge.rs
│       ├── hooks/
│       │   ├── mod.rs
│       │   └── use_robot.rs      # Custom hooks
│       └── contexts/
│           ├── mod.rs
│           └── robot_context.rs
└── shared/
    ├── Cargo.toml
    └── src/
        ├── lib.rs                # Re-exports
        ├── components.rs         # Synced components
        ├── messages.rs           # Messages
        └── requests.rs           # Request/Response types
```

## Workspace Cargo.toml

```toml
[workspace]
resolver = "2"
members = ["server", "client", "shared"]

[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }
bevy = { version = "0.17", default-features = false }
leptos = { version = "0.7" }
pl3xus = { path = "../crates/pl3xus" }
pl3xus_client = { path = "../crates/pl3xus_client" }
pl3xus_sync = { path = "../crates/pl3xus_sync" }
pl3xus_common = { path = "../crates/pl3xus_common" }
```

## Server Cargo.toml

```toml
[package]
name = "my_app_server"
version = "0.1.0"
edition = "2024"

[dependencies]
bevy = { workspace = true, features = ["multi_threaded"] }
pl3xus = { workspace = true }
pl3xus_sync = { workspace = true }
pl3xus_websockets = { path = "../crates/pl3xus_websockets" }
my_app_shared = { path = "../shared" }
serde = { workspace = true }
tokio = { version = "1", features = ["full"] }
```

## Client Cargo.toml

```toml
[package]
name = "my_app_client"
version = "0.1.0"
edition = "2024"

[dependencies]
leptos = { workspace = true }
pl3xus_client = { workspace = true }
my_app_shared = { path = "../shared" }
serde = { workspace = true }
wasm-bindgen = "0.2"
console_error_panic_hook = "0.1"
log = "0.4"
console_log = "1"

[features]
devtools = ["pl3xus_client/devtools"]
```

## Shared Cargo.toml

```toml
[package]
name = "my_app_shared"
version = "0.1.0"
edition = "2024"

[dependencies]
serde = { workspace = true }
pl3xus_common = { workspace = true }
pl3xus_macros = { path = "../crates/pl3xus_macros" }
```

## Client index.html

```html
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>My App</title>
    <link data-trunk rel="css" href="styles/main.css" />
</head>
<body>
    <link data-trunk rel="rust" data-wasm-opt="z" />
</body>
</html>
```

## Client Trunk.toml

```toml
[build]
target = "index.html"
dist = "dist"

[watch]
watch = ["src", "index.html", "styles"]

[serve]
address = "127.0.0.1"
port = 8081
open = false

[[proxy]]
rewrite = "/ws"
backend = "ws://127.0.0.1:8080/ws"
ws = true
```

## Running the Application

```bash
# Terminal 1: Start server
cd server && cargo run

# Terminal 2: Start client (with hot reload)
cd client && trunk serve

# With devtools
cd client && trunk serve --features devtools
```

