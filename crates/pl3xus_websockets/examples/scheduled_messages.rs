//! Scheduled message handling plugin for the hybrid server.
//!
//! This module demonstrates the "scheduled" or "decoupled" approach to handling messages
//! in a multi-protocol server. Application logic writes OutboundMessage<T> events, and the built-in
//! relay system handles the actual network broadcasting in a deterministic system set.
//!
//! **Trade-offs:**
//! - ✅ Complete decoupling - application logic has no Network dependencies
//! - ✅ Deterministic - all messages sent at the same point in the frame
//! - ✅ Easy to test - application logic can be tested without network infrastructure
//! - ✅ Flexible - easy to add new protocols without changing application logic
//! - ❌ Slightly more complex - requires understanding of system sets and OutboundMessage

use bevy::prelude::*;
use pl3xus::{AppNetworkMessage, Network, NetworkData, NetworkEvent, OutboundMessage};
use pl3xus::tcp::TcpProvider;
use pl3xus_websockets::WebSocketProvider;

use super::shared_types;

/// Plugin that implements scheduled message handling.
///
/// This plugin uses the built-in `register_outbound_message` method which automatically
/// sets up the relay system for each provider. This is the recommended approach!
pub struct ScheduledMsgPlugin;

impl Plugin for ScheduledMsgPlugin {
    fn build(&self, app: &mut App) {
        // Define system sets for deterministic message handling
        app.configure_sets(Update, (
            AppLogic,
            NetworkRelay.after(AppLogic),
        ));

        // Register outbound messages for BOTH providers
        // This automatically sets up the relay_outbound system for each provider
        app.register_outbound_message::<shared_types::NewChatMessage, TcpProvider, _>(NetworkRelay.clone());
        app.register_outbound_message::<shared_types::NewChatMessage, WebSocketProvider, _>(NetworkRelay.clone());

        // Add connection event handler (not part of the message flow)
        app.add_systems(Update, handle_connection_events);

        // Add application logic system (reads messages, writes OutboundMessage)
        app.add_systems(Update, handle_messages.in_set(AppLogic));
    }
}

/// System set for application logic that processes incoming messages
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
struct AppLogic;

/// System set for network relay that broadcasts outbound messages
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
struct NetworkRelay;

/// Unified connection event handler that processes events from BOTH TCP and WebSocket networks.
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

/// Application logic that processes incoming messages and writes OutboundMessage events.
///
/// This demonstrates the scheduled pattern: application logic has ZERO dependencies on Network resources!
/// It simply reads incoming messages and writes outbound messages. The relay system handles the rest.
fn handle_messages(
    mut new_messages: MessageReader<NetworkData<shared_types::UserChatMessage>>,
    mut outbound: MessageWriter<OutboundMessage<shared_types::NewChatMessage>>,
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

        // Scheduled pattern: Write OutboundMessage - the built-in relay system handles broadcasting!
        // This completely decouples application logic from network infrastructure
        // The relay_outbound system (registered via register_outbound_message) will automatically
        // broadcast this message to all clients on both TCP and WebSocket providers
        outbound.write(OutboundMessage {
            name: "chat".to_string(),
            message: broadcast_message,
            for_client: None,  // None means broadcast to all
        });
    }
}

