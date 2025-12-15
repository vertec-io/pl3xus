use bevy::prelude::*;
use pl3xus::tcp::TcpProvider;
use serde::{Deserialize, Serialize};

/////////////////////////////////////////////////////////////////////
// Shared message types for the client/server examples.
//
// By defining these in a separate library crate that both binaries
// depend on, we ensure consistent type names for networking across
// different binaries.
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OutboundTestMessage {
    pub content: String,
}

// Custom system set for outbound messages
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub struct OutboundMessageSet;

pub fn client_register_network_messages(app: &mut App) {
    use pl3xus::AppNetworkMessage;

    // The client registers messages that arrives from the server, so that
    // it is prepared to handle them. Otherwise, an error occurs.
    app.register_network_message::<NewChatMessage, TcpProvider>();

    // Register outbound message that the client will send using MessageWriter
    // This will process outbound messages in the OutboundMessageSet system set
    app.register_outbound_message::<OutboundTestMessage, TcpProvider, _>(OutboundMessageSet);
}

pub fn server_register_network_messages(app: &mut App) {
    use pl3xus::AppNetworkMessage;

    // The server registers messages that arrives from a client, so that
    // it is prepared to handle them. Otherwise, an error occurs.
    app.register_network_message::<UserChatMessage, TcpProvider>();

    // Register the outbound test message from clients
    app.register_network_message::<OutboundTestMessage, TcpProvider>();
}

