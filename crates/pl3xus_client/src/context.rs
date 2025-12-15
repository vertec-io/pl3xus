use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};

use leptos::prelude::*;
use leptos_use::core::ConnectionReadyState;

use crate::client_type_registry::ClientTypeRegistry;
use crate::error::SyncError;
use crate::traits::SyncComponent;
use pl3xus_sync::{
    MutateComponent, MutationResponse, MutationStatus, SerializableEntity, SubscriptionRequest,
    UnsubscribeRequest, SyncClientMessage,
};

#[cfg(feature = "stores")]
use reactive_stores::Store;

/// Connection control interface exposed to components.
///
/// This allows components to manually control the WebSocket connection.
#[derive(Clone)]
pub struct SyncConnection {
    /// Current connection state
    pub ready_state: Signal<ConnectionReadyState>,
    /// Open the WebSocket connection
    pub open: Arc<dyn Fn() + Send + Sync>,
    /// Close the WebSocket connection
    pub close: Arc<dyn Fn() + Send + Sync>,
}

/// Per-request mutation state tracked on the client.
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

/// Context providing access to the sync client.
///
/// This context is provided by `SyncProvider` and consumed by hooks like
/// `use_sync_component`. It manages subscription lifecycle, caching, and
/// message routing.
#[derive(Clone)]
pub struct SyncContext {
    /// Current connection state
    pub ready_state: Signal<ConnectionReadyState>,
    /// Last error that occurred
    pub last_error: Signal<Option<SyncError>>,
    /// Function to send messages to the server
    send: Arc<dyn Fn(&[u8]) + Send + Sync>,
    /// Function to open the connection
    open: Arc<dyn Fn() + Send + Sync>,
    /// Function to close the connection
    close: Arc<dyn Fn() + Send + Sync>,
    /// Type registry for deserialization
    registry: Arc<ClientTypeRegistry>,
    /// Cache of signals for each (TypeId, params) pair
    /// Uses Weak references to allow garbage collection
    signal_cache: Arc<Mutex<HashMap<(TypeId, String), Weak<dyn Any + Send + Sync>>>>,
    /// Subscription tracking: component_type -> (subscription_id, ref_count)
    subscriptions: Arc<Mutex<HashMap<String, (u64, usize)>>>,
    /// Next subscription ID
    next_subscription_id: Arc<Mutex<u64>>,
    /// Raw component data storage: (entity_id, component_name) -> raw bytes
    /// This is the central storage that handle_sync_item updates
    /// Effects in subscribe_component watch this and deserialize to typed signals
    pub(crate) component_data: RwSignal<HashMap<(u64, String), Vec<u8>>>,
    /// Mutation state tracking: request_id -> MutationState
    /// This is reactive so components can watch mutation status
    pub(crate) mutations: RwSignal<HashMap<u64, MutationState>>,
    /// Next mutation request ID
    next_request_id: Arc<Mutex<u64>>,
    /// Incoming message data storage: type_name -> raw bytes
    /// This stores arbitrary Pl3xusMessage types (not component sync)
    /// Effects in subscribe_message watch this and deserialize to typed signals
    pub(crate) incoming_messages: RwSignal<HashMap<String, RwSignal<Vec<u8>>>>,
}

impl SyncContext {
    /// Create a new SyncContext.
    ///
    /// This is typically called by `SyncProvider`, not by user code.
    pub fn new(
        ready_state: Signal<ConnectionReadyState>,
        last_error: Signal<Option<SyncError>>,
        send: Arc<dyn Fn(&[u8]) + Send + Sync>,
        open: Arc<dyn Fn() + Send + Sync>,
        close: Arc<dyn Fn() + Send + Sync>,
        registry: Arc<ClientTypeRegistry>,
    ) -> Self {
        Self {
            ready_state,
            last_error,
            send,
            open,
            close,
            registry,
            signal_cache: Arc::new(Mutex::new(HashMap::new())),
            subscriptions: Arc::new(Mutex::new(HashMap::new())),
            next_subscription_id: Arc::new(Mutex::new(0)),
            component_data: RwSignal::new(HashMap::new()),
            mutations: RwSignal::new(HashMap::new()),
            next_request_id: Arc::new(Mutex::new(0)),
            incoming_messages: RwSignal::new(HashMap::new()),
        }
    }

