/// This example demonstrates the new automatic message registration API.
/// 
/// Unlike the traditional approach that requires implementing NetworkMessage,
/// this example shows how to use any Serialize + Deserialize type directly
/// as a network message without boilerplate.
/// 
/// Run the server with:
/// ```
/// cargo run --example automatic_messages -- server
/// ```
/// 
/// Run the client with:
/// ```
/// cargo run --example automatic_messages -- client
/// ```

use bevy::prelude::*;
use bevy::tasks::TaskPoolBuilder;
use pl3xus::{
    AppNetworkMessage, ConnectionId, Pl3xusPlugin, Pl3xusRuntime, Network, NetworkData,
    NetworkEvent, tcp::{NetworkSettings, TcpProvider},
};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

// ============================================================================
// AUTOMATIC MESSAGES - No NetworkMessage implementation needed!
// ============================================================================

/// A simple chat message - just derive Serialize and Deserialize
#[derive(Serialize, Deserialize, Clone, Debug)]
struct ChatMessage {
    username: String,
    message: String,
}

/// Player position update - no boilerplate needed
#[derive(Serialize, Deserialize, Clone, Debug)]
struct PlayerPosition {
    player_id: u32,
    x: f32,
    y: f32,
}

/// Server announcement - works with any serializable type
#[derive(Serialize, Deserialize, Clone, Debug)]
struct ServerAnnouncement {
    text: String,
    priority: u8,
}

// ============================================================================
// SERVER
// ============================================================================

