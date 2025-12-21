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
//! - [`TargetedMessageAuthorizer`] / [`TargetedAuthorizerResource`]: pluggable
//!   authorization policies for targeted messages (commands sent to specific entities).

mod messages;
#[cfg(feature = "runtime")]
mod registry;
#[cfg(feature = "runtime")]
mod subscription;
#[cfg(feature = "runtime")]
mod systems;

/// Pluggable authorization policies for targeted messages.
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
#[cfg(feature = "runtime")]
pub use authorization::{
    TargetedAuthContext,
    TargetedAuthResult,
    TargetedMessageAuthorizer,
    TargetedAuthorizerResource,
    AuthorizedMessage,
    AppAuthorizedMessageExt,
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