    /// Get connection control interface.
    pub fn connection(&self) -> SyncConnection {
        SyncConnection {
            ready_state: self.ready_state,
            open: self.open.clone(),
            close: self.close.clone(),
        }
    }

    /// Handle an incoming message (non-sync message).
    ///
    /// This is called by the provider when it receives a NetworkPacket that is not
    /// a SyncServerMessage. The message bytes are stored by type_name and routed
    /// to any active subscriptions.
    pub(crate) fn handle_incoming_message(&self, type_name: String, data: Vec<u8>) {
        #[cfg(target_arch = "wasm32")]
        leptos::logging::log!(
            "[SyncContext] Routing incoming message: type_name={}, data_len={}",
            type_name,
            data.len()
        );

        // Check if we already have a signal for this type_name
        if let Some(signal) = self.incoming_messages.get().get(&type_name) {
            // Update existing signal
            signal.set(data);
        } else {
            // Create new signal and insert into map
            let new_signal = RwSignal::new(data);
            self.incoming_messages.update(|map| {
                map.insert(type_name, new_signal);
            });
        }
    }

    /// Subscribe to a component type.
    ///
    /// This returns a signal containing a HashMap of entity_id -> component.
    /// Multiple calls with the same type will return the same signal (deduplication).
    ///
    /// The subscription is automatically managed:
    /// - Sends SubscriptionRequest when first component subscribes
    /// - Returns cached signal for subsequent subscriptions
    /// - Sends UnsubscribeRequest when last component unsubscribes
    pub fn subscribe_component<T: SyncComponent + Clone + Default>(
        &self,
    ) -> ReadSignal<HashMap<u64, T>> {
        let component_name = T::component_name();
        let type_id = TypeId::of::<T>();
        let cache_key = (type_id, String::new()); // Empty string for no params

        // Try to get existing signal from cache
        {
            let cache = self.signal_cache.lock().unwrap();
            if let Some(weak_signal) = cache.get(&cache_key) {
                if let Some(strong_signal) = weak_signal.upgrade() {
                    if let Some(signal) = strong_signal.downcast_ref::<Arc<RwSignal<HashMap<u64, T>>>>() {
                        // Increment ref count (but don't send subscription request - already subscribed)
                        self.increment_subscription(component_name);

                        // Set up cleanup on unmount
                        let ctx = self.clone();
                        let component_name_owned = component_name.to_string();
                        on_cleanup(move || {
                            if let Some(subscription_id) = ctx.decrement_subscription(&component_name_owned) {
                                ctx.send_unsubscribe_request(subscription_id);
                            }
                        });

                        return signal.read_only();
                    }
                }
            }
        }

        // Create new signal
        let signal = RwSignal::new(HashMap::new());
        let signal_arc = Arc::new(signal);

        // Cache the signal
        {
            let mut cache = self.signal_cache.lock().unwrap();
            cache.insert(
                cache_key,
                Arc::downgrade(&(signal_arc.clone() as Arc<dyn Any + Send + Sync>)),
            );
        }

        // Increment ref count and send subscription request if this is the first subscription
        let is_first = self.increment_subscription(component_name);
        if is_first {
            // Set up an Effect to send the subscription request when the WebSocket is open
            let ctx = self.clone();
            let component_name_owned = component_name.to_string();
            let ready_state = self.ready_state;

            Effect::new(move |_| {
                if ready_state.get() == ConnectionReadyState::Open {
                    #[cfg(target_arch = "wasm32")]
                    leptos::logging::log!(
                        "[SyncContext] WebSocket is open, sending subscription request for '{}'",
                        component_name_owned
                    );

                    ctx.send_subscription_request(&component_name_owned, None);
                }
            });
        }

        // Set up Effect to watch component_data and deserialize to typed signal
        // This is the Meteorite pattern: raw bytes -> Effect -> typed signal
        let component_data = self.component_data;
        let registry = self.registry.clone();
        let component_name_str = component_name.to_string();
        let signal_clone = signal;

        Effect::new(move |_| {
            let data_map = component_data.get();
            let mut typed_map = HashMap::new();

            #[cfg(target_arch = "wasm32")]
            leptos::logging::log!(
                "[SyncContext] Effect triggered for component '{}', data_map has {} entries",
                component_name_str,
                data_map.len()
            );

            // Iterate through all entities and deserialize components of type T
            for ((entity_id, comp_name), bytes) in data_map.iter() {
                if comp_name == &component_name_str {
                    #[cfg(target_arch = "wasm32")]
                    leptos::logging::log!(
                        "[SyncContext] Found matching component '{}' for entity {}, {} bytes",
                        comp_name,
                        entity_id,
                        bytes.len()
                    );

                    // Deserialize the component
                    match registry.deserialize::<T>(comp_name, bytes) {
                        Ok(component) => {
                            #[cfg(target_arch = "wasm32")]
                            leptos::logging::log!(
                                "[SyncContext] Successfully deserialized {} for entity {}",
                                comp_name,
                                entity_id
                            );
                            typed_map.insert(*entity_id, component);
                        }
                        Err(err) => {
                            #[cfg(target_arch = "wasm32")]
                            leptos::logging::warn!(
                                "[SyncContext] Failed to deserialize {} for entity {}: {:?}",
                                comp_name,
                                entity_id,
                                err
                            );
                        }
                    }
                }
            }

            #[cfg(target_arch = "wasm32")]
            leptos::logging::log!(
                "[SyncContext] Setting signal for '{}' with {} entities",
                component_name_str,
                typed_map.len()
            );

            // Update the typed signal
            signal_clone.set(typed_map);
        });

        // Set up cleanup on unmount
        let ctx = self.clone();
        let component_name_owned = component_name.to_string();
        on_cleanup(move || {
            if let Some(subscription_id) = ctx.decrement_subscription(&component_name_owned) {
                ctx.send_unsubscribe_request(subscription_id);
            }
        });

        signal_clone.read_only()
    }

