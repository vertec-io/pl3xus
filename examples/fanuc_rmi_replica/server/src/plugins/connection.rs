//! Robot connection state machine plugin.
//!
//! Handles connecting to and disconnecting from FANUC robots via RMI driver.

use bevy::prelude::*;
use bevy::ecs::message::MessageReader;
use bevy_tokio_tasks::TokioTasksRuntime;
use pl3xus::AppNetworkMessage;
use pl3xus_websockets::WebSocketProvider;
use std::sync::Arc;
use tokio::sync::broadcast;
use fanuc_rmi::drivers::{FanucDriver, FanucDriverConfig};
use fanuc_replica_types::*;

// ============================================================================
// Components
// ============================================================================

/// Marker component for active FANUC robot entities.
#[derive(Component)]
pub struct FanucRobot;

/// Connection details for a robot.
#[derive(Component, Clone)]
pub struct RobotConnectionDetails {
    pub addr: String,
    pub port: u32,
    pub name: String,
}

/// Driver handle - added when connected.
#[derive(Component, Clone)]
#[allow(dead_code)]
pub struct RmiDriver(pub Arc<FanucDriver>);

/// Response channel for receiving driver responses.
#[derive(Component)]
pub struct RmiResponseChannel(pub broadcast::Receiver<fanuc_rmi::packets::ResponsePacket>);

/// Robot connection state (entity-based state machine).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum RobotConnectionState {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Disconnecting,
}

/// Marker to prevent duplicate connection attempts.
#[derive(Component)]
struct ConnectionInProgress;

// ============================================================================
// Plugin
// ============================================================================

pub struct RobotConnectionPlugin;

impl Plugin for RobotConnectionPlugin {
    fn build(&self, app: &mut App) {
        // Register network messages for connection
        app.register_network_message::<ConnectToRobot, WebSocketProvider>();
        app.register_network_message::<DisconnectRobot, WebSocketProvider>();

        // Add connection systems
        app.add_systems(Startup, spawn_robot_entity);
        app.add_systems(Update, (
            handle_connect_requests,
            handle_connecting_state,
            handle_disconnect_requests,
        ));
    }
}

// ============================================================================
// Systems
// ============================================================================

