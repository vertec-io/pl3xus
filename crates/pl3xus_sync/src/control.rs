//! Optional utilities for exclusive control transfer patterns.
//!
//! This module provides an optional `ExclusiveControlPlugin` that implements
//! common patterns for exclusive control transfer. Applications can use this
//! plugin to reduce boilerplate, or implement their own custom control patterns.
//!
//! # Example
//!
//! ```rust,no_run
//! use bevy::prelude::*;
//! use pl3xus_sync::control::{ExclusiveControlPlugin, EntityControl, ExclusiveControlConfig, AppExclusiveControlExt};
//! use pl3xus_websockets::WebSocketProvider;
//!
//! fn main() {
//!     let mut app = App::new();
//!     
//!     // Add the exclusive control plugin
//!     app.add_plugins(ExclusiveControlPlugin::new(ExclusiveControlConfig {
//!         timeout_seconds: Some(30.0),  // 30 second timeout
//!         propagate_to_children: true,   // Control parent = control children
//!     }));
//!     
//!     // Add the control systems for your network provider
//!     app.add_exclusive_control_systems::<WebSocketProvider>();
//!     
//!     // Sync the control component so clients can see who has control
//!     app.sync_component::<EntityControl>(None);
//!     
//!     app.run();
//! }
//! ```

use bevy::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};

// Re-export control types from pl3xus_common (with Message derive via ecs feature)
pub use pl3xus_common::{ConnectionId, ControlRequest, ControlResponse, ControlResponseKind, EntityControl};

/// Global sequence counter for control responses.
/// Each response gets a unique sequence number to ensure identical responses
/// are treated as distinct messages by the client.
static RESPONSE_SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// Create a new ControlResponse with a unique sequence number.
fn new_response(kind: ControlResponseKind) -> ControlResponse {
    ControlResponse {
        sequence: RESPONSE_SEQUENCE.fetch_add(1, Ordering::Relaxed),
        kind,
    }
}

/// Configuration for the `ExclusiveControlPlugin`.
#[derive(Clone, Debug, Resource)]
pub struct ExclusiveControlConfig {
    /// Timeout in seconds after which inactive control is released.
    /// `None` means no timeout.
    pub timeout_seconds: Option<f32>,
    /// Whether to propagate control to child entities.
    /// If `true`, taking control of a parent entity also grants control of all children.
    pub propagate_to_children: bool,
}

impl Default for ExclusiveControlConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: Some(1800.0), // 30 minute default timeout
            propagate_to_children: true,
        }
    }
}

/// Optional plugin that provides exclusive control transfer utilities.
///
/// This plugin handles:
/// - Control request/release messages
/// - Exclusive control semantics (only one client can control an entity)
/// - Optional timeout for inactive clients
/// - Optional hierarchy propagation (parent control grants child control)
/// - State synchronization (notifies clients of control changes)
///
/// # Example
///
/// ```rust,no_run
/// use bevy::prelude::*;
/// use pl3xus_sync::control::{ExclusiveControlPlugin, EntityControl, AppExclusiveControlExt};
/// use pl3xus_websockets::WebSocketProvider;
///
/// fn main() {
///     App::new()
///         .add_plugins(ExclusiveControlPlugin::default())
///         .add_exclusive_control_systems::<WebSocketProvider>()
///         .sync_component::<EntityControl>(None)
///         .run();
/// }
/// ```
pub struct ExclusiveControlPlugin {
    config: ExclusiveControlConfig,
}

impl ExclusiveControlPlugin {
    /// Create a new `ExclusiveControlPlugin` with the specified configuration.
    pub fn new(config: ExclusiveControlConfig) -> Self {
        Self { config }
    }
}

impl Default for ExclusiveControlPlugin {
    fn default() -> Self {
        Self::new(ExclusiveControlConfig::default())
    }
}

