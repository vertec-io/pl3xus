use bevy::prelude::*;
use bevy::tasks::TaskPoolBuilder;
use pl3xus::{Pl3xusRuntime, Network, NetworkData, NetworkEvent};
use pl3xus_websockets::{NetworkSettings, WebSocketProvider};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::{Duration, Instant};

// Import our diagnostic tools
mod memory_diagnostic;
mod connection_cleanup;
mod message_cleanup;
mod network_memory_plugin;

use memory_diagnostic::*;
use connection_cleanup::*;
use message_cleanup::*;
use network_memory_plugin::*;

fn main() {
    let mut app = App::new();
    
    // Add the basic Bevy plugins
    app.add_plugins(MinimalPlugins)
       .add_plugins(bevy::log::LogPlugin::default());
    
    // Add the pl3xus plugin
    app.add_plugins(pl3xus::Pl3xusPlugin::<
        WebSocketProvider,
        bevy::tasks::TaskPool,
    >::default())
       .insert_resource(Pl3xusRuntime(TaskPoolBuilder::new().build()))
       .insert_resource(NetworkSettings::default());
    
    // Add our memory leak detection and prevention plugin
    app.add_plugins(NetworkMemoryPlugin);
    
    // Add systems
    app.add_systems(Startup, setup_networking)
       .add_systems(Update, handle_network_events)
       .add_systems(Update, print_memory_stats);
    
    // Run the app
    app.run();
}

fn setup_networking(
    mut net: ResMut<Network<WebSocketProvider>>,
    settings: Res<NetworkSettings>,
    task_pool: Res<Pl3xusRuntime<bevy::tasks::TaskPool>>,
) {
    let ip_address = "127.0.0.1".parse().expect("Could not parse ip address");
    println!("Address of the server: {}:8081", ip_address);
    
    match net.listen(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8081),
        &task_pool.0,
        &settings,
    ) {
        Ok(_) => println!("Started listening for new connections!"),
        Err(err) => {
            error!("Could not start listening: {}", err);
            panic!();
        }
    }
}

fn handle_network_events(
    mut network_events: MessageReader<NetworkEvent>,
) {
    for event in network_events.read() {
        match event {
            NetworkEvent::Connected(conn_id) => {
                println!("New client connected: {:?}", conn_id);
            }
            NetworkEvent::Disconnected(conn_id) => {
                println!("Client disconnected: {:?}", conn_id);
            }
            NetworkEvent::Error(err) => {
                println!("Network error: {:?}", err);
            }
        }
    }
}

fn print_memory_stats(
    time: Res<Time>,
) {
    static mut LAST_PRINT: Option<Instant> = None;
    
    let should_print = unsafe {
        match LAST_PRINT {
            Some(last_time) if last_time.elapsed() < Duration::from_secs(10) => false,
            _ => {
                LAST_PRINT = Some(Instant::now());
                true
            }
        }
    };
    
    if should_print {
        println!("Time elapsed: {:.2}s", time.elapsed_secs() as f64);
        
        #[cfg(target_os = "windows")]
        {
            use std::process;
            let pid = process::id();
            let output = std::process::Command::new("powershell")
                .args(&["-Command", &format!("Get-Process -Id {} | Select-Object WorkingSet", pid)])
                .output()
                .expect("Failed to execute powershell command");
            
            let output_str = String::from_utf8_lossy(&output.stdout);
            println!("Memory usage: {}", output_str);
        }
    }
}
