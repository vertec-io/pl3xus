use bevy::prelude::*;
use std::sync::Arc;
use std::collections::HashMap;

use crate::messages::{MutationStatus, SerializableEntity, SyncItem};

/// Configuration for how a component type should be synchronized.
#[derive(Clone)]
pub struct ComponentSyncConfig {
    /// Maximum number of updates per frame (per client); `None` means unlimited.
    pub max_updates_per_frame: Option<usize>,
}

impl Default for ComponentSyncConfig {
    fn default() -> Self {
        Self {
            max_updates_per_frame: None,
        }
    }
}

/// Global settings for the sync system.
#[derive(Resource, Clone)]
pub struct SyncSettings {
    /// Maximum update rate in Hz (updates per second).
    /// For example, 30.0 means clients receive at most 30 updates per second.
    /// Set to None for unlimited (send every frame).
    pub max_update_rate_hz: Option<f32>,

    /// Whether to enable message conflation (keeping only latest update per entity+component).
    /// When true, if multiple updates for the same entity+component arrive before the next
    /// flush, only the latest value is sent.
    pub enable_message_conflation: bool,
}

impl Default for SyncSettings {
    fn default() -> Self {
        Self {
            // Default to 30 Hz update rate (good balance for most applications)
            max_update_rate_hz: Some(30.0),
            // Enable conflation by default (prevents overwhelming slow clients)
            enable_message_conflation: true,
        }
    }
}

/// Key for identifying unique updates in the conflation queue.
/// Updates with the same key will overwrite each other (keeping only the latest).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConflationKey {
    pub subscription_id: u64,
    pub entity: SerializableEntity,
    pub component_type: String,
}

impl ConflationKey {
    pub fn from_sync_item(item: &SyncItem) -> Option<Self> {
        match item {
            SyncItem::Update { subscription_id, entity, component_type, .. } => {
                Some(ConflationKey {
                    subscription_id: *subscription_id,
                    entity: entity.clone(),
                    component_type: component_type.clone(),
                })
            }
            SyncItem::Snapshot { subscription_id, entity, component_type, .. } => {
                Some(ConflationKey {
                    subscription_id: *subscription_id,
                    entity: entity.clone(),
                    component_type: component_type.clone(),
                })
            }
            // Entity removals and component removals can't be conflated
            _ => None,
        }
    }
}

/// Queue of pending sync items waiting to be flushed to clients.
/// This enables message conflation and rate limiting.
#[derive(Resource, Default)]
pub struct ConflationQueue {
    /// Pending items per connection.
    /// For each connection, we maintain a map of ConflationKey -> SyncItem.
    /// When conflation is enabled, updates with the same key overwrite each other.
    pub pending: HashMap<pl3xus_common::ConnectionId, HashMap<ConflationKey, SyncItem>>,

    /// Non-conflatable items (entity removals, component removals) are stored separately
    /// and always sent in order.
    pub non_conflatable: HashMap<pl3xus_common::ConnectionId, Vec<SyncItem>>,

    /// Timer for tracking when to flush the queue.
    pub flush_timer: Timer,
}

impl ConflationQueue {
    pub fn new(update_rate_hz: f32) -> Self {
        let flush_interval = std::time::Duration::from_secs_f32(1.0 / update_rate_hz);
        Self {
            pending: HashMap::new(),
            non_conflatable: HashMap::new(),
            flush_timer: Timer::new(flush_interval, TimerMode::Repeating),
        }
    }

    /// Add a sync item to the queue.
    /// If conflation is enabled and the item is conflatable, it will overwrite any existing
    /// item with the same key.
    pub fn enqueue(&mut self, connection_id: pl3xus_common::ConnectionId, item: SyncItem, enable_conflation: bool) {
        if enable_conflation {
            if let Some(key) = ConflationKey::from_sync_item(&item) {
                // Conflatable item - store in the conflation map
                self.pending
                    .entry(connection_id)
                    .or_default()
                    .insert(key, item);
                return;
            }
        }

        // Non-conflatable item or conflation disabled - store in order
        self.non_conflatable
            .entry(connection_id)
            .or_default()
            .push(item);
    }