/// Spawn initial robot entity with all synced components.
fn spawn_robot_entity(mut commands: Commands) {
    let connection_details = RobotConnectionDetails {
        addr: "127.0.0.1".to_string(),
        port: 16001,
        name: "CRX-10iA Simulator".to_string(),
    };

    commands.spawn((
        Name::new("FANUC_Robot"),
        FanucRobot,
        connection_details,
        RobotConnectionState::Disconnected,
        // Synced state components
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

/// Handle incoming connection requests - transition to Connecting state.
fn handle_connect_requests(
    mut connect_events: MessageReader<pl3xus::NetworkData<ConnectToRobot>>,
    mut robots: Query<(Entity, &mut RobotConnectionState, &mut RobotConnectionDetails, &mut ConnectionState), With<FanucRobot>>,
) {
    for event in connect_events.read() {
        let msg: &ConnectToRobot = &*event;
        info!("📡 Received ConnectToRobot: {:?}:{} (connection_id: {:?})", msg.addr, msg.port, msg.connection_id);

        match robots.single_mut() {
            Ok((entity, mut state, mut details, mut conn_state)) => {
                if *state == RobotConnectionState::Disconnected {
                    // If connection_id is provided, use saved connection details
                    if msg.connection_id.is_some() {
                        conn_state.active_connection_id = msg.connection_id;
                    }

                    // Only update addr/port if provided (not empty)
                    if !msg.addr.is_empty() {
                        details.addr = msg.addr.clone();
                    }
                    if msg.port > 0 {
                        details.port = msg.port;
                    }
                    if let Some(ref name) = msg.name {
                        details.name = name.clone();
                        conn_state.robot_name = name.clone();
                    }

                    // Set connecting state
                    *state = RobotConnectionState::Connecting;
                    conn_state.robot_connecting = true;
                    conn_state.robot_connected = false;

                    info!("🔄 Robot {:?} transitioning to Connecting state", entity);
                } else {
                    warn!("Robot already in state {:?}, ignoring connect request", *state);
                }
            }
            Err(e) => warn!("No single robot found: {:?}", e),
        }
    }
}

/// Handle robots in Connecting state - spawn async task to connect.
fn handle_connecting_state(
    tokio: Res<TokioTasksRuntime>,
    mut commands: Commands,
    robots: Query<(Entity, &RobotConnectionDetails, &RobotConnectionState), (With<FanucRobot>, Without<ConnectionInProgress>)>,
) {
    for (entity, details, state) in robots.iter() {
        if *state != RobotConnectionState::Connecting {
            continue;
        }

        // Mark as in-progress to prevent duplicate attempts
        commands.entity(entity).insert(ConnectionInProgress);

        let config = FanucDriverConfig {
            addr: details.addr.clone(),
            port: details.port,
            max_messages: 30,
        };

        let robot_name = details.name.clone();

        info!("🔌 Connecting to {}:{}", details.addr, details.port);

        // Spawn async connection task
        tokio.spawn_background_task(move |mut ctx| async move {
            match FanucDriver::connect(config).await {
                Ok(driver) => {
                    driver.initialize();
                    let driver_arc = Arc::new(driver);
                    let response_rx = driver_arc.response_tx.subscribe();

                    ctx.run_on_main_thread(move |ctx| {
                        if let Ok(mut entity_mut) = ctx.world.get_entity_mut(entity) {
                            entity_mut.remove::<ConnectionInProgress>();
                            entity_mut.insert(RmiDriver(driver_arc.clone()));
                            entity_mut.insert(RmiResponseChannel(response_rx));
                            entity_mut.insert(RobotConnectionState::Connected);

                            if let Some(mut conn_state) = entity_mut.get_mut::<ConnectionState>() {
                                conn_state.robot_connected = true;
                                conn_state.robot_connecting = false;
                                conn_state.robot_addr = format!("{}:{}",
                                    driver_arc.config.addr, driver_arc.config.port);
                                conn_state.robot_name = robot_name.clone();
                                conn_state.connection_name = Some(robot_name);
                                conn_state.tp_initialized = true;
                            }

                            info!("✅ Robot {:?} connected successfully", entity);
                        }
                    }).await;
                }
                Err(e) => {
                    error!("❌ Connection failed: {}", e);
                    ctx.run_on_main_thread(move |ctx| {
                        if let Ok(mut entity_mut) = ctx.world.get_entity_mut(entity) {
                            entity_mut.remove::<ConnectionInProgress>();
                            entity_mut.insert(RobotConnectionState::Disconnected);

                            if let Some(mut conn_state) = entity_mut.get_mut::<ConnectionState>() {
                                conn_state.robot_connected = false;
                                conn_state.robot_connecting = false;
                            }
                        }
                    }).await;
                }
            }
        });
    }
}

/// Handle disconnect requests.
fn handle_disconnect_requests(
    mut commands: Commands,
    mut disconnect_events: MessageReader<pl3xus::NetworkData<DisconnectRobot>>,
    mut robots: Query<(Entity, &mut RobotConnectionState, &mut ConnectionState), With<FanucRobot>>,
) {
    for _event in disconnect_events.read() {
        info!("📡 Received DisconnectRobot");

        if let Ok((entity, mut state, mut conn_state)) = robots.single_mut() {
            if *state == RobotConnectionState::Connected {
                *state = RobotConnectionState::Disconnected;
                conn_state.robot_connected = false;
                conn_state.robot_connecting = false;
                conn_state.robot_addr = String::new();
                conn_state.robot_name = String::new();
                conn_state.connection_name = None;
                conn_state.active_connection_id = None;
                commands.entity(entity).remove::<RmiDriver>();
                commands.entity(entity).remove::<RmiResponseChannel>();
                info!("🔌 Robot {:?} disconnected", entity);
            }
        }
    }
}
