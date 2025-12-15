use bevy::prelude::*;
use bevy::tasks::TaskPoolBuilder;
use pl3xus::{
    AppNetworkMessage, Pl3xusRuntime, Network, NetworkEvent, OutboundMessage,
    SubscriptionMessage,
};
use pl3xus_common::{Pl3xusMessage, SubscribeById};
use pl3xus_memory::NetworkMemoryPlugin;
use pl3xus_websockets::{NetworkSettings, WebSocketProvider};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;
use tracing::error;

// Define subscription message types
#[derive(SubscribeById, Serialize, Deserialize, Clone, Debug)]
struct TestUpdate {
    pub data: String,
}

// Component to track connected clients
#[derive(Component)]
struct ConnectedClient {
    id: pl3xus::ConnectionId,
    #[allow(dead_code)]
    connected_at: f64,
}

fn main() {
    let mut app = App::new();

    // Add only the minimal required plugins
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::log::LogPlugin::default());

    // Add the pl3xus plugin with minimal configuration
    app.add_plugins(pl3xus::Pl3xusPlugin::<
        WebSocketProvider,
        bevy::tasks::TaskPool,
    >::default())
        .insert_resource(Pl3xusRuntime(TaskPoolBuilder::new().build()))
        .insert_resource(NetworkSettings::default());

    // Add our memory leak detection plugin
    app.add_plugins(NetworkMemoryPlugin);

    // Register subscription message types using Pl3xusMessage
    app.register_subscription::<TestUpdate, WebSocketProvider>();

    // Add only the essential networking system
    app.add_systems(Startup, setup_networking).add_systems(
        Update,
        (
            handle_connection_events,
            log_subscription_stats,
            send_periodic_updates,
        ),
    );

    println!("Starting minimal server with subscription messages...");
    app.run();
}

fn setup_networking(
    mut net: ResMut<Network<WebSocketProvider>>,
    settings: Res<NetworkSettings>,
    task_pool: Res<Pl3xusRuntime<bevy::tasks::TaskPool>>,
) {
    let socket_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8081);
    println!("Starting server on {}", socket_addr);

    match net.listen(socket_addr, &task_pool.0, &settings) {
        Ok(_) => println!("Server listening successfully"),
        Err(err) => {
            error!("Failed to start server: {}", err);
            panic!("Server startup failed");
        }
    }
}

fn handle_connection_events(
    mut commands: Commands,
    mut network_events: MessageReader<NetworkEvent>,
    time: Res<Time>,
    clients: Query<(Entity, &ConnectedClient)>,
) {
    for event in network_events.read() {
        match event {
            NetworkEvent::Connected(conn_id) => {
                println!("Client connected: {}", conn_id);

                // Spawn an entity to track this client
                commands.spawn(ConnectedClient {
                    id: *conn_id,
                    connected_at: time.elapsed_secs() as f64,
                });
            }
            NetworkEvent::Disconnected(conn_id) => {
                println!("Client disconnected: {}", conn_id);

                // Remove the client entity
                for (entity, client) in clients.iter() {
                    if client.id == *conn_id {
                        commands.entity(entity).despawn();
                    }
                }
            }
            NetworkEvent::Error(err) => println!("Network error: {:?}", err),
        }
    }
}

// System to periodically send updates to all connected clients
fn send_periodic_updates(
    time: Res<Time>,
    mut update_timer: Local<Option<Timer>>,
    clients: Query<&ConnectedClient>,
    mut outbound_messages: MessageWriter<OutboundMessage<TestUpdate>>,
) {
    // Initialize timer if needed
    if update_timer.is_none() {
        *update_timer = Some(Timer::new(Duration::from_secs(5), TimerMode::Repeating));
    }

    // Update timer
    let timer = update_timer.as_mut().unwrap();
    timer.tick(time.delta());

    // Send updates when timer finishes
    if timer.just_finished() {
        let timestamp = time.elapsed_secs();
        let update = TestUpdate {
            data: format!("Server update at {:.2}s", timestamp),
        };

        // Send to all connected clients
        for client in clients.iter() {
            let outbound = OutboundMessage::new(TestUpdate::type_name().to_string(), update.clone())
                .for_client(client.id);
            outbound_messages.write(outbound);
            println!("Sent update to client {}", client.id);
        }
    }
}

// System to log subscription-related stats
fn log_subscription_stats(
    time: Res<Time>,
    _network: Res<Network<WebSocketProvider>>,
    clients: Query<&ConnectedClient>,
) {
    static mut LAST_LOG: Option<f64> = None;

    let current_time = time.elapsed_secs() as f64;
    let should_log = unsafe {
        match LAST_LOG {
            Some(last_time) if current_time - last_time < 10.0 => false,
            _ => {
                LAST_LOG = Some(current_time);
                true
            }
        }
    };

    if should_log {
        println!("=== SUBSCRIPTION STATS ===");
        println!("Connected clients: {}", clients.iter().count());
        // println!("Message map entries: {}", network.recv_message_map.len());

        // Log the message types that are registered
        println!("Registered message types:");
        // for (msg_type, _) in network.recv_message_map.iter() {
        //     println!("  - {}", msg_type);
        // }

        println!("=========================");
    }
}
