use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use bevy::tasks::TaskPoolBuilder;
use pl3xus::{Pl3xusRuntime, Network, AppNetworkMessage};
use pl3xus_common::Pl3xusMessage;
use pl3xus_sync::{AppPl3xusSyncExt, Pl3xusSyncPlugin};
use pl3xus_sync::control::{ExclusiveControlPlugin, EntityControl, AppExclusiveControlExt};
use pl3xus_websockets::{NetworkSettings, WebSocketProvider};
use bevy_tokio_tasks::{TokioTasksPlugin, TokioTasksRuntime};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

mod database;
mod jogging;
mod driver_sync;

use database::DatabaseResource;
use fanuc_replica_types::*;
use std::sync::Arc;
use tokio::sync::broadcast;
use fanuc_rmi::drivers::{FanucDriver, FanucDriverConfig};

// ============================================================================
// Robot Entity Components
// ============================================================================

/// Marker component for active FANUC robot entities
#[derive(Component)]
pub struct FanucRobot;

/// Connection details for a robot (stored on entity)
#[derive(Component, Clone)]
pub struct RobotConnectionDetails {
    pub addr: String,
    pub port: u32,
    pub name: String,
}

/// Driver handle - added when connected (on entity, not resource)
#[derive(Component, Clone)]
pub struct RmiDriver(pub Arc<FanucDriver>);

/// Response channel for receiving driver responses
#[derive(Component)]
pub struct RmiResponseChannel(pub broadcast::Receiver<fanuc_rmi::packets::ResponsePacket>);

/// Robot connection state (entity-based state machine)
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RobotConnectionState {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Disconnecting,
}

/// Marker to prevent duplicate connection attempts
#[derive(Component)]
struct ConnectionInProgress;

fn main() {
    let mut app = App::new();

    // 1. Core Bevy Plugins (Headless 60Hz)
    app.add_plugins((
        MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(1.0 / 60.0))),
        bevy::log::LogPlugin::default(),
    ));

    // 2. Tokio Runtime for async driver operations
    app.add_plugins(TokioTasksPlugin::default());

    // 3. Pl3xus Networking
    app.add_plugins(pl3xus::Pl3xusPlugin::<WebSocketProvider, bevy::tasks::TaskPool>::default());
    app.insert_resource(Pl3xusRuntime(TaskPoolBuilder::new().num_threads(2).build()));
    app.insert_resource(NetworkSettings::default());

    // 4. Pl3xus Sync
    app.add_plugins(Pl3xusSyncPlugin::<WebSocketProvider>::default());

    // 5. Pl3xus Exclusive Control (replaces custom RequestControl/ReleaseControl)
    app.add_plugins(ExclusiveControlPlugin::default());
    app.add_exclusive_control_systems::<WebSocketProvider>();
    app.sync_component::<EntityControl>(None);

    // 6. Register Synced Components
    // ✅ One line per component - pl3xus handles all sync automatically
    app.sync_component::<RobotPosition>(None);
    app.sync_component::<JointAngles>(None);
    app.sync_component::<RobotStatus>(None);
    app.sync_component::<IoStatus>(None);
    app.sync_component::<ExecutionState>(None);
    app.sync_component::<ConnectionState>(None);
    app.sync_component::<ActiveConfigState>(None);
    app.sync_component::<JogSettingsState>(None);
    // Note: ControlStatus is replaced by EntityControl from pl3xus

    // 7. Database Setup
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

    // 8. Network Messages (RPC)
    // ✅ Just register the type - pl3xus routes messages automatically
    // Note: RequestControl/ReleaseControl removed - using ExclusiveControlPlugin instead
    // Jog Commands
    app.register_network_message::<JogCommand, WebSocketProvider>();
    // Robot Control
    app.register_network_message::<InitializeRobot, WebSocketProvider>();
    app.register_network_message::<ResetRobot, WebSocketProvider>();
    app.register_network_message::<AbortMotion, WebSocketProvider>();
    app.register_network_message::<SetSpeedOverride, WebSocketProvider>();
    // Motion Commands
    app.register_network_message::<LinearMotionCommand, WebSocketProvider>();
    app.register_network_message::<JointMotionCommand, WebSocketProvider>();
    // Program Execution
    app.register_network_message::<ExecuteProgram, WebSocketProvider>();
    app.register_network_message::<StopExecution, WebSocketProvider>();
    app.register_network_message::<LoadProgram, WebSocketProvider>();
    app.register_network_message::<StartProgram, WebSocketProvider>();
    app.register_network_message::<PauseProgram, WebSocketProvider>();
    app.register_network_message::<ResumeProgram, WebSocketProvider>();
    app.register_network_message::<StopProgram, WebSocketProvider>();
    // Connection
    app.register_network_message::<ConnectToRobot, WebSocketProvider>();
    app.register_network_message::<DisconnectRobot, WebSocketProvider>();
    // Note: ControlRequest/ControlResponse already registered by add_exclusive_control_systems

    // Debug: Log the expected type names for our messages
    info!("📝 Registered ConnectToRobot with type_name: '{}'",
          <ConnectToRobot as Pl3xusMessage>::type_name());

    // 9. Robot Connection State Machine Systems
    app.add_systems(Startup, (setup_server, spawn_robot_entity));
    app.add_systems(Update, (
        handle_connect_requests,
        handle_connecting_state,
        driver_sync::sync_robot_state,
        handle_disconnect_requests,
        jogging::handle_jog_commands,
    ));

    app.run();
}

