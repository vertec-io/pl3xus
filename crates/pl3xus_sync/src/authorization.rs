//! Pluggable authorization policies for messages.
//!
//! This module provides authorization traits and middleware for both targeted
//! (entity-directed) and non-targeted network messages.
//!
//! ## Key Concepts
//!
//! - **Targeted messages** are directed at a specific entity via `target_id`
//! - **Authorization** is a separate concern - checking if a client has permission
//! - These are orthogonal: a targeted message may or may not require authorization
//!
//! ## Authorization Types
//!
//! - [`EntityAccessPolicy`]: For targeted messages - checks if client can access entity
//! - [`MessageAccessPolicy`]: For non-targeted messages - checks if client can send message type
//!
//! ## Builder Pattern
//!
//! The recommended way to register messages is via the [`MessageRegistration`] builder:
//!
//! ```rust,ignore
//! use pl3xus_sync::MessageRegistration;
//!
//! // Simple non-targeted message (anyone can send)
//! app.register_message::<Ping, NP>();
//!
//! // Targeted message without authorization (anyone can query any entity)
//! app.message::<GetEntityInfo, NP>()
//!    .targeted()
//!    .register();
//!
//! // Targeted message WITH authorization (control-based access)
//! app.message::<SendPacket, NP>()
//!    .targeted()
//!    .with_entity_policy(ControlPolicy::new())
//!    .register();
//!
//! // Non-targeted message WITH authorization (role-based)
//! app.message::<AdminCommand, NP>()
//!    .with_message_policy(RolePolicy::admin_only())
//!    .register();
//! ```
//!
//! The [`ExclusiveControlPlugin`](crate::control::ExclusiveControlPlugin) provides
//! a default [`EntityAccessPolicy`] based on [`EntityControl`](crate::control::EntityControl).

use bevy::prelude::*;
use pl3xus_common::ConnectionId;
use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;

// ============================================================================
// AUTHORIZATION RESULT
// ============================================================================

/// Result of an authorization check.
#[derive(Debug, Clone)]
pub enum AuthResult {
    /// The message is authorized.
    Authorized,
    /// The message is not authorized, with a reason.
    Denied(String),
}

impl AuthResult {
    pub fn is_authorized(&self) -> bool {
        matches!(self, AuthResult::Authorized)
    }
}

// ============================================================================
// ENTITY ACCESS (for targeted messages)
// ============================================================================

/// Context for entity access authorization (targeted messages).
///
/// This context is provided when checking if a client can send a message
/// to a specific entity.
pub struct EntityAccessContext<'a> {
    /// Read-only access to the ECS world for querying state.
    pub world: &'a World,
    /// The client that sent the targeted message.
    pub source: ConnectionId,
    /// The entity that the message is targeting.
    pub target_entity: Entity,
}

/// Trait for authorizing access to specific entities.
///
/// Implement this to create custom authorization logic for targeted messages.
/// The authorization check receives the source client and target entity.
///
/// # Example
///
/// ```rust,ignore
/// use pl3xus_sync::authorization::{EntityAccessAuthorizer, EntityAccessContext, AuthResult};
///
/// struct OwnershipAuthorizer;
///
/// impl EntityAccessAuthorizer for OwnershipAuthorizer {
///     fn authorize(&self, ctx: &EntityAccessContext) -> AuthResult {
///         // Check if client owns the entity
///         if let Some(owner) = ctx.world.get::<Owner>(ctx.target_entity) {
///             if owner.client_id == ctx.source {
///                 return AuthResult::Authorized;
///             }
///         }
///         AuthResult::Denied("Not the owner".to_string())
///     }
/// }
/// ```
pub trait EntityAccessAuthorizer: Send + Sync + 'static {
    /// Check if the client can access the target entity.
    fn authorize(&self, ctx: &EntityAccessContext) -> AuthResult;
}

/// Wrapper resource for an entity access policy.
///
/// This can be:
/// - A global default policy (used when no per-message policy is registered)
/// - A per-message-type policy (stored in [`EntityAccessPolicies`])
#[derive(Clone)]
pub struct EntityAccessPolicy {
    inner: Arc<dyn EntityAccessAuthorizer>,
}

impl EntityAccessPolicy {
    /// Create a policy from an authorizer implementation.
    pub fn new<A: EntityAccessAuthorizer>(authorizer: A) -> Self {
        Self {
            inner: Arc::new(authorizer),
        }
    }

    /// Create a policy from a closure.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let policy = EntityAccessPolicy::from_fn(|world, source, entity| {
    ///     // Custom logic
    ///     Ok(())
    /// });
    /// ```
    pub fn from_fn<F>(f: F) -> Self
    where
        F: Fn(&World, ConnectionId, Entity) -> Result<(), String> + Send + Sync + 'static,
    {
        struct ClosureAuthorizer<F>(F);

        impl<F> EntityAccessAuthorizer for ClosureAuthorizer<F>
        where
            F: Fn(&World, ConnectionId, Entity) -> Result<(), String> + Send + Sync + 'static,
        {
            fn authorize(&self, ctx: &EntityAccessContext) -> AuthResult {
                match (self.0)(ctx.world, ctx.source, ctx.target_entity) {
                    Ok(()) => AuthResult::Authorized,
                    Err(reason) => AuthResult::Denied(reason),
                }
            }
        }

        Self {
            inner: Arc::new(ClosureAuthorizer(f)),
        }
    }

    /// Allow all access (no authorization check).
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

    /// Check if access is authorized.
    pub fn check(&self, world: &World, source: ConnectionId, target_entity: Entity) -> AuthResult {
        let ctx = EntityAccessContext {
            world,
            source,
            target_entity,
        };
        self.inner.authorize(&ctx)
    }
}

/// Resource storing per-message-type entity access policies.
///
/// When a targeted message is received, the middleware first checks for a
/// policy specific to that message type. If none is found, it falls back
/// to the [`DefaultEntityAccessPolicy`] resource (if present).
#[derive(Resource, Default)]
pub struct EntityAccessPolicies {
    policies: HashMap<TypeId, EntityAccessPolicy>,
}

impl EntityAccessPolicies {
    /// Register a policy for a specific message type.
    pub fn insert<T: 'static>(&mut self, policy: EntityAccessPolicy) {
        self.policies.insert(TypeId::of::<T>(), policy);
    }

    /// Get the policy for a specific message type.
    pub fn get<T: 'static>(&self) -> Option<&EntityAccessPolicy> {
        self.policies.get(&TypeId::of::<T>())
    }
}

/// Default entity access policy used when no per-message policy is registered.
///
/// If neither a per-message policy nor this default is present, targeted
/// messages are allowed without authorization.
#[derive(Resource, Clone)]
pub struct DefaultEntityAccessPolicy(pub EntityAccessPolicy);

// ============================================================================
// MESSAGE ACCESS (for non-targeted messages)
// ============================================================================

