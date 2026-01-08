# pl3xus_async

Tokio runtime integration for pl3xus/Bevy applications, enabling background async task execution with main thread synchronization.

This crate is part of the [pl3xus](https://github.com/vertec-io/pl3xus) ecosystem for building real-time networked applications with Bevy.

> **Attribution**: This crate is based on [bevy-tokio-tasks](https://github.com/EkardNT/bevy-tokio-tasks) by EkardNT, adapted and integrated into the pl3xus ecosystem.

## Features

- **Background Task Execution**: Spawn async tasks on a Tokio runtime from Bevy systems
- **Main Thread Synchronization**: Safely execute callbacks on the main Bevy thread from background tasks
- **WASM Compatibility**: Automatically uses current-thread runtime on wasm32 targets
- **Tick-based Sleeping**: Sleep for a specified number of Bevy update ticks

## Usage

### Installing the Plugin

Add the `TokioTasksPlugin` to your Bevy app:

```rust
use pl3xus_async::TokioTasksPlugin;

fn main() {
    App::new()
        .add_plugins(TokioTasksPlugin::default())
        .run();
}
```

### Spawning Background Tasks

Use `TokioTasksRuntime` as a system resource to spawn async tasks:

```rust
use pl3xus_async::TokioTasksRuntime;

fn connect_to_device(runtime: Res<TokioTasksRuntime>) {
    runtime.spawn_background_task(|mut ctx| async move {
        // Perform async work (network I/O, driver communication, etc.)
        let result = some_async_operation().await;

        // Synchronize results back to the main thread
        ctx.run_on_main_thread(move |ctx| {
            ctx.world.insert_resource(ConnectionResult(result));
        }).await;
    });
}
```

### Main Thread Synchronization

Background tasks can safely access and modify the Bevy World through `run_on_main_thread`:

```rust
runtime.spawn_background_task(|mut ctx| async move {
    loop {
        // Poll external device
        let device_state = poll_device().await;

        // Update ECS components on main thread
        ctx.run_on_main_thread(move |ctx| {
            if let Some(mut query) = ctx.world.query::<&mut DeviceState>().iter(ctx.world).next() {
                *query = device_state;
            }
        }).await;

        tokio::time::sleep(Duration::from_millis(100)).await;
    }
});
```

## Common Use Cases in pl3xus

- **Robot Driver Communication**: Connect to FANUC robots via RMI protocol
- **External Device Polling**: Continuously poll sensors, PLCs, or other equipment
- **Database Operations**: Async SQLite or network database operations
- **WebSocket Clients**: Background WebSocket connections to external services

## Version Compatibility

| pl3xus_async | bevy | tokio |
|--------------|------|-------|
| 0.1.0        | 0.17 | 1.x   |
