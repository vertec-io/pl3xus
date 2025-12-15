use bevy::prelude::*;
use pl3xus::{managers::NetworkProvider, managers::Network, NetworkData, NetworkEvent};

use crate::messages::{SyncClientMessage, SyncServerMessage, SyncBatch, SyncItem};
use crate::registry::{ComponentChangeEvent, EntityDespawnEvent, MutationQueue, QueuedMutation, SnapshotQueue, SnapshotRequest, SubscriptionEntry, SubscriptionManager, SyncSettings, ConflationQueue};

/// System that reads incoming SyncClientMessage messages and updates the
/// SubscriptionManager / dispatches actions accordingly.
pub fn handle_client_messages<NP: NetworkProvider>(
    mut reader: MessageReader<NetworkData<SyncClientMessage>>,
    subscriptions: Option<ResMut<SubscriptionManager>>,
    mut mutations: Option<ResMut<MutationQueue>>,
    snapshots: Option<ResMut<SnapshotQueue>>,
    _net: Option<Res<Network<NP>>>,
) {
    // If the core sync resources are not yet available, this system should be
    // a no-op rather than causing a hard panic. Subscriptions and snapshots are
    // required; the mutation queue is treated as optional so that read-only
    // inspection still works even if mutation support is not fully wired.
    let Some(mut subscriptions) = subscriptions else {
        trace!("[pl3xus_sync] handle_client_messages: SubscriptionManager resource missing; system will be idle this frame");
        return;
    };
    let Some(mut snapshots) = snapshots else {
        trace!("[pl3xus_sync] handle_client_messages: SnapshotQueue resource missing; system will be idle this frame");
        return;
    };

    if reader.is_empty() {
        trace!("[pl3xus_sync] handle_client_messages: no client messages this frame");
        return;
    }

    use crate::messages::SyncClientMessage as C;

    for msg in reader.read() {
        let source = *msg.source();
        match &**msg {
            C::Subscription(req) => {
                info!(
                    "[pl3xus_sync] New subscription: conn={:?}, sub_id={}, component_type={}, entity={:?}",
                    source,
                    req.subscription_id,
                    req.component_type,
                    req.entity,
                );

                subscriptions.add_subscription(SubscriptionEntry {
                    connection_id: source,
                    subscription_id: req.subscription_id,
                    component_type: req.component_type.clone(),
                    entity: req.entity,
                });

                // Queue a snapshot request so the client receives an initial
                // view of the current world state matching this subscription.
                snapshots.pending.push(SnapshotRequest {
                    connection_id: source,
                    subscription_id: req.subscription_id,
                    component_type: req.component_type.clone(),
                    entity: req.entity,
                });

                info!(
                    "[pl3xus_sync] Queued snapshot request: conn={:?}, sub_id={}, component_type={}, entity={:?}",
                    source,
                    req.subscription_id,
                    req.component_type,
                    req.entity,
                );
            }
            C::Unsubscribe(req) => {
                subscriptions.remove_subscription(source, req.subscription_id);
            }
            C::Mutate(m) => {
                // Queue the mutation for processing in a dedicated system so that
                // we can apply it with proper reflection / auth in a later pass.
                if let Some(mutations) = mutations.as_deref_mut() {
                    mutations.pending.push(QueuedMutation {
                        connection_id: source,
                        request_id: m.request_id,
                        entity: m.entity,
                        component_type: m.component_type.clone(),
                        value: m.value.clone(),
                    });
                } else {
                    trace!(
                        "[pl3xus_sync] handle_client_messages: MutationQueue resource missing; incoming mutation will be ignored (conn={:?}, request_id={:?})",
                        source,
                        m.request_id
                    );
                }
            }
            C::Query(_q) => {
                // Query handling is deferred to v1.1; for now, this is a no-op.
            }
            C::QueryCancel(_c) => {
                // Likewise, query cancellation behavior will be implemented later.
            }
        }
    }
}