/// Context for message access authorization (non-targeted messages).
///
/// This context is provided when checking if a client can send a particular
/// message type, regardless of any target entity.
pub struct MessageAccessContext<'a> {
    /// Read-only access to the ECS world for querying state.
    pub world: &'a World,
    /// The client that sent the message.
    pub source: ConnectionId,
}

/// Trait for authorizing message types (non-targeted).
///
/// Use this for role-based access control, rate limiting, or other
/// message-level authorization that doesn't depend on a target entity.
///
/// # Example
///
/// ```rust,ignore
/// use pl3xus_sync::authorization::{MessageAccessAuthorizer, MessageAccessContext, AuthResult};
///
/// struct AdminOnlyAuthorizer;
///
/// impl MessageAccessAuthorizer for AdminOnlyAuthorizer {
///     fn authorize(&self, ctx: &MessageAccessContext) -> AuthResult {
///         if let Some(roles) = ctx.world.get_resource::<ClientRoles>() {
///             if roles.is_admin(ctx.source) {
///                 return AuthResult::Authorized;
///             }
///         }
///         AuthResult::Denied("Admin role required".to_string())
///     }
/// }
/// ```
pub trait MessageAccessAuthorizer: Send + Sync + 'static {
    /// Check if the client can send this message type.
    fn authorize(&self, ctx: &MessageAccessContext) -> AuthResult;
}

/// Wrapper for a message access policy.
#[derive(Clone)]
pub struct MessageAccessPolicy {
    inner: Arc<dyn MessageAccessAuthorizer>,
}

impl MessageAccessPolicy {
    /// Create a policy from an authorizer implementation.
    pub fn new<A: MessageAccessAuthorizer>(authorizer: A) -> Self {
        Self {
            inner: Arc::new(authorizer),
        }
    }

    /// Create a policy from a closure.
    pub fn from_fn<F>(f: F) -> Self
    where
        F: Fn(&World, ConnectionId) -> Result<(), String> + Send + Sync + 'static,
    {
        struct ClosureAuthorizer<F>(F);

        impl<F> MessageAccessAuthorizer for ClosureAuthorizer<F>
        where
            F: Fn(&World, ConnectionId) -> Result<(), String> + Send + Sync + 'static,
        {
            fn authorize(&self, ctx: &MessageAccessContext) -> AuthResult {
                match (self.0)(ctx.world, ctx.source) {
                    Ok(()) => AuthResult::Authorized,
                    Err(reason) => AuthResult::Denied(reason),
                }
            }
        }

        Self {
            inner: Arc::new(ClosureAuthorizer(f)),
        }
    }

    /// Allow all messages (no authorization check).
    pub fn allow_all() -> Self {
        Self::from_fn(|_, _| Ok(()))
    }

    /// Only the server can send this message type.
    pub fn server_only() -> Self {
        Self::from_fn(|_, source| {
            if source.is_server() {
                Ok(())
            } else {
                Err("Only server can send this message".to_string())
            }
        })
    }

    /// Check if the message is authorized.
    pub fn check(&self, world: &World, source: ConnectionId) -> AuthResult {
        let ctx = MessageAccessContext { world, source };
        self.inner.authorize(&ctx)
    }
}

/// Resource storing per-message-type access policies for non-targeted messages.
#[derive(Resource, Default)]
pub struct MessageAccessPolicies {
    policies: HashMap<TypeId, MessageAccessPolicy>,
}

impl MessageAccessPolicies {
    /// Register a policy for a specific message type.
    pub fn insert<T: 'static>(&mut self, policy: MessageAccessPolicy) {
        self.policies.insert(TypeId::of::<T>(), policy);
    }

    /// Get the policy for a specific message type.
    pub fn get<T: 'static>(&self) -> Option<&MessageAccessPolicy> {
        self.policies.get(&TypeId::of::<T>())
    }
}

/// Default message access policy used when no per-message policy is registered.
///
/// If neither a per-message policy nor this default is present, non-targeted
/// messages are allowed without authorization.
///
/// # Example
///
/// ```rust,ignore
/// // Install a default policy that requires authentication
/// app.insert_resource(DefaultMessageAccessPolicy(
///     MessageAccessPolicy::from_fn(|world, source| {
///         if let Some(auth) = world.get_resource::<AuthenticatedClients>() {
///             if auth.is_authenticated(source) {
///                 return Ok(());
///             }
///         }
///         Err("Authentication required".to_string())
///     })
/// ));
///
/// // Messages using with_default_message_policy() will use this policy
/// app.message::<SomeCommand, NP>()
///    .with_default_message_policy()
///    .register();
/// ```
#[derive(Resource, Clone)]
pub struct DefaultMessageAccessPolicy(pub MessageAccessPolicy);

// ============================================================================
// AUTHORIZED MESSAGE TYPES (output of authorization middleware)
// ============================================================================

/// A targeted message that has passed authorization.
///
/// Systems should read this message type instead of `NetworkData<TargetedMessage<T>>`
/// when they want only authorized messages.
#[derive(Debug, Clone, bevy::prelude::Message)]
pub struct AuthorizedTargetedMessage<T: pl3xus_common::Pl3xusMessage> {
    /// The original message payload.
    pub message: T,
    /// The client that sent the message.
    pub source: ConnectionId,
    /// The target entity (verified to exist).
    pub target_entity: Entity,
}

/// A non-targeted message that has passed authorization.
///
/// Systems should read this message type instead of `NetworkData<T>`
/// when they want only authorized messages.
#[derive(Debug, Clone, bevy::prelude::Message)]
pub struct AuthorizedMessage<T: pl3xus_common::Pl3xusMessage> {
    /// The original message payload.
    pub message: T,
    /// The client that sent the message.
    pub source: ConnectionId,
}

use bevy::ecs::message::Messages;
use pl3xus::{Network, NetworkData};
use crate::NetworkProvider;
use pl3xus_common::{Pl3xusMessage, ServerNotification, TargetedMessage};

// ============================================================================
// BUILDER PATTERN FOR MESSAGE REGISTRATION
// ============================================================================

/// Builder for registering messages with optional targeting and authorization.
///
/// This provides an ergonomic API for configuring how messages are received
/// and validated.
///
/// # Examples
///
/// ```rust,ignore
/// use pl3xus_sync::MessageRegistration;
///
/// // Simple non-targeted message (no auth)
/// app.message::<Ping, NP>().register();
///
/// // Targeted message without authorization
/// app.message::<GetEntityInfo, NP>()
///    .targeted()
///    .register();
///
/// // Targeted message WITH entity access policy
/// app.message::<SendPacket, NP>()
///    .targeted()
///    .with_entity_policy(EntityAccessPolicy::from_fn(|world, src, ent| {
///        // custom logic
///        Ok(())
///    }))
///    .register();
///
/// // Non-targeted message WITH message access policy
/// app.message::<AdminCommand, NP>()
///    .with_message_policy(MessageAccessPolicy::server_only())
///    .register();
/// ```
pub struct MessageRegistration<'a, T, NP>
where
    T: Pl3xusMessage + Clone + 'static,
    NP: NetworkProvider,
{
    app: &'a mut App,
    targeted: bool,
    entity_policy: Option<EntityAccessPolicy>,
    use_default_entity_policy: bool,
    message_policy: Option<MessageAccessPolicy>,
    use_default_message_policy: bool,
    _marker: std::marker::PhantomData<(T, NP)>,
}

