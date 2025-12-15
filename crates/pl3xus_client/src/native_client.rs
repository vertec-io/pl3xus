#![allow(deprecated)]
#![allow(dead_code)]

//! Native client utilities for pl3xus_sync (DEPRECATED).
//!
//! **⚠️ DEPRECATED**: This module is deprecated and will be removed in a future version.
//!
//! This module provides a non-reactive `SyncClient` for native Rust applications
//! (non-Leptos, non-WASM) that want to connect to an `pl3xus_sync` server.
//!
//! ## Deprecation Notice
//!
//! This API is deprecated because:
//! - It's not currently used in any examples or production code
//! - The base `pl3xus` crate provides better Bevy-to-Bevy networking
//! - Future versions will provide a dedicated Bevy-to-Bevy sync engine
//!
//! ## Migration Path
//!
//! - **For Bevy clients**: Use the base `pl3xus` crate with `TcpProvider` or `WebSocketProvider`
//! - **For WASM/Leptos clients**: Use `pl3xus_client::SyncProvider` and hooks
//! - **For future Bevy-to-Bevy sync**: Wait for the upcoming Bevy sync engine
//!
//! ## Current Use Cases
//!
//! This module is only useful if you need to:
//! - Connect a non-Bevy, non-Leptos Rust application to an `pl3xus_sync` server
//! - Build a custom client that uses the `pl3xus_sync` protocol
//!
//! For most use cases, you should use one of the alternatives above.

use std::collections::HashMap;
use std::sync::Arc;
use serde_json::Value as JsonValue;

use crate::client_type_registry::ClientTypeRegistry;

use pl3xus_sync::{
    MutateComponent,
    MutationResponse,
    MutationStatus,
    SerializableEntity,
    SyncClientMessage,
    SyncServerMessage,
};

/// Per-request mutation state tracked on the client.
#[derive(Clone, Debug)]
pub struct NativeMutationState {
    pub request_id: u64,
    pub status: Option<MutationStatus>,
    pub message: Option<String>,
}

impl NativeMutationState {
    pub fn new_pending(request_id: u64) -> Self {
        Self {
            request_id,
            status: None,
            message: None,
        }
    }
}

/// Native client for pl3xus_sync (DEPRECATED).
///
/// **⚠️ DEPRECATED**: Use base `pl3xus` for Bevy-to-Bevy networking or
/// `pl3xus_client::SyncProvider` for WASM/Leptos clients.
///
/// This provides a non-reactive API for native Rust applications to connect
/// to an `pl3xus_sync` server. It is transport-agnostic - you provide a
/// `send` function that handles the actual transmission of messages.
///
/// # Example
///
/// ```rust,ignore
/// use pl3xus_client::{NativeSyncClient, ClientTypeRegistry};
///
/// let registry = ClientTypeRegistry::builder()
///     .register::<MyComponent>()
///     .build();
///
/// let client = NativeSyncClient::new(
///     |msg| { /* send msg over websocket */ },
///     registry
/// );
///
/// // Send a mutation
/// let request_id = client.mutate(entity, "MyComponent", json_value);
///
/// // Later, when you receive a server message:
/// client.handle_server_message(&server_msg);
/// ```
#[deprecated(
    since = "0.1.0",
    note = "Use base `pl3xus` for Bevy-to-Bevy networking or `pl3xus_client::SyncProvider` for WASM/Leptos clients"
)]
pub struct NativeSyncClient {
    send: Arc<dyn Fn(SyncClientMessage) + Send + Sync>,
    next_request_id: std::sync::Mutex<u64>,
    mutations: std::sync::Mutex<HashMap<u64, NativeMutationState>>,
    registry: Arc<ClientTypeRegistry>,
}

impl NativeSyncClient {
    /// Create a new `NativeSyncClient` with the given send function and type registry.
    ///
    /// The `send` function is responsible for serializing and transmitting
    /// `SyncClientMessage` values over the wire (typically via WebSocket).
    pub fn new(
        send: impl Fn(SyncClientMessage) + Send + Sync + 'static,
        registry: Arc<ClientTypeRegistry>,
    ) -> Self {
        Self {
            send: Arc::new(send),
            next_request_id: std::sync::Mutex::new(1),
            mutations: std::sync::Mutex::new(HashMap::new()),
            registry,
        }
    }

