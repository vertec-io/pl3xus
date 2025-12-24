use std::sync::Arc;

use leptos::prelude::*;
use leptos_use::{use_websocket_with_options, DummyEncoder, UseWebSocketOptions, UseWebSocketReturn};
use pl3xus_common::codec::Pl3xusBincodeCodec;
use pl3xus_common::NetworkPacket;

use crate::client_type_registry::ClientTypeRegistry;
use crate::context::SyncContext;
use crate::error::SyncError;
use pl3xus_sync::{SyncClientMessage, SyncServerMessage};

/// Decode all length-prefixed NetworkPackets from a byte buffer.
/// The server may batch multiple messages into a single WebSocket frame.
/// Each message is prefixed with an 8-byte little-endian length.
fn decode_all_packets(data: &[u8]) -> Vec<NetworkPacket> {
    let mut packets = Vec::new();
    let mut offset = 0;

    while offset + 8 <= data.len() {
        // Read 8-byte length prefix
        let length_bytes: [u8; 8] = match data[offset..offset + 8].try_into() {
            Ok(b) => b,
            Err(_) => break,
        };
        let length = u64::from_le_bytes(length_bytes) as usize;
        offset += 8;

        // Check if we have enough data for this message
        if offset + length > data.len() {
            #[cfg(target_arch = "wasm32")]
            leptos::logging::warn!(
                "[decode_all_packets] Incomplete message: need {} bytes, have {}",
                length,
                data.len() - offset
            );
            break;
        }

        // Decode the NetworkPacket
        match bincode::serde::decode_from_slice::<NetworkPacket, _>(
            &data[offset..offset + length],
            bincode::config::standard(),
        ) {
            Ok((packet, _)) => {
                packets.push(packet);
            }
            Err(_e) => {
                #[cfg(target_arch = "wasm32")]
                leptos::logging::warn!(
                    "[decode_all_packets] Failed to decode packet at offset {}: {:?}",
                    offset,
                    _e
                );
                break;
            }
        }

        offset += length;
    }

    #[cfg(target_arch = "wasm32")]
    if packets.len() > 1 {
        leptos::logging::log!(
            "[decode_all_packets] Decoded {} packets from {} bytes",
            packets.len(),
            data.len()
        );
    }

    packets
}

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

    // Create error signal (before context so it can be captured in closures)
    let last_error = RwSignal::new(None::<SyncError>);

    // Create SyncContext early so we can use it in the on_message_raw_bytes callback
    // We'll set the send/open/close functions after we have them from use_websocket
    let send_fn: StoredValue<Option<Arc<dyn Fn(&NetworkPacket) + Send + Sync>>> = StoredValue::new(None);
    let open_fn: StoredValue<Option<Arc<dyn Fn() + Send + Sync>>> = StoredValue::new(None);
    let close_fn: StoredValue<Option<Arc<dyn Fn() + Send + Sync>>> = StoredValue::new(None);

    // Temporary send function that will be replaced
    let send_arc = Arc::new({
        let send_fn = send_fn.clone();
        move |data: &[u8]| {
            let config = bincode::config::standard();
            let is_network_packet = bincode::serde::decode_from_slice::<NetworkPacket, _>(data, config)
                .ok()
                .filter(|(packet, _)| !packet.type_name.is_empty() && packet.type_name.contains("::"));

            if let Some(send) = send_fn.get_value() {
                if let Some((packet, _)) = is_network_packet {
                    #[cfg(target_arch = "wasm32")]
                    leptos::logging::log!(
                        "[SyncProvider] Sending raw NetworkPacket: type_name={}, data_len={}",
                        packet.type_name,
                        packet.data.len()
                    );
                    send(&packet);
                } else {
                    let packet = NetworkPacket {
                        type_name: std::any::type_name::<SyncClientMessage>().to_string(),
                        schema_hash: 0,
                        data: data.to_vec(),
                    };
                    #[cfg(target_arch = "wasm32")]
                    leptos::logging::log!(
                        "[SyncProvider] Sending SyncClientMessage NetworkPacket: type_name={}, data_len={}",
                        packet.type_name,
                        packet.data.len()
                    );
                    send(&packet);
                }
            }
        }
    });

    let open_arc = Arc::new({
        let open_fn = open_fn.clone();
        move || {
            if let Some(open) = open_fn.get_value() {
                open();
            }
        }
    });

    let close_arc = Arc::new({
        let close_fn = close_fn.clone();
        move || {
            if let Some(close) = close_fn.get_value() {
                close();
            }
        }
    });

    // Create a temporary ready_state signal - we'll update it from the WebSocket
    let ready_state_signal = RwSignal::new(leptos_use::core::ConnectionReadyState::Closed);

    let ctx = SyncContext::new(
        ready_state_signal.into(),
        last_error.into(),
        send_arc,
        open_arc,
        close_arc,
        registry.clone(),
    );

    // Provide context to children early so closures can use it
    provide_context(ctx.clone());

    // Set up WebSocket connection using NetworkPacket wrapper
    // Use on_message_raw_bytes to handle batched messages from the server
    let ctx_for_callback = ctx.clone();
    let UseWebSocketReturn {
        ready_state,
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
            .immediate(auto_connect)
            .on_open(move |_| {
                #[cfg(target_arch = "wasm32")]
                leptos::logging::log!("[SyncProvider] WebSocket opened!");
            })
            .on_error(move |_e| {
                #[cfg(target_arch = "wasm32")]
                leptos::logging::warn!("[SyncProvider] WebSocket error: {:?}", _e);
            })
            .on_message_raw_bytes(Arc::new(move |data: &[u8]| {
                // Decode all packets from the raw bytes (handles batched messages)
                let packets = decode_all_packets(data);

                for packet in packets {
                    #[cfg(target_arch = "wasm32")]
                    leptos::logging::log!(
                        "[SyncProvider] Received NetworkPacket: type_name={}, data_len={}",
                        packet.type_name,
                        packet.data.len()
                    );

                    handle_packet(&ctx_for_callback, &packet, &last_error);
                }
            })),
    );

    // Store the actual send/open/close functions
    send_fn.set_value(Some(Arc::new(move |packet: &NetworkPacket| {
        raw_send(packet);
    })));
    open_fn.set_value(Some(Arc::new(move || {
        open();
    })));
    close_fn.set_value(Some(Arc::new(move || {
        close();
    })));

    // Sync the ready_state from WebSocket to our signal
    Effect::new(move || {
        let state = ready_state.get();
        ready_state_signal.set(state);
    });

    // Render children
    children()
}

