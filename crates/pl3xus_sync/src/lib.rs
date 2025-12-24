//! pl3xus_sync
//!
//! Reflection-driven synchronization middleware between a Bevy ECS server and
//! arbitrary clients over pl3xus.
//!
//! This crate is intentionally generic and does not contain any application-
//! specific logic (no robotics/meteorite assumptions). It exposes:
//!
//! - [`Pl3xusSyncPlugin`]: wires core resources and systems.
//! - [`AppPl3xusSyncExt`]: `sync_component::<T>()` for opt-in component sync.
//! - Wire-level message types for subscriptions, updates, mutations, and
//!   database-backed queries.
//! - [`MutationAuthorizer`] / [`MutationAuthorizerResource`]: pluggable
//!   authorization policies for client-driven mutations, plus a built-in
//!   [`ServerOnlyMutationAuthorizer`] for "server-only" mutation deployments.
//!
//! ## Message Authorization
//!
//! The authorization module provides a builder pattern for registering messages
//! with optional authorization:
//!
//! ```rust,ignore
//! use pl3xus_sync::{AppMessageRegistrationExt, EntityAccessPolicy};
//!
//! // Simple non-targeted message
//! app.message::<Ping, NP>().register();
//!
//! // Targeted message with entity access authorization
//! app.message::<SendPacket, NP>()
//!    .targeted()
//!    .with_entity_policy(EntityAccessPolicy::from_fn(|world, src, ent| Ok(())))
//!    .register();
//! ```

mod messages;
#[cfg(feature = "runtime")]
mod registry;
#[cfg(feature = "runtime")]
mod subscription;
#[cfg(feature = "runtime")]
mod systems;
#[cfg(feature = "runtime")]
mod invalidation;

/// Pluggable authorization policies for messages.
#[cfg(feature = "runtime")]
pub mod authorization;

/// Optional utilities for exclusive control transfer patterns.
#[cfg(feature = "runtime")]
pub mod control;

pub use messages::*;
#[cfg(feature = "runtime")]
pub use registry::{
    ComponentSyncConfig,
    SyncSettings,
    ConflationQueue,
    ComponentRegistration,
    SyncRegistry,
    SubscriptionManager,
    SubscriptionEntry,
    MutationQueue,
    SnapshotQueue,
    ComponentChangeEvent,
    EntityDespawnEvent,
    MutationAuthContext,
    MutationAuthorizer,
    MutationAuthorizerResource,
    ServerOnlyMutationAuthorizer,
    has_control_hierarchical,
    // Component mutation handler types
    ComponentMutation,
    AuthorizedComponentMutation,
    ComponentMutationQueue,
    MutationResponseQueue,
    PendingMutationResponse,
};
#[cfg(feature = "runtime")]
pub use subscription::*;

// New authorization API (v0.2+)
#[cfg(feature = "runtime")]
pub use authorization::{
    // Authorization result
    AuthResult,
    // Entity access (for targeted messages)
    EntityAccessContext,
    EntityAccessAuthorizer,
    EntityAccessPolicy,
    EntityAccessPolicies,
    DefaultEntityAccessPolicy,
    // Message access (for non-targeted messages)
    MessageAccessContext,
    MessageAccessAuthorizer,
    MessageAccessPolicy,
    MessageAccessPolicies,
    DefaultMessageAccessPolicy,
    // Authorized message types (output of authorization middleware)
    AuthorizedTargetedMessage,
    AuthorizedMessage,
    // Single message builder pattern
    MessageRegistration,
    AppMessageRegistrationExt,
    // Batch message registration
    BatchMessageConfig,
    BatchMessageRegistration,
    BatchRegisterMessages,
    AppBatchMessageRegistrationExt,
    // Request registration (request/response pattern)
    TargetedRequest,
    AuthorizedRequest,
    RequestRegistration,
    AppRequestRegistrationExt,
    // Batch request registration
    BatchRequestConfig,
    BatchRequestRegistration,
    BatchRegisterRequests,
    BatchRegisterRequestsWithErrorResponse,
    AppBatchRequestRegistrationExt,
};

