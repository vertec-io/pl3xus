# Pl3xus Memory

Memory leak detection and prevention tools for the pl3xus networking library.

## Version Compatibility

| pl3xus_memory | pl3xus | Bevy | Rust |
| :--------------: | :-------: | :--: | :--: |
| 1.1.1 | 1.1.1 | 0.17 | 1.88 (nightly) |
| 1.1.0 | 1.1.0 | 0.17 | 1.88 (nightly) |
| 1.0.0 | 1.0.0 | 0.16 | 1.85 |

**Note**: Bevy 0.17 requires Rust 1.88.0 (nightly). Create `rust-toolchain.toml`:
```toml
[toolchain]
channel = "nightly"
```

## Overview

This crate provides tools to help identify and fix memory leaks in Bevy applications that use pl3xus for networking. It includes systems for monitoring memory usage, cleaning up stale connections, and preventing message queue accumulation.

## Features

- **Memory Usage Monitoring**: Tracks memory usage over time and logs warnings if it detects potential memory leaks.
- **Connection Cleanup**: Periodically checks for stale connections and cleans them up.
- **Message Queue Monitoring**: Monitors message queue sizes and logs warnings if they exceed a threshold.
- **Resource Cleanup**: Periodically cleans up resources to prevent memory accumulation.

## Usage

Add the `NetworkMemoryPlugin` to your Bevy app:

```rust
use bevy::prelude::*;
use pl3xus_memory::NetworkMemoryPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(NetworkMemoryPlugin)
        .run();
}
```

## Diagnostics

The plugin will log diagnostic information to the console, including:

- Active connection count
- Message queue sizes
- Memory usage (on Windows)
- Warnings about potential memory leaks

## Configuration

You can configure the plugin by modifying the resource values after adding the plugin:

```rust
use bevy::prelude::*;
use pl3xus_memory::{NetworkMemoryPlugin, MessageCleanupConfig, ConnectionCleanupConfig, MemoryStats};
use std::time::Duration;

fn configure_memory_plugin(
    mut message_config: ResMut<MessageCleanupConfig>,
    mut connection_config: ResMut<ConnectionCleanupConfig>,
    mut memory_stats: ResMut<MemoryStats>,
) {
    message_config.check_interval = Duration::from_secs(30);
    message_config.max_message_queue_size = 500;
    
    connection_config.check_interval = Duration::from_secs(60);
    
    memory_stats.check_interval = Duration::from_secs(15);
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(NetworkMemoryPlugin)
        .add_systems(Startup, configure_memory_plugin)
        .run();
}
```

## License

MIT OR Apache-2.0