impl Plugin for ExclusiveControlPlugin {
    fn build(&self, app: &mut App) {
        // Insert the config as a resource
        app.insert_resource(self.config.clone());

        // Register messages as Bevy messages
        app.add_message::<ControlRequest>();
        app.add_message::<ControlResponse>();

        // Install the exclusive control authorization policy for targeted messages.
        // This policy uses EntityControl to determine if a client can send commands
        // to a specific entity.
        let propagate = self.config.propagate_to_children;
        app.insert_resource(crate::TargetedAuthorizerResource::from_fn(
            move |world, source, entity| {
                exclusive_control_authorization_check(world, source, entity, propagate)
            },
        ));

        // Note: The user must also register these messages with their network provider
        // and add the control systems. See `AppExclusiveControlExt` trait below.
    }
}

/// Check if a client is authorized to send a targeted message to an entity.
///
/// Authorization is granted if:
/// - The source is the server (always authorized)
/// - The entity has no EntityControl component (uncontrolled = anyone can command)
/// - The entity's EntityControl.client_id matches the source
/// - The entity's EntityControl.client_id is 0 (default = no active controller)
/// - If `check_hierarchy` is true: any ancestor has control matching the source
fn exclusive_control_authorization_check(
    world: &World,
    source: ConnectionId,
    entity: Entity,
    check_hierarchy: bool,
) -> Result<(), String> {
    // Server is always authorized
    if source.is_server() {
        return Ok(());
    }

    let check_control = |control: &EntityControl| {
        // client_id 0 means "no controller" - anyone can command
        control.client_id == source || control.client_id.id == 0
    };

    let authorized = if check_hierarchy {
        // Use hierarchical check - parent control grants child control
        crate::has_control_hierarchical::<EntityControl, _>(world, entity, check_control)
    } else {
        // Just check this entity
        match world.get_entity(entity) {
            Ok(entity_ref) => {
                match entity_ref.get::<EntityControl>() {
                    Some(control) => check_control(control),
                    None => true, // No control component = uncontrolled = allow
                }
            }
            Err(_) => false, // Entity doesn't exist
        }
    };

    if authorized {
        Ok(())
    } else {
        Err("Entity controlled by another client".to_string())
    }
}

/// Extension trait for `App` to add exclusive control systems.
///
/// This trait provides a convenient way to add the control systems for a specific
/// network provider type.
///
/// # Example
///
/// ```rust,no_run
/// use bevy::prelude::*;
/// use pl3xus_sync::control::{ExclusiveControlPlugin, AppExclusiveControlExt};
/// use pl3xus_websockets::WebSocketProvider;
///
/// fn main() {
///     App::new()
///         .add_plugins(ExclusiveControlPlugin::default())
///         .add_exclusive_control_systems::<WebSocketProvider>()
///         .run();
/// }
/// ```
pub trait AppExclusiveControlExt {
    /// Add the exclusive control systems for the specified network provider.
    ///
    /// This also registers the `ControlRequest` and `ControlResponse` messages
    /// with the network provider.
    fn add_exclusive_control_systems<NP: crate::NetworkProvider>(&mut self) -> &mut Self;
}

impl AppExclusiveControlExt for App {
    fn add_exclusive_control_systems<NP: crate::NetworkProvider>(&mut self) -> &mut Self {
        use pl3xus::AppNetworkMessage;

        // Register messages with the network provider
        self.register_network_message::<ControlRequest, NP>();
        self.register_network_message::<ControlResponse, NP>();

        // Add the control systems
        self.add_systems(
            Update,
            (
                handle_control_requests::<NP>,
                cleanup_disconnected_control,
                timeout_inactive_control,
                notify_control_changes,
            )
                .chain(),
        );

        self
    }
}

use bevy::ecs::message::MessageReader;
use pl3xus::{Network, NetworkData};

