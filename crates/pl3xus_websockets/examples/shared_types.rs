use serde::{Deserialize, Serialize};

/////////////////////////////////////////////////////////////////////
// Shared message types for WebSocket examples.
//
// These types are defined in a separate file without Bevy dependencies
// so they can be shared between Bevy clients/servers and non-Bevy
// clients like the Leptos web client.
//
// By importing from the same module path, we ensure that
// std::any::type_name() returns consistent type names across all
// clients and servers, which is required for the Pl3xusMessage
// type registry to work correctly.
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