impl<'a, T, NP> MessageRegistration<'a, T, NP>
where
    T: Pl3xusMessage + Clone + 'static,
    NP: NetworkProvider,
{
    /// Create a new message registration builder.
    pub fn new(app: &'a mut App) -> Self {
        Self {
            app,
            targeted: false,
            entity_policy: None,
            use_default_entity_policy: false,
            message_policy: None,
            use_default_message_policy: false,
            _marker: std::marker::PhantomData,
        }
    }

    /// Mark this message as targeted (directed at a specific entity).
    ///
    /// When targeted, the message will be received as `TargetedMessage<T>`
    /// with a `target_id` field identifying the entity.
    pub fn targeted(mut self) -> Self {
        self.targeted = true;
        self
    }

    /// Add an entity access policy for targeted messages.
    ///
    /// This policy checks if the client can access the target entity.
    /// Only applicable when `.targeted()` is also called.
    ///
    /// If this is set, authorized messages are emitted as `AuthorizedTargetedMessage<T>`.
    pub fn with_entity_policy(mut self, policy: EntityAccessPolicy) -> Self {
        self.entity_policy = Some(policy);
        self
    }

    /// Use the default entity access policy (from `DefaultEntityAccessPolicy` resource).
    ///
    /// This is useful when you want to use the policy installed by `ExclusiveControlPlugin`
    /// without specifying a custom policy for this message type.
    ///
    /// Only applicable when `.targeted()` is also called.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Uses the default policy from ExclusiveControlPlugin
    /// app.message::<JogCommand, NP>()
    ///    .targeted()
    ///    .with_default_entity_policy()
    ///    .register();
    /// ```
    pub fn with_default_entity_policy(mut self) -> Self {
        self.use_default_entity_policy = true;
        self
    }

    /// Add a message access policy for non-targeted messages.
    ///
    /// This policy checks if the client can send this message type at all.
    /// Only applicable when the message is NOT targeted.
    ///
    /// If this is set, authorized messages are emitted as `AuthorizedMessage<T>`.
    pub fn with_message_policy(mut self, policy: MessageAccessPolicy) -> Self {
        self.message_policy = Some(policy);
        self
    }

    /// Use the default message access policy (from `DefaultMessageAccessPolicy` resource).
    ///
    /// This is useful for non-targeted messages that should use a globally configured policy.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Uses the default message policy
    /// app.message::<AdminCommand, NP>()
    ///    .with_default_message_policy()
    ///    .register();
    /// ```
    pub fn with_default_message_policy(mut self) -> Self {
        self.use_default_message_policy = true;
        self
    }

    /// Complete the registration and add systems to the app.
    pub fn register(self) -> &'a mut App {
        use pl3xus::AppNetworkMessage;

        if self.targeted {
            // Register as targeted message
            self.app.register_targeted_message::<T, NP>();

            // Check if we need authorization middleware
            let needs_auth = self.entity_policy.is_some() || self.use_default_entity_policy;

            if let Some(policy) = self.entity_policy {
                // Store per-message policy
                if !self.app.world().contains_resource::<EntityAccessPolicies>() {
                    self.app.init_resource::<EntityAccessPolicies>();
                }
                self.app
                    .world_mut()
                    .resource_mut::<EntityAccessPolicies>()
                    .insert::<T>(policy);
            }

            if needs_auth {
                // Add authorization middleware
                // If no per-message policy, the middleware will use DefaultEntityAccessPolicy
                self.app.add_message::<AuthorizedTargetedMessage<T>>();
                self.app
                    .add_systems(PreUpdate, authorize_targeted_messages::<T, NP>);
            }
            // If no policy and not using default, just register the targeted message - no middleware
        } else {
            // Register as plain message
            self.app.register_network_message::<T, NP>();

            // Check if we need message authorization middleware
            let needs_auth = self.message_policy.is_some() || self.use_default_message_policy;

            if let Some(policy) = self.message_policy {
                // Store per-message policy
                if !self.app.world().contains_resource::<MessageAccessPolicies>() {
                    self.app.init_resource::<MessageAccessPolicies>();
                }
                self.app
                    .world_mut()
                    .resource_mut::<MessageAccessPolicies>()
                    .insert::<T>(policy);
            }

            if needs_auth {
                // Add authorization middleware
                self.app.add_message::<AuthorizedMessage<T>>();
                self.app.add_systems(PreUpdate, authorize_messages::<T, NP>);
            }
            // If no policy, just register the message - no middleware
        }

        self.app
    }
}

/// Extension trait for the builder pattern.
pub trait AppMessageRegistrationExt {
    /// Start building a message registration.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // Simple message
    /// app.message::<Ping, NP>().register();
    ///
    /// // Targeted with authorization
    /// app.message::<SendPacket, NP>()
    ///    .targeted()
    ///    .with_entity_policy(policy)
    ///    .register();
    /// ```
    fn message<T, NP>(&mut self) -> MessageRegistration<'_, T, NP>
    where
        T: Pl3xusMessage + Clone + 'static,
        NP: NetworkProvider;
}

impl AppMessageRegistrationExt for App {
    fn message<T, NP>(&mut self) -> MessageRegistration<'_, T, NP>
    where
        T: Pl3xusMessage + Clone + 'static,
        NP: NetworkProvider,
    {
        MessageRegistration::new(self)
    }
}

// ============================================================================
// BATCH MESSAGE REGISTRATION
// ============================================================================

/// Configuration for batch message registration.
///
/// This struct holds the shared configuration that will be applied to all
/// messages in the batch.
#[derive(Clone)]
pub struct BatchMessageConfig {
    /// Whether messages are targeted at specific entities.
    pub targeted: bool,
    /// Whether to use the default entity access policy.
    pub use_default_entity_policy: bool,
    /// Custom entity access policy (overrides default).
    pub entity_policy: Option<EntityAccessPolicy>,
    /// Custom message access policy (for non-targeted messages).
    pub message_policy: Option<MessageAccessPolicy>,
    /// Whether to use the default message access policy.
    pub use_default_message_policy: bool,
}

impl Default for BatchMessageConfig {
    fn default() -> Self {
        Self {
            targeted: false,
            use_default_entity_policy: false,
            entity_policy: None,
            message_policy: None,
            use_default_message_policy: false,
        }
    }
}

