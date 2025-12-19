use bevy::prelude::*;
use serde::Serialize;

use pl3xus::{managers::Network, managers::NetworkProvider, NetworkEvent};

use crate::messages::{
    MutationResponse,
    SyncBatch,
    SyncClientMessage,
    SyncItem,
    SyncServerMessage,
    WelcomeMessage,
};
use crate::registry::{
    ComponentChangeEvent,
    ComponentRemovedEvent,
    EntityDespawnEvent,
    MutationAuthContext,
    MutationAuthorizerResource,
    MutationQueue,
    SnapshotQueue,
    SubscriptionManager,
    SyncRegistry,
    SyncSettings,
    ConflationQueue,
    short_type_name,
};
use crate::subscription::{broadcast_component_changes, handle_client_messages};

/// System set for sync-related systems so downstream apps can schedule around
/// them if needed.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Pl3xusSyncSystems {
    /// Systems that read client messages and update subscriptions/mutations.
    Inbound,
    /// Systems that observe component changes and emit ComponentChangeEvent.
    Observe,
    /// Systems that broadcast updates to clients.
    Outbound,
}

/// Install core resources and systems for Pl3xusSync into the app.
pub(crate) fn install<NP: NetworkProvider>(app: &mut App) {
    // Initialize SyncSettings first (needed to create ConflationQueue)
    app.init_resource::<SyncSettings>();

    // Initialize ConflationQueue with settings from SyncSettings
    {
        let settings = app.world().resource::<SyncSettings>();
        let update_rate_hz = settings.max_update_rate_hz.unwrap_or(60.0);
        app.insert_resource(ConflationQueue::new(update_rate_hz));
    }

    app.init_resource::<SubscriptionManager>()
        .init_resource::<MutationQueue>()
        .init_resource::<SnapshotQueue>()
        .add_message::<ComponentChangeEvent>()
        .add_message::<ComponentRemovedEvent>()
        .add_message::<EntityDespawnEvent>();

    // Verify resources were initialized
    let world = app.world();
    info!(
        "[pl3xus_sync::install] Resources initialized: SubscriptionManager={}, MutationQueue={}, SnapshotQueue={}, SyncSettings={}, ConflationQueue={}",
        world.contains_resource::<SubscriptionManager>(),
        world.contains_resource::<MutationQueue>(),
        world.contains_resource::<SnapshotQueue>(),
        world.contains_resource::<SyncSettings>(),
        world.contains_resource::<ConflationQueue>()
    );

    app.configure_sets(
            Update,
            (
                Pl3xusSyncSystems::Inbound,
                Pl3xusSyncSystems::Observe,
                Pl3xusSyncSystems::Outbound,
            )
                .chain(),
        )
        // Client-side messages -> subscription manager
        .add_systems(
            Update,
            handle_client_messages::<NP>.in_set(Pl3xusSyncSystems::Inbound),
        )
        // Send Welcome message to newly connected clients (must run before cleanup_disconnected
        // since both read NetworkEvent and events can only be read once)
        // We handle both Connected and Disconnected events in a single system now
        .add_systems(
            Update,
            handle_connection_events::<NP>.in_set(Pl3xusSyncSystems::Inbound),
        )
        // Process queued mutations: authorization + apply + MutationResponse
        .add_systems(
            Update,
            process_mutations::<NP>.in_set(Pl3xusSyncSystems::Inbound),
        )
        // Process queued snapshot requests and send initial SyncBatch snapshots
        // back to subscribing clients.
        .add_systems(
            Update,
            process_snapshot_queue::<NP>.in_set(Pl3xusSyncSystems::Observe),
        )
        // ComponentChangeEvent -> SyncServerMessage batches (or queue for conflation)
        .add_systems(
            Update,
            broadcast_component_changes::<NP>.in_set(Pl3xusSyncSystems::Outbound),
        )
        // Flush conflation queue on timer
        .add_systems(
            Update,
            flush_conflation_queue::<NP>.in_set(Pl3xusSyncSystems::Outbound),
        );

    // Register sync messages with pl3xus so they can be transported
    register_network_messages::<NP>(app);
}

fn register_network_messages<NP: NetworkProvider>(app: &mut App) {
    use pl3xus::AppNetworkMessage;

    app.register_network_message::<SyncClientMessage, NP>();
    app.register_network_message::<crate::messages::SyncServerMessage, NP>();
}