// Re-export DeferredResponder for async request handling
#[cfg(feature = "runtime")]
pub use pl3xus::DeferredResponder;

// Automatic query invalidation API
#[cfg(feature = "runtime")]
pub use invalidation::{
    // Trait for derive macro
    Invalidates,
    // Legacy builder pattern (deprecated)
    InvalidationRule,
    InvalidationRules,
    InvalidationRulesBuilder,
    InvalidationRuleBuilder,
    AppInvalidationExt,
    broadcast_invalidations,
    // New trait-based broadcast function
    broadcast_invalidations_for,
    // Request extension for auto-invalidation
    RequestInvalidateExt,
};

#[cfg(feature = "runtime")]
use bevy::prelude::*;
#[cfg(feature = "runtime")]
use pl3xus::managers::NetworkProvider;

/// Top-level plugin that adds sync resources, registers network messages, and
/// installs core systems.
#[cfg(feature = "runtime")]
#[derive(Debug, Clone)]
pub struct Pl3xusSyncPlugin<NP: NetworkProvider> {
    _marker: std::marker::PhantomData<NP>,
}

#[cfg(feature = "runtime")]
impl<NP: NetworkProvider> Default for Pl3xusSyncPlugin<NP> {
    fn default() -> Self {
        Self { _marker: std::marker::PhantomData }
    }
}

#[cfg(feature = "runtime")]
impl<NP: NetworkProvider> Plugin for Pl3xusSyncPlugin<NP> {
    fn build(&self, app: &mut App) {
        info!("[Pl3xusSyncPlugin::build] CALLED - about to call systems::install");
        systems::install::<NP>(app);
        info!("[Pl3xusSyncPlugin::build] COMPLETED - systems::install returned");
    }
}

/// Extension trait for registering components for synchronization.
#[cfg(feature = "runtime")]
pub trait AppPl3xusSyncExt {
    /// Register a component type `T` to be synchronized with remote clients.
    ///
    /// This is the only call most applications need to make per component type.
    ///
    /// # Deprecated
    ///
    /// Prefer using `sync_component_builder::<T>()` for new code, which provides
    /// a builder pattern with better ergonomics.
    fn sync_component<T>(&mut self, config: Option<ComponentSyncConfig>) -> &mut Self
    where
        T: Component + serde::Serialize + for<'de> serde::Deserialize<'de> + Send + Sync + 'static + std::fmt::Debug + Clone;

    /// Start building a component synchronization registration.
    ///
    /// Returns a builder that allows configuring the component sync with a fluent API.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Basic registration (read-write, no handler)
    /// app.sync_component_builder::<Position>().build();
    ///
    /// // Read-only component
    /// app.sync_component_builder::<RobotStatus>()
    ///     .read_only()
    ///     .build();
    ///
    /// // Component with mutation handler
    /// app.sync_component_builder::<FrameToolDataState>()
    ///     .with_handler::<WebSocketProvider>(handle_frame_tool_mutation)
    ///     .build();
    /// ```
    fn sync_component_builder<T>(&mut self) -> SyncComponentBuilder<'_, T>
    where
        T: Component + serde::Serialize + for<'de> serde::Deserialize<'de> + Send + Sync + 'static + std::fmt::Debug + Clone;
}

#[cfg(feature = "runtime")]
impl AppPl3xusSyncExt for App {
    fn sync_component<T>(&mut self, config: Option<ComponentSyncConfig>) -> &mut Self
    where
        T: Component + serde::Serialize + for<'de> serde::Deserialize<'de> + Send + Sync + 'static + std::fmt::Debug + Clone,
    {
        registry::register_component::<T>(self, config);
        self
    }

    fn sync_component_builder<T>(&mut self) -> SyncComponentBuilder<'_, T>
    where
        T: Component + serde::Serialize + for<'de> serde::Deserialize<'de> + Send + Sync + 'static + std::fmt::Debug + Clone,
    {
        SyncComponentBuilder::new(self)
    }
}

