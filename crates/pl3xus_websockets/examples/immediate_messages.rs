//! Immediate message handling plugin for the hybrid server.
//!
//! This module demonstrates the "immediate" or "raw" approach to handling messages
//! in a multi-protocol server. Application logic systems directly use Network<T> resources
//! to send messages, providing maximum control and simplicity.
//!
//! **Trade-offs:**
//! - ✅ Simple and direct - no extra abstractions
//! - ✅ Full control over when messages are sent
//! - ❌ Application logic is coupled to Network resources
//! - ❌ Harder to test without network infrastructure
//! - ❌ Less deterministic - messages sent whenever systems run

use bevy::prelude::*;
use pl3xus::{Network, NetworkData, NetworkEvent};
use pl3xus::tcp::TcpProvider;
use pl3xus_websockets::WebSocketProvider;

use super::shared_types;

/// Plugin that implements immediate message handling.
///
/// This plugin adds systems that directly use Network<TcpProvider> and
/// Network<WebSocketProvider> resources to handle messages and broadcast them.
pub struct ImmediateMsgPlugin;

impl Plugin for ImmediateMsgPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            handle_connection_events,
            handle_messages,
        ));
    }
}

/// Unified connection event handler that processes events from BOTH TCP and WebSocket networks.
///
/// This demonstrates the immediate pattern: we directly check Network resources to determine
/// which protocol the connection belongs to.
fn handle_connection_events(
    tcp_net: Res<Network<TcpProvider>>,
    ws_net: Res<Network<WebSocketProvider>>,
    mut network_events: MessageReader<NetworkEvent>,
) {
    for event in network_events.read() {
        match event {
            NetworkEvent::Connected(conn_id) => {
                // Determine which network this connection belongs to
                // Check WebSocket first since it's more specific
                let is_ws = ws_net.has_connection(*conn_id);
                let is_tcp = tcp_net.has_connection(*conn_id);

                if is_ws {
                    info!("🌐 WebSocket client connected: {} (Total TCP: {}, WS: {})",
                        conn_id, tcp_net.connection_count(), ws_net.connection_count());
                } else if is_tcp {
                    info!("📡 TCP client connected: {} (Total TCP: {}, WS: {})",
                        conn_id, tcp_net.connection_count(), ws_net.connection_count());
                } else {
                    warn!("Connection event for unknown connection: {}", conn_id);
                }
            }
            NetworkEvent::Disconnected(conn_id) => {
                // After disconnection, the connection is already removed from the Network resource,
                // so we can't use has_connection(). We'll just log the disconnect without protocol prefix.
                info!("Client disconnected: {} (Total TCP: {}, WS: {})",
                    conn_id, tcp_net.connection_count(), ws_net.connection_count());
            }
            NetworkEvent::Error(err) => {
                error!("Network error: {}", err);
            }
        }
    }
}

/// Message handler that directly broadcasts using Network resources.
///
/// This demonstrates the immediate pattern: application logic directly uses Network<T> resources
/// to send messages. This provides maximum control but couples application logic to network infrastructure.
fn handle_messages(
    mut new_messages: MessageReader<NetworkData<shared_types::UserChatMessage>>,
    tcp_net: Res<Network<TcpProvider>>,
    ws_net: Res<Network<WebSocketProvider>>,
) {
    for message in new_messages.read() {
        let sender_id = message.source();
        let provider = message.provider_name();

        // Determine log emoji based on provider
        let log_emoji = if provider == "TCP" { "📡" } else { "🌐" };

        info!("{} Received {} message from {}: {}", log_emoji, provider, sender_id, message.message);

        // Create the broadcast message with protocol prefix
        let broadcast_message = shared_types::NewChatMessage {
            name: format!("{}-{}", provider, sender_id),
            message: message.message.clone(),
        };

        // Immediate pattern: Directly broadcast to both networks
        // This is simple and direct, but couples application logic to Network resources
        tcp_net.broadcast(broadcast_message.clone());
        ws_net.broadcast(broadcast_message);
    }
}

