use bevy::prelude::*;
use pl3xus::Network;
use pl3xus_websockets::WebSocketProvider;
use std::time::{Duration, Instant};

#[derive(Resource)]
pub struct MessageCleanupConfig {
    pub check_interval: Duration,
    pub last_check: Instant,
    pub max_message_queue_size: usize,
}

impl Default for MessageCleanupConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(10),
            last_check: Instant::now(),
            max_message_queue_size: 1000, // Adjust based on your needs
        }
    }
}

pub fn setup_message_cleanup(mut commands: Commands) {
    commands.insert_resource(MessageCleanupConfig {
        check_interval: Duration::from_secs(10),
        last_check: Instant::now(),
        max_message_queue_size: 1000, // Adjust based on your needs
    });
}

pub fn cleanup_message_queues(
    mut config: ResMut<MessageCleanupConfig>,
    network: Res<Network<WebSocketProvider>>,
) {
    if config.last_check.elapsed() < config.check_interval {
        return;
    }

    config.last_check = Instant::now();

    // We can't directly access the message queues, so we'll just log the status
    println!(
        "Message queue cleanup check performed at: {:?}",
        std::time::Instant::now()
    );
    println!("Has active connections: {}", network.has_connections());
}

pub fn register_message_cleanup_plugin(app: &mut App) {
    app.add_systems(Startup, setup_message_cleanup)
        .add_systems(Update, cleanup_message_queues);
}
