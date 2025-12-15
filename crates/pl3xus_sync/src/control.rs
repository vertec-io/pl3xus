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
use pl3xus_common::ConnectionId;
use serde::{Deserialize, Serialize};

/// Request to take or release control of an entity.
#[derive(Message, Serialize, Deserialize, Clone, Debug)]
pub enum ControlRequest {
    /// Request to take control of the specified entity.
    Take(u64),
    /// Request to release control of the specified entity.
    Release(u64),
}

/// Response to a control request.
#[derive(Message, Serialize, Deserialize, Clone, Debug)]
pub enum ControlResponse {
    /// Control was successfully taken.
    Taken,
    /// Control was successfully released.
    Released,
    /// Entity is already controlled by another client.
    AlreadyControlled {
        /// The client that currently has control.
        by_client: ConnectionId,
    },
    /// Entity is not currently controlled (when trying to release).
    NotControlled,
    /// An error occurred.
    Error(String),
}

/// Component that tracks which client has control of an entity.
///
/// This is a default control component that can be used with `ExclusiveControlPlugin`.
/// Applications can also define their own control components with additional fields.
#[derive(Component, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EntityControl {
    /// The client that currently has control.
    pub client_id: ConnectionId,
    /// Timestamp of last activity (for timeout detection).
    pub last_activity: f32,
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
            timeout_seconds: Some(30.0), // 30 second default timeout
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

        // Note: The user must also register these messages with their network provider
        // and add the control systems. See `AppExclusiveControlExt` trait below.
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

        match **request {
            ControlRequest::Take(entity_bits) => {
                let entity = Entity::from_bits(entity_bits);

                // Try to get the entity
                let Ok((entity, control, children)) = entities.get_mut(entity) else {
                    let _ = net.send(client_id, ControlResponse::Error("Entity not found".to_string()));
                    continue;
                };

                // Check if already controlled by another client
                if let Some(existing_control) = control {
                    if existing_control.client_id != client_id {
                        let _ = net.send(
                            client_id,
                            ControlResponse::AlreadyControlled {
                                by_client: existing_control.client_id,
                            },
                        );
                        continue;
                    } else {
                        // Already controlled by this client, just update activity
                        let _ = net.send(client_id, ControlResponse::Taken);
                        continue;
                    }
                }

                // Grant control
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

                let _ = net.send(client_id, ControlResponse::Taken);
            }

            ControlRequest::Release(entity_bits) => {
                let entity = Entity::from_bits(entity_bits);

                // Try to get the entity
                let Ok((entity, control, children)) = entities.get_mut(entity) else {
                    let _ = net.send(client_id, ControlResponse::Error("Entity not found".to_string()));
                    continue;
                };

                // Check if controlled by this client
                if let Some(existing_control) = control {
                    if existing_control.client_id != client_id {
                        let _ = net.send(
                            client_id,
                            ControlResponse::Error("Not controlled by you".to_string()),
                        );
                        continue;
                    }

                    // Release control
                    commands.entity(entity).remove::<EntityControl>();

                    // Propagate to children if configured
                    if config.propagate_to_children {
                        if let Some(children) = children {
                            for child in children.iter() {
                                commands.entity(child).remove::<EntityControl>();
                            }
                        }
                    }

                    let _ = net.send(client_id, ControlResponse::Released);
                } else {
                    let _ = net.send(client_id, ControlResponse::NotControlled);
                }
            }
        }
    }
}

/// System that automatically releases control from inactive clients.
///
/// This system checks all entities with `EntityControl` and removes control
/// if the client has been inactive for longer than the configured timeout.
fn timeout_inactive_control(
    entities: Query<(Entity, &EntityControl, Option<&Children>)>,
    config: Res<ExclusiveControlConfig>,
    mut commands: Commands,
    time: Res<Time>,
) {
    let Some(timeout_seconds) = config.timeout_seconds else {
        return; // No timeout configured
    };

    let current_time = time.elapsed_secs();

    for (entity, control, children) in entities.iter() {
        let inactive_duration = current_time - control.last_activity;

        if inactive_duration > timeout_seconds {
            info!(
                "[ExclusiveControl] Releasing control from inactive client {:?} on entity {:?} (inactive for {:.1}s)",
                control.client_id, entity, inactive_duration
            );

            // Release control
            commands.entity(entity).remove::<EntityControl>();

            // Propagate to children if configured
            if config.propagate_to_children {
                if let Some(children) = children {
                    for child in children.iter() {
                        commands.entity(child).remove::<EntityControl>();
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

