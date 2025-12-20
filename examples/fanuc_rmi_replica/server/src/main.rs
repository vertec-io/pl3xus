//! FANUC RMI Replica Server
//!
//! A demonstration server for the pl3xus real-time synchronization framework.
//!
//! # Architecture
//! - Uses Bevy's plugin system for modularity
//! - Each feature is encapsulated in a plugin
//! - pl3xus handles WebSocket sync automatically

use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use bevy::tasks::TaskPoolBuilder;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use pl3xus::{Pl3xusRuntime, Network};
use pl3xus_sync::{AppPl3xusSyncExt, Pl3xusSyncPlugin};
use pl3xus_sync::control::{ExclusiveControlPlugin, EntityControl, AppExclusiveControlExt};
use pl3xus_websockets::{NetworkSettings, WebSocketProvider};
use bevy_tokio_tasks::TokioTasksPlugin;

mod database;
mod jogging;
mod plugins;

use database::DatabaseResource;
use plugins::{RobotConnectionPlugin, RobotSyncPlugin, RequestHandlerPlugin, RobotPollingPlugin, ProgramExecutionPlugin};
use fanuc_replica_types::*;

fn main() {
    let mut app = App::new();

    // ========================================================================
    // Core Bevy Setup (Headless 60Hz)
    // ========================================================================
    app.add_plugins((
        MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(1.0 / 60.0))),
        bevy::log::LogPlugin::default(),
    ));

    // ========================================================================
    // Async Runtime (for driver connections)
    // ========================================================================
    app.add_plugins(TokioTasksPlugin::default());

    // ========================================================================
    // Pl3xus Networking & Sync
    // ========================================================================
    app.add_plugins(pl3xus::Pl3xusPlugin::<WebSocketProvider, bevy::tasks::TaskPool>::default());
    app.insert_resource(Pl3xusRuntime(TaskPoolBuilder::new().num_threads(2).build()));
    app.insert_resource(NetworkSettings::default());
    app.add_plugins(Pl3xusSyncPlugin::<WebSocketProvider>::default());

    // ========================================================================
    // Exclusive Control (replaces custom RequestControl/ReleaseControl)
    // ========================================================================
    app.add_plugins(ExclusiveControlPlugin::default());
    app.add_exclusive_control_systems::<WebSocketProvider>();
    app.sync_component::<EntityControl>(None);

    // ========================================================================
    // Synced Components (pl3xus handles all sync automatically)
    // ========================================================================
    app.sync_component::<RobotPosition>(None);
    app.sync_component::<JointAngles>(None);
    app.sync_component::<RobotStatus>(None);
    app.sync_component::<IoStatus>(None);
    app.sync_component::<ExecutionState>(None);
    app.sync_component::<ConnectionState>(None);
    app.sync_component::<ActiveConfigState>(None);
    app.sync_component::<JogSettingsState>(None);

    // ========================================================================
    // Database
    // ========================================================================
    match DatabaseResource::open("fanuc_replica.db") {
        Ok(db) => {
            if let Err(e) = db.init_schema() {
                error!("Failed to initialize DB schema: {}", e);
            }
            app.insert_resource(db);
        }
        Err(e) => {
            error!("Failed to open database: {}", e);
            panic!("Database initialization failed");
        }
    }

    // ========================================================================
    // Application Plugins
    // ========================================================================
    app.add_plugins((
        RobotConnectionPlugin,      // Connection state machine
        RobotSyncPlugin,            // Driver polling and jogging
        RequestHandlerPlugin,       // Database request handlers
        RobotPollingPlugin,         // Periodic position/status polling
        ProgramExecutionPlugin,     // Program execution with buffered streaming
    ));

    // ========================================================================
    // Server Startup
    // ========================================================================
    app.add_systems(Startup, setup_server);

    info!("🚀 Starting FANUC RMI Replica Server...");
    app.run();
}

fn setup_server(
    mut net: ResMut<Network<WebSocketProvider>>,
    task_pool: Res<Pl3xusRuntime<bevy::tasks::TaskPool>>,
    settings: Res<NetworkSettings>,
) {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8083);
    match net.listen(addr, &task_pool.0, &settings) {
        Ok(_) => info!("✅ FANUC Replica Server listening on {}", addr),
        Err(e) => error!("❌ Failed to start listening: {}", e),
    }
}