/// System that handles control take/release requests from clients.
///
/// This system:
/// - Checks if the entity is already controlled by another client
/// - Grants or denies control based on exclusive control semantics
/// - Optionally propagates control to child entities
/// - Sends responses back to the requesting client
fn handle_control_requests<NP: crate::NetworkProvider>(
    mut requests: MessageReader<NetworkData<ControlRequest>>,
    mut entities: Query<(Entity, Option<&mut EntityControl>, Option<&Children>)>,
    config: Res<ExclusiveControlConfig>,
    net: Res<Network<NP>>,
    mut commands: Commands,
    time: Res<Time>,
) {
    for request in requests.read() {
        let client_id = *request.source();
        let current_time = time.elapsed_secs();

        info!("[ExclusiveControl] Received control request from {:?}: {:?}", client_id, **request);

        match **request {
            ControlRequest::Take(entity_bits) => {
                let entity = Entity::from_bits(entity_bits);
                info!("[ExclusiveControl] Take request for entity {:?} from {:?}", entity, client_id);

                // Try to get the entity
                let Ok((entity, control, children)) = entities.get_mut(entity) else {
                    let _ = net.send(client_id, new_response(ControlResponseKind::Error("Entity not found".to_string())));
                    continue;
                };

                // Check if already controlled by another client
                // client_id 0 means "no controller" (default state) so it's available for taking
                if let Some(existing_control) = control {
                    let has_active_controller = existing_control.client_id.id != 0;

                    if has_active_controller && existing_control.client_id != client_id {
                        info!("[ExclusiveControl] Entity {:?} already controlled by {:?}, denying {:?}", entity, existing_control.client_id, client_id);

                        // Notify the requesting client that control is denied
                        let _ = net.send(
                            client_id,
                            new_response(ControlResponseKind::AlreadyControlled {
                                by_client: existing_control.client_id,
                            }),
                        );

                        // Notify the controlling client that someone else is requesting control
                        info!("[ExclusiveControl] Notifying {:?} that {:?} is requesting control", existing_control.client_id, client_id);
                        let _ = net.send(
                            existing_control.client_id,
                            new_response(ControlResponseKind::ControlRequested {
                                by_client: client_id,
                            }),
                        );
                        continue;
                    } else if has_active_controller && existing_control.client_id == client_id {
                        // Already controlled by this client, just update activity
                        info!("[ExclusiveControl] Entity {:?} already controlled by {:?}, refreshing", entity, client_id);
                        let _ = net.send(client_id, new_response(ControlResponseKind::Taken));
                        continue;
                    }
                    // If no active controller (client_id == 0), fall through to grant control
                }

                // Grant control
                info!("[ExclusiveControl] Granting control of {:?} to {:?}", entity, client_id);
                let control = EntityControl {
                    client_id,
                    last_activity: current_time,
                };
                commands.entity(entity).insert(control.clone());

                // Propagate to children if configured
                if config.propagate_to_children {
                    if let Some(children) = children {
                        for child in children.iter() {
                            commands.entity(child).insert(control.clone());
                        }
                    }
                }

                info!("[ExclusiveControl] Sending Taken response to {:?}", client_id);
                let _ = net.send(client_id, new_response(ControlResponseKind::Taken));
            }

            ControlRequest::Release(entity_bits) => {
                let entity = Entity::from_bits(entity_bits);

                // Try to get the entity
                let Ok((_entity, mut control, children)) = entities.get_mut(entity) else {
                    let _ = net.send(client_id, new_response(ControlResponseKind::Error("Entity not found".to_string())));
                    continue;
                };

                // Check if controlled by this client
                if let Some(ref mut existing_control) = control {
                    // Check if there's an active controller and it's not this client
                    if existing_control.client_id.id != 0 && existing_control.client_id != client_id {
                        let _ = net.send(
                            client_id,
                            new_response(ControlResponseKind::Error("Not controlled by you".to_string())),
                        );
                        continue;
                    }

                    // Check if already released (no active controller)
                    if existing_control.client_id.id == 0 {
                        let _ = net.send(client_id, new_response(ControlResponseKind::NotControlled));
                        continue;
                    }

                    // Release control by resetting to default
                    **existing_control = EntityControl::default();

                    // Propagate to children if configured
                    if config.propagate_to_children {
                        if let Some(children) = children {
                            for child in children.iter() {
                                // Note: Children might not have the component if they were spawned
                                // without one - use commands to insert default
                                commands.entity(child).insert(EntityControl::default());
                            }
                        }
                    }

                    let _ = net.send(client_id, new_response(ControlResponseKind::Released));
                } else {
                    let _ = net.send(client_id, new_response(ControlResponseKind::NotControlled));
                }
            }
        }
    }
}

