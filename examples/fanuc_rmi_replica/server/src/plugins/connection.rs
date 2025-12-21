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
use super::system::SystemMarker;

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

/// Response channel for receiving driver responses (for polling).
#[derive(Component)]
pub struct RmiResponseChannel(pub broadcast::Receiver<fanuc_rmi::packets::ResponsePacket>);

/// Response channel for program execution (separate subscription to avoid contention).
#[derive(Component)]
pub struct RmiExecutionResponseChannel(pub broadcast::Receiver<fanuc_rmi::packets::ResponsePacket>);

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

/// Handle incoming connection requests - spawn robot entity if needed, then transition to Connecting state.
///
/// If a robot entity doesn't exist, spawn one as a child of the System entity with connection details from:
/// 1. Database (if connection_id is provided)
/// 2. Direct parameters (addr, port, name)
///
/// IMPORTANT: Only the client who has control of the System entity can connect.
fn handle_connect_requests(
    mut commands: Commands,
    db: Option<Res<DatabaseResource>>,
    mut connect_events: MessageReader<pl3xus::NetworkData<ConnectToRobot>>,
    system_query: Query<(Entity, &EntityControl), With<SystemMarker>>,
    mut robots: Query<(Entity, &mut RobotConnectionState, &mut RobotConnectionDetails, &mut ConnectionState), With<FanucRobot>>,
) {
    for event in connect_events.read() {
        let client_id = *event.source();
        let msg: &ConnectToRobot = &*event;
        info!("📡 Received ConnectToRobot: {:?}:{} (connection_id: {:?}) from {:?}", msg.addr, msg.port, msg.connection_id, client_id);

        // Get the System entity and check control
        let Ok((system_entity, system_control)) = system_query.single() else {
            error!("No System entity found - cannot process connection request");
            continue;
        };

        // Check if client has control of the System
        if system_control.client_id != client_id {
            warn!("ConnectToRobot rejected from {:?}: No control of System (held by {:?})", client_id, system_control.client_id);
            continue;
        }

        // Check if robot entity already exists
        let robot_exists = robots.single().is_ok();

        if !robot_exists {
            // No robot entity - spawn one as child of System with connection details from database or message
            let (connection_details, jog_settings) = if let Some(conn_id) = msg.connection_id {
                // Load connection details from database
                if let Some(ref db) = db {
                    match db.get_robot_connection(conn_id) {
                        Ok(Some(robot_conn)) => {
                            info!("📋 Loading robot connection '{}' from database", robot_conn.name);
                            let details = RobotConnectionDetails {
                                addr: robot_conn.ip_address,
                                port: robot_conn.port as u32,
                                name: robot_conn.name,
                            };
                            let jog = JogSettingsState {
                                cartesian_jog_speed: robot_conn.default_cartesian_jog_speed,
                                rotation_jog_speed: robot_conn.default_rotation_jog_speed,
                                cartesian_jog_step: robot_conn.default_cartesian_jog_step,
                                rotation_jog_step: robot_conn.default_rotation_jog_step,
                                joint_jog_speed: robot_conn.default_joint_jog_speed,
                                joint_jog_step: robot_conn.default_joint_jog_step,
                            };
                            (details, jog)
                        }
                        Ok(None) => {
                            warn!("Robot connection {} not found in database", conn_id);
                            continue;
                        }
                        Err(e) => {
                            error!("Failed to load robot connection {}: {}", conn_id, e);
                            continue;
                        }
                    }
                } else {
                    warn!("Database not available, cannot load robot connection");
                    continue;
                }
            } else {
                // Use direct connection details from message
                if msg.addr.is_empty() || msg.port == 0 {
                    warn!("ConnectToRobot requires either connection_id or valid addr/port");
                    continue;
                }
                let details = RobotConnectionDetails {
                    addr: msg.addr.clone(),
                    port: msg.port,
                    name: msg.name.clone().unwrap_or_else(|| format!("{}:{}", msg.addr, msg.port)),
                };
                (details, JogSettingsState::default())
            };

            // Build initial ConnectionState
            let mut initial_conn_state = ConnectionState::default();
            initial_conn_state.active_connection_id = msg.connection_id;
            initial_conn_state.robot_name = connection_details.name.clone();
            initial_conn_state.robot_connecting = true;

            // Spawn the robot entity as a child of the System entity
            let robot_entity = commands.spawn((
                Name::new("FANUC_Robot"),
                FanucRobot,
                connection_details,
                RobotConnectionState::Connecting,
                // Synced state components
                RobotPosition::default(),
                JointAngles::default(),
                RobotStatus::default(),
                IoStatus::default(),
                IoConfigState::default(),
                FrameToolDataState::default(),
                ExecutionState::default(),
                initial_conn_state,
                ActiveConfigState::default(),
                jog_settings,
            )).id();

            // Set the robot as a child of the System entity
            commands.entity(system_entity).add_child(robot_entity);

            info!("🔄 Spawned robot entity {:?} as child of System {:?} in Connecting state", robot_entity, system_entity);
            continue;
        }

        // Robot entity exists - update it
        match robots.single_mut() {
            Ok((entity, mut state, mut details, mut conn_state)) => {
                if *state == RobotConnectionState::Disconnected {
                    // Load connection details from database if connection_id provided
                    if let Some(conn_id) = msg.connection_id {
                        conn_state.active_connection_id = Some(conn_id);
                        if let Some(ref db) = db {
                            if let Ok(Some(robot_conn)) = db.get_robot_connection(conn_id) {
                                details.addr = robot_conn.ip_address;
                                details.port = robot_conn.port as u32;
                                details.name = robot_conn.name.clone();
                                conn_state.robot_name = robot_conn.name;
                            }
                        }
                    } else {
                        // Use direct connection details
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
                    }

                    *state = RobotConnectionState::Connecting;
                    conn_state.robot_connecting = true;
                    conn_state.robot_connected = false;

                    info!("🔄 Robot {:?} transitioning to Connecting state", entity);
                } else {
                    warn!("Robot already in state {:?}, ignoring connect request", *state);
                }
            }
            Err(e) => warn!("Failed to get robot entity: {:?}", e),
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
                    // Use the smart startup sequence that checks status before initializing
                    // Per FANUC B-84184EN_02 manual, this:
                    // 1. Checks robot status (servo ready, AUTO mode)
                    // 2. Aborts if RMI already running
                    // 3. Initializes (which resets sequence counter to 1)
                    if let Err(e) = driver.startup_sequence().await {
                        error!("❌ Robot startup sequence failed: {}", e);
                        ctx.run_on_main_thread(move |ctx| {
                            if let Ok(mut entity_mut) = ctx.world.get_entity_mut(entity) {
                                entity_mut.remove::<ConnectionInProgress>();
                                entity_mut.insert(RobotConnectionState::Disconnected);
                                if let Some(mut conn_state) = entity_mut.get_mut::<ConnectionState>() {
                                    conn_state.robot_connecting = false;
                                    conn_state.robot_connected = false;
                                }
                            }
                        }).await;
                        return;
                    }

                    let driver_arc = Arc::new(driver);
                    // Create separate subscriptions for polling and execution
                    let polling_response_rx = driver_arc.response_tx.subscribe();
                    let execution_response_rx = driver_arc.response_tx.subscribe();
                    let sent_instruction_rx = driver_arc.sent_instruction_tx.subscribe();

                    ctx.run_on_main_thread(move |ctx| {
                        if let Ok(mut entity_mut) = ctx.world.get_entity_mut(entity) {
                            entity_mut.remove::<ConnectionInProgress>();
                            entity_mut.insert(RmiDriver(driver_arc.clone()));
                            entity_mut.insert(RmiResponseChannel(polling_response_rx));
                            entity_mut.insert(RmiExecutionResponseChannel(execution_response_rx));
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
/// IMPORTANT: Only the client who has control of the System entity can disconnect.
/// This properly notifies the FANUC controller before disconnecting.
fn handle_disconnect_requests(
    tokio: Res<TokioTasksRuntime>,
    mut disconnect_events: MessageReader<pl3xus::NetworkData<DisconnectRobot>>,
    system_query: Query<&EntityControl, With<SystemMarker>>,
    mut robots: Query<(Entity, &RmiDriver, &mut RobotConnectionState, &mut ConnectionState), With<FanucRobot>>,
) {
    for event in disconnect_events.read() {
        let client_id = *event.source();
        info!("📡 Received DisconnectRobot from {:?}", client_id);

        // Check if client has control of the System
        let Ok(system_control) = system_query.single() else {
            error!("No System entity found - cannot process disconnect request");
            continue;
        };

        if system_control.client_id != client_id {
            warn!("DisconnectRobot rejected from {:?}: No control of System (held by {:?})", client_id, system_control.client_id);
            continue;
        }

        if let Ok((entity, driver, mut state, mut conn_state)) = robots.single_mut() {
            if *state == RobotConnectionState::Connected {
                // Set state to Disconnecting while we clean up
                *state = RobotConnectionState::Disconnecting;
                conn_state.robot_connected = false;
                conn_state.robot_connecting = false;

                // Clone driver for async task
                let driver_arc = driver.0.clone();

                // Spawn async task to properly disconnect from FANUC controller
                tokio.spawn_background_task(move |mut ctx| async move {
                    // Send disconnect command to FANUC controller
                    match driver_arc.disconnect().await {
                        Ok(response) => {
                            info!("✅ FANUC controller acknowledged disconnect: {:?}", response);
                        }
                        Err(e) => {
                            warn!("⚠️ Failed to send disconnect to FANUC controller: {} (continuing cleanup)", e);
                        }
                    }

                    // Clean up entity on main thread
                    ctx.run_on_main_thread(move |ctx| {
                        if let Ok(mut entity_mut) = ctx.world.get_entity_mut(entity) {
                            entity_mut.remove::<RmiDriver>();
                            entity_mut.remove::<RmiResponseChannel>();
                            entity_mut.remove::<RmiExecutionResponseChannel>();
                            entity_mut.remove::<RmiSentInstructionChannel>();
                            entity_mut.insert(RobotConnectionState::Disconnected);

                            if let Some(mut conn_state) = entity_mut.get_mut::<ConnectionState>() {
                                conn_state.robot_addr = String::new();
                                conn_state.robot_name = String::new();
                                conn_state.connection_name = None;
                                conn_state.active_connection_id = None;
                            }

                            info!("🔌 Robot {:?} disconnected and cleaned up", entity);
                        }
                    }).await;
                });
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
