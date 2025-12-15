use bevy::prelude::*;
use pl3xus::Network;
use pl3xus_websockets::WebSocketProvider;
use std::time::{Duration, Instant};

#[derive(Resource)]
pub struct ConnectionCleanupConfig {
    pub check_interval: Duration,
    pub last_check: Instant,
}

impl Default for ConnectionCleanupConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(30),
            last_check: Instant::now(),
        }
    }
}

pub fn setup_connection_cleanup(mut commands: Commands) {
    commands.insert_resource(ConnectionCleanupConfig {
        check_interval: Duration::from_secs(30),
        last_check: Instant::now(),
    });
}

pub fn cleanup_stale_connections(
    mut config: ResMut<ConnectionCleanupConfig>,
    network: Res<Network<WebSocketProvider>>,
) {
    if config.last_check.elapsed() < config.check_interval {
        return;
    }

    config.last_check = Instant::now();

    // Log the current connection status
    println!("Has active connections: {}", network.has_connections());

    // We can't directly access the connections or channels, so we'll just log the status
    println!(
        "Connection cleanup check performed at: {:?}",
        std::time::Instant::now()
    );
}

pub fn register_connection_cleanup_plugin(app: &mut App) {
    app.add_systems(Startup, setup_connection_cleanup)
        .add_systems(Update, cleanup_stale_connections);
}