/// Builder for configuring component synchronization.
///
/// Created by [`AppPl3xusSyncExt::sync_component_builder`].
#[cfg(feature = "runtime")]
pub struct SyncComponentBuilder<'a, T>
where
    T: Component + serde::Serialize + for<'de> serde::Deserialize<'de> + Send + Sync + 'static + std::fmt::Debug + Clone,
{
    app: &'a mut App,
    config: ComponentSyncConfig,
    /// Track if we need to register AuthorizedComponentMutation<T> message type
    register_authorized_mutation: bool,
    _marker: std::marker::PhantomData<T>,
}

#[cfg(feature = "runtime")]
impl<'a, T> SyncComponentBuilder<'a, T>
where
    T: Component + serde::Serialize + for<'de> serde::Deserialize<'de> + Send + Sync + 'static + std::fmt::Debug + Clone,
{
    fn new(app: &'a mut App) -> Self {
        Self {
            app,
            config: ComponentSyncConfig::default(),
            register_authorized_mutation: false,
            _marker: std::marker::PhantomData,
        }
    }

    /// Make this component read-only (clients cannot mutate).
    pub fn read_only(mut self) -> Self {
        self.config.allow_client_mutations = false;
        self
    }

    /// Set a custom denial message for when mutations are rejected.
    pub fn with_denial_message(mut self, message: impl Into<String>) -> Self {
        self.config.mutation_denied_message = Some(message.into());
        self
    }

    /// Register a mutation handler for this component.
    ///
    /// When a handler is registered, client mutations are routed to the handler
    /// system instead of being applied directly. The handler receives
    /// `ComponentMutation<T>` events via `MessageReader` and is responsible for:
    ///
    /// 1. Validating the mutation
    /// 2. Performing any side effects (e.g., calling external systems)
    /// 3. Applying the mutation to the component if appropriate
    /// 4. Responding to the client via `MutationResponseQueue`
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// fn handle_frame_tool_mutation(
    ///     mut mutations: MessageReader<ComponentMutation<FrameToolDataState>>,
    ///     mut robots: Query<(&mut FrameToolDataState, &RobotDriver)>,
    ///     mut responses: ResMut<MutationResponseQueue>,
    /// ) {
    ///     for mutation in mutations.read() {
    ///         let new_state = mutation.new_value();
    ///
    ///         if let Ok((mut state, driver)) = robots.get_mut(mutation.entity()) {
    ///             if let Err(e) = driver.set_frame_tool(new_state.active_frame, new_state.active_tool) {
    ///                 responses.respond_error(mutation.connection_id(), mutation.request_id(), e.to_string());
    ///                 continue;
    ///             }
    ///
    ///             *state = new_state.clone();
    ///             responses.respond_ok(mutation.connection_id(), mutation.request_id());
    ///         }
    ///     }
    /// }
    ///
    /// app.sync_component_builder::<FrameToolDataState>()
    ///     .with_handler::<WebSocketProvider, _, _>(handle_frame_tool_mutation)
    ///     .build();
    /// ```
    pub fn with_handler<NP, S, M>(mut self, handler: S) -> Self
    where
        NP: NetworkProvider,
        S: bevy::ecs::schedule::IntoScheduleConfigs<bevy::ecs::system::ScheduleSystem, M>,
    {
        self.config.has_mutation_handler = true;

        // Add the handler system in the appropriate set
        self.app.add_systems(
            Update,
            handler,
        );

        self
    }

    /// Mark this component's mutations as requiring entity-level authorization.
    ///
    /// When a component is targeted, mutations are only allowed if the client has
    /// control of the target entity. This must be used together with `with_handler()`.
    ///
    /// When targeted, the handler should read `AuthorizedComponentMutation<T>` instead
    /// of `ComponentMutation<T>`.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// fn handle_jog_settings_mutation(
    ///     mut events: MessageReader<AuthorizedComponentMutation<JogSettingsState>>,
    ///     mut settings_query: Query<&mut JogSettingsState>,
    ///     mut response_queue: ResMut<MutationResponseQueue>,
    /// ) {
    ///     for event in events.read() {
    ///         // Authorization already verified - client has control of the entity
    ///         if let Ok(mut settings) = settings_query.get_mut(event.entity) {
    ///             *settings = event.new_value.clone();
    ///             response_queue.respond_ok(event.connection_id, event.request_id);
    ///         }
    ///     }
    /// }
    ///
    /// app.sync_component_builder::<JogSettingsState>()
    ///     .with_handler::<WebSocketProvider, _, _>(handle_jog_settings_mutation)
    ///     .targeted()
    ///     .with_default_entity_policy()
    ///     .build();
    /// ```
    pub fn targeted(mut self) -> Self {
        self.config.requires_entity_authorization = true;
        self.register_authorized_mutation = true;
        self
    }

    /// Use the default entity access policy for authorization.
    ///
    /// This uses `DefaultEntityAccessPolicy` which is typically set by `ExclusiveControlPlugin`.
    /// The policy checks if the client has control of the target entity via `EntityControl`.
    ///
    /// Only applicable when `targeted()` has been called.
    pub fn with_default_entity_policy(mut self) -> Self {
        self.config.use_default_entity_policy = true;
        self
    }

    /// Finalize the registration and apply the configuration.
    pub fn build(self) -> &'a mut App {
        // Register the appropriate message type based on authorization mode
        if self.register_authorized_mutation {
            self.app.add_message::<AuthorizedComponentMutation<T>>();
        } else if self.config.has_mutation_handler {
            self.app.add_message::<ComponentMutation<T>>();
        }

        registry::register_component::<T>(self.app, Some(self.config));
        self.app
    }
}