    /// Drain all pending items for a connection and return them as a Vec.
    pub fn drain_for_connection(&mut self, connection_id: pl3xus_common::ConnectionId) -> Vec<SyncItem> {
        let mut items = Vec::new();

        // First, add all conflated items
        if let Some(conflated) = self.pending.remove(&connection_id) {
            items.extend(conflated.into_values());
        }

        // Then, add all non-conflatable items (in order)
        if let Some(non_conflatable) = self.non_conflatable.remove(&connection_id) {
            items.extend(non_conflatable);
        }

        items
    }

    /// Get the total number of pending items for a connection.
    pub fn pending_count(&self, connection_id: pl3xus_common::ConnectionId) -> usize {
        let conflated = self.pending.get(&connection_id).map(|m| m.len()).unwrap_or(0);
        let non_conflatable = self.non_conflatable.get(&connection_id).map(|v| v.len()).unwrap_or(0);
        conflated + non_conflatable
    }
}

/// Per-type registration data stored in the [`SyncRegistry`].
pub struct ComponentRegistration {
    pub type_id: std::any::TypeId,
    pub type_name: String,
    pub config: ComponentSyncConfig,
    /// Type-specific function that knows how to deserialize and apply a
    /// queued mutation for this component.
    pub apply_mutation: fn(&mut World, &QueuedMutation) -> MutationStatus,
    /// Type-specific function that can produce a full snapshot of all
    /// `(Entity, Component)` pairs for this component type, encoded as bincode
    /// bytes suitable for transmission over the wire.
    pub snapshot_all: fn(&mut World) -> Vec<(SerializableEntity, Vec<u8>)>,
}

/// Registry of component types that participate in synchronization.
#[derive(Resource, Default)]
pub struct SyncRegistry {
    pub components: Vec<ComponentRegistration>,
}

impl SyncRegistry {
    pub fn register_component(&mut self, registration: ComponentRegistration) {
        // Avoid double registration for the same TypeId.
        if self
            .components
            .iter()
            .any(|c| c.type_id == registration.type_id)
        {
            return;
        }
        self.components.push(registration);
    }
}

/// Subscription tracking keyed by (connection, subscription_id).
#[derive(Resource, Default)]
pub struct SubscriptionManager {
    // For v1, keep this simple; we can optimize later.
    pub subscriptions: Vec<SubscriptionEntry>,
}

/// One subscription from a specific client.
#[derive(Clone)]
pub struct SubscriptionEntry {
    pub connection_id: pl3xus_common::ConnectionId,
    pub subscription_id: u64,
    pub component_type: String,
    pub entity: Option<SerializableEntity>,
}

impl SubscriptionManager {
    pub fn add_subscription(&mut self, entry: SubscriptionEntry) {
        self.subscriptions.push(entry);
    }

    pub fn remove_subscription(
        &mut self,
        connection: pl3xus_common::ConnectionId,
        subscription_id: u64,
    ) {
        self.subscriptions.retain(|s| {
            !(s.connection_id == connection && s.subscription_id == subscription_id)
        });
    }

    pub fn remove_all_for_connection(&mut self, connection: pl3xus_common::ConnectionId) {
        self.subscriptions
            .retain(|s| s.connection_id != connection);
    }
}

/// A single snapshot request queued when a client first subscribes.
#[derive(Clone)]
pub struct SnapshotRequest {
    pub connection_id: pl3xus_common::ConnectionId,
    pub subscription_id: u64,
    pub component_type: String,
    pub entity: Option<SerializableEntity>,
}

/// Queue of pending snapshot requests to be processed by a dedicated system.
#[derive(Resource, Default)]
pub struct SnapshotQueue {
    pub pending: Vec<SnapshotRequest>,
}
/// Queue of pending component mutations requested by clients.
#[derive(Resource, Default)]
pub struct MutationQueue {
    /// Pending mutations received from clients via `SyncClientMessage::Mutate`.
    ///
    /// These are processed by an internal system which will consult any
    /// configured [`MutationAuthorizer`] and, if authorized, deserialize and
    /// apply the change to the ECS world.
    pub pending: Vec<QueuedMutation>,
}

/// A single queued mutation request.
#[derive(Clone)]
pub struct QueuedMutation {
    /// Connection that originated the mutation request.
    pub connection_id: pl3xus_common::ConnectionId,
    /// Optional client-chosen correlation id.
    pub request_id: Option<u64>,
    pub entity: SerializableEntity,
    pub component_type: String,
    /// Full component value encoded as bincode bytes (v1 uses full replacement semantics).
    pub value: Vec<u8>,
}