fn setup_server(mut commands: Commands) {
    commands.spawn(Camera2d);
    
    commands.spawn((
        Text::new("Automatic Messages Server\nPress SPACE to send announcement"),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}

fn server_listen(
    mut net: ResMut<Network<TcpProvider>>,
    settings: Res<NetworkSettings>,
    runtime: Res<Pl3xusRuntime<bevy::tasks::TaskPool>>,
) {
    let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    let socket_address = SocketAddr::new(ip, 9999);

    match net.listen(socket_address, &runtime.0, &settings) {
        Ok(_) => info!("Server listening on {}", socket_address),
        Err(err) => error!("Failed to start server: {}", err),
    }
}

fn server_handle_connections(mut events: MessageReader<NetworkEvent>) {
    for event in events.read() {
        match event {
            NetworkEvent::Connected(conn_id) => {
                info!("Client connected: {:?}", conn_id);
            }
            NetworkEvent::Disconnected(conn_id) => {
                info!("Client disconnected: {:?}", conn_id);
            }
            NetworkEvent::Error(err) => {
                error!("Network error: {:?}", err);
            }
        }
    }
}

fn server_handle_chat_messages(
    mut messages: MessageReader<NetworkData<ChatMessage>>,
    net: Res<Network<TcpProvider>>,
) {
    for msg in messages.read() {
        info!("Received chat: {} says '{}'", msg.username, msg.message);

        // Broadcast to all clients (use ** to deref NetworkData)
        net.broadcast((**msg).clone());
    }
}

fn server_handle_positions(
    mut positions: MessageReader<NetworkData<PlayerPosition>>,
) {
    for pos in positions.read() {
        info!("Player {} moved to ({}, {})", pos.player_id, pos.x, pos.y);
    }
}

fn server_send_announcements(
    keyboard: Res<ButtonInput<KeyCode>>,
    net: Res<Network<TcpProvider>>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        let announcement = ServerAnnouncement {
            text: "Welcome to the automatic messages demo!".to_string(),
            priority: 1,
        };
        
        info!("Broadcasting announcement");
        net.broadcast(announcement);
    }
}

// ============================================================================
// CLIENT
// ============================================================================

fn setup_client(mut commands: Commands) {
    commands.spawn(Camera2d);
    
    commands.spawn((
        Text::new("Automatic Messages Client\nPress SPACE to send chat\nPress ENTER to send position"),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}

fn client_connect(
    net: ResMut<Network<TcpProvider>>,
    settings: Res<NetworkSettings>,
    runtime: Res<Pl3xusRuntime<bevy::tasks::TaskPool>>,
) {
    let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    let socket_address = SocketAddr::new(ip, 9999);

    net.connect(socket_address, &runtime.0, &settings);
    info!("Connecting to server at {}", socket_address);
}

fn client_handle_connections(mut events: MessageReader<NetworkEvent>) {
    for event in events.read() {
        match event {
            NetworkEvent::Connected(_) => {
                info!("Connected to server!");
            }
            NetworkEvent::Disconnected(_) => {
                info!("Disconnected from server");
            }
            NetworkEvent::Error(err) => {
                error!("Network error: {:?}", err);
            }
        }
    }
}

fn client_handle_chat_messages(mut messages: MessageReader<NetworkData<ChatMessage>>) {
    for msg in messages.read() {
        info!("Chat: {} says '{}'", msg.username, msg.message);
    }
}

fn client_handle_announcements(mut announcements: MessageReader<NetworkData<ServerAnnouncement>>) {
    for announcement in announcements.read() {
        info!("📢 Server announcement (priority {}): {}", 
              announcement.priority, announcement.text);
    }
}

fn client_send_messages(
    keyboard: Res<ButtonInput<KeyCode>>,
    net: Res<Network<TcpProvider>>,
) {
    let server_id = ConnectionId { id: 0 };
    
    if keyboard.just_pressed(KeyCode::Space) {
        let chat = ChatMessage {
            username: "Player1".to_string(),
            message: "Hello from automatic messages!".to_string(),
        };
        
        if let Err(err) = net.send(server_id, chat) {
            error!("Failed to send chat: {:?}", err);
        } else {
            info!("Sent chat message");
        }
    }
    
    if keyboard.just_pressed(KeyCode::Enter) {
        let position = PlayerPosition {
            player_id: 1,
            x: 100.0,
            y: 200.0,
        };
        
        if let Err(err) = net.send(server_id, position) {
            error!("Failed to send position: {:?}", err);
        } else {
            info!("Sent position update");
        }
    }
}

// ============================================================================
// MAIN
// ============================================================================

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let is_server = args.get(1).map(|s| s.as_str()) == Some("server");

    let mut app = App::new();
    
    // Add Bevy plugins
    app.add_plugins(DefaultPlugins);
    
    // Add Pl3xus plugin
    app.add_plugins(Pl3xusPlugin::<TcpProvider, bevy::tasks::TaskPool>::default());
    
    // Add runtime and settings
    app.insert_resource(Pl3xusRuntime(
        TaskPoolBuilder::new().num_threads(2).build(),
    ));
    app.insert_resource(NetworkSettings::default());

    // ========================================================================
    // REGISTER MESSAGES - No NetworkMessage implementation needed!
    // Just use register_network_message with any Serialize + Deserialize type
    // ========================================================================
    
    if is_server {
        info!("Starting server...");
        
        // Server receives chat messages and positions from clients
        app.register_network_message::<ChatMessage, TcpProvider>();
        app.register_network_message::<PlayerPosition, TcpProvider>();
        
        app.add_systems(Startup, (setup_server, server_listen));
        app.add_systems(Update, (
            server_handle_connections,
            server_handle_chat_messages,
            server_handle_positions,
            server_send_announcements,
        ));
    } else {
        info!("Starting client...");
        
        // Client receives chat messages and announcements from server
        app.register_network_message::<ChatMessage, TcpProvider>();
        app.register_network_message::<ServerAnnouncement, TcpProvider>();
        
        app.add_systems(Startup, (setup_client, client_connect));
        app.add_systems(Update, (
            client_handle_connections,
            client_handle_chat_messages,
            client_handle_announcements,
            client_send_messages,
        ));
    }

    app.run();
}

