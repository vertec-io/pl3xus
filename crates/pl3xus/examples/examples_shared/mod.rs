use bevy::prelude::*;
use pl3xus::tcp::TcpProvider;
use serde::{Deserialize, Serialize};

/////////////////////////////////////////////////////////////////////
// Shared message types for the client/server examples.
//
// By defining these in a separate module that both binaries include
// with the same path, we ensure consistent type names for networking.
/////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserChatMessage {
    pub message: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NewChatMessage {
    pub name: String,
    pub message: String,
}

pub fn client_register_network_messages(app: &mut App) {
    use pl3xus::AppNetworkMessage;

    // The client registers messages that arrives from the server, so that
    // it is prepared to handle them. Otherwise, an error occurs.
    app.register_network_message::<NewChatMessage, TcpProvider>();
}

pub fn server_register_network_messages(app: &mut App) {
    use pl3xus::AppNetworkMessage;

    // The server registers messages that arrives from a client, so that
    // it is prepared to handle them. Otherwise, an error occurs.
    app.register_network_message::<UserChatMessage, TcpProvider>();
}