/// Builder for batch message registration.
///
/// # Examples
///
/// ```rust,ignore
/// // Register multiple messages with the same configuration
/// app.messages::<(JogCommand, JogConfig, JogMode), NP>()
///    .targeted()
///    .with_default_entity_policy()
///    .register();
///
/// // Register messages as plain network messages
/// app.messages::<(Ping, Pong, StatusRequest), NP>()
///    .register();
/// ```
pub struct BatchMessageRegistration<'a, M, NP>
where
    NP: NetworkProvider,
{
    app: &'a mut App,
    config: BatchMessageConfig,
    _marker: std::marker::PhantomData<(M, NP)>,
}

impl<'a, M, NP> BatchMessageRegistration<'a, M, NP>
where
    NP: NetworkProvider,
{
    /// Create a new batch message registration builder.
    pub fn new(app: &'a mut App) -> Self {
        Self {
            app,
            config: BatchMessageConfig::default(),
            _marker: std::marker::PhantomData,
        }
    }

    /// Mark all messages as targeted (directed at specific entities).
    pub fn targeted(mut self) -> Self {
        self.config.targeted = true;
        self
    }

    /// Use the default entity access policy for all targeted messages.
    pub fn with_default_entity_policy(mut self) -> Self {
        self.config.use_default_entity_policy = true;
        self
    }

    /// Add a custom entity access policy for all targeted messages.
    pub fn with_entity_policy(mut self, policy: EntityAccessPolicy) -> Self {
        self.config.entity_policy = Some(policy);
        self
    }

    /// Add a message access policy for all non-targeted messages.
    pub fn with_message_policy(mut self, policy: MessageAccessPolicy) -> Self {
        self.config.message_policy = Some(policy);
        self
    }

    /// Use the default message access policy for all non-targeted messages.
    pub fn with_default_message_policy(mut self) -> Self {
        self.config.use_default_message_policy = true;
        self
    }
}

/// Trait for types that can be batch-registered as messages.
///
/// This is implemented for tuples of message types.
pub trait BatchRegisterMessages<NP: NetworkProvider> {
    /// Register all message types with the given configuration.
    fn register_batch(app: &mut App, config: &BatchMessageConfig);
}

/// Helper function to register a single message with the given configuration.
fn register_single_message<T, NP>(app: &mut App, config: &BatchMessageConfig)
where
    T: Pl3xusMessage + Clone + 'static,
    NP: NetworkProvider,
{
    use pl3xus::AppNetworkMessage;

    if config.targeted {
        // Register as targeted message
        app.register_targeted_message::<T, NP>();

        let needs_auth = config.entity_policy.is_some() || config.use_default_entity_policy;

        if let Some(policy) = config.entity_policy.clone() {
            // Store per-message policy
            if !app.world().contains_resource::<EntityAccessPolicies>() {
                app.init_resource::<EntityAccessPolicies>();
            }
            app.world_mut()
                .resource_mut::<EntityAccessPolicies>()
                .insert::<T>(policy);
        }

        if needs_auth {
            app.add_message::<AuthorizedTargetedMessage<T>>();
            app.add_systems(PreUpdate, authorize_targeted_messages::<T, NP>);
        }
    } else {
        // Register as plain message
        app.register_network_message::<T, NP>();

        let needs_auth = config.message_policy.is_some() || config.use_default_message_policy;

        if let Some(policy) = config.message_policy.clone() {
            if !app.world().contains_resource::<MessageAccessPolicies>() {
                app.init_resource::<MessageAccessPolicies>();
            }
            app.world_mut()
                .resource_mut::<MessageAccessPolicies>()
                .insert::<T>(policy);
        }

        if needs_auth {
            app.add_message::<AuthorizedMessage<T>>();
            app.add_systems(PreUpdate, authorize_messages::<T, NP>);
        }
    }
}

// Macro to implement BatchRegisterMessages for tuples of different sizes
macro_rules! impl_batch_register_tuple {
    ($($T:ident),+) => {
        impl<NP, $($T),+> BatchRegisterMessages<NP> for ($($T,)+)
        where
            NP: NetworkProvider,
            $($T: Pl3xusMessage + Clone + 'static,)+
        {
            fn register_batch(app: &mut App, config: &BatchMessageConfig) {
                $(
                    register_single_message::<$T, NP>(app, config);
                )+
            }
        }
    };
}

// Implement for tuples of 1-24 elements
impl_batch_register_tuple!(T1);
impl_batch_register_tuple!(T1, T2);
impl_batch_register_tuple!(T1, T2, T3);
impl_batch_register_tuple!(T1, T2, T3, T4);
impl_batch_register_tuple!(T1, T2, T3, T4, T5);
impl_batch_register_tuple!(T1, T2, T3, T4, T5, T6);
impl_batch_register_tuple!(T1, T2, T3, T4, T5, T6, T7);
impl_batch_register_tuple!(T1, T2, T3, T4, T5, T6, T7, T8);
impl_batch_register_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_batch_register_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_batch_register_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
impl_batch_register_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
impl_batch_register_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
impl_batch_register_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);
impl_batch_register_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15);
impl_batch_register_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16);
impl_batch_register_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17);
impl_batch_register_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18);
impl_batch_register_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19);
impl_batch_register_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20);
impl_batch_register_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21);
impl_batch_register_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22);
impl_batch_register_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23);
impl_batch_register_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24);

impl<'a, M, NP> BatchMessageRegistration<'a, M, NP>
where
    M: BatchRegisterMessages<NP>,
    NP: NetworkProvider,
{
    /// Complete the registration and add systems to the app.
    pub fn register(self) -> &'a mut App {
        M::register_batch(self.app, &self.config);
        self.app
    }
}

/// Extension trait for batch message registration.
pub trait AppBatchMessageRegistrationExt {
    /// Start building a batch message registration.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // Register multiple messages with the same configuration
    /// app.messages::<(JogCommand, JogConfig, JogMode), NP>()
    ///    .targeted()
    ///    .with_default_entity_policy()
    ///    .register();
    /// ```
    fn messages<M, NP>(&mut self) -> BatchMessageRegistration<'_, M, NP>
    where
        M: BatchRegisterMessages<NP>,
        NP: NetworkProvider;
}

impl AppBatchMessageRegistrationExt for App {
    fn messages<M, NP>(&mut self) -> BatchMessageRegistration<'_, M, NP>
    where
        M: BatchRegisterMessages<NP>,
        NP: NetworkProvider,
    {
        BatchMessageRegistration::new(self)
    }
}

// ============================================================================
// REQUEST REGISTRATION (Request/Response Pattern)
// ============================================================================

use pl3xus::managers::network_request::{Request, AppNetworkRequestMessage};
use pl3xus_common::RequestMessage;
use serde::{Serialize, Deserialize};

