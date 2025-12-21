//! Pluggable authorization policies for targeted messages.
//!
//! This module provides the [`TargetedMessageAuthorizer`] trait, which mirrors
//! the [`MutationAuthorizer`] pattern but for targeted network messages.
//!
//! Applications can implement their own authorization policies by implementing
//! this trait or using [`TargetedAuthorizerResource::from_fn`] for simple closures.
//!
//! The [`ExclusiveControlPlugin`](crate::control::ExclusiveControlPlugin) provides
//! a built-in implementation that uses [`EntityControl`](crate::control::EntityControl)
//! for authorization decisions.

use bevy::prelude::*;
use pl3xus_common::ConnectionId;
use std::sync::Arc;

/// Context passed to the authorizer when checking targeted messages.
pub struct TargetedAuthContext<'a> {
    /// Read-only access to the ECS world for querying state.
    pub world: &'a World,
    /// The client that sent the targeted message.
    pub source: ConnectionId,
    /// The entity that the message is targeting.
    pub target_entity: Entity,
}

/// Result of an authorization check.
#[derive(Debug, Clone)]
pub enum TargetedAuthResult {
    /// The message is authorized.
    Authorized,
    /// The message is not authorized, with a reason.
    Denied(String),
}

impl TargetedAuthResult {
    pub fn is_authorized(&self) -> bool {
        matches!(self, TargetedAuthResult::Authorized)
    }
}

/// Pluggable policy for deciding if a client can send to a target entity.
///
/// Implementations can inspect arbitrary application state via the
/// [`TargetedAuthContext::world`] reference (for example, checking if a client
/// owns an entity or has the appropriate role).
///
/// # Example
///
/// ```rust,ignore
/// use pl3xus_sync::authorization::{TargetedMessageAuthorizer, TargetedAuthContext, TargetedAuthResult};
///
/// struct RoleBasedAuthorizer;
///
/// impl TargetedMessageAuthorizer for RoleBasedAuthorizer {
///     fn authorize(&self, ctx: &TargetedAuthContext) -> TargetedAuthResult {
///         // Check if client has admin role
///         if let Some(roles) = ctx.world.get_resource::<ClientRoles>() {
///             if roles.is_admin(ctx.source) {
///                 return TargetedAuthResult::Authorized;
///             }
///         }
///         TargetedAuthResult::Denied("Admin role required".to_string())
///     }
/// }
/// ```
pub trait TargetedMessageAuthorizer: Send + Sync + 'static {
    /// Decide whether the message should be authorized.
    fn authorize(&self, ctx: &TargetedAuthContext) -> TargetedAuthResult;
}

/// Resource wrapping the active targeted message authorization policy.
///
/// If this resource is not present, all targeted messages are allowed by default.
/// Applications can install their own policy by inserting this resource into the `App`.
#[derive(Resource, Clone)]
pub struct TargetedAuthorizerResource {
    pub inner: Arc<dyn TargetedMessageAuthorizer>,
}

impl TargetedAuthorizerResource {
    /// Construct an authorizer from a simple closure.
    ///
    /// This is the most convenient way for downstream apps to express custom
    /// authorization logic.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use pl3xus_sync::authorization::TargetedAuthorizerResource;
    ///
    /// app.insert_resource(TargetedAuthorizerResource::from_fn(
    ///     |world, source, entity| {
    ///         // Custom authorization logic
    ///         Ok(())
    ///     }
    /// ));
    /// ```
    pub fn from_fn<F>(f: F) -> Self
    where
        F: Fn(&World, ConnectionId, Entity) -> Result<(), String> + Send + Sync + 'static,
    {
        struct ClosureAuthorizer<F>(F);

        impl<F> TargetedMessageAuthorizer for ClosureAuthorizer<F>
        where
            F: Fn(&World, ConnectionId, Entity) -> Result<(), String> + Send + Sync + 'static,
        {
            fn authorize(&self, ctx: &TargetedAuthContext) -> TargetedAuthResult {
                match (self.0)(ctx.world, ctx.source, ctx.target_entity) {
                    Ok(()) => TargetedAuthResult::Authorized,
                    Err(reason) => TargetedAuthResult::Denied(reason),
                }
            }
        }

        Self {
            inner: Arc::new(ClosureAuthorizer(f)),
        }
    }

    /// Allow all targeted messages (no authorization check).
    pub fn allow_all() -> Self {
        Self::from_fn(|_, _, _| Ok(()))
    }

    /// Only the server can send targeted messages.
    pub fn server_only() -> Self {
        Self::from_fn(|_, source, _| {
            if source.is_server() {
                Ok(())
            } else {
                Err("Only server can send targeted messages".to_string())
            }
        })
    }

    /// Check if a targeted message is authorized.
    pub fn check(&self, world: &World, source: ConnectionId, target_entity: Entity) -> TargetedAuthResult {
        let ctx = TargetedAuthContext {
            world,
            source,
            target_entity,
        };
        self.inner.authorize(&ctx)
    }
}