/// Context passed into a [`MutationAuthorizer`] when deciding whether to allow
/// a mutation.
pub struct MutationAuthContext<'a> {
    pub world: &'a World,
}

/// Pluggable policy for deciding whether a queued mutation is allowed to be
/// applied to the world.
///
/// Implementations can inspect arbitrary application state via the
/// [`MutationAuthContext::world`] reference (for example, relationships between
/// connections and entities using Bevy's built-in parent/child hierarchy).
pub trait MutationAuthorizer: Send + Sync + 'static {
    /// Decide whether `mutation` should be applied.
    ///
    /// Returning any status other than [`MutationStatus::Ok`] will prevent the
    /// mutation from being applied and will be propagated back to the client via
    /// [`crate::messages::MutationResponse`].
    fn authorize(&self, ctx: &MutationAuthContext, mutation: &QueuedMutation) -> MutationStatus;
}

/// Resource wrapping the active mutation authorization policy, if any.
///
/// If this resource is not present, all client mutations are treated as
/// authorized by default. Applications can install their own policy by
/// inserting this resource into the `App`.
#[derive(Resource)]
pub struct MutationAuthorizerResource {
    pub inner: Arc<dyn MutationAuthorizer>,
}

impl MutationAuthorizerResource {
    /// Construct an authorizer from a simple closure.
    ///
    /// This is the most convenient way for downstream apps to express custom
    /// authorization logic.
    pub fn from_fn<F>(f: F) -> Self
    where
        F: Fn(&World, &QueuedMutation) -> MutationStatus + Send + Sync + 'static,
    {
        struct ClosureAuthorizer<F>(F);

        impl<F> MutationAuthorizer for ClosureAuthorizer<F>
        where
            F: Fn(&World, &QueuedMutation) -> MutationStatus + Send + Sync + 'static,
        {
            fn authorize(
                &self,
                ctx: &MutationAuthContext,
                mutation: &QueuedMutation,
            ) -> MutationStatus {
                (self.0)(ctx.world, mutation)
            }
        }

        Self {
            inner: Arc::new(ClosureAuthorizer(f)),
        }
    }

    /// Convenience constructor for a built-in "server-only" policy.
    ///
    /// Under this policy, only the special `ConnectionId::SERVER` is allowed to
    /// issue mutations. All other clients will receive
    /// [`MutationStatus::Forbidden`].
    pub fn server_only() -> Self {
        Self {
            inner: Arc::new(ServerOnlyMutationAuthorizer),
        }
    }
}

/// Simple built-in policy that only allows mutations originating from the
/// server connection id. This is useful for deployments where the server is the
/// sole authority that ever mutates ECS state, while clients are strictly
/// read-only observers.
pub struct ServerOnlyMutationAuthorizer;

impl MutationAuthorizer for ServerOnlyMutationAuthorizer {
    fn authorize(&self, _ctx: &MutationAuthContext, mutation: &QueuedMutation) -> MutationStatus {
        if mutation.connection_id.is_server() {
            MutationStatus::Ok
        } else {
            MutationStatus::Forbidden
        }
    }
}

/// Helper function for hierarchy-aware authorization.
///
/// Traverses the entity hierarchy (using Bevy's `Parent` component) to check if
/// the given entity or any of its ancestors has a control component matching the
/// provided predicate.
///
/// This is useful for implementing authorization policies where control of a parent
/// entity grants control over all child entities.
///
/// # Type Parameters
///
/// - `C`: The control component type (e.g., `NodeControl`, `PlayerControl`)
/// - `F`: Predicate function that checks if the control component grants access
///
/// # Arguments
///
/// - `world`: Reference to the Bevy world
/// - `entity`: The entity to check (will traverse up to ancestors)
/// - `predicate`: Function that returns `true` if the control component grants access
///
/// # Returns
///
/// `true` if the entity or any ancestor has a control component matching the predicate,
/// `false` otherwise.
///
/// # Example
///
/// ```rust,ignore
/// use pl3xus_sync::{has_control_hierarchical, MutationAuthorizerResource};
/// use bevy::prelude::*;
///
/// #[derive(Component)]
/// struct NodeControl {
///     client_id: ConnectionId,
/// }
///
/// app.insert_resource(MutationAuthorizerResource::from_fn(
///     |world, mutation| {
///         let entity = Entity::from_bits(mutation.entity.0);
///
///         if has_control_hierarchical::<NodeControl, _>(
///             world,
///             entity,
///             |control| control.client_id == mutation.connection_id
///         ) {
///             MutationStatus::Ok
///         } else {
///             MutationStatus::Forbidden
///         }
///     }
/// ));
/// ```
pub fn has_control_hierarchical<C, F>(
    world: &World,
    entity: Entity,
    predicate: F,
) -> bool
where
    C: Component,
    F: Fn(&C) -> bool,
{
    let mut current = Some(entity);

    while let Some(e) = current {
        if let Ok(entity_ref) = world.get_entity(e) {
            // Check if this entity has the control component
            if let Some(control) = entity_ref.get::<C>() {
                if predicate(control) {
                    return true;
                }
            }

            // Traverse to parent using Bevy's built-in ChildOf component
            current = entity_ref.get::<ChildOf>().map(|child_of| child_of.parent());
        } else {
            break;
        }
    }

    false
}

