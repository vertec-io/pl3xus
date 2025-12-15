use bevy::prelude::*;
use pl3xus::{Network, NetworkEvent};
use pl3xus_websockets::WebSocketProvider;
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Resource)]
pub struct MemoryStats {
    pub last_check: Instant,
    pub check_interval: Duration,
    pub connection_counts: Vec<(f64, usize)>, // (time_elapsed, count)
    pub message_counts: HashMap<String, Vec<(f64, usize)>>, // (message_type, [(time_elapsed, count)])
    pub start_time: Instant,
}

impl Default for MemoryStats {
    fn default() -> Self {
        Self {
            last_check: Instant::now(),
            check_interval: Duration::from_secs(5),
            connection_counts: Vec::new(),
            message_counts: HashMap::new(),
            start_time: Instant::now(),
        }
    }
}

// System to initialize memory diagnostics
pub fn setup_memory_diagnostics(mut commands: Commands) {
    commands.insert_resource(MemoryStats {
        last_check: Instant::now(),
        check_interval: Duration::from_secs(5),
        connection_counts: Vec::new(),
        message_counts: HashMap::new(),
        start_time: Instant::now(),
    });
}

// System to monitor memory usage with WebSocketProvider
pub fn monitor_memory_usage(
    _time: Res<Time>,
    mut stats: ResMut<MemoryStats>,
    network: Res<Network<WebSocketProvider>>,
) {
    if stats.last_check.elapsed() < stats.check_interval {
        return;
    }

    stats.last_check = Instant::now();
    let elapsed = stats.start_time.elapsed().as_secs_f64();

    // Monitor connection count
    let connection_count = network.has_connections();
    stats
        .connection_counts
        .push((elapsed, if connection_count { 1 } else { 0 }));

    // Print connection count
    println!("Has active connections: {}", connection_count);

    // Print memory stats
    #[cfg(target_os = "windows")]
    {
        use std::process;
        let pid = process::id();
        let output = std::process::Command::new("powershell")
            .args([
                "-Command",
                &format!("Get-Process -Id {} | Select-Object WorkingSet", pid),
            ])
            .output()
            .expect("Failed to execute powershell command");

        let output_str = String::from_utf8_lossy(&output.stdout);
        println!("Memory usage: {}", output_str);
    }
}

// System to monitor network events
pub fn monitor_network_events(mut network_events: MessageReader<NetworkEvent>) {
    for event in network_events.read() {
        match event {
            NetworkEvent::Connected(conn_id) => {
                println!("Connection established: {:?}", conn_id);
            }
            NetworkEvent::Disconnected(conn_id) => {
                println!("Connection closed: {:?}", conn_id);
            }
            NetworkEvent::Error(err) => {
                println!("Network error: {:?}", err);
            }
        }
    }
}

pub fn register_memory_diagnostic_plugin(app: &mut App) {
    app.add_systems(Startup, setup_memory_diagnostics)
        .add_systems(Update, monitor_memory_usage)
        .add_systems(Update, monitor_network_events);
}
