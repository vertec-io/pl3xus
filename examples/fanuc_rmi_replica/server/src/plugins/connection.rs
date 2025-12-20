//! Robot connection state machine plugin.
//!
//! Handles connecting to and disconnecting from FANUC robots via RMI driver.
//!
//! IMPORTANT: Only the client who has control of the apparatus/system can
//! connect to or disconnect from the robot. The robot connection is a shared
//! resource visible to all clients, but only controllable by the controller.

use bevy::prelude::*;
use bevy::ecs::message::MessageReader;
use bevy_tokio_tasks::TokioTasksRuntime;
use pl3xus::AppNetworkMessage;
use pl3xus_sync::control::EntityControl;
use pl3xus_websockets::WebSocketProvider;
use std::sync::Arc;
use tokio::sync::broadcast;
use fanuc_rmi::drivers::{FanucDriver, FanucDriverConfig, LogLevel};
use fanuc_replica_types::*;
use crate::database::DatabaseResource;
use super::execution::RmiSentInstructionChannel;

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

/// Marker to indicate that the default configuration needs to be loaded.
/// Added when connection succeeds, removed after configuration is loaded.
#[derive(Component)]
struct NeedsDefaultConfigLoad {
    connection_id: Option<i64>,
}

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
            load_default_configuration,
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
        IoConfigState::default(),
        FrameToolDataState::default(),
        ExecutionState::default(),
        ConnectionState::default(),
        ActiveConfigState::default(),
        JogSettingsState::default(),
    ));

    info!("✅ Spawned FANUC_Robot entity (multi-robot ready)");
}

/// Handle incoming connection requests - transition to Connecting state.
///
/// IMPORTANT: Only the client who has control of the apparatus can connect.
/// This enforces the "control = system ownership" model.
fn handle_connect_requests(
    mut connect_events: MessageReader<pl3xus::NetworkData<ConnectToRobot>>,
    mut robots: Query<(Entity, &mut RobotConnectionState, &mut RobotConnectionDetails, &mut ConnectionState, Option<&EntityControl>), With<FanucRobot>>,
) {
    for event in connect_events.read() {
        let client_id = *event.source();
        let msg: &ConnectToRobot = &*event;
        info!("📡 Received ConnectToRobot: {:?}:{} (connection_id: {:?}) from {:?}", msg.addr, msg.port, msg.connection_id, client_id);

        match robots.single_mut() {
            Ok((entity, mut state, mut details, mut conn_state, control)) => {
                // Check if client has control of the apparatus
                if let Some(entity_control) = control {
                    if entity_control.client_id != client_id {
                        warn!("ConnectToRobot rejected from {:?}: No control (held by {:?})", client_id, entity_control.client_id);
                        continue;
                    }
                } else {
                    // No control assigned yet - allow first connection attempt
                    // This is development mode behavior; in production, control should be required
                    trace!("No EntityControl on apparatus {:?}, allowing connect", entity);
                }

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
            log_level: LogLevel::Info,
        };

        let robot_name = details.name.clone();

        info!("🔌 Connecting to {}:{}", details.addr, details.port);

        // Spawn async connection task
        tokio.spawn_background_task(move |mut ctx| async move {
            match FanucDriver::connect(config).await {
                Ok(driver) => {
                    let _ = driver.initialize().await;
                    let driver_arc = Arc::new(driver);
                    let response_rx = driver_arc.response_tx.subscribe();
                    let sent_instruction_rx = driver_arc.sent_instruction_tx.subscribe();

                    ctx.run_on_main_thread(move |ctx| {
                        if let Ok(mut entity_mut) = ctx.world.get_entity_mut(entity) {
                            entity_mut.remove::<ConnectionInProgress>();
                            entity_mut.insert(RmiDriver(driver_arc.clone()));
                            entity_mut.insert(RmiResponseChannel(response_rx));
                            entity_mut.insert(RmiSentInstructionChannel(sent_instruction_rx));
                            entity_mut.insert(RobotConnectionState::Connected);

                            // Get the connection_id before modifying conn_state
                            let connection_id = entity_mut.get::<ConnectionState>()
                                .and_then(|cs| cs.active_connection_id);

                            if let Some(mut conn_state) = entity_mut.get_mut::<ConnectionState>() {
                                conn_state.robot_connected = true;
                                conn_state.robot_connecting = false;
                                conn_state.robot_addr = format!("{}:{}",
                                    driver_arc.config.addr, driver_arc.config.port);
                                conn_state.robot_name = robot_name.clone();
                                conn_state.connection_name = Some(robot_name);
                                conn_state.tp_initialized = true;
                            }

                            // Add marker to load default configuration
                            entity_mut.insert(NeedsDefaultConfigLoad { connection_id });

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
                                // Clear active connection on failure so UI shows "Connect" not "Disconnect"
                                conn_state.active_connection_id = None;
                                conn_state.robot_name = String::new();
                                conn_state.connection_name = None;
                            }
                        }
                    }).await;
                }
            }
        });
    }
}