/// Minimal representation of a component change event emitted by typed systems.
#[derive(Debug, Clone, Message)]
pub struct ComponentChangeEvent {
    pub entity: SerializableEntity,
    pub component_type: String,
    pub value: Vec<u8>,
}

/// Event emitted when an entity is despawned.
#[derive(Debug, Clone, Message)]
pub struct EntityDespawnEvent {
    pub entity: SerializableEntity,
}

fn apply_typed_mutation<T>(world: &mut World, mutation: &QueuedMutation) -> MutationStatus
where
    T: Component + serde::Serialize + for<'de> serde::Deserialize<'de> + Send + Sync + 'static + std::fmt::Debug,
{
    // Deserialize bincode bytes → concrete component type
    let value: T = match bincode::serde::decode_from_slice(&mutation.value, bincode::config::standard()) {
        Ok((v, _)) => v,
        Err(_err) => {
            return MutationStatus::ValidationError;
        }
    };

    bevy::log::info!("[apply_typed_mutation] Applying mutation: entity={:?}, type={}, value={:?}",
        mutation.entity, mutation.component_type, value);

    // Check if this is a request to spawn a new entity
    if mutation.entity == SerializableEntity::DANGLING {
        // Spawn a new entity with the component
        world.spawn(value);
        bevy::log::info!("[apply_typed_mutation] Spawned new entity with component {}", mutation.component_type);
        return MutationStatus::Ok;
    }

    let entity = mutation.entity.to_entity();
    match world.get_entity_mut(entity) {
        Ok(mut entity_mut) => {
            // Bevy's insert semantics: insert or replace the component value.
            entity_mut.insert(value);
            MutationStatus::Ok
        }
        Err(_) => MutationStatus::NotFound,
    }
}
fn snapshot_typed<T>(world: &mut World) -> Vec<(SerializableEntity, Vec<u8>)>
where
    T: Component + serde::Serialize + for<'de> serde::Deserialize<'de> + Send + Sync + 'static,
{
    let mut results = Vec::new();

    // Use a temporary query to iterate all entities with this component type.
    let mut query = world.query::<(Entity, &T)>();
    for (entity, component) in query.iter(world) {
        // Serialize component directly to bincode bytes
        let bytes = bincode::serde::encode_to_vec(component, bincode::config::standard())
            .unwrap_or_default();
        results.push((SerializableEntity::from(entity), bytes));
    }

    results
}



/// Helper used by [`AppPl3xusSyncExt::sync_component`] to register a type.
#[cfg(feature = "runtime")]
pub fn register_component<T>(app: &mut App, config: Option<ComponentSyncConfig>)
where
    T: Component + serde::Serialize + for<'de> serde::Deserialize<'de> + Send + Sync + 'static + std::fmt::Debug,
{
    // Register in SyncRegistry
    {
        let mut registry = app.world_mut().get_resource_or_insert_with(SyncRegistry::default);
        // Use short type name (just the struct name, no module path) for stability
        // This ensures client and server use the same type identifier
        let full_type_name = std::any::type_name::<T>();
        let type_name = full_type_name.rsplit("::").next().unwrap_or(full_type_name).to_string();
        registry.register_component(ComponentRegistration {
            type_id: std::any::TypeId::of::<T>(),
            type_name,
            config: config.unwrap_or_default(),
            apply_mutation: apply_typed_mutation::<T>,
            snapshot_all: snapshot_typed::<T>,
        });
    }

    // Add the typed system that will emit change events for this component type.
    crate::systems::register_component_system::<T>(app);
}

