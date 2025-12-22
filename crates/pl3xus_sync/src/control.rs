//! Optional utilities for exclusive control transfer patterns.
//!
//! This module provides an optional `ExclusiveControlPlugin` that implements
//! common patterns for exclusive control transfer. Applications can use this
//! plugin to reduce boilerplate, or implement their own custom control patterns.
//!
//! # Example (Builder Pattern - Recommended)
//!
//! ```rust,no_run
//! use bevy::prelude::*;
//! use pl3xus_sync::control::ExclusiveControlPlugin;
//! use pl3xus_websockets::WebSocketProvider;
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(
//!             ExclusiveControlPlugin::builder()
//!                 .timeout_seconds(30.0)           // 30 second timeout
//!                 .propagate_to_children(true)     // Control parent = control children
//!                 .build::<WebSocketProvider>()
//!         )
//!         .run();
//! }
//! ```
//!
//! # Example (Quick Start with Defaults)
//!
//! ```rust,no_run
//! use bevy::prelude::*;
//! use pl3xus_sync::control::ExclusiveControlPlugin;
//! use pl3xus_websockets::WebSocketProvider;
//!
//! fn main() {
//!     App::new()
//!         // One-liner with defaults: 30 min timeout, propagate to children
//!         .add_plugins(ExclusiveControlPlugin::default_with_provider::<WebSocketProvider>())
//!         .run();
//! }
//! ```
//!
//! # Authorization Policy
//!
//! This plugin installs a default [`EntityAccessPolicy`](crate::EntityAccessPolicy)
//! that checks [`EntityControl`] components. Messages registered with
//! `.targeted().with_entity_policy(...)` will use this policy unless overridden
//! with a per-message policy.
//!
//! For custom authorization per message type, use the builder pattern:
//!
//! ```rust,ignore
//! // Uses the default EntityControl-based policy
//! app.message::<JogCommand, NP>()
//!    .targeted()
//!    .with_default_entity_policy()  // Use the plugin's default policy
//!    .register();
//!
//! // Custom policy for this specific message type
//! app.message::<AdminCommand, NP>()
//!    .targeted()
//!    .with_entity_policy(EntityAccessPolicy::from_fn(custom_check))
//!    .register();
//! ```

use bevy::prelude::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::authorization::{DefaultEntityAccessPolicy, EntityAccessPolicy};

// Re-export control types from pl3xus_common (with Message derive via ecs feature)
pub use pl3xus_common::{
    AssociateSubConnection, AssociateSubConnectionResponse,
    ConnectionId, ControlRequest, ControlResponse, ControlResponseKind, EntityControl,
};

// ============================================================================
// SUB-CONNECTION TRACKING
// ============================================================================

/// Resource that tracks sub-connections for each parent connection.
///
/// This is used to look up sub-connection IDs when a client takes control
/// of an entity, so that all sub-connections can be authorized to send commands.
#[derive(Resource, Default, Clone, Debug)]
pub struct SubConnections {
    /// Map from parent connection ID to list of sub-connection IDs.
    pub by_parent: HashMap<ConnectionId, Vec<ConnectionId>>,
    /// Map from sub-connection ID to parent connection ID (for reverse lookup).
    pub parent_of: HashMap<ConnectionId, ConnectionId>,
}

impl SubConnections {
    /// Get the sub-connection IDs for a parent connection.
    pub fn get_sub_connections(&self, parent_id: ConnectionId) -> Vec<ConnectionId> {
        self.by_parent.get(&parent_id).cloned().unwrap_or_default()
    }

    /// Get the parent connection ID for a sub-connection.
    pub fn get_parent(&self, sub_id: ConnectionId) -> Option<ConnectionId> {
        self.parent_of.get(&sub_id).copied()
    }

    /// Associate a sub-connection with a parent connection.
    pub fn associate(&mut self, parent_id: ConnectionId, sub_id: ConnectionId) {
        // Remove from any existing parent first
        if let Some(old_parent) = self.parent_of.remove(&sub_id) {
            if let Some(subs) = self.by_parent.get_mut(&old_parent) {
                subs.retain(|&id| id != sub_id);
            }
        }

        // Add to new parent
        self.by_parent
            .entry(parent_id)
            .or_default()
            .push(sub_id);
        self.parent_of.insert(sub_id, parent_id);
    }

    /// Remove a sub-connection.
    pub fn remove_sub(&mut self, sub_id: ConnectionId) {
        if let Some(parent) = self.parent_of.remove(&sub_id) {
            if let Some(subs) = self.by_parent.get_mut(&parent) {
                subs.retain(|&id| id != sub_id);
            }
        }
    }

    /// Remove a parent connection and all its sub-connections.
    pub fn remove_parent(&mut self, parent_id: ConnectionId) {
        if let Some(subs) = self.by_parent.remove(&parent_id) {
            for sub_id in subs {
                self.parent_of.remove(&sub_id);
            }
        }
    }
}

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

// ============================================================================
// BUILDER PATTERN
// ============================================================================

/// Builder for configuring the `ExclusiveControlPlugin`.
///
/// # Example
///
/// ```rust,ignore
/// use pl3xus_sync::control::ExclusiveControlPlugin;
///
/// let plugin = ExclusiveControlPlugin::<WebSocketProvider>::builder()
///     .timeout_seconds(300.0)         // 5 minute timeout
///     .propagate_to_children(true)    // Control parent = control children
///     .build();
/// ```
#[derive(Clone, Debug)]
pub struct ExclusiveControlPluginBuilder<NP: crate::NetworkProvider> {
    timeout_seconds: Option<f32>,
    propagate_to_children: bool,
    _marker: std::marker::PhantomData<NP>,
}

impl<NP: crate::NetworkProvider> Default for ExclusiveControlPluginBuilder<NP> {
    fn default() -> Self {
        Self {
            timeout_seconds: Some(1800.0), // 30 minute default
            propagate_to_children: true,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<NP: crate::NetworkProvider> ExclusiveControlPluginBuilder<NP> {
    /// Create a new builder with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the timeout in seconds after which inactive control is released.
    ///
    /// Default: 1800.0 (30 minutes)
    ///
    /// Set to `0.0` or use `.no_timeout()` to disable timeout.
    pub fn timeout_seconds(mut self, seconds: f32) -> Self {
        self.timeout_seconds = if seconds <= 0.0 {
            None
        } else {
            Some(seconds)
        };
        self
    }

    /// Disable the inactivity timeout (control is held indefinitely).
    pub fn no_timeout(mut self) -> Self {
        self.timeout_seconds = None;
        self
    }

    /// Set whether control of a parent entity grants control of children.
    ///
    /// Default: true
    ///
    /// When enabled, if a client takes control of an entity, they also
    /// gain control of all child entities in the hierarchy.
    pub fn propagate_to_children(mut self, propagate: bool) -> Self {
        self.propagate_to_children = propagate;
        self
    }

    /// Build the plugin.
    ///
    /// This is the final step - it creates a plugin that can be added to
    /// your Bevy app with `.add_plugins(...)`.
    pub fn build(self) -> ExclusiveControlPlugin<NP> {
        ExclusiveControlPlugin {
            config: ExclusiveControlConfig {
                timeout_seconds: self.timeout_seconds,
                propagate_to_children: self.propagate_to_children,
            },
            _marker: std::marker::PhantomData,
        }
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

// ============================================================================
// PLUGIN
// ============================================================================

/// Plugin that provides exclusive control transfer utilities.
///
/// This plugin handles:
/// - Control request/release messages
/// - Exclusive control semantics (only one client can control an entity)
/// - Optional timeout for inactive clients
/// - Optional hierarchy propagation (parent control grants child control)
/// - Default authorization policy for targeted messages
///
/// # Example (Builder Pattern - Recommended)
///
/// ```rust,ignore
/// use pl3xus_sync::control::ExclusiveControlPlugin;
///
/// app.add_plugins(
///     ExclusiveControlPlugin::builder()
///         .timeout_seconds(300.0)
///         .build::<WebSocketProvider>()
/// );
/// ```
///
/// # Example (Quick Start)
///
/// ```rust,ignore
/// app.add_plugins(ExclusiveControlPlugin::default_with_provider::<WebSocketProvider>());
/// ```
pub struct ExclusiveControlPlugin<NP: crate::NetworkProvider> {
    config: ExclusiveControlConfig,
    _marker: std::marker::PhantomData<NP>,
}

impl<NP: crate::NetworkProvider> ExclusiveControlPlugin<NP> {
    /// Create a plugin builder for configuration.
    ///
    /// This is the recommended way to create the plugin.
    pub fn builder() -> ExclusiveControlPluginBuilder<NP> {
        ExclusiveControlPluginBuilder::new()
    }

    /// Create a plugin with default settings for the specified network provider.
    ///
    /// This is a convenience method for quick setup:
    /// - 30 minute inactivity timeout
    /// - Control propagates to child entities
    pub fn default_with_provider() -> Self {
        Self::builder().build()
    }

    /// Create a policy that uses the EntityControl component for authorization.
    ///
    /// This is the same policy installed by the plugin as the default.
    /// Use this when you want to explicitly use entity control authorization
    /// for a specific message type.
    pub fn control_based_policy(&self) -> EntityAccessPolicy {
        let propagate = self.config.propagate_to_children;
        EntityAccessPolicy::from_fn(move |world, source, entity| {
            exclusive_control_authorization_check(world, source, entity, propagate)
        })
    }
}

impl<NP: crate::NetworkProvider> Plugin for ExclusiveControlPlugin<NP> {
    fn build(&self, app: &mut App) {
        use pl3xus::AppNetworkMessage;

        // Insert the config as a resource
        app.insert_resource(self.config.clone());

        // Initialize sub-connections tracking
        app.init_resource::<SubConnections>();

        // Register messages as Bevy messages
        app.add_message::<ControlRequest>();
        app.add_message::<ControlResponse>();
        app.add_message::<AssociateSubConnection>();
        app.add_message::<AssociateSubConnectionResponse>();

        // Register control messages with the network provider
        app.register_network_message::<ControlRequest, NP>();
        app.register_network_message::<ControlResponse, NP>();
        app.register_network_message::<AssociateSubConnection, NP>();
        app.register_network_message::<AssociateSubConnectionResponse, NP>();

        // Install the default entity access policy for targeted messages.
        // This policy uses EntityControl to determine if a client can send commands
        // to a specific entity. Individual message types can override this with
        // their own per-message policy.
        let propagate = self.config.propagate_to_children;
        app.insert_resource(DefaultEntityAccessPolicy(EntityAccessPolicy::from_fn(
            move |world, source, entity| {
                exclusive_control_authorization_check(world, source, entity, propagate)
            },
        )));

        // Add the control handling systems
        app.add_systems(
            Update,
            (
                handle_sub_connection_requests::<NP>,
                handle_control_requests::<NP>,
                update_entity_control_sub_connections,
                cleanup_disconnected_control::<NP>,
                timeout_inactive_control,
                propagate_control_to_new_children,
                notify_control_changes,
            )
                .chain(),
        );
    }
}

// Legacy compatibility: allow creating plugin without network provider type param
// (requires separate add_exclusive_control_systems call)

/// Legacy plugin configuration without network provider.
///
/// Deprecated: Use `ExclusiveControlPlugin::builder().build::<NP>()` instead.
#[deprecated(
    since = "0.2.0",
    note = "Use ExclusiveControlPlugin::builder().build::<NP>() instead"
)]
pub struct LegacyExclusiveControlPlugin {
    config: ExclusiveControlConfig,
}

#[allow(deprecated)]
impl LegacyExclusiveControlPlugin {
    /// Create with the specified configuration.
    pub fn new(config: ExclusiveControlConfig) -> Self {
        Self { config }
    }
}

#[allow(deprecated)]
impl Default for LegacyExclusiveControlPlugin {
    fn default() -> Self {
        Self::new(ExclusiveControlConfig::default())
    }
}

#[allow(deprecated)]
impl Plugin for LegacyExclusiveControlPlugin {
    fn build(&self, app: &mut App) {
        // Insert the config as a resource
        app.insert_resource(self.config.clone());

        // Register messages as Bevy messages
        app.add_message::<ControlRequest>();
        app.add_message::<ControlResponse>();

        // Install the default entity access policy
        let propagate = self.config.propagate_to_children;
        app.insert_resource(DefaultEntityAccessPolicy(EntityAccessPolicy::from_fn(
            move |world, source, entity| {
                exclusive_control_authorization_check(world, source, entity, propagate)
            },
        )));

        // Note: User must call add_exclusive_control_systems separately
    }
}

/// Check if a client is authorized to send a targeted message to an entity.
///
/// Authorization is granted if:
/// - The source is the server (always authorized)
/// - The entity's EntityControl.client_id matches the source
/// - The source is in the entity's EntityControl.sub_connection_ids
/// - If `check_hierarchy` is true: any ancestor has control matching the source
///
/// Authorization is DENIED if:
/// - The entity has no EntityControl component
/// - The entity's EntityControl.client_id is 0 (no one has control - must take control first)
/// - The entity is controlled by a different client
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

    // Check if the source client has control of the entity.
    // Control semantics:
    // - client_id == 0 means NO ONE has control, so NO ONE can send commands
    // - client_id == X means client X has control, only they can send commands
    // - Taking control is done via ControlRequest, not by sending commands
    let check_control = |control: &EntityControl| {
        // has_control checks both primary client_id and sub_connection_ids
        control.has_control(source)
    };

    let (authorized, reason) = if check_hierarchy {
        // Use hierarchical check - parent control grants child control
        let has_control = crate::has_control_hierarchical::<EntityControl, _>(world, entity, check_control);
        if has_control {
            (true, String::new())
        } else {
            // Check what state the entity/ancestors are in for better error messages
            // First, get the entity's own control state for debugging
            let entity_control = world.get_entity(entity)
                .ok()
                .and_then(|e| e.get::<EntityControl>())
                .cloned();

            debug!(
                "[ExclusiveControl] Auth check failed for {:?} from {:?}. Entity control: {:?}",
                entity, source, entity_control
            );

            // Check if entity (or ancestors) have no controller (client_id == 0)
            let no_controller = crate::has_control_hierarchical::<EntityControl, _>(world, entity, |c| c.client_id.id == 0);
            if no_controller {
                (false, "No client has control of this entity. Take control first.".to_string())
            } else {
                // Someone else has control - find out who
                let controller = entity_control.map(|c| c.client_id.id).unwrap_or(0);
                (false, format!("Entity controlled by client {} (you are {:?})", controller, source))
            }
        }
    } else {
        // Just check this entity
        match world.get_entity(entity) {
            Ok(entity_ref) => {
                match entity_ref.get::<EntityControl>() {
                    Some(control) => {
                        if check_control(control) {
                            (true, String::new())
                        } else if control.client_id.id == 0 {
                            (false, "No client has control of this entity. Take control first.".to_string())
                        } else {
                            (false, format!("Entity controlled by client {} (you are {:?})", control.client_id.id, source))
                        }
                    }
                    None => (false, "Entity has no control component".to_string()),
                }
            }
            Err(_) => (false, "Entity does not exist".to_string()),
        }
    };

    if authorized {
        Ok(())
    } else {
        Err(reason)
    }
}

/// Deprecated: Extension trait for adding control systems separately.
///
/// This is no longer needed - the new `ExclusiveControlPlugin::builder().build::<NP>()`
/// automatically adds all control systems.
///
/// # Migration
///
/// ```rust,ignore
/// // Old way (deprecated):
/// app.add_plugins(ExclusiveControlPlugin::default())
///    .add_exclusive_control_systems::<WebSocketProvider>();
///
/// // New way:
/// app.add_plugins(ExclusiveControlPlugin::default_with_provider::<WebSocketProvider>());
/// // OR with configuration:
/// app.add_plugins(
///     ExclusiveControlPlugin::builder()
///         .timeout_seconds(30.0)
///         .build::<WebSocketProvider>()
/// );
/// ```
#[deprecated(
    since = "0.2.0",
    note = "Use ExclusiveControlPlugin::builder().build::<NP>() which includes systems automatically"
)]
pub trait AppExclusiveControlExt {
    /// Deprecated: Add the exclusive control systems for the specified network provider.
    fn add_exclusive_control_systems<NP: crate::NetworkProvider>(&mut self) -> &mut Self;
}

#[allow(deprecated)]
impl AppExclusiveControlExt for App {
    fn add_exclusive_control_systems<NP: crate::NetworkProvider>(&mut self) -> &mut Self {
        use pl3xus::AppNetworkMessage;

        // Initialize sub-connections tracking
        self.init_resource::<SubConnections>();

        // Register messages with the network provider
        self.register_network_message::<ControlRequest, NP>();
        self.register_network_message::<ControlResponse, NP>();
        self.register_network_message::<AssociateSubConnection, NP>();
        self.register_network_message::<AssociateSubConnectionResponse, NP>();

        // Add the control systems
        self.add_systems(
            Update,
            (
                handle_sub_connection_requests::<NP>,
                handle_control_requests::<NP>,
                update_entity_control_sub_connections,
                cleanup_disconnected_control::<NP>,
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
/// - Includes sub-connections when granting control
/// - Sends responses back to the requesting client
fn handle_control_requests<NP: crate::NetworkProvider>(
    mut requests: MessageReader<NetworkData<ControlRequest>>,
    mut entities: Query<(Entity, Option<&mut EntityControl>, Option<&Children>)>,
    config: Res<ExclusiveControlConfig>,
    sub_connections: Option<Res<SubConnections>>,
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

                // Get sub-connections for this client
                let sub_connection_ids = sub_connections
                    .as_ref()
                    .map(|sc| sc.get_sub_connections(client_id))
                    .unwrap_or_default();

                // Grant control
                info!("[ExclusiveControl] Granting control of {:?} to {:?} (with {} sub-connections)",
                    entity, client_id, sub_connection_ids.len());
                let control = EntityControl {
                    client_id,
                    sub_connection_ids,
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

/// System that handles sub-connection association requests.
///
/// When a client sends an `AssociateSubConnection` message, this system
/// registers the requesting connection as a sub-connection of the specified parent.
fn handle_sub_connection_requests<NP: crate::NetworkProvider>(
    mut requests: MessageReader<NetworkData<AssociateSubConnection>>,
    mut sub_connections: ResMut<SubConnections>,
    net: Res<Network<NP>>,
) {
    for request in requests.read() {
        let sub_id = *request.source();
        let parent_id = request.parent_connection_id;

        info!(
            "[ExclusiveControl] Associating sub-connection {:?} with parent {:?}",
            sub_id, parent_id
        );

        // Register the sub-connection
        sub_connections.associate(parent_id, sub_id);

        // Send success response
        let _ = net.send(
            sub_id,
            AssociateSubConnectionResponse {
                success: true,
                error: None,
                parent_connection_id: parent_id,
            },
        );
    }
}

/// System that updates EntityControl components when sub-connections change.
///
/// When a new sub-connection is associated with a parent, this system updates
/// all EntityControl components where the parent has control to include the
/// new sub-connection.
fn update_entity_control_sub_connections(
    sub_connections: Res<SubConnections>,
    mut entities: Query<&mut EntityControl>,
) {
    // Only run if sub_connections changed
    if !sub_connections.is_changed() {
        return;
    }

    for mut control in entities.iter_mut() {
        // Skip entities with no controller
        if control.client_id.id == 0 {
            continue;
        }

        // Update sub-connection IDs from the SubConnections resource
        let new_sub_ids = sub_connections.get_sub_connections(control.client_id);
        if control.sub_connection_ids != new_sub_ids {
            control.sub_connection_ids = new_sub_ids;
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
/// This system listens for `NetworkEvent::Disconnected` events and:
/// 1. Resets `EntityControl` components to the default (no client) state for any entities
///    controlled by that client
/// 2. Removes the client from sub-connections tracking
/// 3. Removes the client from any EntityControl sub_connection_ids lists
fn cleanup_disconnected_control<NP: crate::NetworkProvider>(
    mut events: MessageReader<pl3xus::NetworkEvent>,
    mut entities: Query<(Entity, &mut EntityControl, Option<&Children>)>,
    config: Res<ExclusiveControlConfig>,
    mut sub_connections: ResMut<SubConnections>,
    mut commands: Commands,
) {
    for event in events.read() {
        if let pl3xus::NetworkEvent::Disconnected(disconnected_id) = event {
            info!(
                "[ExclusiveControl] Client {:?} disconnected, releasing any controlled entities",
                disconnected_id
            );

            // Clean up sub-connections tracking
            // If this was a parent, remove all its sub-connections
            sub_connections.remove_parent(*disconnected_id);
            // If this was a sub-connection, remove it from its parent
            sub_connections.remove_sub(*disconnected_id);

            for (entity, mut control, children) in entities.iter_mut() {
                // Check if this client was the primary controller
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
                } else {
                    // Remove from sub_connection_ids if present
                    control.sub_connection_ids.retain(|id| id != disconnected_id);
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

/// System that propagates control to newly spawned child entities.
///
/// When a child entity is added to a parent that has `EntityControl`, this system
/// copies the parent's control to the child. This ensures that children spawned
/// after control is granted still inherit the parent's control state.
///
/// This runs every frame and checks for entities that:
/// 1. Have a `ChildOf` component (are children)
/// 2. Don't have an `EntityControl` component yet
/// 3. Have a parent with `EntityControl`
///
/// When `propagate_to_children` is enabled in the config, the parent's control
/// is copied to the child.
fn propagate_control_to_new_children(
    config: Res<ExclusiveControlConfig>,
    children_without_control: Query<(Entity, &ChildOf), Without<EntityControl>>,
    parents_with_control: Query<&EntityControl>,
    mut commands: Commands,
) {
    if !config.propagate_to_children {
        return;
    }

    for (child_entity, child_of) in children_without_control.iter() {
        let parent_entity = child_of.parent();
        // Check if parent has EntityControl - always propagate to maintain hierarchy
        if let Ok(parent_control) = parents_with_control.get(parent_entity) {
            debug!(
                "[ExclusiveControl] Propagating control from parent {:?} to new child {:?}",
                parent_entity,
                child_entity
            );
            commands.entity(child_entity).insert(parent_control.clone());
        }
    }
}