/// A targeted request that has passed authorization.
///
/// Systems should read this type instead of `Request<TargetedRequest<T>>` when they want
/// only authorized targeted requests. This wraps the full targeted request and provides
/// convenient access to the inner request data.
#[derive(Debug, Clone, bevy::prelude::Message)]
pub struct AuthorizedRequest<T: RequestMessage> {
    /// The original targeted request (with respond capability).
    inner: Request<TargetedRequest<T>>,
    /// The target entity (verified to exist).
    pub target_entity: Entity,
}

impl<T: RequestMessage> AuthorizedRequest<T> {
    /// Create a new authorized request.
    pub fn new(request: Request<TargetedRequest<T>>, target_entity: Entity) -> Self {
        Self {
            inner: request,
            target_entity,
        }
    }

    /// Read the underlying request payload (the inner T, not the TargetedRequest wrapper).
    #[inline(always)]
    pub fn get_request(&self) -> &T {
        &self.inner.get_request().request
    }

    /// Read the source of the underlying request.
    #[inline(always)]
    pub fn source(&self) -> &ConnectionId {
        self.inner.source()
    }

    /// Get the target entity ID string (as sent by the client).
    #[inline(always)]
    pub fn target_id(&self) -> &str {
        &self.inner.get_request().target_id
    }

    /// Consume the request and automatically send the response back to the client.
    pub fn respond(self, response: T::ResponseMessage) -> Result<(), pl3xus_common::error::NetworkError> {
        self.inner.respond(response)
    }

    /// Take the responder for async response handling.
    ///
    /// This consumes the request and returns a `DeferredResponder` that can be
    /// used to send the response later (e.g., from an async task).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// for event in events.read() {
    ///     let responder = event.take_responder();
    ///     tokio_runtime.spawn_background_task(move |_| async move {
    ///         // Do async work...
    ///         let _ = responder.respond(MyResponse { success: true });
    ///     });
    /// }
    /// ```
    pub fn take_responder(self) -> pl3xus::DeferredResponder<T::ResponseMessage> {
        self.inner.take_responder()
    }
}

/// Wire format for targeted requests.
///
/// This wraps a request with a target entity ID, similar to `TargetedMessage<T>`.
/// The response type is the same as the inner request's response type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "T: RequestMessage")]
pub struct TargetedRequest<T: RequestMessage> {
    /// The target entity ID (as entity bits string).
    pub target_id: String,
    /// The inner request.
    pub request: T,
}

// Implement RequestMessage for TargetedRequest so it can be used with Request<T>
impl<T: RequestMessage> RequestMessage for TargetedRequest<T> {
    type ResponseMessage = T::ResponseMessage;

    fn request_name() -> &'static str {
        // Use the inner request's name with a prefix
        // Note: This creates a static string at compile time
        T::request_name()
    }
}

/// Builder for registering requests with optional targeting and authorization.
///
/// This provides an ergonomic API for configuring how requests are received
/// and validated, parallel to `MessageRegistration`.
///
/// # Examples
///
/// ```rust,ignore
/// use pl3xus_sync::RequestRegistration;
///
/// // Simple non-targeted request (no auth)
/// app.request::<GetStatus, NP>().register();
///
/// // Targeted request with default entity policy
/// app.request::<SetSpeedOverride, NP>()
///    .targeted()
///    .with_default_entity_policy()
///    .register();
///
/// // Targeted request with custom policy
/// app.request::<AdminCommand, NP>()
///    .targeted()
///    .with_entity_policy(EntityAccessPolicy::from_fn(custom_check))
///    .register();
/// ```
pub struct RequestRegistration<'a, T, NP>
where
    T: RequestMessage + Clone + 'static,
    NP: NetworkProvider,
{
    app: &'a mut App,
    targeted: bool,
    entity_policy: Option<EntityAccessPolicy>,
    use_default_entity_policy: bool,
    message_policy: Option<MessageAccessPolicy>,
    use_default_message_policy: bool,
    _marker: std::marker::PhantomData<(T, NP)>,
}

impl<'a, T, NP> RequestRegistration<'a, T, NP>
where
    T: RequestMessage + Clone + 'static,
    NP: NetworkProvider,
{
    /// Create a new request registration builder.
    pub fn new(app: &'a mut App) -> Self {
        Self {
            app,
            targeted: false,
            entity_policy: None,
            use_default_entity_policy: false,
            message_policy: None,
            use_default_message_policy: false,
            _marker: std::marker::PhantomData,
        }
    }

    /// Mark this request as targeted (directed at a specific entity).
    ///
    /// When targeted, the request will be wrapped in `TargetedRequest<T>`
    /// with a `target_id` field identifying the entity.
    pub fn targeted(mut self) -> Self {
        self.targeted = true;
        self
    }

    /// Add an entity access policy for targeted requests.
    ///
    /// This policy checks if the client can access the target entity.
    /// Only applicable when `.targeted()` is also called.
    ///
    /// If this is set, authorized requests are emitted as `AuthorizedRequest<T>`.
    pub fn with_entity_policy(mut self, policy: EntityAccessPolicy) -> Self {
        self.entity_policy = Some(policy);
        self
    }

    /// Use the default entity access policy (from `DefaultEntityAccessPolicy` resource).
    ///
    /// This is useful when you want to use the policy installed by `ExclusiveControlPlugin`
    /// without specifying a custom policy for this request type.
    ///
    /// Only applicable when `.targeted()` is also called.
    pub fn with_default_entity_policy(mut self) -> Self {
        self.use_default_entity_policy = true;
        self
    }

    /// Add a message access policy for non-targeted requests.
    ///
    /// This policy checks if the client can send this request type at all.
    /// Only applicable when the request is NOT targeted.
    pub fn with_message_policy(mut self, policy: MessageAccessPolicy) -> Self {
        self.message_policy = Some(policy);
        self
    }

    /// Use the default message access policy (from `DefaultMessageAccessPolicy` resource).
    pub fn with_default_message_policy(mut self) -> Self {
        self.use_default_message_policy = true;
        self
    }

    /// Complete the registration and add systems to the app.
    ///
    /// Note: For targeted requests with authorization, if authorization fails,
    /// the request is dropped silently and the client will timeout.
    /// Use [`with_error_response`] if `T` implements [`ErrorResponse`]
    /// to send proper error responses.
    pub fn register(self) -> &'a mut App {
        if self.targeted {
            // Register as targeted request
            self.app.listen_for_request_message::<TargetedRequest<T>, NP>();

            // Check if we need authorization middleware
            let needs_auth = self.entity_policy.is_some() || self.use_default_entity_policy;

            if let Some(policy) = self.entity_policy {
                // Store per-request policy (using the same storage as messages)
                if !self.app.world().contains_resource::<EntityAccessPolicies>() {
                    self.app.init_resource::<EntityAccessPolicies>();
                }
                self.app
                    .world_mut()
                    .resource_mut::<EntityAccessPolicies>()
                    .insert::<TargetedRequest<T>>(policy);
            }

            if needs_auth {
                // Add authorization middleware
                self.app.add_message::<AuthorizedRequest<T>>();
                self.app
                    .add_systems(PreUpdate, authorize_targeted_requests::<T, NP>);
            }
        } else {
            // Register as plain request
            self.app.listen_for_request_message::<T, NP>();

            // Note: Non-targeted request authorization could be added here if needed
            // For now, non-targeted requests don't have authorization middleware
        }

        self.app
    }
}