/// Handle connection events: send Welcome to new connections and cleanup disconnected ones.
/// This combines both operations in a single system since NetworkEvent can only be read once.
fn handle_connection_events<NP: NetworkProvider>(
    net: Res<Network<NP>>,
    mut network_events: MessageReader<NetworkEvent>,
    subscriptions: Option<ResMut<SubscriptionManager>>,
    mutations: Option<ResMut<MutationQueue>>,
) {
    let (mut subscriptions, mut mutations) = match (subscriptions, mutations) {
        (Some(s), Some(m)) => (s, m),
        _ => return,
    };

    for event in network_events.read() {
        match event {
            NetworkEvent::Connected(conn_id) => {
                info!("[pl3xus_sync] Sending Welcome message to client {:?}", conn_id);
                let welcome = SyncServerMessage::Welcome(WelcomeMessage {
                    connection_id: *conn_id,
                });
                if let Err(e) = net.send(*conn_id, welcome) {
                    warn!("[pl3xus_sync] Failed to send Welcome to {:?}: {:?}", conn_id, e);
                }
            }
            NetworkEvent::Disconnected(connection_id) => {
                info!("[pl3xus_sync] Connection disconnected: {:?}", connection_id);
                // Remove all subscriptions for this connection
                subscriptions.subscriptions.retain(|sub| {
                    let keep = sub.connection_id != *connection_id;
                    if !keep {
                        info!("[pl3xus_sync] Removed subscription for {:?}", connection_id);
                    }
                    keep
                });
                // Remove pending mutations from this connection
                let before_count = mutations.pending.len();
                mutations
                    .pending
                    .retain(|m| m.connection_id != *connection_id);
                let after_count = mutations.pending.len();
                info!("[pl3xus_sync] Removed {} pending mutations for {:?}", before_count - after_count, connection_id);
            }
            _ => {}
        }
    }
}

/// Drain the global mutation queue, run authorization, apply mutations and
/// emit `MutationResponse` messages back to the originating client.
pub fn process_mutations<NP: NetworkProvider>(world: &mut World) {
    use crate::messages::MutationStatus as Status;

    // Take ownership of the pending mutations so we can freely borrow the
    // world while iterating.
    let mut pending = {
        if let Some(mut queue) = world.get_resource_mut::<MutationQueue>() {
            std::mem::take(&mut queue.pending)
        } else {
            return;
        }
    };

    if pending.is_empty() {
        return;
    }

    for mutation in pending.drain(..) {
        let mut status = Status::Ok;

        // Optional authorization step.
        if let Some(auth_res) = world.get_resource::<MutationAuthorizerResource>() {
            let ctx = MutationAuthContext { world: &*world };
            status = auth_res.inner.authorize(&ctx, &mutation);
        }

        if let Status::Ok = status {
            // Look up the per-type mutation handler based on the registered
            // component type name.
            let apply_fn = world
                .get_resource::<SyncRegistry>()
                .and_then(|registry| {
                    registry
                        .components
                        .iter()
                        .find(|reg| reg.type_name == mutation.component_type)
                        .map(|reg| reg.apply_mutation)
                });

            match apply_fn {
                None => {
                    status = Status::NotFound;
                }
                Some(apply) => {
                    // Ensure that panics while applying a mutation are contained
                    // and reported back as an internal error rather than
                    // crashing the entire app.
                    let apply_result = std::panic::catch_unwind(
                        std::panic::AssertUnwindSafe(|| apply(world, &mutation)),
                    );

                    match apply_result {
                        Ok(result_status) => {
                            status = result_status;
                        }
                        Err(_) => {
                            status = Status::InternalError;
                        }
                    }
                }
            }
        }

        // Respond back to the originating client, if we have a network
        // provider for this plugin's `NetworkProvider` type.
        if let Some(net) = world.get_resource::<Network<NP>>() {
            let response = MutationResponse {
                request_id: mutation.request_id,
                status: status.clone(),
                message: None,
            };
            let _ = net.send(
                mutation.connection_id,
                SyncServerMessage::MutationResponse(response),
            );
        }
    }
}
/// Drain the snapshot queue and send initial snapshot batches to clients.
pub fn process_snapshot_queue<NP: NetworkProvider>(world: &mut World) {
    // Take ownership of pending snapshot requests.
    let mut pending = {
        if let Some(mut queue) = world.get_resource_mut::<SnapshotQueue>() {
            std::mem::take(&mut queue.pending)
        } else {
            return;
        }
    };

    if pending.is_empty() {
        return;
    }

    // Collect per-type snapshot functions up front so we don't hold
    // references into the registry while invoking them.
    let type_snapshot_fns: Vec<(
        String,
        fn(&mut World) -> Vec<(crate::messages::SerializableEntity, Vec<u8>)>,
    )> = world
        .get_resource::<SyncRegistry>()
        .map(|registry| {
            registry
                .components
                .iter()
                .map(|reg| (reg.type_name.clone(), reg.snapshot_all))
                .collect()
        })
        .unwrap_or_default();

    if type_snapshot_fns.is_empty() {
        return;
    }

    // Accumulate items per connection so we can batch sends.
    let mut per_connection: std::collections::HashMap<
        pl3xus_common::ConnectionId,
        Vec<SyncItem>,
    > = std::collections::HashMap::new();

    for request in pending.drain(..) {
        for (type_name, snapshot_fn) in &type_snapshot_fns {
            if request.component_type != "*" && type_name != &request.component_type {
                continue;
            }

            let snapshots = snapshot_fn(world);

            for (entity, value) in snapshots {
                if let Some(target) = request.entity {
                    if target != entity {
                        continue;
                    }
                }

                per_connection
                    .entry(request.connection_id)
                    .or_default()
                    .push(SyncItem::Snapshot {
                        subscription_id: request.subscription_id,
                        entity,
                        component_type: type_name.clone(),
                        value,
                    });
            }
        }
    }

    if per_connection.is_empty() {
        return;
    }

    info!(
        "[pl3xus_sync] Processing {} snapshot batches for {} connections",
        per_connection.values().map(|items| items.len()).sum::<usize>(),
        per_connection.len()
    );

    if let Some(net) = world.get_resource::<Network<NP>>() {
        for (connection_id, items) in per_connection {
            if items.is_empty() {
                continue;
            }

            info!(
                "[pl3xus_sync] Sending snapshot batch: conn={:?}, items={}",
                connection_id,
                items.len()
            );

            let batch = SyncBatch { items };
            let _ = net.send(connection_id, SyncServerMessage::SyncBatch(batch));
        }
    }
}


