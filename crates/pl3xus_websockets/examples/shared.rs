use bevy::prelude::*;
use pl3xus_websockets::WebSocketProvider;

/////////////////////////////////////////////////////////////////////
// In this example the client sends `UserChatMessage`s to the server,
// the server then broadcasts to all connected clients.
//
// We use two different types here, because only the server should
// decide the identity of a given connection and thus also sends a
// name.
//
// You can have a single message be sent both ways, it simply needs
// to be Serialize + Deserialize and both client and server can
// send and receive
/////////////////////////////////////////////////////////////////////

// Import shared types from shared_types.rs
// This ensures consistent type names across all clients and servers
#[path = "shared_types.rs"]
mod shared_types;
pub use shared_types::{UserChatMessage, NewChatMessage};

#[allow(unused)]
pub fn client_register_network_messages(app: &mut App) {
    use pl3xus::AppNetworkMessage;

    // The client registers messages that arrives from the server, so that
    // it is prepared to handle them. Otherwise, an error occurs.
    app.register_network_message::<NewChatMessage, WebSocketProvider>();
}

#[allow(unused)]
pub fn server_register_network_messages(app: &mut App) {
    use pl3xus::AppNetworkMessage;

    // The server registers messages that arrives from a client, so that
    // it is prepared to handle them. Otherwise, an error occurs.
    app.register_network_message::<UserChatMessage, WebSocketProvider>();
}
