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
    fn sync_component<T>(&mut self, config: Option<ComponentSyncConfig>) -> &mut Self
    where
        T: Component + serde::Serialize + for<'de> serde::Deserialize<'de> + Send + Sync + 'static + std::fmt::Debug;
}

#[cfg(feature = "runtime")]
impl AppPl3xusSyncExt for App {
    fn sync_component<T>(&mut self, config: Option<ComponentSyncConfig>) -> &mut Self
    where
        T: Component + serde::Serialize + for<'de> serde::Deserialize<'de> + Send + Sync + 'static + std::fmt::Debug,
    {
        registry::register_component::<T>(self, config);
        self
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