    /// Send a raw `SyncClientMessage` without any local bookkeeping.
    ///
    /// This is useful for subscription management or other operations
    /// that don't need per-request client-side tracking.
    pub fn send_raw(&self, message: SyncClientMessage) {
        (self.send)(message);
    }

    /// Get a snapshot of all tracked mutations.
    pub fn mutations(&self) -> HashMap<u64, NativeMutationState> {
        self.mutations.lock().unwrap().clone()
    }

    /// Get the state of a specific mutation by request_id.
    pub fn mutation_state(&self, request_id: u64) -> Option<NativeMutationState> {
        self.mutations.lock().unwrap().get(&request_id).cloned()
    }

    /// Queue a new mutation for `(entity, component_type)` with the
    /// provided JSON value. Returns the generated `request_id` that will
    /// be echoed back by the server in its `MutationResponse`.
    ///
    /// # Arguments
    /// - `entity`: The entity to mutate (use `SerializableEntity::DANGLING` to spawn a new entity)
    /// - `component_type`: The component type name (e.g., "RobotPosition")
    /// - `value`: The component value as a JSON object
    ///
    /// # Returns
    /// The request_id that can be used to track the mutation status.
    pub fn mutate(
        &self,
        entity: SerializableEntity,
        component_type: impl Into<String>,
        value: JsonValue,
    ) -> u64 {
        let component_type = component_type.into();

        // Generate request ID
        let request_id = {
            let mut id = self.next_request_id.lock().unwrap();
            let current = *id;
            *id += 1;
            current
        };

        // Serialize JSON to bincode using registry
        match self.registry.serialize_from_json(&component_type, &value) {
            Ok(value_bytes) => {
                // Send mutation message
                let message = SyncClientMessage::Mutate(MutateComponent {
                    request_id: Some(request_id),
                    entity,
                    component_type,
                    value: value_bytes,
                });
                (self.send)(message);

                // Track in mutation state
                self.mutations.lock().unwrap().insert(
                    request_id,
                    NativeMutationState::new_pending(request_id),
                );

                request_id
            }
            Err(err) => {
                // Track failed mutation
                self.mutations.lock().unwrap().insert(
                    request_id,
                    NativeMutationState {
                        request_id,
                        status: Some(MutationStatus::InternalError),
                        message: Some(format!("Serialization failed: {:?}", err)),
                    },
                );

                request_id
            }
        }
    }

    /// Handle a server-side message, updating mutation state when a
    /// `MutationResponse` is observed.
    pub fn handle_server_message(&self, message: &SyncServerMessage) {
        if let SyncServerMessage::MutationResponse(response) = message {
            self.handle_mutation_response(response);
        }
    }

    /// Handle a `MutationResponse` directly, for cases where
    /// the transport layer already demultiplexes server messages.
    pub fn handle_mutation_response(&self, response: &MutationResponse) {
        if let Some(request_id) = response.request_id {
            let mut mutations = self.mutations.lock().unwrap();
            mutations
                .entry(request_id)
                .and_modify(|state| {
                    state.status = Some(response.status.clone());
                    state.message = response.message.clone();
                })
                .or_insert_with(|| NativeMutationState {
                    request_id,
                    status: Some(response.status.clone()),
                    message: response.message.clone(),
                });
        }
    }

    /// Get a reference to the component type registry.
    pub fn registry(&self) -> &Arc<ClientTypeRegistry> {
        &self.registry
    }
}

impl Clone for NativeSyncClient {
    fn clone(&self) -> Self {
        Self {
            send: Arc::clone(&self.send),
            next_request_id: std::sync::Mutex::new(*self.next_request_id.lock().unwrap()),
            mutations: std::sync::Mutex::new(self.mutations.lock().unwrap().clone()),
            registry: Arc::clone(&self.registry),
        }
    }
}