/// A targeted message that has passed authorization.
///
/// Systems should read this event type instead of `NetworkData<TargetedMessage<T>>`
/// when they want only authorized messages.
///
/// If no [`TargetedAuthorizerResource`] is installed, all targeted messages are
/// authorized and passed through.
#[derive(Debug, Clone, bevy::prelude::Message)]
pub struct AuthorizedMessage<T: pl3xus_common::Pl3xusMessage> {
    /// The original message payload.
    pub message: T,
    /// The client that sent the message.
    pub source: ConnectionId,
    /// The target entity (validated to exist).
    pub target_entity: Entity,
}

use bevy::ecs::message::Messages;
use pl3xus::{Network, NetworkData};
use crate::NetworkProvider;
use pl3xus_common::{Pl3xusMessage, ServerNotification, TargetedMessage};

/// Extension trait for registering authorized messages.
pub trait AppAuthorizedMessageExt {
    /// Register authorization middleware for a targeted message type.
    ///
    /// This adds:
    /// - `AuthorizedMessage<T>` event type
    /// - Authorization middleware system
    ///
    /// Call this AFTER calling `register_message` or `register_targeted_message`.
    fn add_authorized_message<T, NP>(&mut self) -> &mut Self
    where
        T: Pl3xusMessage + Clone + 'static,
        NP: NetworkProvider;
}

impl AppAuthorizedMessageExt for App {
    fn add_authorized_message<T, NP>(&mut self) -> &mut Self
    where
        T: Pl3xusMessage + Clone + 'static,
        NP: NetworkProvider,
    {
        self.add_message::<AuthorizedMessage<T>>();
        self.add_systems(PreUpdate, authorize_targeted_messages::<T, NP>);
        self
    }
}

/// Authorization middleware that filters targeted messages.
///
/// This is an exclusive system that:
/// 1. Reads all incoming `NetworkData<TargetedMessage<T>>`
/// 2. Parses the target_id as entity bits (u64)
/// 3. Checks authorization via `TargetedAuthorizerResource` (if present)
/// 4. Emits `AuthorizedMessage<T>` for authorized messages
/// 5. Drops unauthorized messages (logs a warning)
///
/// Uses exclusive world access to allow the authorizer to query arbitrary state.
fn authorize_targeted_messages<T, NP>(world: &mut World)
where
    T: Pl3xusMessage + Clone + 'static,
    NP: NetworkProvider,
{
    // Collect incoming messages first (we need to drain them before writing)
    let incoming: Vec<_> = {
        let mut messages = world.resource_mut::<Messages<NetworkData<TargetedMessage<T>>>>();
        messages.drain().collect()
    };

    if incoming.is_empty() {
        return;
    }

    // Get the authorizer (if present)
    let auth_res = world.get_resource::<TargetedAuthorizerResource>().cloned();

    // Process each message
    let mut authorized_messages = Vec::new();
    let mut rejections: Vec<(ConnectionId, String)> = Vec::new();

    for msg in incoming {
        let source = *msg.source();

        // Parse target_id as entity bits (u64)
        let entity = match msg.target_id.parse::<u64>() {
            Ok(bits) => Entity::from_bits(bits),
            Err(_) => {
                warn!(
                    "Invalid target_id '{}' from {:?} - expected entity bits (u64)",
                    msg.target_id, source
                );
                rejections.push((source, format!("Invalid target entity: {}", msg.target_id)));
                continue;
            }
        };

        // Check authorization
        let auth_result = match &auth_res {
            Some(res) => res.check(world, source, entity),
            None => TargetedAuthResult::Authorized,
        };

        match auth_result {
            TargetedAuthResult::Authorized => {
                authorized_messages.push(AuthorizedMessage {
                    message: msg.message.clone(),
                    source,
                    target_entity: entity,
                });
            }
            TargetedAuthResult::Denied(reason) => {
                warn!(
                    "Targeted {} from {:?} to entity {:?} denied: {}",
                    T::type_name(), source, entity, reason
                );
                rejections.push((source, reason));
            }
        }
    }

    // Write authorized messages
    if !authorized_messages.is_empty() {
        let mut messages = world.resource_mut::<Messages<AuthorizedMessage<T>>>();
        for msg in authorized_messages {
            messages.write(msg);
        }
    }

    // Send rejection notifications to clients
    if !rejections.is_empty() {
        if let Some(net) = world.get_resource::<Network<NP>>() {
            for (client_id, reason) in rejections {
                let notification = ServerNotification::warning(reason)
                    .with_context(T::type_name().to_string());
                if let Err(e) = net.send(client_id, notification) {
                    warn!("Failed to send rejection notification to {:?}: {:?}", client_id, e);
                }
            }
        }
    }
}