/// Register the typed observation system for a given component type T.
pub fn register_component_system<T>(app: &mut App)
where
    T: Component + Serialize + for<'de> serde::Deserialize<'de> + Send + Sync + 'static,
{
    // We add a Changed<T>-based system that fires late in the frame (Observe
    // set) and emits ComponentChangeEvent instances.
    app.add_systems(
        Update,
        observe_component_changes::<T>.in_set(Pl3xusSyncSystems::Observe),
    );

    // Also add a system to observe entity despawns for this component type.
    // This will emit EntityDespawnEvent when entities with this component are despawned.
    app.add_systems(
        Update,
        observe_entity_despawns::<T>.in_set(Pl3xusSyncSystems::Observe),
    );
}

/// Observe Changed<T> and convert into generic ComponentChangeEvent instances.
fn observe_component_changes<T>(
    query: Query<(Entity, &T), Changed<T>>,
    mut writer: MessageWriter<ComponentChangeEvent>,
) where
    T: Component + Serialize + for<'de> serde::Deserialize<'de> + Send + Sync + 'static,
{
    // Use short type name (just the struct name, no module path) for stability
    // This ensures client and server use the same type identifier
    let full_type_name = std::any::type_name::<T>();
    let type_name = full_type_name.rsplit("::").next().unwrap_or(full_type_name).to_string();

    for (entity, component) in query.iter() {
        // Serialize component directly to bincode bytes
        let bytes = bincode::serde::encode_to_vec(component, bincode::config::standard())
            .unwrap_or_default();
        writer.write(ComponentChangeEvent {
            entity: crate::messages::SerializableEntity::from(entity),
            component_type: type_name.clone(),
            value: bytes,
        });
    }
}

/// Observe component removals and entity despawns.
///
/// - If entity still exists: emit ComponentRemovedEvent (component was removed)
/// - If entity no longer exists: emit EntityDespawnEvent (entity was despawned)
fn observe_entity_despawns<T>(
    mut removed: RemovedComponents<T>,
    entities: &bevy::ecs::entity::Entities,
    mut despawn_writer: MessageWriter<EntityDespawnEvent>,
    mut removal_writer: MessageWriter<ComponentRemovedEvent>,
) where
    T: Component + Send + Sync + 'static,
{
    let component_type = short_type_name::<T>();

    for entity in removed.read() {
        if entities.contains(entity) {
            // Entity still exists - this was just a component removal
            removal_writer.write(ComponentRemovedEvent {
                entity: crate::messages::SerializableEntity::from(entity),
                component_type: component_type.clone(),
            });
        } else {
            // Entity no longer exists - this was a despawn
            despawn_writer.write(EntityDespawnEvent {
                entity: crate::messages::SerializableEntity::from(entity),
            });
        }
    }
}

/// Flush the conflation queue on a timer, sending batched updates to clients.
/// This system only runs when conflation is enabled (max_update_rate_hz is set).
pub fn flush_conflation_queue<NP: NetworkProvider>(
    mut conflation_queue: ResMut<ConflationQueue>,
    settings: Res<SyncSettings>,
    net: Option<Res<Network<NP>>>,
    time: Res<Time>,
) {
    // Only flush if conflation is enabled
    if !settings.enable_message_conflation || settings.max_update_rate_hz.is_none() {
        return;
    }

    // Tick the timer
    conflation_queue.flush_timer.tick(time.delta());

    // Only flush when timer finishes
    if !conflation_queue.flush_timer.just_finished() {
        return;
    }

    // Get all connection IDs that have pending items
    let connection_ids: Vec<pl3xus_common::ConnectionId> = conflation_queue
        .pending
        .keys()
        .chain(conflation_queue.non_conflatable.keys())
        .copied()
        .collect();

    if connection_ids.is_empty() {
        return;
    }

    let Some(net) = net else {
        return;
    };

    // Flush each connection's pending items
    for connection_id in connection_ids {
        let items = conflation_queue.drain_for_connection(connection_id);

        if items.is_empty() {
            continue;
        }

        debug!(
            "[pl3xus_sync] Flushing {} conflated items to connection {:?}",
            items.len(),
            connection_id
        );

        let batch = SyncBatch { items };
        let _ = net.send(connection_id, SyncServerMessage::SyncBatch(batch));
    }
}