    /// Subscribe to a component type and return a reactive Store.
    ///
    /// This method provides fine-grained reactivity using the `reactive_stores` crate.
    /// Unlike signals which are atomic, stores allow reactive access to individual fields
    /// of the component data structure.
    ///
    /// # Type Parameters
    ///
    /// - `T`: The component type to subscribe to. Must implement `SyncComponent`.
    ///
    /// # Returns
    ///
    /// Returns a `Store<HashMap<u64, T>>` that reactively updates when component data changes.
    /// Individual entity fields can be accessed reactively using store field accessors.
    ///
    /// # Subscription Management
    ///
    /// - Automatically sends subscription request on first call for this component type
    /// - Shares subscription with other components using the same type
    /// - Automatically unsubscribes when the last component unmounts
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use pl3xus_client::use_sync_component_store;
    /// use reactive_stores::Store;
    ///
    /// #[component]
    /// fn AppView() -> impl IntoView {
    ///     let positions = use_sync_component_store::<Position>();
    ///
    ///     // Access individual entity fields reactively
    ///     view! {
    ///         <For
    ///             each=move || positions.read().keys().copied().collect::<Vec<_>>()
    ///             key=|id| *id
    ///             let:entity_id
    ///         >
    ///             {move || {
    ///                 // Fine-grained reactivity: only updates when this entity's position changes
    ///                 let pos = positions.read().get(&entity_id).cloned();
    ///                 view! { <div>{format!("{:?}", pos)}</div> }
    ///             }}
    ///         </For>
    ///     }
    /// }
    /// ```
    #[cfg(feature = "stores")]
    pub fn subscribe_component_store<T>(&self) -> Store<HashMap<u64, T>>
    where
        T: SyncComponent + Clone + Default + 'static,
    {
        let component_name = T::component_name();
        let type_id = TypeId::of::<T>();
        let cache_key = (type_id, component_name.to_string());

        // Check if we already have a store for this component type
        {
            let cache = self.signal_cache.lock().unwrap();
            if let Some(weak_store) = cache.get(&cache_key) {
                if let Some(arc_store) = weak_store.upgrade() {
                    // Downcast to Store<HashMap<u64, T>>
                    if let Ok(store) = arc_store.downcast::<Store<HashMap<u64, T>>>() {
                        // Increment ref count
                        self.increment_subscription(component_name);

                        // Set up cleanup
                        let ctx = self.clone();
                        let component_name_owned = component_name.to_string();
                        on_cleanup(move || {
                            if let Some(subscription_id) = ctx.decrement_subscription(&component_name_owned) {
                                ctx.send_unsubscribe_request(subscription_id);
                            }
                        });

                        return (*store).clone();
                    }
                }
            }
        }

        // Create a new store
        let store = Store::new(HashMap::<u64, T>::new());
        let store_clone = store.clone();

        // Cache the store with a weak reference
        {
            let mut cache = self.signal_cache.lock().unwrap();
            cache.insert(
                cache_key,
                Arc::downgrade(&(Arc::new(store.clone()) as Arc<dyn Any + Send + Sync>)),
            );
        }

        // Increment subscription ref count and send subscription request if needed
        let is_first_subscription = self.increment_subscription(component_name);
        if is_first_subscription {
            self.send_subscription_request(component_name, None);
        }

        // Set up an Effect to watch component_data and update the store
        let component_data = self.component_data;
        let registry = self.registry.clone();
        let component_name_str = component_name.to_string();

        Effect::new(move |_| {
            let data_map = component_data.get();
            let mut typed_map = HashMap::new();

            for ((entity_id, comp_name), bytes) in data_map.iter() {
                if comp_name == &component_name_str {
                    match registry.deserialize::<T>(comp_name, bytes) {
                        Ok(component) => {
                            typed_map.insert(*entity_id, component);
                        }
                        Err(_err) => {
                            #[cfg(target_arch = "wasm32")]
                            leptos::logging::warn!(
                                "[SyncContext] Failed to deserialize {} for entity {}: {:?}",
                                comp_name,
                                entity_id,
                                _err
                            );
                        }
                    }
                }
            }

            // Update the store
            store_clone.write().clone_from(&typed_map);
        });

        // Set up cleanup on unmount
        let ctx = self.clone();
        let component_name_owned = component_name.to_string();
        on_cleanup(move || {
            if let Some(subscription_id) = ctx.decrement_subscription(&component_name_owned) {
                ctx.send_unsubscribe_request(subscription_id);
            }
        });

        store
    }