impl<'a, T, NP> RequestRegistration<'a, T, NP>
where
    T: RequestMessage + pl3xus_common::ErrorResponse + Clone + 'static,
    NP: NetworkProvider,
{
    /// Complete the registration with error response support.
    ///
    /// Unlike [`register`], this method requires `T: ErrorResponse` and will
    /// send proper error responses when:
    /// - The target entity ID is invalid
    /// - The target entity does not exist
    /// - Authorization is denied
    ///
    /// This is the recommended method for targeted requests with authorization.
    pub fn with_error_response(self) -> &'a mut App {
        if self.targeted {
            // Register as targeted request
            self.app.listen_for_request_message::<TargetedRequest<T>, NP>();

            // Check if we need authorization middleware
            let needs_auth = self.entity_policy.is_some() || self.use_default_entity_policy;

            if let Some(policy) = self.entity_policy {
                // Store per-request policy
                if !self.app.world().contains_resource::<EntityAccessPolicies>() {
                    self.app.init_resource::<EntityAccessPolicies>();
                }
                self.app
                    .world_mut()
                    .resource_mut::<EntityAccessPolicies>()
                    .insert::<TargetedRequest<T>>(policy);
            }

            if needs_auth {
                // Add authorization middleware with error response support
                self.app.add_message::<AuthorizedRequest<T>>();
                self.app
                    .add_systems(PreUpdate, authorize_targeted_requests_with_error_response::<T, NP>);
            }
        } else {
            // Register as plain request
            self.app.listen_for_request_message::<T, NP>();
        }

        self.app
    }
}

/// Extension trait for request registration builder pattern.
pub trait AppRequestRegistrationExt {
    /// Start building a request registration.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // Simple request
    /// app.request::<GetStatus, NP>().register();
    ///
    /// // Targeted with authorization
    /// app.request::<SetSpeedOverride, NP>()
    ///    .targeted()
    ///    .with_default_entity_policy()
    ///    .register();
    /// ```
    fn request<T, NP>(&mut self) -> RequestRegistration<'_, T, NP>
    where
        T: RequestMessage + Clone + 'static,
        NP: NetworkProvider;
}

impl AppRequestRegistrationExt for App {
    fn request<T, NP>(&mut self) -> RequestRegistration<'_, T, NP>
    where
        T: RequestMessage + Clone + 'static,
        NP: NetworkProvider,
    {
        RequestRegistration::new(self)
    }
}

// ============================================================================
// BATCH REQUEST REGISTRATION
// ============================================================================

/// Configuration for batch request registration.
#[derive(Default, Clone)]
pub struct BatchRequestConfig {
    /// Whether requests are targeted (directed at specific entities).
    pub targeted: bool,
    /// Whether to use the default entity access policy.
    pub use_default_entity_policy: bool,
    /// Whether to use error response mode (requires ErrorResponse trait).
    pub with_error_response: bool,
}

/// Builder for batch request registration.
///
/// # Examples
///
/// ```rust,ignore
/// // Register multiple requests with the same configuration
/// app.requests::<(SetSpeedOverride, InitializeRobot, ResetRobot), NP>()
///    .targeted()
///    .with_default_entity_policy()
///    .with_error_response();
///
/// // Register requests as plain network requests
/// app.requests::<(GetStatus, ListRobots), NP>()
///    .register();
/// ```
pub struct BatchRequestRegistration<'a, R, NP>
where
    NP: NetworkProvider,
{
    app: &'a mut App,
    config: BatchRequestConfig,
    _marker: std::marker::PhantomData<(R, NP)>,
}

impl<'a, R, NP> BatchRequestRegistration<'a, R, NP>
where
    NP: NetworkProvider,
{
    /// Create a new batch request registration builder.
    pub fn new(app: &'a mut App) -> Self {
        Self {
            app,
            config: BatchRequestConfig::default(),
            _marker: std::marker::PhantomData,
        }
    }

    /// Mark all requests as targeted (directed at specific entities).
    pub fn targeted(mut self) -> Self {
        self.config.targeted = true;
        self
    }

    /// Use the default entity access policy for all targeted requests.
    pub fn with_default_entity_policy(mut self) -> Self {
        self.config.use_default_entity_policy = true;
        self
    }
}

/// Trait for types that can be batch-registered as requests (without ErrorResponse).
///
/// This is implemented for tuples of request types.
pub trait BatchRegisterRequests<NP: NetworkProvider> {
    /// Register all request types with the given configuration.
    fn register_batch(app: &mut App, config: &BatchRequestConfig);
}

/// Trait for types that can be batch-registered as requests with error response support.
///
/// This is implemented for tuples of request types that implement ErrorResponse.
pub trait BatchRegisterRequestsWithErrorResponse<NP: NetworkProvider> {
    /// Register all request types with error response support.
    fn register_batch_with_error_response(app: &mut App, config: &BatchRequestConfig);
}

/// Helper function to register a single request with the given configuration.
fn register_single_request<T, NP>(app: &mut App, config: &BatchRequestConfig)
where
    T: RequestMessage + Clone + 'static,
    NP: NetworkProvider,
{
    let mut reg = RequestRegistration::<T, NP>::new(app);

    if config.targeted {
        reg = reg.targeted();
    }

    if config.use_default_entity_policy {
        reg = reg.with_default_entity_policy();
    }

    reg.register();
}

/// Helper function to register a single request with error response support.
fn register_single_request_with_error_response<T, NP>(app: &mut App, config: &BatchRequestConfig)
where
    T: RequestMessage + pl3xus_common::ErrorResponse + Clone + 'static,
    NP: NetworkProvider,
{
    let mut reg = RequestRegistration::<T, NP>::new(app);

    if config.targeted {
        reg = reg.targeted();
    }

    if config.use_default_entity_policy {
        reg = reg.with_default_entity_policy();
    }

    reg.with_error_response();
}