/// Handle a single NetworkPacket by routing it to the appropriate handler.
fn handle_packet(
    ctx: &SyncContext,
    packet: &NetworkPacket,
    last_error: &RwSignal<Option<SyncError>>,
) {
    // Check type_name first to determine how to deserialize
    // This prevents misinterpreting ResponseInternal data as SyncServerMessage
    if packet.type_name.contains("SyncServerMessage") {
        // This is a SyncServerMessage - deserialize it
        match bincode::serde::decode_from_slice::<SyncServerMessage, _>(
            &packet.data,
            bincode::config::standard(),
        ) {
            Ok((server_msg, _)) => {
                #[cfg(target_arch = "wasm32")]
                leptos::logging::log!("[SyncProvider] Successfully deserialized SyncServerMessage");

                handle_server_message(ctx, server_msg, last_error);
            }
            Err(_e) => {
                #[cfg(target_arch = "wasm32")]
                leptos::logging::log!(
                    "[SyncProvider] Failed to deserialize SyncServerMessage: {:?}",
                    _e
                );
            }
        }
    } else if packet.type_name.contains("ResponseInternal<") {
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
            "[SyncProvider] Routing as Pl3xusMessage: type_name={}",
            packet.type_name
        );

        ctx.handle_incoming_message(packet.type_name.clone(), packet.data.clone());
    }
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
        SyncServerMessage::QueryInvalidation(invalidation) => {
            // Handle query cache invalidation
            ctx.handle_query_invalidation(&invalidation);
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

