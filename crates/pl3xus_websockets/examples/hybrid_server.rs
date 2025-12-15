//! Hybrid Server Example
//!
//! This example demonstrates a server that supports BOTH TCP and WebSocket protocols simultaneously.
//! Clients from either protocol can connect and share a common chat room.
//!
//! The example showcases two different architectural patterns controlled by feature flags:
//!
//! ## Immediate Pattern (feature = "immediate")
//! - Application logic directly uses Network<T> resources to send messages
//! - Simple and direct, provides maximum control
//! - Couples application logic to network infrastructure
//!
//! ## Scheduled Pattern (feature = "scheduled", default)
//! - Application logic writes OutboundMessage<T> events
//! - Relay system broadcasts messages in a deterministic system set
//! - Complete decoupling of application logic from network infrastructure
//! - Deterministic message timing
//!
//! Run with:
//! ```bash
//! # Scheduled pattern (default)
//! cargo run --example hybrid_server --package pl3xus_websockets
//!
//! # Immediate pattern
//! cargo run --example hybrid_server --package pl3xus_websockets --features immediate
//! ```

use bevy::tasks::TaskPool;
use bevy::{prelude::*, tasks::TaskPoolBuilder};
use pl3xus::{AppNetworkMessage, Pl3xusRuntime, Network};
use pl3xus::tcp::{NetworkSettings as TcpNetworkSettings, TcpProvider};
use pl3xus_websockets::{NetworkSettings as WsNetworkSettings, WebSocketProvider};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

mod shared_types;

// Import the appropriate message handling plugin based on feature flags
#[cfg(feature = "immediate")]
mod immediate_messages;
#[cfg(feature = "immediate")]
use immediate_messages::ImmediateMsgPlugin;

#[cfg(not(feature = "immediate"))]  // Default to scheduled
mod scheduled_messages;
#[cfg(not(feature = "immediate"))]
use scheduled_messages::ScheduledMsgPlugin;

fn main() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::log::LogPlugin::default()));

    // Add BOTH Pl3xusPlugins - one for TCP, one for WebSocket
    // These create separate Network<TcpProvider> and Network<WebSocketProvider> resources
    app.add_plugins(pl3xus::Pl3xusPlugin::<
        TcpProvider,
        bevy::tasks::TaskPool,
    >::default());

    app.add_plugins(pl3xus::Pl3xusPlugin::<
        WebSocketProvider,
        bevy::tasks::TaskPool,
    >::default());

    // Shared runtime for both
    app.insert_resource(Pl3xusRuntime(
        TaskPoolBuilder::new().num_threads(4).build(),
    ));

    // Insert settings for both providers
    app.insert_resource(TcpNetworkSettings::default());
    app.insert_resource(WsNetworkSettings::default());

    // Register incoming messages (what clients send to server)
    app.register_network_message::<shared_types::UserChatMessage, TcpProvider>();
    app.register_network_message::<shared_types::UserChatMessage, WebSocketProvider>();

    // Add the appropriate message handling plugin based on feature flags
    #[cfg(feature = "immediate")]
    {
        info!("🚀 Starting hybrid server with IMMEDIATE message pattern");
        app.add_plugins(ImmediateMsgPlugin);
    }

    #[cfg(not(feature = "immediate"))]
    {
        info!("🚀 Starting hybrid server with SCHEDULED message pattern");
        app.add_plugins(ScheduledMsgPlugin);
    }

    app.add_systems(Startup, setup_networking);

    app.run();
}

/// Setup networking for both TCP and WebSocket servers
fn setup_networking(
    mut tcp_net: ResMut<Network<TcpProvider>>,
    mut ws_net: ResMut<Network<WebSocketProvider>>,
    tcp_settings: Res<TcpNetworkSettings>,
    ws_settings: Res<WsNetworkSettings>,
    task_pool: Res<Pl3xusRuntime<TaskPool>>,
) {
    // Start TCP server on port 3030
    match tcp_net.listen(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 3030),
        &task_pool.0,
        &tcp_settings,
    ) {
        Ok(_) => info!("📡 TCP server listening on 127.0.0.1:3030"),
        Err(err) => {
            error!("Could not start TCP server: {}", err);
            panic!();
        }
    }

    // Start WebSocket server on port 8081
    match ws_net.listen(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8081),
        &task_pool.0,
        &ws_settings,
    ) {
        Ok(_) => info!("🌐 WebSocket server listening on 127.0.0.1:8081"),
        Err(err) => {
            error!("Could not start WebSocket server: {}", err);
            panic!();
        }
    }

    info!("🚀 Hybrid server started! Accepting both TCP and WebSocket connections.");
}