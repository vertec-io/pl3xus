#![deny(
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unstable_features,
    unused_import_braces,
    unused_qualifications,
    clippy::unwrap_used
)]
#![allow(clippy::type_complexity)]
// //

/*!
A simple networking plugin for Bevy designed to work with Bevy's event architecture.

Using this plugin is meant to be straightforward and highly configurable.
You simply add the `Pl3xusPlugin` to the respective bevy app with the runtime you wish to use
and the networking provider you wish to use. Next add your runtime to the app as the [`Pl3xusRuntime`] Resource.
Then, register which kind of messages can be received through [`managers::network::AppNetworkMessage::register_network_message`],
as well as which provider you want to handle these messages and you
can start receiving packets as events of [`NetworkData<T>`].

This plugin also supports Request/Response style messages, see that modules documentation for further info: **[Request Documentation](https://docs.rs/pl3xus/latest/pl3xus/managers/network_request/index.html)**

## Example Client
```rust,no_run
use bevy::prelude::*;
use pl3xus::{Pl3xusRuntime, Pl3xusPlugin, NetworkData, NetworkEvent, AppNetworkMessage, tcp::TcpProvider,tcp::NetworkSettings};
use serde::{Serialize, Deserialize};
use bevy::tasks::TaskPoolBuilder;

#[derive(Serialize, Deserialize)]
struct WorldUpdate;

fn main() {
     let mut app = App::new();
     app.add_plugins(Pl3xusPlugin::<
        TcpProvider,
        bevy::tasks::TaskPool,
    >::default());

    //Insert our runtime and the neccessary settings for the TCP transport
    app.insert_resource(Pl3xusRuntime(
        TaskPoolBuilder::new().num_threads(2).build(),
    ));
    app.insert_resource(NetworkSettings::default());

    // We are receiving this from the server, so we need to register it
     app.register_network_message::<WorldUpdate, TcpProvider>();
     app.add_systems(Update, (handle_world_updates,handle_connection_events));
}

fn handle_world_updates(
    mut chunk_updates: MessageReader<NetworkData<WorldUpdate>>,
) {
    for chunk in chunk_updates.read() {
        info!("Got chunk update!");
    }
}

fn handle_connection_events(mut network_events: MessageReader<NetworkEvent>,) {
    for event in network_events.read() {
        match event {
            &NetworkEvent::Connected(_) => info!("Connected to server!"),
            _ => (),
        }
    }
}

```

## Example Server
```rust,no_run
use bevy::prelude::*;
use pl3xus::{Pl3xusRuntime,
    Pl3xusPlugin, NetworkData,
    Network, NetworkEvent, AppNetworkMessage,
    tcp::TcpProvider,tcp::NetworkSettings
};
use bevy::tasks::TaskPoolBuilder;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct UserInput;

#[derive(Serialize, Deserialize)]
struct PlayerUpdate;

fn main() {
     let mut app = App::new();
     app.add_plugins(Pl3xusPlugin::<
        TcpProvider,
        bevy::tasks::TaskPool,
    >::default());


    //Insert our runtime and the neccessary settings for the TCP transport
    app.insert_resource(Pl3xusRuntime(
        TaskPoolBuilder::new().num_threads(2).build(),
    ));
    app.insert_resource(NetworkSettings::default());

     // We are receiving this from a client, so we need to register it!
     app.register_network_message::<UserInput, TcpProvider>();
     app.add_systems(Update, (handle_world_updates,handle_connection_events));
}

fn handle_world_updates(
    mut chunk_updates: MessageReader<NetworkData<UserInput>>,
) {
    for chunk in chunk_updates.read() {
        info!("Got chunk update!");
    }
}

fn handle_connection_events(
    net: Res<Network<TcpProvider>>,
    mut network_events: MessageReader<NetworkEvent>,
) {
    for event in network_events.read() {
        match event {
            &NetworkEvent::Connected(conn_id) => {
                net.send(conn_id, PlayerUpdate);
                info!("New client connected: {:?}", conn_id);
            }
            _ => (),
        }
    }
}

```
As you can see, they are both quite similar, and provide everything a basic networked application needs.

Currently, Bevy's [TaskPool](bevy::tasks::TaskPool) is the default runtime used by Pl3xus.
*/

/// Contains error enum.
// pub mod error;
// mod network_message;
/// Contains all functionality for starting a server or client, sending, and recieving messages from clients.
pub mod managers;
pub use managers::{Network, network::AppNetworkMessage};
pub use managers::registration::{register_message, register_message_unscheduled};
pub use managers::network_request::DeferredResponder;
mod runtime;
use managers::NetworkProvider;
pub use runtime::Pl3xusRuntime;
use runtime::JoinHandle;
pub use runtime::Runtime;

use std::{fmt::Debug, marker::PhantomData};

pub use async_channel;
use async_channel::{Receiver, Sender, unbounded};
pub use async_trait::async_trait;
use bevy::prelude::*;