    /// Increment subscription ref count. Returns true if this is the first subscription.
    fn increment_subscription(&self, component_name: &str) -> bool {
        let mut subs = self.subscriptions.lock().unwrap();
        if let Some((_, ref_count)) = subs.get_mut(component_name) {
            *ref_count += 1;
            false // Not the first subscription
        } else {
            // First subscription - allocate a new subscription ID
            let subscription_id = {
                let mut id = self.next_subscription_id.lock().unwrap();
                let current = *id;
                *id += 1;
                current
            };
            subs.insert(component_name.to_string(), (subscription_id, 1));
            true // First subscription
        }
    }

    /// Decrement subscription ref count. Returns Some(subscription_id) if this was the last subscription.
    fn decrement_subscription(&self, component_name: &str) -> Option<u64> {
        let mut subs = self.subscriptions.lock().unwrap();
        if let Some((subscription_id, ref_count)) = subs.get_mut(component_name) {
            *ref_count -= 1;
            if *ref_count == 0 {
                let id = *subscription_id;
                subs.remove(component_name);
                return Some(id);
            }
        }
        None
    }

    /// Send a subscription request to the server.
    fn send_subscription_request(&self, component_name: &str, entity: Option<SerializableEntity>) {
        // Get the subscription ID for this component type
        let subscription_id = {
            let subs = self.subscriptions.lock().unwrap();
            subs.get(component_name).map(|(id, _)| *id).unwrap_or(0)
        };

        let request = SubscriptionRequest {
            subscription_id,
            component_type: component_name.to_string(),
            entity,
        };

        // Wrap in SyncClientMessage and serialize
        let message = SyncClientMessage::Subscription(request);
        if let Ok(bytes) = bincode::serde::encode_to_vec(&message, bincode::config::standard()) {
            (self.send)(&bytes);
        }
    }