/// System that automatically releases control from inactive clients.
///
/// This system checks all entities with `EntityControl` and resets control
/// to default if the client has been inactive for longer than the configured timeout.
/// Skips entities that are already in default state (no active controller).
fn timeout_inactive_control(
    mut entities: Query<(Entity, &mut EntityControl, Option<&Children>)>,
    config: Res<ExclusiveControlConfig>,
    mut commands: Commands,
    time: Res<Time>,
) {
    let Some(timeout_seconds) = config.timeout_seconds else {
        return; // No timeout configured
    };

    let current_time = time.elapsed_secs();

    for (entity, mut control, children) in entities.iter_mut() {
        // Skip if no one is in control (client_id 0 is the default "no controller" state)
        if control.client_id.id == 0 {
            continue;
        }

        let inactive_duration = current_time - control.last_activity;

        if inactive_duration > timeout_seconds {
            info!(
                "[ExclusiveControl] Releasing control from inactive client {:?} on entity {:?} (inactive for {:.1}s)",
                control.client_id, entity, inactive_duration
            );

            // Reset control to default (no client)
            *control = EntityControl::default();

            // Propagate to children if configured
            if config.propagate_to_children {
                if let Some(children) = children {
                    for child in children.iter() {
                        // Use commands to insert default control on children
                        commands.entity(child).insert(EntityControl::default());
                    }
                }
            }
        }
    }
}

/// System that releases control from disconnected clients.
///
/// This system listens for `NetworkEvent::Disconnected` events and resets
/// `EntityControl` components to the default (no client) state for any entities
/// controlled by that client.
fn cleanup_disconnected_control(
    mut events: MessageReader<pl3xus::NetworkEvent>,
    mut entities: Query<(Entity, &mut EntityControl, Option<&Children>)>,
    config: Res<ExclusiveControlConfig>,
    mut commands: Commands,
) {
    for event in events.read() {
        if let pl3xus::NetworkEvent::Disconnected(disconnected_id) = event {
            info!(
                "[ExclusiveControl] Client {:?} disconnected, releasing any controlled entities",
                disconnected_id
            );

            for (entity, mut control, children) in entities.iter_mut() {
                if control.client_id == *disconnected_id {
                    info!(
                        "[ExclusiveControl] Releasing control from disconnected client {:?} on entity {:?}",
                        disconnected_id, entity
                    );

                    // Reset control to default (no client)
                    *control = EntityControl::default();

                    // Propagate to children if configured
                    if config.propagate_to_children {
                        if let Some(children) = children {
                            for child in children.iter() {
                                // Use commands to insert default control on children
                                commands.entity(child).insert(EntityControl::default());
                            }
                        }
                    }
                }
            }
        }
    }
}

/// System that notifies clients when control state changes.
///
/// This system detects when `EntityControl` components are added or removed
/// and broadcasts the changes to all clients so they can update their UI.
///
/// Note: This relies on the `EntityControl` component being registered with
/// `sync_component` so that changes are automatically synchronized.
fn notify_control_changes() {
    // This is intentionally a no-op because control state synchronization
    // is handled by the normal component sync mechanism.
    //
    // Users should call `app.sync_component::<EntityControl>(None)` to enable
    // automatic synchronization of control state to all clients.
}