fn setup_server(
    mut net: ResMut<Network<WebSocketProvider>>,
    task_pool: Res<Pl3xusRuntime<bevy::tasks::TaskPool>>,
    settings: Res<NetworkSettings>,
) {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8083);
    match net.listen(addr, &task_pool.0, &settings) {
        Ok(_) => info!("FANUC Replica Server listening on {}", addr),
        Err(e) => error!("Failed to start listening: {}", e),
    }
}

/// Spawn robot entity with all synced components (entity-based, not singleton)
fn spawn_robot_entity(mut commands: Commands) {
    // Default connection details (can be configured via database later)
    let connection_details = RobotConnectionDetails {
        addr: "127.0.0.1".to_string(),
        port: 16001,
        name: "CRX-10iA Simulator".to_string(),
    };

    // Spawn robot entity with all components - NO singleton assumptions
    commands.spawn((
        Name::new("FANUC_Robot"),
        FanucRobot, // Marker component
        connection_details,
        RobotConnectionState::Disconnected,
        // Synced state components (updated when connected)
        RobotPosition::default(),
        JointAngles::default(),
        RobotStatus::default(),
        IoStatus::default(),
        ExecutionState::default(),
        ConnectionState::default(),
        ActiveConfigState::default(),
        JogSettingsState::default(),
    ));

    info!("✅ Spawned FANUC_Robot entity (multi-robot ready)");
}

// ============================================================================
// Connection State Machine Handlers
// ============================================================================

use bevy::ecs::message::MessageReader;

/// Handle incoming connection requests - transition to Connecting state
fn handle_connect_requests(
    mut connect_events: MessageReader<pl3xus::NetworkData<ConnectToRobot>>,
    mut robots: Query<(Entity, &mut RobotConnectionState, &mut RobotConnectionDetails), With<FanucRobot>>,
) {
    // Debug: Log that this system is running (only once per batch of events)
    let event_count = connect_events.len();
    if event_count > 0 {
        info!("🔍 handle_connect_requests: Processing {} ConnectToRobot events", event_count);
    }

    for event in connect_events.read() {
        // NetworkData<T> implements Deref<Target=T>
        let msg: &ConnectToRobot = &*event;
        info!("📡 Received ConnectToRobot: {}:{}", msg.addr, msg.port);

        // Debug: Check how many robots we have
        let robot_count = robots.iter().count();
        info!("📊 Found {} robot entities with FanucRobot marker", robot_count);

        // For now, update the first robot (in future, match by entity ID or name)
        match robots.single_mut() {
            Ok((entity, mut state, mut details)) => {
                info!("📊 Robot {:?} current state: {:?}", entity, *state);

                if *state == RobotConnectionState::Disconnected {
                    // Update connection details from message
                    details.addr = msg.addr.clone();
                    details.port = msg.port;
                    if let Some(ref name) = msg.name {
                        details.name = name.clone();
                    }

                    // Transition to connecting state
                    *state = RobotConnectionState::Connecting;
                    info!("🔄 Robot {:?} transitioning to Connecting state", entity);
                } else {
                    warn!("Robot state is {:?}, not Disconnected - ignoring request", *state);
                }
            }
            Err(e) => {
                error!("❌ Failed to get robot entity: {:?}", e);
            }
        }
    }
}