    /// Send an unsubscribe request to the server.
    fn send_unsubscribe_request(&self, subscription_id: u64) {
        let request = UnsubscribeRequest {
            subscription_id,
        };

        // Wrap in SyncClientMessage and serialize
        let message = SyncClientMessage::Unsubscribe(request);
        if let Ok(bytes) = bincode::serde::encode_to_vec(&message, bincode::config::standard()) {
            (self.send)(&bytes);
        }
    }

    /// Handle incoming component update from the server.
    ///
    /// This deserializes the component data and updates the appropriate signal.
    pub fn handle_component_update<T: SyncComponent + Clone>(
        &self,
        entity_id: u64,
        data: &[u8],
    ) -> Result<(), SyncError> {
        let component_name = T::component_name();
        let component: T = self.registry.deserialize(component_name, data)?;

        // Find the signal in the cache and update it
        let type_id = TypeId::of::<T>();
        let cache_key = (type_id, String::new());

        let cache = self.signal_cache.lock().unwrap();
        if let Some(weak_signal) = cache.get(&cache_key) {
            if let Some(strong_signal) = weak_signal.upgrade() {
                if let Some(signal_arc) = strong_signal.downcast_ref::<Arc<RwSignal<HashMap<u64, T>>>>() {
                    signal_arc.update(|map| {
                        map.insert(entity_id, component);
                    });
                }
            }
        }

        Ok(())
    }

    /// Handle component removal from the server.
    pub fn handle_component_removed<T: SyncComponent>(&self, entity_id: u64) {
        let type_id = TypeId::of::<T>();
        let cache_key = (type_id, String::new());

        let cache = self.signal_cache.lock().unwrap();
        if let Some(weak_signal) = cache.get(&cache_key) {
            if let Some(strong_signal) = weak_signal.upgrade() {
                if let Some(signal_arc) = strong_signal.downcast_ref::<Arc<RwSignal<HashMap<u64, T>>>>() {
                    signal_arc.update(|map| {
                        map.remove(&entity_id);
                    });
                }
            }
        }
    }

