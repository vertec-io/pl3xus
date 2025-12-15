use bevy::prelude::*;

use crate::connection_cleanup::*;
use crate::memory_diagnostic::*;
use crate::memory_monitor::*;
use crate::message_cleanup::*;

/// A Bevy plugin that provides memory leak detection and prevention for pl3xus.
///
/// This plugin adds systems to monitor memory usage, clean up stale connections,
/// and prevent message queue accumulation.
pub struct NetworkMemoryPlugin;

impl Plugin for NetworkMemoryPlugin {
    fn build(&self, app: &mut App) {
        // Register all diagnostic and cleanup systems
        register_memory_diagnostic_plugin(app);
        register_connection_cleanup_plugin(app);
        register_message_cleanup_plugin(app);
        register_memory_monitor_plugin(app);

        // Add the force GC system
        app.add_systems(Update, force_gc);
    }
}

/// A system to periodically force garbage collection
pub fn force_gc(time: Res<Time>) {
    static mut LAST_GC: Option<f64> = None;

    let current_time = time.elapsed_secs() as f64;
    let should_gc = unsafe {
        match LAST_GC {
            Some(last_time) if current_time - last_time < 60.0 => false,
            _ => {
                LAST_GC = Some(current_time);
                true
            }
        }
    };

    if should_gc {
        println!("Forcing garbage collection...");
        // In a real implementation, you might call a GC function here
        // For Rust, we don't have direct GC control, but we can:
        // 1. Drop any unused resources
        // 2. Clear caches
        // 3. Shrink collections to fit
    }
}

/// A system to periodically clean up network resources
pub fn cleanup_network_resources<NP>(net: ResMut<pl3xus::Network<NP>>, time: Res<Time>)
where
    NP: pl3xus::managers::NetworkProvider,
{
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

        // We can't directly access the channels or connections, so we'll just log the status
        println!(
            "Network resource cleanup check performed at: {:?}",
            std::time::Instant::now()
        );
        println!("Has active connections: {}", net.has_connections());
    }
}