/// Handle disconnect requests.
///
/// IMPORTANT: Only the client who has control of the apparatus can disconnect.
fn handle_disconnect_requests(
    mut commands: Commands,
    mut disconnect_events: MessageReader<pl3xus::NetworkData<DisconnectRobot>>,
    mut robots: Query<(Entity, &mut RobotConnectionState, &mut ConnectionState, Option<&EntityControl>), With<FanucRobot>>,
) {
    for event in disconnect_events.read() {
        let client_id = *event.source();
        info!("📡 Received DisconnectRobot from {:?}", client_id);

        if let Ok((entity, mut state, mut conn_state, control)) = robots.single_mut() {
            // Check if client has control of the apparatus
            if let Some(entity_control) = control {
                if entity_control.client_id != client_id {
                    warn!("DisconnectRobot rejected from {:?}: No control (held by {:?})", client_id, entity_control.client_id);
                    continue;
                }
            } else {
                trace!("No EntityControl on apparatus {:?}, allowing disconnect", entity);
            }

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
                commands.entity(entity).remove::<RmiSentInstructionChannel>();
                info!("🔌 Robot {:?} disconnected", entity);
            }
        }
    }
}

/// Load the default configuration for a robot after connection.
/// This system runs when a robot has the NeedsDefaultConfigLoad marker.
fn load_default_configuration(
    mut commands: Commands,
    db: Option<Res<DatabaseResource>>,
    mut robots: Query<(Entity, &NeedsDefaultConfigLoad, &mut ActiveConfigState), With<FanucRobot>>,
) {
    for (entity, needs_config, mut active_config) in robots.iter_mut() {
        // Remove the marker first to prevent re-running
        commands.entity(entity).remove::<NeedsDefaultConfigLoad>();

        let Some(connection_id) = needs_config.connection_id else {
            info!("No connection_id provided, skipping default config load");
            continue;
        };

        let Some(ref db) = db else {
            warn!("Database not available, cannot load default configuration");
            continue;
        };

        // Try to load the default configuration for this robot connection
        match db.get_default_configuration_for_robot(connection_id) {
            Ok(Some(config)) => {
                info!("📋 Loading default configuration '{}' for connection {}", config.name, connection_id);
                active_config.loaded_from_id = Some(config.id);
                active_config.loaded_from_name = Some(config.name);
                active_config.u_frame_number = config.u_frame_number;
                active_config.u_tool_number = config.u_tool_number;
                active_config.front = config.front;
                active_config.up = config.up;
                active_config.left = config.left;
                active_config.flip = config.flip;
                active_config.turn4 = config.turn4;
                active_config.turn5 = config.turn5;
                active_config.turn6 = config.turn6;
                active_config.changes_count = 0;
            }
            Ok(None) => {
                info!("No default configuration found for connection {}", connection_id);
            }
            Err(e) => {
                error!("Failed to load default configuration: {}", e);
            }
        }
    }
}