    /// Send a mutation request to the server.
    ///
    /// This serializes the component and sends a mutation request to the server.
    /// Returns the request_id that can be used to track the mutation status.
    ///
    /// # Arguments
    /// - `entity_id`: The entity to mutate
    /// - `component`: The new component value
    ///
    /// # Returns
    /// The request_id that will be echoed back in the MutationResponse.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use pl3xus_client::{use_sync_context, SyncComponent};
    ///
    /// #[component]
    /// fn UpdatePosition() -> impl IntoView {
    ///     let ctx = use_sync_context();
    ///
    ///     let update_position = move |_| {
    ///         let new_pos = Position { x: 10.0, y: 20.0 };
    ///         let request_id = ctx.mutate(entity_id, new_pos);
    ///         // Track mutation status with use_sync_mutations()
    ///     };
    ///
    ///     view! {
    ///         <button on:click=update_position>"Update Position"</button>
    ///     }
    /// }
    /// ```
    pub fn mutate<T: SyncComponent>(&self, entity_id: u64, component: T) -> u64 {
        let component_name = T::component_name();

        // Generate request ID
        let request_id = {
            let mut next_id = self.next_request_id.lock().unwrap();
            *next_id += 1;
            *next_id
        };

        // Track the pending mutation locally
        self.mutations.update(|map| {
            map.insert(request_id, MutationState::new_pending(request_id));
        });

        // Serialize the component to bincode bytes
        let value_bytes = match bincode::serde::encode_to_vec(&component, bincode::config::standard()) {
            Ok(bytes) => bytes,
            Err(e) => {
                #[cfg(target_arch = "wasm32")]
                leptos::logging::error!(
                    "[SyncContext] Failed to serialize mutation for '{}': {:?}",
                    component_name,
                    e
                );

                // Update mutation state to error
                self.mutations.update(|map| {
                    if let Some(state) = map.get_mut(&request_id) {
                        state.status = Some(MutationStatus::InternalError);
                        state.message = Some(format!("Serialization failed: {}", e));
                    }
                });

                return request_id;
            }
        };

        // Create mutation message
        let msg = SyncClientMessage::Mutate(MutateComponent {
            request_id: Some(request_id),
            entity: SerializableEntity { bits: entity_id },
            component_type: component_name.to_string(),
            value: value_bytes,
        });

        // Serialize and send
        if let Ok(bytes) = bincode::serde::encode_to_vec(&msg, bincode::config::standard()) {
            (self.send)(&bytes);
        } else {
            #[cfg(target_arch = "wasm32")]
            leptos::logging::error!(
                "[SyncContext] Failed to serialize SyncClientMessage for mutation"
            );

            // Update mutation state to error
            self.mutations.update(|map| {
                if let Some(state) = map.get_mut(&request_id) {
                    state.status = Some(MutationStatus::InternalError);
                    state.message = Some("Failed to serialize message".to_string());
                }
            });
        }

        request_id
    }

    /// Handle a mutation response from the server.
    ///
    /// This is called by the provider when a MutationResponse is received.
    /// It updates the reactive mutation state so components can track status.
    pub(crate) fn handle_mutation_response(&self, response: &MutationResponse) {
        if let Some(request_id) = response.request_id {
            self.mutations.update(|map| {
                map.entry(request_id)
                    .and_modify(|state| {
                        state.status = Some(response.status.clone());
                        state.message = response.message.clone();
                    })
                    .or_insert_with(|| MutationState {
                        request_id,
                        status: Some(response.status.clone()),
                        message: response.message.clone(),
                    });
            });

            #[cfg(target_arch = "wasm32")]
            leptos::logging::log!(
                "[SyncContext] Mutation {} completed with status {:?}",
                request_id,
                response.status
            );
        }
    }

    /// Get a read-only signal for tracking mutation states.
    ///
    /// This allows components to reactively watch mutation status.
    pub fn mutations(&self) -> ReadSignal<HashMap<u64, MutationState>> {
        self.mutations.read_only()
    }