// Macro to implement BatchRegisterRequests for tuples of different sizes
macro_rules! impl_batch_register_requests_tuple {
    ($($T:ident),+) => {
        impl<NP, $($T),+> BatchRegisterRequests<NP> for ($($T,)+)
        where
            NP: NetworkProvider,
            $($T: RequestMessage + Clone + 'static,)+
        {
            fn register_batch(app: &mut App, config: &BatchRequestConfig) {
                $(
                    register_single_request::<$T, NP>(app, config);
                )+
            }
        }

        impl<NP, $($T),+> BatchRegisterRequestsWithErrorResponse<NP> for ($($T,)+)
        where
            NP: NetworkProvider,
            $($T: RequestMessage + pl3xus_common::ErrorResponse + Clone + 'static,)+
        {
            fn register_batch_with_error_response(app: &mut App, config: &BatchRequestConfig) {
                $(
                    register_single_request_with_error_response::<$T, NP>(app, config);
                )+
            }
        }
    };
}

// Implement for tuples of 1-24 elements
impl_batch_register_requests_tuple!(T1);
impl_batch_register_requests_tuple!(T1, T2);
impl_batch_register_requests_tuple!(T1, T2, T3);
impl_batch_register_requests_tuple!(T1, T2, T3, T4);
impl_batch_register_requests_tuple!(T1, T2, T3, T4, T5);
impl_batch_register_requests_tuple!(T1, T2, T3, T4, T5, T6);
impl_batch_register_requests_tuple!(T1, T2, T3, T4, T5, T6, T7);
impl_batch_register_requests_tuple!(T1, T2, T3, T4, T5, T6, T7, T8);
impl_batch_register_requests_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_batch_register_requests_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_batch_register_requests_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
impl_batch_register_requests_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
impl_batch_register_requests_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
impl_batch_register_requests_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);
impl_batch_register_requests_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15);
impl_batch_register_requests_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16);
impl_batch_register_requests_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17);
impl_batch_register_requests_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18);
impl_batch_register_requests_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19);
impl_batch_register_requests_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20);
impl_batch_register_requests_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21);
impl_batch_register_requests_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22);
impl_batch_register_requests_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23);
impl_batch_register_requests_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24);

impl<'a, R, NP> BatchRequestRegistration<'a, R, NP>
where
    R: BatchRegisterRequests<NP>,
    NP: NetworkProvider,
{
    /// Complete the registration and add systems to the app.
    pub fn register(self) -> &'a mut App {
        R::register_batch(self.app, &self.config);
        self.app
    }
}

impl<'a, R, NP> BatchRequestRegistration<'a, R, NP>
where
    R: BatchRegisterRequestsWithErrorResponse<NP>,
    NP: NetworkProvider,
{
    /// Complete the registration with error response support.
    ///
    /// All request types in the tuple must implement `ErrorResponse`.
    pub fn with_error_response(self) -> &'a mut App {
        R::register_batch_with_error_response(self.app, &self.config);
        self.app
    }
}

/// Extension trait for batch request registration.
pub trait AppBatchRequestRegistrationExt {
    /// Start building a batch request registration.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // Register multiple requests with the same configuration
    /// app.requests::<(SetSpeedOverride, InitializeRobot, ResetRobot), NP>()
    ///    .targeted()
    ///    .with_default_entity_policy()
    ///    .with_error_response();
    /// ```
    fn requests<R, NP>(&mut self) -> BatchRequestRegistration<'_, R, NP>
    where
        NP: NetworkProvider;
}

impl AppBatchRequestRegistrationExt for App {
    fn requests<R, NP>(&mut self) -> BatchRequestRegistration<'_, R, NP>
    where
        NP: NetworkProvider,
    {
        BatchRequestRegistration::new(self)
    }
}

// ============================================================================
// AUTHORIZATION MIDDLEWARE SYSTEMS
// ============================================================================

/// Authorization middleware for targeted messages.
///
/// This exclusive system:
/// 1. Reads all incoming `NetworkData<TargetedMessage<T>>`
/// 2. Parses the target_id as entity bits (u64)
/// 3. Checks authorization (per-message policy, then default policy, then allow)
/// 4. Emits `AuthorizedTargetedMessage<T>` for authorized messages
/// 5. Sends rejection notifications to unauthorized clients
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

    // Get per-message policy first, then fall back to default
    let policy: Option<EntityAccessPolicy> = world
        .get_resource::<EntityAccessPolicies>()
        .and_then(|p| p.get::<T>().cloned())
        .or_else(|| {
            world
                .get_resource::<DefaultEntityAccessPolicy>()
                .map(|p| p.0.clone())
        });

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
        let auth_result = match &policy {
            Some(p) => p.check(world, source, entity),
            None => AuthResult::Authorized, // No policy = allow all
        };

        match auth_result {
            AuthResult::Authorized => {
                authorized_messages.push(AuthorizedTargetedMessage {
                    message: msg.message.clone(),
                    source,
                    target_entity: entity,
                });
            }
            AuthResult::Denied(reason) => {
                warn!(
                    "Targeted {} from {:?} to entity {:?} denied: {}",
                    T::type_name(),
                    source,
                    entity,
                    reason
                );
                rejections.push((source, reason));
            }
        }
    }

    // Write authorized messages
    if !authorized_messages.is_empty() {
        let mut messages = world.resource_mut::<Messages<AuthorizedTargetedMessage<T>>>();
        for msg in authorized_messages {
            messages.write(msg);
        }
    }

    // Send rejection notifications to clients
    if !rejections.is_empty() {
        if let Some(net) = world.get_resource::<Network<NP>>() {
            for (client_id, reason) in rejections {
                let notification =
                    ServerNotification::warning(reason).with_context(T::type_name().to_string());
                if let Err(e) = net.send(client_id, notification) {
                    warn!(
                        "Failed to send rejection notification to {:?}: {:?}",
                        client_id, e
                    );
                }
            }
        }
    }
}

