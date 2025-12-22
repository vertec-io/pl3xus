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
use pl3xus_sync::control::{ExclusiveControlPlugin, EntityControl};
use pl3xus_websockets::{NetworkSettings, WebSocketProvider};
use bevy_tokio_tasks::TokioTasksPlugin;

mod database;
mod jogging;
mod plugins;

use database::DatabaseResource;
use plugins::{SystemPlugin, RobotConnectionPlugin, RobotSyncPlugin, RequestHandlerPlugin, RobotPollingPlugin, ProgramExecutionPlugin, ProgramPlugin};
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
    app.add_plugins(
        ExclusiveControlPlugin::<WebSocketProvider>::builder()
            .timeout_seconds(1800.0)      // 30 minute timeout
            .propagate_to_children(true)  // Control parent = control children
            .build(),
    );
    app.sync_component::<EntityControl>(None);

    // ========================================================================
    // Synced Components (pl3xus handles all sync automatically)
    // ========================================================================
    //
    // Components are configured as either:
    // - Read-only: Server is authoritative, clients cannot mutate directly
    // - Mutable: Clients can mutate (with proper authorization)
    //
    // Read-only components provide custom error messages guiding users to the
    // correct command-based API for modifying robot state.

    use pl3xus_sync::ComponentSyncConfig;

    // Marker components (read-only, no meaningful mutations)
    app.sync_component::<ActiveSystem>(Some(ComponentSyncConfig::read_only()));
    app.sync_component::<ActiveRobot>(Some(ComponentSyncConfig::read_only()));

    // Robot status/state components (read-only, updated by server from robot)
    app.sync_component::<RobotPosition>(Some(ComponentSyncConfig::read_only_with_message(
        "RobotPosition is read-only. Robot position is controlled by the robot controller."
    )));
    app.sync_component::<JointAngles>(Some(ComponentSyncConfig::read_only_with_message(
        "JointAngles is read-only. Joint positions are controlled by the robot controller."
    )));
    app.sync_component::<RobotStatus>(Some(ComponentSyncConfig::read_only_with_message(
        "RobotStatus is read-only. Use SetSpeedOverride command to change speed."
    )));
    app.sync_component::<IoStatus>(Some(ComponentSyncConfig::read_only_with_message(
        "IoStatus is read-only. Use SetDigitalOutput command to control outputs."
    )));
    app.sync_component::<ExecutionState>(Some(ComponentSyncConfig::read_only_with_message(
        "ExecutionState is read-only. Use program execution commands (Start, Stop, Pause, etc)."
    )));
    app.sync_component::<ConnectionState>(Some(ComponentSyncConfig::read_only_with_message(
        "ConnectionState is read-only. Use ConnectToRobot/DisconnectFromRobot commands."
    )));

    // User-configurable components (clients can mutate with proper authorization)
    app.sync_component::<ActiveConfigState>(None);  // User can change active configuration
    app.sync_component::<JogSettingsState>(None);   // User can change jog settings

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
        SystemPlugin,               // System/Apparatus entity (hierarchy root)
        RobotConnectionPlugin,      // Connection state machine
        RobotSyncPlugin,            // Driver polling and jogging
        RequestHandlerPlugin,       // Database request handlers
        RobotPollingPlugin,         // Periodic position/status polling
        ProgramExecutionPlugin,     // Program execution with buffered streaming (LEGACY)
        ProgramPlugin,              // New orchestrator-based program execution
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
