//! DevtoolsSync - Leptos-reactive wrapper for DevTools sync functionality
//!
//! This module provides a reactive wrapper for DevTools that uses ClientTypeRegistry
//! for JSON conversion and mutation support.

use leptos::prelude::*;
use reactive_graph::traits::{Get, Update};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;

use crate::client_type_registry::ClientTypeRegistry;

use pl3xus_sync::{
    MutateComponent,
    MutationResponse,
    MutationStatus,
    SerializableEntity,
    SyncClientMessage,
    SyncServerMessage,
};

/// Mutation state for tracking client-side mutation requests.
#[derive(Clone, Debug)]
pub struct MutationState {
    pub request_id: u64,
    pub status: Option<MutationStatus>,
    pub message: Option<String>,
}

impl MutationState {
    pub fn new_pending(request_id: u64) -> Self {
        Self {
            request_id,
            status: None,
            message: None,
        }
    }
}

/// Leptos-reactive sync client for DevTools.
///
/// This provides mutation tracking and JSON-based component editing
/// using `ClientTypeRegistry` for serialization/deserialization.
#[derive(Clone)]
pub struct DevtoolsSync {
    send: Arc<dyn Fn(SyncClientMessage) + Send + Sync>,
    registry: Arc<ClientTypeRegistry>,
    mutations: RwSignal<HashMap<u64, MutationState>>,
    next_request_id: Arc<std::sync::Mutex<u64>>,
}

/// General-purpose sync hook for wiring the pl3xus_sync wire protocol
/// into an arbitrary transport (typically a WebSocket using pl3xus's
/// binary codec).
///
/// The `send` closure is responsible for serializing and transmitting
/// `SyncClientMessage` values. This keeps the devtools crate agnostic of
/// any particular WebSocket or HTTP client implementation.
///
/// The `registry` is used to serialize mutations from JSON back to the
/// concrete component types expected by the server.
pub fn use_sync(
    send: impl Fn(SyncClientMessage) + Send + Sync + 'static,
    registry: Arc<ClientTypeRegistry>,
) -> DevtoolsSync {
    DevtoolsSync {
        send: Arc::new(send),
        registry,
        mutations: RwSignal::new(HashMap::new()),
        next_request_id: Arc::new(std::sync::Mutex::new(1)),
    }
}

impl DevtoolsSync {
    /// Send a raw `SyncClientMessage` without any local bookkeeping.
    ///
    /// This is useful for subscription management or other operations
    /// that don't need per-request client-side tracking.
    pub fn send_raw(&self, message: SyncClientMessage) {
        (self.send)(message);
    }

    /// Read-only view of all tracked mutations keyed by `request_id`.
    pub fn mutations(&self) -> RwSignal<HashMap<u64, MutationState>> {
        self.mutations
    }

    /// Convenience accessor for a single mutation state, if known.
    pub fn mutation_state(&self, request_id: u64) -> Option<MutationState> {
        self.mutations.get().get(&request_id).cloned()
    }

    /// Queue a new mutation for `(entity, component_type)` with the
    /// provided JSON value. Returns the generated `request_id` that will
    /// be echoed back by the server in its `MutationResponse`.
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

                // Track in reactive signal for Leptos
                self.mutations.update(|map| {
                    map.insert(request_id, MutationState::new_pending(request_id));
                });

                request_id
            }
            Err(_err) => {
                // Track failed mutation
                self.mutations.update(|map| {
                    map.insert(request_id, MutationState {
                        request_id,
                        status: Some(MutationStatus::InternalError),
                        message: Some("Serialization failed".to_string()),
                    });
                });

                request_id
            }
        }
    }

    /// Handle a server-side message, updating mutation state when a
    /// `MutationResponse` is observed.
    pub fn handle_server_message(&self, message: &SyncServerMessage) {
        // Update reactive state for MutationResponse
        if let SyncServerMessage::MutationResponse(response) = message {
            if let Some(request_id) = response.request_id {
                self.mutations.update(|map| {
                    if let Some(state) = map.get_mut(&request_id) {
                        state.status = Some(response.status.clone());
                        state.message = response.message.clone();
                    }
                });
            }
        }
    }

    /// Helper to handle a `MutationResponse` directly, for cases where
    /// the transport layer already demultiplexes server messages.
    pub fn handle_mutation_response(&self, response: &MutationResponse) {
        if let Some(request_id) = response.request_id {
            self.mutations.update(|map| {
                if let Some(state) = map.get_mut(&request_id) {
                    state.status = Some(response.status.clone());
                    state.message = response.message.clone();
                }
            });
        }
    }
}

