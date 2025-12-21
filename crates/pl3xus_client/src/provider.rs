use std::sync::Arc;

use leptos::prelude::*;
use leptos_use::{use_websocket_with_options, DummyEncoder, UseWebSocketOptions, UseWebSocketReturn};
use pl3xus_common::codec::Pl3xusBincodeCodec;
use pl3xus_common::NetworkPacket;

use crate::client_type_registry::ClientTypeRegistry;
use crate::context::SyncContext;
use crate::error::SyncError;
use pl3xus_sync::{SyncClientMessage, SyncServerMessage};

/// Provider component that sets up WebSocket connection and provides SyncContext.
///
/// This component should wrap your application or the part of your application
/// that needs access to synchronized ECS data.
///
/// # Example
///
/// ```rust,ignore
/// use pl3xus_client::{SyncProvider, ClientTypeRegistry};
///
/// #[component]
/// pub fn App() -> impl IntoView {
///     let registry = ClientTypeRegistry::builder()
///         .register::<Position>()
///         .register::<Velocity>()
///         .build();
///
///     view! {
///         <SyncProvider
///             url="ws://localhost:3000/sync"
///             registry=registry
///         >
///             <MyAppUI />
///         </SyncProvider>
///     }
/// }
/// ```
#[component]
pub fn SyncProvider(
    /// WebSocket URL to connect to
    url: String,
    /// Type registry for deserializing components
    registry: Arc<ClientTypeRegistry>,
    /// Whether to automatically connect on mount (default: true)
    #[prop(optional)]
    auto_connect: Option<bool>,
    /// Child components
    children: Children,
) -> impl IntoView {
    let auto_connect = auto_connect.unwrap_or(true);

    // Set up WebSocket connection using NetworkPacket wrapper
    // This matches the pl3xus wire protocol
    let UseWebSocketReturn {
        ready_state,
        message: raw_message,
        send: raw_send,
        open,
        close,
        ..
    } = use_websocket_with_options::<
        NetworkPacket,
        NetworkPacket,
        Pl3xusBincodeCodec,
        (),
        DummyEncoder,
    >(
        &url,
        UseWebSocketOptions::default()
            .immediate(auto_connect) // Auto-connect if requested
            .on_open(move |_| {
                #[cfg(target_arch = "wasm32")]
                leptos::logging::log!("[SyncProvider] WebSocket opened!");
            })
            .on_error(move |_e| {
                #[cfg(target_arch = "wasm32")]
                leptos::logging::warn!("[SyncProvider] WebSocket error: {:?}", _e);
            }),
    );

    // Create error signal
    let last_error = RwSignal::new(None::<SyncError>);

    // Create SyncContext
    // The send function handles two cases:
    // 1. SyncClientMessage bytes (for subscriptions, mutations) - need to wrap in NetworkPacket
    // 2. Already-serialized NetworkPacket bytes (from SyncContext::send()) - send directly
    let send_arc = Arc::new(move |data: &[u8]| {
        // Try to deserialize as NetworkPacket first - if it works AND has a valid type_name,
        // send it directly. This handles the case where SyncContext::send() already created a NetworkPacket.
        let config = bincode::config::standard();
        let is_network_packet = bincode::serde::decode_from_slice::<NetworkPacket, _>(data, config)
            .ok()
            .filter(|(packet, _)| !packet.type_name.is_empty() && packet.type_name.contains("::"));

        if let Some((packet, _)) = is_network_packet {
            #[cfg(target_arch = "wasm32")]
            leptos::logging::log!(
                "[SyncProvider] Sending raw NetworkPacket: type_name={}, data_len={}",
                packet.type_name,
                packet.data.len()
            );
            raw_send(&packet);
        } else {
            // Otherwise, wrap in a SyncClientMessage packet (for subscription requests etc.)
            let packet = NetworkPacket {
                type_name: std::any::type_name::<SyncClientMessage>().to_string(),
                schema_hash: 0, // Schema hash not used for sync messages
                data: data.to_vec(),
            };

            #[cfg(target_arch = "wasm32")]
            leptos::logging::log!(
                "[SyncProvider] Sending SyncClientMessage NetworkPacket: type_name={}, data_len={}",
                packet.type_name,
                packet.data.len()
            );

            raw_send(&packet);
        }
    });

    let open_arc = Arc::new(move || {
        open();
    });

    let close_arc = Arc::new(move || {
        close();
    });

    let ctx = SyncContext::new(
        ready_state.into(),
        last_error.into(),
        send_arc,
        open_arc,
        close_arc,
        registry.clone(),
    );

    // Provide context to children
    provide_context(ctx.clone());

    // Set up message handler
    // We only want to track changes to raw_message, not signals that we update
    // when handling messages. Use untrack() around the handler to prevent
    // reactive cascades.
    Effect::new(move || {
        // Only subscribe to raw_message changes
        let packet_opt = raw_message.get();

        // Untrack everything else to prevent reactive loops
        untrack(|| {
            if let Some(packet) = packet_opt {
                #[cfg(target_arch = "wasm32")]
                leptos::logging::log!(
                    "[SyncProvider] Received NetworkPacket: type_name={}, data_len={}",
                    packet.type_name,
                    packet.data.len()
                );

                // Try to deserialize as SyncServerMessage first
                match bincode::serde::decode_from_slice::<SyncServerMessage, _>(
                    &packet.data,
                    bincode::config::standard(),
                ) {
                    Ok((server_msg, _)) => {
                        #[cfg(target_arch = "wasm32")]
                        leptos::logging::log!("[SyncProvider] Successfully deserialized SyncServerMessage");

                        handle_server_message(&ctx, server_msg, &last_error);
                    }
                    Err(_) => {
                        // Not a SyncServerMessage - check if it's a ResponseInternal
                        if packet.type_name.contains("ResponseInternal<") {
                            // This is a response to a request - extract response_id and route it
                            #[cfg(target_arch = "wasm32")]
                            leptos::logging::log!(
                                "[SyncProvider] Received ResponseInternal: type_name={}",
                                packet.type_name
                            );

                            // The ResponseInternal struct has: { response_id: u64, response: T }
                            // We need to extract the response_id from the beginning of the data
                            // bincode with standard() uses variable-length integer encoding
                            if let Ok((response_id, bytes_read)) = bincode::serde::decode_from_slice::<u64, _>(
                                &packet.data,
                                bincode::config::standard()
                            ) {
                                // The actual response data starts after the varint-encoded u64
                                let response_bytes = packet.data[bytes_read..].to_vec();

                                #[cfg(target_arch = "wasm32")]
                                leptos::logging::log!(
                                    "[SyncProvider] Routing response_id={} with {} bytes (header was {} bytes)",
                                    response_id,
                                    response_bytes.len(),
                                    bytes_read
                                );

                                ctx.handle_request_response(response_id, response_bytes);
                            } else {
                                #[cfg(target_arch = "wasm32")]
                                leptos::logging::warn!(
                                    "[SyncProvider] Failed to decode response_id from {} bytes",
                                    packet.data.len()
                                );
                            }
                        } else {
                            // Treat as arbitrary Pl3xusMessage
                            #[cfg(target_arch = "wasm32")]
                            leptos::logging::log!(
                                "[SyncProvider] Not a SyncServerMessage, routing as Pl3xusMessage: type_name={}",
                                packet.type_name
                            );

                            ctx.handle_incoming_message(packet.type_name.clone(), packet.data.clone());
                        }
                    }
                }
            }
        });
    });

    // Render children
    children()
}