// =============================================================================
// Query Invalidation API
// =============================================================================

/// Helper to broadcast query invalidations to all connected clients.
///
/// This should be called on the server when data changes that would affect
/// cached query results. Clients will automatically refetch invalidated queries.
///
/// # Example
///
/// ```rust,ignore
/// use pl3xus_sync::invalidate_queries;
///
/// // After creating a program, invalidate the list query
/// fn handle_create_program<NP: NetworkProvider>(
///     world: &mut World,
///     // ... program creation logic ...
/// ) {
///     // Create program...
///
///     // Notify all clients to refetch their ListPrograms queries
///     invalidate_queries::<NP>(world, &["ListPrograms"]);
/// }
/// ```
#[cfg(feature = "runtime")]
pub fn invalidate_queries<NP: NetworkProvider>(world: &World, query_types: &[&str]) {
    if let Some(net) = world.get_resource::<pl3xus::Network<NP>>() {
        let invalidation = QueryInvalidation {
            query_types: query_types.iter().map(|s| s.to_string()).collect(),
            keys: None,
        };
        net.broadcast(SyncServerMessage::QueryInvalidation(invalidation));
    }
}

/// Helper to broadcast query invalidations for specific keys.
///
/// Use this when only specific instances of a keyed query need to be invalidated.
///
/// # Example
///
/// ```rust,ignore
/// // After updating a specific program
/// invalidate_queries_with_keys::<NP>(
///     world,
///     &["GetProgram"],
///     &[program_id.to_string()],
/// );
/// ```
#[cfg(feature = "runtime")]
pub fn invalidate_queries_with_keys<NP: NetworkProvider>(
    world: &World,
    query_types: &[&str],
    keys: &[String],
) {
    if let Some(net) = world.get_resource::<pl3xus::Network<NP>>() {
        let invalidation = QueryInvalidation {
            query_types: query_types.iter().map(|s| s.to_string()).collect(),
            keys: Some(keys.to_vec()),
        };
        net.broadcast(SyncServerMessage::QueryInvalidation(invalidation));
    }
}

/// Helper to invalidate all queries on all clients.
///
/// Use sparingly - this forces a full refetch of all cached data.
#[cfg(feature = "runtime")]
pub fn invalidate_all_queries<NP: NetworkProvider>(world: &World) {
    if let Some(net) = world.get_resource::<pl3xus::Network<NP>>() {
        let invalidation = QueryInvalidation {
            query_types: vec![],
            keys: None,
        };
        net.broadcast(SyncServerMessage::QueryInvalidation(invalidation));
    }
}