pub use pl3xus_common::error;
use pl3xus_common::error::NetworkError;

pub use pl3xus_common::*;

use std::ops::Deref;

#[cfg(feature = "tcp")]
/// A default tcp provider to help get you started.
pub mod tcp;

struct AsyncChannel<T> {
    pub(crate) sender: Sender<T>,
    pub(crate) receiver: Receiver<T>,
}

impl<T> AsyncChannel<T> {
    fn new() -> Self {
        let (sender, receiver) = unbounded();

        Self { sender, receiver }
    }
}

#[derive(Debug, Message)]
/// A network event originating from another pl3xus app
pub enum NetworkEvent {
    /// A new client has connected
    Connected(ConnectionId),
    /// A client has disconnected
    Disconnected(ConnectionId),
    /// An error occured while trying to do a network operation
    Error(NetworkError),
}

#[derive(Debug, Message)]
/// [`NetworkData`] is what is sent over the bevy event system
///
/// Please check the root documentation how to up everything
pub struct NetworkData<T> {
    source: ConnectionId,
    inner: T,
    /// The name of the provider that received this message (e.g., "TcpProvider", "WebSocketProvider")
    /// This allows application logic to determine which protocol the message came from without
    /// needing direct access to Network resources
    provider_name: &'static str,
}

impl<T> Deref for NetworkData<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> NetworkData<T> {
    /// Allows manual creation of networkdata for sending events within bevy
    pub fn new(source: &ConnectionId, inner: T) -> NetworkData<T> {
        Self {
            source: *source,
            inner,
            provider_name: "Unknown",
        }
    }

    /// Create NetworkData with a specific provider name
    pub fn with_provider(source: &ConnectionId, inner: T, provider_name: &'static str) -> NetworkData<T> {
        Self {
            source: *source,
            inner,
            provider_name,
        }
    }

    /// The source of this network data
    pub fn source(&self) -> &ConnectionId {
        &self.source
    }

    /// The name of the provider that received this message
    pub fn provider_name(&self) -> &'static str {
        self.provider_name
    }

    /// Get the inner data out of it
    pub fn into_inner(self) -> T {
        self.inner
    }
}

struct Connection {
    receive_task: Box<dyn JoinHandle>,
    map_receive_task: Box<dyn JoinHandle>,
    send_task: Box<dyn JoinHandle>,
    send_message: Sender<NetworkPacket>,
}

impl Connection {
    fn stop(mut self) {
        self.receive_task.abort();
        self.send_task.abort();
        self.map_receive_task.abort();
    }
}
#[derive(Default, Copy, Clone, Debug)]
/// The plugin to add to your bevy [`App``] when you want
/// to instantiate a server
pub struct Pl3xusPlugin<NP: NetworkProvider, RT: Runtime = bevy::tasks::TaskPool>(
    PhantomData<(NP, RT)>,
);

impl<NP: NetworkProvider + Default, RT: Runtime> Plugin for Pl3xusPlugin<NP, RT> {
    fn build(&self, app: &mut App) {
        app.insert_resource(Network::new(NP::default()));
        app.add_message::<NetworkEvent>();
        app.add_systems(
            PreUpdate,
            managers::network::handle_new_incoming_connections::<NP, RT>,
        );
    }
}

/// Represents an outbound message to be sent to clients.
///
/// This struct encapsulates the message payload (`message`),
/// an optional target client (`for_client`), and a name (`name`)
/// associated with the message.
#[derive(Message, Debug, Clone, Eq, PartialEq, Hash)]
pub struct OutboundMessage<T>
where
    T: Pl3xusMessage,
{
    /// The name associated with the outbound message.
    pub name: String,

    /// The actual message payload to be sent.
    pub message: T,

    /// Optional target client for the message.
    /// If `None`, the message will be broadcasted.
    pub for_client: Option<ConnectionId>,
}

impl<T> OutboundMessage<T>
where
    T: Pl3xusMessage,
{
    /// Creates a new `OutboundMessage` instance with the given name and message payload.
    ///
    /// # Arguments
    ///
    /// * `name` - A `String` representing the name of the message.
    /// * `message` - The message payload that implements `Pl3xusMessage`.
    ///
    /// # Returns
    ///
    /// Returns a new `OutboundMessage` instance.
    pub fn new(name: String, message: T) -> Self {
        Self {
            name,
            message,
            for_client: None,
        }
    }

    /// Sets a specific client connection ID to target the message to.
    ///
    /// # Arguments
    ///
    /// * `id` - The `ConnectionId` of the client to send the message to.
    ///
    /// # Returns
    ///
    /// Returns an updated `OutboundMessage` instance with the target client set.
    pub fn for_client(mut self, id: ConnectionId) -> Self {
        self.for_client = Some(id);
        self
    }
}

impl<T: Pl3xusMessage> Deref for OutboundMessage<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.message
    }
}
