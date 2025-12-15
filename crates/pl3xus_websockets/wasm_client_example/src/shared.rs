use bevy::prelude::*;
use pl3xus_mod_websockets::WebSocketProvider;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserChatMessage {
    pub message: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NewChatMessage {
    pub name: String,
    pub message: String,
}

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