    /// Subscribe to arbitrary Pl3xusMessage broadcasts from the server.
    ///
    /// This is for one-way broadcast messages (e.g., notifications, events, video frames)
    /// that are not part of the component sync system.
    ///
    /// # Type Parameters
    ///
    /// - `T`: The message type. Must implement `SyncComponent` (which provides type_name).
    ///
    /// # Returns
    ///
    /// Returns a `ReadSignal<T>` that updates whenever a message of type `T` is received.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[derive(Clone, Default, Serialize, Deserialize)]
    /// struct Notification {
    ///     message: String,
    ///     level: String,
    /// }
    ///
    /// #[component]
    /// fn NotificationBanner() -> impl IntoView {
    ///     let ctx = use_sync_context();
    ///     let notification = ctx.subscribe_message::<Notification>();
    ///
    ///     view! {
    ///         <div>{move || notification.get().message}</div>
    ///     }
    /// }
    /// ```
    pub fn subscribe_message<T>(&self) -> ReadSignal<T>
    where
        T: SyncComponent + Clone + Default + 'static,
    {
        let type_name = T::component_name();
        let (read, write) = signal(T::default());

        // Create a local signal to track the raw bytes
        let incoming_messages = self.incoming_messages;
        let serialized_message = RwSignal::new(Vec::new());

        // Effect to update serialized_message when new messages arrive
        Effect::new(move |_| {
            if let Some(bytes_signal) = incoming_messages.get().get(type_name) {
                serialized_message.set(bytes_signal.get());
            }
        });

        // Effect to deserialize and update the typed signal
        Effect::new(move |_| {
            let bytes = serialized_message.get();
            if !bytes.is_empty() {
                match bincode::serde::decode_from_slice::<T, _>(&bytes, bincode::config::standard()) {
                    Ok((deserialized, _)) => {
                        #[cfg(target_arch = "wasm32")]
                        leptos::logging::log!(
                            "[SyncContext] Deserialized message of type {}",
                            type_name
                        );
                        write.set(deserialized);
                    }
                    Err(_e) => {
                        #[cfg(target_arch = "wasm32")]
                        leptos::logging::warn!(
                            "[SyncContext] Failed to deserialize message of type {}: {:?}",
                            type_name,
                            _e
                        );
                    }
                }
            }
        });

        read
    }

    /// Subscribe to arbitrary Pl3xusMessage broadcasts using a Store.
    ///
    /// This provides fine-grained reactivity for message fields, similar to
    /// `subscribe_component_store` but for broadcast messages.
    ///
    /// # Type Parameters
    ///
    /// - `T`: The message type. Must implement `SyncComponent` (which provides type_name).
    ///
    /// # Returns
    ///
    /// Returns a `Store<T>` that updates whenever a message of type `T` is received.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[derive(Clone, Default, Serialize, Deserialize)]
    /// struct ServerStats {
    ///     cpu_usage: f32,
    ///     memory_usage: f32,
    ///     active_connections: u32,
    /// }
    ///
    /// #[component]
    /// fn StatsDisplay() -> impl IntoView {
    ///     let ctx = use_sync_context();
    ///     let stats = ctx.subscribe_message_store::<ServerStats>();
    ///
    ///     view! {
    ///         <div>
    ///             <p>"CPU: " {move || stats.cpu_usage().get()}</p>
    ///             <p>"Memory: " {move || stats.memory_usage().get()}</p>
    ///         </div>
    ///     }
    /// }
    /// ```
    #[cfg(feature = "stores")]
    pub fn subscribe_message_store<T>(&self) -> Store<T>
    where
        T: SyncComponent + Clone + Default + 'static,
    {
        let type_name = T::component_name();
        let store = Store::new(T::default());
        let store_clone = store.clone();

        // Create a local signal to track the raw bytes
        let incoming_messages = self.incoming_messages;
        let serialized_message = RwSignal::new(Vec::new());

        // Effect to update serialized_message when new messages arrive
        Effect::new(move |_| {
            if let Some(bytes_signal) = incoming_messages.get().get(type_name) {
                serialized_message.set(bytes_signal.get());
            }
        });

        // Effect to deserialize and update the store
        Effect::new(move |_| {
            let bytes = serialized_message.get();
            if !bytes.is_empty() {
                match bincode::serde::decode_from_slice::<T, _>(&bytes, bincode::config::standard()) {
                    Ok((deserialized, _)) => {
                        #[cfg(target_arch = "wasm32")]
                        leptos::logging::log!(
                            "[SyncContext] Deserialized message of type {} for store",
                            type_name
                        );
                        store_clone.update(|value| *value = deserialized);
                    }
                    Err(_e) => {
                        #[cfg(target_arch = "wasm32")]
                        leptos::logging::warn!(
                            "[SyncContext] Failed to deserialize message of type {} for store: {:?}",
                            type_name,
                            _e
                        );
                    }
                }
            }
        });

        store
    }
}