/// System that takes aggregated ComponentChangeEvent and EntityDespawnEvent items
/// and routes them to all interested subscribers.
///
/// If conflation is enabled, items are queued in the ConflationQueue and will be
/// sent later by flush_conflation_queue. Otherwise, they are sent immediately.
pub fn broadcast_component_changes<NP: NetworkProvider>(
    mut component_events: MessageReader<ComponentChangeEvent>,
    mut despawn_events: MessageReader<EntityDespawnEvent>,
    subscriptions: Option<Res<SubscriptionManager>>,
    settings: Option<Res<SyncSettings>>,
    mut conflation_queue: Option<ResMut<ConflationQueue>>,
    net: Option<Res<Network<NP>>>,
) {
    // If the required resources aren't available yet (for example, if the
    // network has been torn down during shutdown), bail out quietly.
    let Some(subscriptions) = subscriptions else {
        return;
    };

    if component_events.is_empty() && despawn_events.is_empty() {
        return;
    }

    // Determine if we should use conflation
    let use_conflation = settings
        .as_ref()
        .map(|s| s.enable_message_conflation && s.max_update_rate_hz.is_some())
        .unwrap_or(false);

    // For v1 we use a simple O(N*M) strategy: for each change, scan
    // subscriptions. This is sufficient to validate the pipeline and can be
    // optimized later.
    let mut per_connection: std::collections::HashMap<pl3xus_common::ConnectionId, Vec<SyncItem>> =
        std::collections::HashMap::new();

    // Process component changes
    for change in component_events.read() {
        for sub in &subscriptions.subscriptions {
            if sub.component_type != "*" && sub.component_type != change.component_type {
                continue;
            }
            if let Some(entity) = sub.entity {
                if entity != change.entity {
                    continue;
                }
            }

            per_connection
                .entry(sub.connection_id)
                .or_default()
                .push(SyncItem::Update {
                    subscription_id: sub.subscription_id,
                    entity: change.entity,
                    component_type: change.component_type.clone(),
                    value: change.value.clone(),
                });
        }
    }

    // Process entity despawns
    for despawn in despawn_events.read() {
        for sub in &subscriptions.subscriptions {
            // Entity despawns match all subscriptions for that entity
            if let Some(entity) = sub.entity {
                if entity != despawn.entity {
                    continue;
                }
            }

            per_connection
                .entry(sub.connection_id)
                .or_default()
                .push(SyncItem::EntityRemoved {
                    subscription_id: sub.subscription_id,
                    entity: despawn.entity,
                });
        }
    }

    // Either queue items for later (with conflation) or send immediately
    if use_conflation {
        // Queue items in the conflation queue
        if let Some(ref mut queue) = conflation_queue {
            let enable_conflation = settings.as_ref().unwrap().enable_message_conflation;
            for (connection_id, items) in per_connection {
                for item in items {
                    queue.enqueue(connection_id, item, enable_conflation);
                }
            }
        }
    } else {
        // Send immediately (original behavior)
        if let Some(net) = net {
            for (connection_id, items) in per_connection {
                if items.is_empty() {
                    continue;
                }
                let batch = SyncBatch { items };
                let _ = net.send(connection_id, SyncServerMessage::SyncBatch(batch));
            }
        }
    }
}
/// Cleanup subscriptions and pending mutations when connections disconnect.
pub fn cleanup_disconnected(
    mut events: MessageReader<NetworkEvent>,
    subscriptions: Option<ResMut<SubscriptionManager>>,
    mutations: Option<ResMut<MutationQueue>>,
) {
    let (Some(mut subscriptions), Some(mut mutations)) = (subscriptions, mutations) else {
        return;
    };

    for event in events.read() {
        match event {
            NetworkEvent::Disconnected(connection_id) => {
                info!("[pl3xus_sync] Cleaning up disconnected connection: {:?}", connection_id);

                let before_subs = subscriptions.subscriptions.len();
                subscriptions.remove_all_for_connection(*connection_id);
                let after_subs = subscriptions.subscriptions.len();
                info!("[pl3xus_sync] Removed {} subscriptions for {:?}", before_subs - after_subs, connection_id);

                let before_count = mutations.pending.len();
                mutations
                    .pending
                    .retain(|m| m.connection_id != *connection_id);
                let after_count = mutations.pending.len();
                info!("[pl3xus_sync] Removed {} pending mutations for {:?}", before_count - after_count, connection_id);
            }
            NetworkEvent::Connected(connection_id) => {
                info!("[pl3xus_sync] New connection: {:?}", connection_id);
            }
            _ => {}
        }
    }
}