/// Authorization middleware for non-targeted messages.
///
/// This exclusive system:
/// 1. Reads all incoming `NetworkData<T>`
/// 2. Checks authorization via per-message policy or default policy
/// 3. Emits `AuthorizedMessage<T>` for authorized messages
/// 4. Sends rejection notifications to unauthorized clients
fn authorize_messages<T, NP>(world: &mut World)
where
    T: Pl3xusMessage + Clone + 'static,
    NP: NetworkProvider,
{
    // Collect incoming messages first
    let incoming: Vec<_> = {
        let mut messages = world.resource_mut::<Messages<NetworkData<T>>>();
        messages.drain().collect()
    };

    if incoming.is_empty() {
        return;
    }

    // Get per-message policy, falling back to default policy
    let policy: Option<MessageAccessPolicy> = world
        .get_resource::<MessageAccessPolicies>()
        .and_then(|p| p.get::<T>().cloned())
        .or_else(|| {
            world
                .get_resource::<DefaultMessageAccessPolicy>()
                .map(|d| d.0.clone())
        });

    // Process each message
    let mut authorized_messages = Vec::new();
    let mut rejections: Vec<(ConnectionId, String)> = Vec::new();

    for msg in incoming {
        let source = *msg.source();

        // Check authorization
        let auth_result = match &policy {
            Some(p) => p.check(world, source),
            None => AuthResult::Authorized, // No policy = allow all
        };

        match auth_result {
            AuthResult::Authorized => {
                authorized_messages.push(AuthorizedMessage {
                    message: (*msg).clone(),
                    source,
                });
            }
            AuthResult::Denied(reason) => {
                warn!(
                    "Message {} from {:?} denied: {}",
                    T::type_name(),
                    source,
                    reason
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

    // Send rejection notifications
    if !rejections.is_empty() {
        if let Some(net) = world.get_resource::<Network<NP>>() {
            for (client_id, reason) in rejections {
                let notification =
                    ServerNotification::warning(reason).with_context(T::type_name().to_string());
                if let Err(e) = net.send(client_id, notification) {
                    warn!(
                        "Failed to send rejection notification to {:?}: {:?}",
                        client_id, e
                    );
                }
            }
        }
    }
}

/// Authorization middleware for targeted requests.
///
/// This exclusive system:
/// 1. Reads all incoming `Request<TargetedRequest<T>>`
/// 2. Parses the target_id as entity bits (u64)
/// 3. Checks authorization (per-request policy, then default policy, then allow)
/// 4. Emits `AuthorizedRequest<T>` for authorized requests
/// 5. Sends rejection responses to unauthorized clients
fn authorize_targeted_requests<T, NP>(world: &mut World)
where
    T: RequestMessage + Clone + 'static,
    NP: NetworkProvider,
{
    // Collect incoming requests first
    let incoming: Vec<_> = {
        let mut messages = world.resource_mut::<Messages<Request<TargetedRequest<T>>>>();
        messages.drain().collect()
    };

    if incoming.is_empty() {
        return;
    }

    // Get per-request policy first, then fall back to default
    let policy: Option<EntityAccessPolicy> = world
        .get_resource::<EntityAccessPolicies>()
        .and_then(|p| p.get::<TargetedRequest<T>>().cloned())
        .or_else(|| {
            world
                .get_resource::<DefaultEntityAccessPolicy>()
                .map(|p| p.0.clone())
        });

    // Process each request
    let mut authorized_requests = Vec::new();

    for req in incoming {
        let source = *req.source();
        let target_id_str = &req.get_request().target_id;

        // Parse target_id as entity bits
        let target_entity = match target_id_str.parse::<u64>() {
            Ok(bits) => Entity::from_bits(bits),
            Err(_) => {
                warn!(
                    "Request {} from {:?}: invalid target_id '{}' - request dropped",
                    T::request_name(),
                    source,
                    target_id_str
                );
                // Drop the request - client will timeout
                // TODO: Add ErrorResponse trait to allow sending error responses
                continue;
            }
        };

        // Verify entity exists
        if world.get_entity(target_entity).is_err() {
            warn!(
                "Request {} from {:?}: target entity {:?} does not exist - request dropped",
                T::request_name(),
                source,
                target_entity
            );
            // Drop the request - client will timeout
            continue;
        }

        // Check authorization
        let auth_result = match &policy {
            Some(p) => p.check(world, source, target_entity),
            None => AuthResult::Authorized, // No policy = allow all
        };

        match auth_result {
            AuthResult::Authorized => {
                authorized_requests.push((req, target_entity));
            }
            AuthResult::Denied(reason) => {
                warn!(
                    "Request {} from {:?} to {:?} denied: {} - request dropped",
                    T::request_name(),
                    source,
                    target_entity,
                    reason
                );
                // Drop the request - client will timeout
                // TODO: Add ErrorResponse trait to allow sending error responses
            }
        }
    }

    // Write authorized requests
    if !authorized_requests.is_empty() {
        let mut messages = world.resource_mut::<Messages<AuthorizedRequest<T>>>();
        for (req, target_entity) in authorized_requests {
            messages.write(AuthorizedRequest::new(req, target_entity));
        }
    }
}

/// Authorization middleware for targeted requests that implement ErrorResponse.
///
/// This is similar to `authorize_targeted_requests` but sends error responses
/// when authorization fails instead of silently dropping requests.
fn authorize_targeted_requests_with_error_response<T, NP>(world: &mut World)
where
    T: RequestMessage + pl3xus_common::ErrorResponse + Clone + 'static,
    NP: NetworkProvider,
{
    // Collect incoming requests first
    let incoming: Vec<_> = {
        let mut messages = world.resource_mut::<Messages<Request<TargetedRequest<T>>>>();
        messages.drain().collect()
    };

    if incoming.is_empty() {
        return;
    }

    // Get per-request policy first, then fall back to default
    let policy: Option<EntityAccessPolicy> = world
        .get_resource::<EntityAccessPolicies>()
        .and_then(|p| p.get::<TargetedRequest<T>>().cloned())
        .or_else(|| {
            world
                .get_resource::<DefaultEntityAccessPolicy>()
                .map(|p| p.0.clone())
        });

    // Process each request
    let mut authorized_requests = Vec::new();
    let mut error_responses: Vec<(Request<TargetedRequest<T>>, T::ResponseMessage)> = Vec::new();

    for req in incoming {
        let source = *req.source();
        let target_id_str = req.get_request().target_id.clone();

        // Parse target_id as entity bits
        let target_entity = match target_id_str.parse::<u64>() {
            Ok(bits) => Entity::from_bits(bits),
            Err(_) => {
                warn!(
                    "Request {} from {:?}: invalid target_id '{}'",
                    T::request_name(),
                    source,
                    target_id_str
                );
                error_responses.push((
                    req,
                    T::error_response(format!("Invalid target entity: {}", target_id_str)),
                ));
                continue;
            }
        };

        // Verify entity exists
        if world.get_entity(target_entity).is_err() {
            warn!(
                "Request {} from {:?}: target entity {:?} does not exist",
                T::request_name(),
                source,
                target_entity
            );
            error_responses.push((
                req,
                T::error_response("Target entity does not exist".to_string()),
            ));
            continue;
        }

        // Check authorization
        let auth_result = match &policy {
            Some(p) => p.check(world, source, target_entity),
            None => AuthResult::Authorized, // No policy = allow all
        };

        match auth_result {
            AuthResult::Authorized => {
                authorized_requests.push((req, target_entity));
            }
            AuthResult::Denied(reason) => {
                warn!(
                    "Request {} from {:?} to {:?} denied: {}",
                    T::request_name(),
                    source,
                    target_entity,
                    reason
                );
                error_responses.push((req, T::error_response(reason)));
            }
        }
    }

    // Write authorized requests
    if !authorized_requests.is_empty() {
        let mut messages = world.resource_mut::<Messages<AuthorizedRequest<T>>>();
        for (req, target_entity) in authorized_requests {
            messages.write(AuthorizedRequest::new(req, target_entity));
        }
    }

    // Send error responses
    for (req, response) in error_responses {
        if let Err(e) = req.respond(response) {
            warn!("Failed to send error response: {:?}", e);
        }
    }
}