/// Handle incoming server messages.
fn handle_server_message(
    ctx: &SyncContext,
    msg: SyncServerMessage,
    last_error: &RwSignal<Option<SyncError>>,
) {
    match msg {
        SyncServerMessage::Welcome(welcome) => {
            // Store our connection ID so we can compare with EntityControl
            // Use try_update_untracked + notify to avoid reactive graph issues
            // when updating signals inside Effects (per research/LESSONS_LEARNED.md)
            #[cfg(target_arch = "wasm32")]
            leptos::logging::log!("Received Welcome message with connection ID: {:?}", welcome.connection_id);
            ctx.my_connection_id.try_update_untracked(|id| *id = Some(welcome.connection_id));
            ctx.my_connection_id.notify();
        }
        SyncServerMessage::SyncBatch(batch) => {
            // Process each sync item in the batch
            for item in batch.items {
                if let Err(e) = handle_sync_item(ctx, item) {
                    // Use try_update_untracked + notify to avoid reactive graph issues
                    last_error.try_update_untracked(|err| *err = Some(e));
                    last_error.notify();
                }
            }
        }
        SyncServerMessage::MutationResponse(response) => {
            // Handle mutation response
            ctx.handle_mutation_response(&response);
        }
        SyncServerMessage::QueryResponse(_response) => {
            // TODO: Handle query responses when we implement queries
        }
    }
}

/// Handle a single sync item.
fn handle_sync_item(
    ctx: &SyncContext,
    item: pl3xus_sync::SyncItem,
) -> Result<(), SyncError> {
    use pl3xus_sync::SyncItem;

    #[cfg(target_arch = "wasm32")]
    let is_snapshot = matches!(&item, SyncItem::Snapshot { .. });

    match item {
        SyncItem::Snapshot {
            subscription_id: _,
            entity,
            component_type,
            value,
        } | SyncItem::Update {
            subscription_id: _,
            entity,
            component_type,
            value,
        } => {
            let entity_id = entity.bits;

            // Log for debugging
            #[cfg(target_arch = "wasm32")]
            {
                leptos::logging::log!(
                    "[SyncProvider] Received {} for entity {} component {} ({} bytes)",
                    if is_snapshot { "Snapshot" } else { "Update" },
                    entity_id,
                    component_type,
                    value.len()
                );
            }

            // Update the component_data signal with raw bytes
            // The Effect in subscribe_component will deserialize and update typed signals
            // Use try_update_untracked + notify to avoid reactive graph issues
            ctx.component_data.try_update_untracked(|data| {
                data.insert((entity_id, component_type.clone()), value);
            });
            ctx.component_data.notify();

            Ok(())
        }
        SyncItem::ComponentRemoved {
            subscription_id: _,
            entity,
            component_type,
        } => {
            let entity_id = entity.bits;

            #[cfg(target_arch = "wasm32")]
            {
                leptos::logging::log!(
                    "[SyncProvider] Component {} removed from entity {}",
                    component_type,
                    entity_id
                );
            }

            // Remove the component from component_data
            // Use try_update_untracked + notify to avoid reactive graph issues
            ctx.component_data.try_update_untracked(|data| {
                data.remove(&(entity_id, component_type.clone()));
            });
            ctx.component_data.notify();

            Ok(())
        }
        SyncItem::EntityRemoved {
            subscription_id: _,
            entity,
        } => {
            let entity_id = entity.bits;

            #[cfg(target_arch = "wasm32")]
            {
                leptos::logging::log!(
                    "[SyncProvider] Entity {} removed",
                    entity_id
                );
            }

            // Remove all components for this entity
            // Use try_update_untracked + notify to avoid reactive graph issues
            ctx.component_data.try_update_untracked(|data| {
                data.retain(|(eid, _), _| *eid != entity_id);
            });
            ctx.component_data.notify();

            Ok(())
        }
    }
}

