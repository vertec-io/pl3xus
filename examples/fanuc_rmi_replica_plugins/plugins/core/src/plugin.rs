//! Core plugin - registers networking, database, and ActiveSystem.

use bevy::prelude::*;
use bevy::app::ScheduleRunnerPlugin;
use bevy::tasks::TaskPoolBuilder;
use std::time::Duration;

use pl3xus::Pl3xusRuntime;
use pl3xus_sync::AppRequestRegistrationExt;
use pl3xus_sync::{Pl3xusSyncPlugin, ComponentSyncConfig, AppPl3xusSyncExt};
use pl3xus_sync::control::{ExclusiveControlPlugin, EntityControl};
use pl3xus_websockets::{NetworkSettings, WebSocketProvider};

use crate::database::{DatabaseResource, DatabaseInitRegistry};
use crate::handlers::handle_reset_database;
use crate::plugin_schedule::PluginSchedule;
use crate::plugin_schedule::configure_plugin_schedule;
use crate::types::{ActiveSystem, ResetDatabase};

/// Core plugin providing foundational infrastructure.
///
/// This plugin sets up:
/// - Bevy minimal plugins (headless 60Hz loop)
/// - Async tokio runtime for driver communication
/// - pl3xus networking and sync
/// - Exclusive control with hierarchy support
/// - Database resource
/// - ActiveSystem entity
pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        // Core Bevy plugins (headless 60Hz)
        app.add_plugins((
            MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(1.0 / 60.0))),
            bevy::log::LogPlugin::default(),
        ));

        // Configure plugin schedule system sets
        configure_plugin_schedule(app);

        // Async runtime for driver communication
        app.add_plugins(bevy_tokio_tasks::TokioTasksPlugin::default());

        // Pl3xus networking & sync
        app.add_plugins(pl3xus::Pl3xusPlugin::<WebSocketProvider, bevy::tasks::TaskPool>::default());
        app.insert_resource(Pl3xusRuntime(TaskPoolBuilder::new().num_threads(2).build()));
        app.insert_resource(NetworkSettings::default());
        app.add_plugins(Pl3xusSyncPlugin::<WebSocketProvider>::default());

        // Exclusive control (30 minute timeout, propagate to children)
        app.add_plugins(
            ExclusiveControlPlugin::<WebSocketProvider>::builder()
                .timeout_seconds(1800.0)
                .propagate_to_children(true)
                .build(),
        );
        app.sync_component::<EntityControl>(None);

        // Sync ActiveSystem component
        app.sync_component::<ActiveSystem>(Some(ComponentSyncConfig::read_only()));

        // Initialize database registry (plugins will add their initializers)
        app.init_resource::<DatabaseInitRegistry>();

        // Database initialization (runs after all plugins have registered)
        app.add_systems(Startup, init_database);

        // Spawn ActiveSystem entity
        app.add_systems(Startup, spawn_active_system.after(init_database));

        // Server startup (bind to port)
        app.add_systems(Startup, setup_server.after(spawn_active_system));

        // Register request handlers
        app.request::<ResetDatabase, WebSocketProvider>().register();
        app.add_systems(
            Update,
            handle_reset_database.in_set(PluginSchedule::ClientRequests),
        );
    }
}

/// System to initialize the database on startup.
pub fn init_database(mut commands: Commands, registry: Res<DatabaseInitRegistry>) {
    let db_path = std::env::var("DATABASE_PATH")
        .unwrap_or_else(|_| "fanuc_replica.db".to_string());

    match DatabaseResource::open(&db_path) {
        Ok(db) => {
            if let Err(e) = db.init_all(&registry) {
                error!("❌ Failed to initialize DB schemas: {}", e);
            }
            info!("✅ Database opened at: {}", db_path);
            commands.insert_resource(db);
        }
        Err(e) => {
            error!("❌ Failed to open database: {}", e);
        }
    }
}

/// Spawn the ActiveSystem entity on startup.
fn spawn_active_system(mut commands: Commands) {
    info!("🏭 Spawning ActiveSystem entity");
    commands.spawn((
        ActiveSystem,
        EntityControl::default(),
        Name::new("System"),
    ));
}

fn setup_server(
    mut net: ResMut<pl3xus::Network<WebSocketProvider>>,
    task_pool: Res<Pl3xusRuntime<bevy::tasks::TaskPool>>,
    settings: Res<NetworkSettings>,
) {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8083);
    match net.listen(addr, &task_pool.0, &settings) {
        Ok(_) => info!("✅ FANUC Replica Server listening on {}", addr),
        Err(e) => error!("❌ Failed to start listening: {}", e),
    }
}

