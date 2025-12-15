use bevy::prelude::*;
use pl3xus::tcp::TcpProvider;
use serde::{Deserialize, Serialize};

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
//
// NOTE: For cross-binary communication (client/server in separate binaries),
// the auto-generated type names will include the binary name in the path
// (e.g., "client::shared::NewChatMessage" vs "server::shared::NewChatMessage").
// In a real application, you would put shared types in a common library crate
// that both binaries depend on, ensuring consistent type paths.
// For this example, we're keeping it simple with separate binaries.
/////////////////////////////////////////////////////////////////////

// Define the message types in a nested module to give them a consistent path
pub mod messages {
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
}

// Re-export for convenience
pub use messages::*;

#[allow(unused)]
pub fn client_register_network_messages(app: &mut App) {
    use pl3xus::AppNetworkMessage;

    // The client registers messages that arrives from the server, so that
    // it is prepared to handle them. Otherwise, an error occurs.
    app.register_network_message::<NewChatMessage, TcpProvider>();
}

#[allow(unused)]
pub fn server_register_network_messages(app: &mut App) {
    use pl3xus::AppNetworkMessage;

    // The server registers messages that arrives from a client, so that
    // it is prepared to handle them. Otherwise, an error occurs.
    app.register_network_message::<UserChatMessage, TcpProvider>();
}
