//! Basic example server for pl3xus_client
//!
//! This example demonstrates a simple Bevy server that:
//! - Spawns entities with Position, Velocity, and EntityName components
//! - Moves entities based on their velocity
//! - Broadcasts component changes to connected clients via pl3xus_sync
//!
//! Run with: cargo run -p pl3xus_client --example basic_server

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use bevy::tasks::{TaskPool, TaskPoolBuilder};
use pl3xus::{Pl3xusRuntime, Network};
use pl3xus_sync::{AppPl3xusSyncExt, Pl3xusSyncPlugin};
use pl3xus_websockets::{NetworkSettings, WebSocketProvider};

use basic_types::{EntityName, Position, Velocity};

fn main() {
    let mut app = App::new();

    // Configure MinimalPlugins with a schedule runner that runs at 60 FPS. To slow it down, use 4.0
    app.add_plugins(
        MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
            1.0/60.0, // 0.25 Hz
        ))),
    );
    app.add_plugins(bevy::log::LogPlugin::default());

    // Pl3xus networking over WebSockets
    app.add_plugins(pl3xus::Pl3xusPlugin::<WebSocketProvider, TaskPool>::default());
    app.insert_resource(Pl3xusRuntime(TaskPoolBuilder::new().num_threads(2).build()));
    app.insert_resource(NetworkSettings::default());

    // Install the sync middleware
    app.add_plugins(Pl3xusSyncPlugin::<WebSocketProvider>::default());

    // Register components for synchronization
    app.sync_component::<Position>(None);
    app.sync_component::<Velocity>(None);
    app.sync_component::<EntityName>(None);

    app.add_systems(Startup, (setup, setup_networking));
    app.add_systems(Update, move_entities);

    app.run();
}

fn setup(mut commands: Commands) {
    info!("Starting basic pl3xus_client example server");

    // Spawn some entities with position, velocity, and names
    commands.spawn((
        Position { x: 0.0, y: 0.0 },
        Velocity { x: 1.0, y: 0.5 },
        EntityName {
            name: "Entity A".to_string(),
        },
    ));

    commands.spawn((
        Position { x: 100.0, y: 50.0 },
        Velocity { x: -0.5, y: 1.0 },
        EntityName {
            name: "Entity B".to_string(),
        },
    ));

    commands.spawn((
        Position { x: -50.0, y: 100.0 },
        Velocity { x: 0.3, y: -0.8 },
        EntityName {
            name: "Entity C".to_string(),
        },
    ));
}

fn setup_networking(
    mut net: ResMut<Network<WebSocketProvider>>,
    settings: Res<NetworkSettings>,
    task_pool: Res<Pl3xusRuntime<TaskPool>>,
) {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 3000);

    match net.listen(addr, &task_pool.0, &settings) {
        Ok(_) => info!("Server listening on ws://{addr}/sync"),
        Err(err) => {
            error!("Could not start listening: {err}");
            panic!("Failed to bind WebSocket listener");
        }
    }
}

fn move_entities(mut query: Query<(&mut Position, &Velocity)>, time: Res<Time>) {
    for (mut pos, vel) in query.iter_mut() {
        pos.x += vel.x * time.delta_secs() * 10.0;
        pos.y += vel.y * time.delta_secs() * 10.0;

        // Wrap around screen bounds
        if pos.x > 200.0 {
            pos.x = -200.0;
        }
        if pos.x < -200.0 {
            pos.x = 200.0;
        }
        if pos.y > 200.0 {
            pos.y = -200.0;
        }
        if pos.y < -200.0 {
            pos.y = 200.0;
        }
    }
}

