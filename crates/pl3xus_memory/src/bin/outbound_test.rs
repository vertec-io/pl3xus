use bevy::prelude::*;
use bevy::tasks::TaskPoolBuilder;
use pl3xus::{
    AppNetworkMessage, Pl3xusRuntime, Network, NetworkEvent, OutboundMessage,
};
use pl3xus_memory::NetworkMemoryPlugin;
use pl3xus_websockets::{NetworkSettings, WebSocketProvider};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tracing::error;

// Define a custom system set for outbound messages
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
struct OutboundMessageSet;

// Define a simple message type
#[derive(Serialize, Deserialize, Clone, Debug)]
struct TestMessage {
    pub data: String,
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

    // Configure our custom system set
    app.configure_sets(Update, OutboundMessageSet);

    // ONLY register the outbound message type - don't send any
    app.register_outbound_message::<TestMessage, WebSocketProvider, _>(OutboundMessageSet);

    // Add only the essential networking system
    app.add_systems(Startup, setup_networking)
        .add_systems(Update, log_events);

    println!("Starting outbound message test...");
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

fn log_events(
    mut events: MessageReader<NetworkEvent>,
    time: Res<Time>,
    outbound_queue: MessageReader<OutboundMessage<TestMessage>>,
) {
    for event in events.read() {
        match event {
            NetworkEvent::Connected(id) => println!("Client connected: {}", id),
            NetworkEvent::Disconnected(id) => println!("Client disconnected: {}", id),
            NetworkEvent::Error(err) => println!("Network error: {:?}", err),
        }
    }

    // Log the outbound queue size periodically
    static mut LAST_LOG: Option<f64> = None;
    let current_time = time.elapsed_secs() as f64;

    let should_log = unsafe {
        match LAST_LOG {
            Some(last_time) if current_time - last_time < 5.0 => false,
            _ => {
                LAST_LOG = Some(current_time);
                true
            }
        }
    };

    if should_log {
        println!("Outbound queue size: {}", outbound_queue.len());

        // Check if we have a memory leak by looking at the system memory
        #[cfg(target_os = "windows")]
        {
            use std::process;
            let pid = process::id();
            let output = std::process::Command::new("powershell")
                .args([
                    "-Command",
                    &format!("Get-Process -Id {} | Select-Object WorkingSet", pid),
                ])
                .output()
                .expect("Failed to execute powershell command");

            let output_str = String::from_utf8_lossy(&output.stdout);
            println!("Memory usage: {}", output_str);
        }
    }
}

// Add a system to clear the outbound message queue
#[allow(dead_code)]
fn clear_outbound_queue(
    mut outbound_queue: MessageReader<OutboundMessage<TestMessage>>,
    network: Res<Network<WebSocketProvider>>,
) {
    // Only clear if there are no connections
    if !network.has_connections() && !outbound_queue.is_empty() {
        println!(
            "Clearing {} outbound messages with no connections",
            outbound_queue.len()
        );

        // Read all messages to clear the queue
        for _ in outbound_queue.read() {
            // Just iterate to clear the queue
        }
    }
}
