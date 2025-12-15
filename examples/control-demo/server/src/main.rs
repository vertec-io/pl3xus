//! Control Demo Server
//!
//! This example demonstrates the ExclusiveControlPlugin for managing
//! exclusive control of entities across multiple clients.
//!
//! Run this server, then connect multiple clients to see how control
//! is managed when multiple clients try to control the same robot.

use bevy::prelude::*;
use bevy::tasks::{TaskPool, TaskPoolBuilder};
use control_demo_types::{MoveCommand, Robot, RobotStatus};
use pl3xus::{AppNetworkMessage, Pl3xusPlugin, Pl3xusRuntime};
use pl3xus_sync::control::{AppExclusiveControlExt, EntityControl, ExclusiveControlConfig, ExclusiveControlPlugin};
use pl3xus_sync::{AppPl3xusSyncExt, Pl3xusSyncPlugin};
use pl3xus_websockets::{NetworkSettings, WebSocketProvider};

fn main() {
    let mut app = App::new();

    // Add Bevy plugins
    app.add_plugins(bevy::log::LogPlugin::default());

    // Add pl3xus networking over WebSockets
    app.add_plugins(Pl3xusPlugin::<WebSocketProvider, TaskPool>::default());
    app.insert_resource(Pl3xusRuntime(
        TaskPoolBuilder::new().num_threads(2).build(),
    ));
    app.insert_resource(NetworkSettings::default());

    // Add pl3xus_sync plugin for component synchronization
    app.add_plugins(Pl3xusSyncPlugin::<WebSocketProvider>::default());

    // Add the ExclusiveControlPlugin
    app.add_plugins(ExclusiveControlPlugin::new(ExclusiveControlConfig {
        timeout_seconds: Some(30.0),  // 30 second timeout
        propagate_to_children: true,   // Control parent = control children
    }));

    // Add the control systems for WebSocket provider
    app.add_exclusive_control_systems::<WebSocketProvider>();

    // Register components for synchronization
    app.sync_component::<Robot>(None);
    app.sync_component::<RobotStatus>(None);
    app.sync_component::<EntityControl>(None); // Sync control state to clients

    // Register messages
    app.register_network_message::<MoveCommand, WebSocketProvider>();

    // Add application systems
    app.add_systems(Startup, spawn_robots);
    app.add_systems(Update, (handle_move_commands, update_robot_status));

    info!("🚀 Control Demo Server starting on ws://127.0.0.1:8083/sync");
    info!("📝 Connect multiple clients to test exclusive control");

    app.run();
}

/// Spawn some robots for clients to control
fn spawn_robots(mut commands: Commands) {
    commands.spawn((
        Robot {
            name: "Robot A".to_string(),
            x: 100.0,
            y: 100.0,
        },
        RobotStatus {
            battery: 100.0,
            is_moving: false,
        },
    ));

    commands.spawn((
        Robot {
            name: "Robot B".to_string(),
            x: 200.0,
            y: 200.0,
        },
        RobotStatus {
            battery: 100.0,
            is_moving: false,
        },
    ));

    commands.spawn((
        Robot {
            name: "Robot C".to_string(),
            x: 300.0,
            y: 300.0,
        },
        RobotStatus {
            battery: 100.0,
            is_moving: false,
        },
    ));

    info!("✅ Spawned 3 robots");
}

/// Handle move commands from clients
fn handle_move_commands(
    mut message_reader: bevy::ecs::message::MessageReader<pl3xus::NetworkData<MoveCommand>>,
    mut robots: Query<(Entity, &mut Robot, &mut RobotStatus, &mut EntityControl)>,
    time: Res<Time>,
) {
    for cmd in message_reader.read() {
        let client_id = *cmd.source();
        let entity = Entity::from_bits(cmd.entity_id);

        // Find the robot
        let Ok((entity, mut robot, mut status, mut control)) = robots.get_mut(entity) else {
            warn!("Client {:?} tried to move non-existent robot {:?}", client_id, entity);
            continue;
        };

        // Check if the client has control
        if control.client_id != client_id {
            warn!(
                "Client {:?} tried to move robot {:?} but it's controlled by {:?}",
                client_id, entity, control.client_id
            );
            continue;
        }

        // Move the robot
        robot.x = cmd.target_x;
        robot.y = cmd.target_y;
        status.is_moving = true;

        info!(
            "✅ Client {:?} moved {} to ({}, {})",
            client_id, robot.name, robot.x, robot.y
        );

        // Update last activity timestamp
        control.last_activity = time.elapsed_secs();
    }
}

/// Update robot status (simulate battery drain, etc.)
fn update_robot_status(mut robots: Query<&mut RobotStatus>, time: Res<Time>) {
    for mut status in robots.iter_mut() {
        if status.is_moving {
            status.battery -= 0.1 * time.delta_secs();
            if status.battery <= 0.0 {
                status.battery = 0.0;
                status.is_moving = false;
            }
        }
    }
}

