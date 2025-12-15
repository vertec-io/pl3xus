use bevy::prelude::*;
use bevy::tasks::{TaskPool, TaskPoolBuilder};
use pl3xus::{Pl3xusRuntime, Network, NetworkData, NetworkEvent};
use pl3xus::OutboundMessage;
use pl3xus_websockets::{NetworkSettings, WebSocketProvider};
use pl3xus_common::{ConnectionId, Pl3xusMessage};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::ops::Deref;

use crate::core::authorization::AuthorizedNetworkData;
use crate::core::models::{
    AssociateSubConnectionRequest, Client, GetConnectionDetailsRequest, ConnectionDetails,
    MarkedForDespawn, SubConnections,
};
use crate::core::plugin_schedule::PluginSchedule;

mod network_memory_plugin;
use network_memory_plugin::register_network_memory_plugin;

pub struct NetworkingPlugin;

impl Plugin for NetworkingPlugin {
    fn build(&self, app: &mut App) {
        // Networking plugins
        app.add_plugins(pl3xus::Pl3xusPlugin::<
            WebSocketProvider,
            bevy::tasks::TaskPool,
        >::default())
            .insert_resource(Pl3xusRuntime(TaskPoolBuilder::new().build()))
            .insert_resource(NetworkSettings::default());

        // Add our memory leak detection and prevention plugin
        register_network_memory_plugin(app);

        // Networking systems
        app.add_systems(Startup, setup_networking).add_systems(
            Update,
            handle_new_connection_events.in_set(PluginSchedule::ClientConnections),
        )
        .add_systems(
            Update, 
            handle_associate_sub_connection_request.in_set(PluginSchedule::ClientRequests),
        )
        // Add a system to periodically clean up resources
        .add_systems(Update, cleanup_network_resources);

        // Register networking messages
        super::server_register_network_messages(app);
        super::server_register_requests(app);
    }
}

pub fn new_server_message<T: Pl3xusMessage>(message: T) -> OutboundMessage<T> {
    OutboundMessage::new(module_path!().to_string(), message)
}

pub fn new_authorized_server_message<T: Pl3xusMessage>(message:T, target_node: String) -> AuthorizedNetworkData<T>{
    AuthorizedNetworkData{
        inner: message,
        authorized: true,
        source: ConnectionId::SERVER,
        node_id: target_node,
        control_state: None,
    }
}

fn setup_networking(
    mut net: ResMut<Network<WebSocketProvider>>,
    settings: Res<NetworkSettings>,
    task_pool: Res<Pl3xusRuntime<TaskPool>>,
) {
    let ip_address = "127.0.0.1".parse().expect("Could not parse ip address");

    println!("Address of the server: {}:8081", ip_address);

    let _socket_address = SocketAddr::new(ip_address, 8081);

    match net.listen(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8081),
        &task_pool.0,
        &settings,
    ) {
        Ok(_) => (),
        Err(err) => {
            error!("Could not start listening: {}", err);
            panic!();
        }
    }

    println!("Started listening for new connections!");
}

// System to periodically clean up network resources
fn cleanup_network_resources(
    mut net: ResMut<Network<WebSocketProvider>>,
    time: Res<Time>,
) {
    static mut LAST_CLEANUP: Option<f64> = None;
    
    let current_time = time.elapsed_secs() as f64;
    let should_cleanup = unsafe {
        match LAST_CLEANUP {
            Some(last_time) if current_time - last_time < 30.0 => false,
            _ => {
                LAST_CLEANUP = Some(current_time);
                true
            }
        }
    };
    
    if should_cleanup {
        println!("Performing network resource cleanup...");
        
        // Clear any pending messages in channels
        while net.new_connections.receiver.try_recv().is_ok() {
            println!("Cleaned up a pending new connection");
        }
        
        while net.disconnected_connections.receiver.try_recv().is_ok() {
            println!("Cleaned up a pending disconnection");
        }
        
        while net.error_channel.receiver.try_recv().is_ok() {
            println!("Cleaned up a pending error");
        }
        
        // Log the current state
        println!("Active connections: {}", net.established_connections.len());
        println!("Connection tasks: {}", net.connection_tasks.len());
    }
}

pub fn handle_new_connection_events(
    mut commands: Commands,
    net: Res<Network<WebSocketProvider>>,
    mut network_events: MessageReader<NetworkEvent>,
    mut clients: Query<(Entity, &mut Client, Option<&SubConnections>)>,
    mut connection_details_writer: MessageWriter<OutboundMessage<ConnectionDetails>>,
) {
    for event in network_events.read() {
        match event {
            NetworkEvent::Connected(conn_id) => {
                let client = Client {
                    id: *conn_id,
                    in_control: (conn_id.id == 0),
                };

                // Spawn the new client entity
                commands.spawn((client.clone(),));

                // Send initial client info
                let _ = net.send_message(*conn_id, client);

                // Send initial connection details
                connection_details_writer.write(
                    new_server_message(ConnectionDetails {
                        connection_id: Some(*conn_id),
                        sub_connections_ids: vec![],
                        parent_connection_id: None,
                    }).for_client(*conn_id)
                );
                
                println!("New client connected: {:?}", conn_id);
            }
            NetworkEvent::Disconnected(conn_id) => {
                for (entity, mut client, _) in clients.iter_mut() {
                    if client.id == *conn_id {
                        if client.in_control {
                            client.in_control = false;
                        }
                        commands.entity(entity).insert(MarkedForDespawn);
                    }
                }
                
                println!("Client disconnected: {:?}", conn_id);
            }
            NetworkEvent::Error(err) => {
                info!("An error occurred: {:?}", err);
            }
        };
    }
}

fn handle_associate_sub_connection_request(
    mut commands: Commands,
    mut requests: MessageReader<NetworkData<AssociateSubConnectionRequest>>,
    mut clients: Query<(Entity, &mut Client, Option<&mut SubConnections>)>,
){
    for request in requests.read() {
        let id = request.source();
        let request = request.deref();
        let parent_id = request.parent_connection_id;

        for (entity, client, sub_connections) in clients.iter_mut() {
            if client.id == parent_id {
                match sub_connections {
                    Some(mut sub_connections) => {
                        if sub_connections.ids.contains(id) {
                            continue;
                        }else{
                            sub_connections.ids.push(*id);
                            println!("Added {} to parent connection {}", id, parent_id);
                        }
                    }
                    None => {
                        let sub_connections = SubConnections {
                            ids: vec![*id],
                            parent_connection_id: request.parent_connection_id,
                        };
                        println!("Added {} to parent connection {}", id, parent_id);
                        commands.entity(entity).insert(sub_connections);
                    }
                }
            }
        }
    }
}