/// Handle Connecting state - spawn async task to connect driver
fn handle_connecting_state(
    runtime: Res<TokioTasksRuntime>,
    mut commands: Commands,
    robots: Query<(Entity, &RobotConnectionDetails, &RobotConnectionState), (With<FanucRobot>, Without<ConnectionInProgress>)>,
) {
    for (entity, details, state) in robots.iter() {
        if *state != RobotConnectionState::Connecting {
            continue;
        }

        // Mark as in-progress to prevent duplicate connection attempts
        commands.entity(entity).insert(ConnectionInProgress);

        let config = FanucDriverConfig {
            addr: details.addr.clone(),
            port: details.port,
            max_messages: 30,
        };

        let robot_name = details.name.clone();

        info!("🔌 Starting async connection to {}:{}", details.addr, details.port);

        runtime.spawn_background_task(move |mut ctx| async move {
            match FanucDriver::connect(config).await {
                Ok(driver) => {
                    driver.initialize();
                    let driver_arc = Arc::new(driver);
                    let response_rx = driver_arc.response_tx.subscribe();

                    ctx.run_on_main_thread(move |ctx| {
                        // Insert driver and response channel as components on the entity
                        if let Ok(mut entity_mut) = ctx.world.get_entity_mut(entity) {
                            entity_mut.remove::<ConnectionInProgress>();
                            entity_mut.insert(RmiDriver(driver_arc.clone()));
                            entity_mut.insert(RmiResponseChannel(response_rx));
                            entity_mut.insert(RobotConnectionState::Connected);

                            // Update the synced ConnectionState
                            if let Some(mut conn_state) = entity_mut.get_mut::<ConnectionState>() {
                                conn_state.robot_connected = true;
                                conn_state.robot_addr = format!("{}:{}",
                                    driver_arc.config.addr, driver_arc.config.port);
                                conn_state.connection_name = Some(robot_name);
                                conn_state.tp_initialized = true;
                            }

                            info!("✅ Robot {:?} connected successfully", entity);
                        }
                    }).await;
                }
                Err(e) => {
                    error!("❌ Failed to connect: {}", e);
                    ctx.run_on_main_thread(move |ctx| {
                        if let Ok(mut entity_mut) = ctx.world.get_entity_mut(entity) {
                            entity_mut.remove::<ConnectionInProgress>();
                            entity_mut.insert(RobotConnectionState::Disconnected);

                            if let Some(mut conn_state) = entity_mut.get_mut::<ConnectionState>() {
                                conn_state.robot_connected = false;
                            }
                        }
                    }).await;
                }
            }
        });
    }
}

/// Handle disconnect requests
fn handle_disconnect_requests(
    mut disconnect_events: MessageReader<pl3xus::NetworkData<DisconnectRobot>>,
    mut robots: Query<(Entity, &mut RobotConnectionState, &mut ConnectionState), With<FanucRobot>>,
    mut commands: Commands,
) {
    for _event in disconnect_events.read() {
        for (entity, mut state, mut conn_state) in robots.iter_mut() {
            if *state == RobotConnectionState::Connected {
                *state = RobotConnectionState::Disconnected;
                conn_state.robot_connected = false;
                conn_state.tp_initialized = false;

                // Remove driver components
                commands.entity(entity).remove::<RmiDriver>();
                commands.entity(entity).remove::<RmiResponseChannel>();

                info!("🔌 Robot {:?} disconnected", entity);
            }
        }
    }
}

// ============================================================================
// Legacy Resources (for gradual migration of driver_sync)
// ============================================================================

#[derive(Resource)]
pub struct DriverResponseReceiver(pub broadcast::Receiver<fanuc_rmi::dto::ResponsePacket>);
