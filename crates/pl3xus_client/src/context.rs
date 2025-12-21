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
    /// This client's own connection ID (set when server sends Welcome message)
    /// Used to determine if we have control by comparing with EntityControl.client_id
    pub my_connection_id: RwSignal<Option<pl3xus_common::ConnectionId>>,
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
    /// We use ArcRwSignal for the inner signals because they are created inside
    /// the provider's Effect and need to outlive that Effect's execution scope.
    /// (See Leptos docs: "Appendix: The Life Cycle of a Signal" - arena-allocated
    /// signals are disposed when their owning Effect re-runs)
    pub(crate) incoming_messages: RwSignal<HashMap<String, ArcRwSignal<Vec<u8>>>>,
    /// Request state tracking: request_id -> RequestState
    /// Tracks pending requests and their responses
    pub(crate) requests: RwSignal<HashMap<u64, RequestState>>,
}

/// State tracking for a single request/response cycle.
#[derive(Clone, Debug)]
pub struct RequestState {
    /// The unique request ID
    pub request_id: u64,
    /// Type name of the expected response
    pub response_type: String,
    /// Current status of the request
    pub status: RequestStatus,
    /// Raw response bytes (if received)
    pub response_bytes: Option<Vec<u8>>,
}

/// Status of a request.
#[derive(Clone, Debug, PartialEq)]
pub enum RequestStatus {
    /// Request is pending (sent, waiting for response)
    Pending,
    /// Response received successfully
    Success,
    /// Request failed (timeout, network error, etc.)
    Error(String),
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
            my_connection_id: RwSignal::new(None),
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
            requests: RwSignal::new(HashMap::new()),
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

    /// Send a raw byte message to the server.
    ///
    /// This allows sending arbitrary data, such as manual NetworkPackets for RPC.
    pub fn send_bytes(&self, bytes: &[u8]) {
        (self.send)(bytes);
    }

    /// Send a typed message to the server.
    ///
    /// This is the ergonomic way to send RPC-style messages. The message is automatically
    /// serialized to a NetworkPacket with proper type information for routing on the server.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use pl3xus_client::use_sync_context;
    ///
    /// #[derive(Serialize, Deserialize, Clone)]
    /// struct InitializeRobot {
    ///     group_mask: Option<u8>,
    /// }
    ///
    /// #[component]
    /// fn Controls() -> impl IntoView {
    ///     let ctx = use_sync_context();
    ///
    ///     let init = move |_| {
    ///         ctx.send(InitializeRobot { group_mask: Some(1) });
    ///     };
    ///
    ///     view! { <button on:click=init>"Initialize"</button> }
    /// }
    /// ```
    pub fn send<T>(&self, message: T)
    where
        T: serde::Serialize + pl3xus_common::Pl3xusMessage,
    {
        use pl3xus_common::NetworkPacket;

        // Serialize the message to bincode
        let data = match bincode::serde::encode_to_vec(&message, bincode::config::standard()) {
            Ok(bytes) => bytes,
            Err(_e) => {
                #[cfg(target_arch = "wasm32")]
                leptos::logging::error!(
                    "[SyncContext::send] Failed to serialize message '{}': {:?}",
                    T::type_name(),
                    _e
                );
                return;
            }
        };

        // Create NetworkPacket with type info for server routing
        let packet = NetworkPacket {
            type_name: T::type_name().to_string(),
            schema_hash: T::schema_hash(),
            data,
        };

        #[cfg(target_arch = "wasm32")]
        leptos::logging::log!(
            "[SyncContext::send] Sending message type: '{}' (hash: 0x{:016x})",
            T::type_name(),
            T::schema_hash()
        );

        // Serialize the packet and send
        match bincode::serde::encode_to_vec(&packet, bincode::config::standard()) {
            Ok(bytes) => {
                #[cfg(target_arch = "wasm32")]
                leptos::logging::log!(
                    "[SyncContext::send] Sending message '{}' ({} bytes)",
                    T::short_name(),
                    bytes.len()
                );
                (self.send)(&bytes);
            }
            Err(_e) => {
                #[cfg(target_arch = "wasm32")]
                leptos::logging::error!(
                    "[SyncContext::send] Failed to serialize packet for '{}': {:?}",
                    T::type_name(),
                    _e
                );
            }
        }
    }

    /// Send a targeted message to the server for a specific entity.
    ///
    /// This wraps the message in a `TargetedMessage<T>` with the entity's bits as the target_id.
    /// On the server, this will be processed by the authorization middleware and converted
    /// to an `AuthorizedMessage<T>` if the client has control of the target entity.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use pl3xus_client::use_sync_context;
    ///
    /// #[derive(Serialize, Deserialize, Clone)]
    /// struct JogCommand {
    ///     axis: u8,
    ///     direction: i8,
    /// }
    ///
    /// #[component]
    /// fn JogControls(entity_bits: u64) -> impl IntoView {
    ///     let ctx = use_sync_context();
    ///
    ///     let jog_x_plus = move |_| {
    ///         ctx.send_targeted(entity_bits, JogCommand { axis: 0, direction: 1 });
    ///     };
    ///
    ///     view! { <button on:click=jog_x_plus>"X+"</button> }
    /// }
    /// ```
    pub fn send_targeted<T>(&self, entity_bits: u64, message: T)
    where
        T: serde::Serialize + pl3xus_common::Pl3xusMessage,
    {
        use pl3xus_common::{NetworkPacket, TargetedMessage};

        // Wrap in TargetedMessage with entity bits as string
        let targeted = TargetedMessage {
            target_id: entity_bits.to_string(),
            message,
        };

        // Serialize the targeted message to bincode
        let data = match bincode::serde::encode_to_vec(&targeted, bincode::config::standard()) {
            Ok(bytes) => bytes,
            Err(_e) => {
                #[cfg(target_arch = "wasm32")]
                leptos::logging::error!(
                    "[SyncContext::send_targeted] Failed to serialize targeted message '{}': {:?}",
                    T::type_name(),
                    _e
                );
                return;
            }
        };

        // Create NetworkPacket with TargetedMessage type info
        let packet = NetworkPacket {
            type_name: TargetedMessage::<T>::name().to_string(),
            schema_hash: T::schema_hash(), // Use inner type's hash for matching
            data,
        };

        #[cfg(target_arch = "wasm32")]
        leptos::logging::log!(
            "[SyncContext::send_targeted] Sending targeted message '{}' to entity {} (hash: 0x{:016x})",
            T::type_name(),
            entity_bits,
            T::schema_hash()
        );

        // Serialize the packet and send
        match bincode::serde::encode_to_vec(&packet, bincode::config::standard()) {
            Ok(bytes) => {
                #[cfg(target_arch = "wasm32")]
                leptos::logging::log!(
                    "[SyncContext::send_targeted] Sending targeted '{}' ({} bytes)",
                    T::short_name(),
                    bytes.len()
                );
                (self.send)(&bytes);
            }
            Err(_e) => {
                #[cfg(target_arch = "wasm32")]
                leptos::logging::error!(
                    "[SyncContext::send_targeted] Failed to serialize packet for '{}': {:?}",
                    T::type_name(),
                    _e
                );
            }
        }
    }

    /// Handle an incoming message (non-sync message).
    ///
    /// This is called by the provider when it receives a NetworkPacket that is not
    /// a SyncServerMessage. The message bytes are stored by type_name and routed
    /// to any active subscriptions.
    pub(crate) fn handle_incoming_message(&self, type_name: String, data: Vec<u8>) {
        // Extract short name from full type name (e.g., "pl3xus_common::ControlResponse" -> "ControlResponse")
        // This matches how SyncComponent::component_name() works
        let short_name = type_name.rsplit("::").next().unwrap_or(&type_name).to_string();

        #[cfg(target_arch = "wasm32")]
        leptos::logging::log!(
            "[SyncContext] Routing incoming message: type_name={}, short_name={}, data_len={}",
            type_name,
            short_name,
            data.len()
        );

        // Check if we already have a signal for this short_name
        // Use get_untracked() to avoid reactive issues when called from Effects
        if let Some(signal) = self.incoming_messages.get_untracked().get(&short_name).cloned() {
            // Update existing ArcRwSignal - these are reference-counted so they don't get disposed
            signal.try_update_untracked(|bytes| *bytes = data.clone());
            signal.notify();
        } else {
            // Create new ArcRwSignal and insert into map
            // We use ArcRwSignal because this code runs inside the provider's Effect,
            // and arena-allocated RwSignals would be disposed when the Effect re-runs.
            // ArcRwSignal uses reference counting for lifecycle management instead.
            let new_signal = ArcRwSignal::new(data);
            self.incoming_messages.try_update_untracked(|map| {
                map.insert(short_name, new_signal);
            });
            self.incoming_messages.notify();
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
        //
        // IMPORTANT: We track the previous raw bytes and only notify subscribers
        // when this specific component type actually changes. This prevents
        // cascading reactivity when unrelated components update.
        let component_data = self.component_data;
        let registry = self.registry.clone();
        let component_name_str = component_name.to_string();
        let signal_clone = signal;

        // Track previous bytes for this component type to detect actual changes
        let prev_bytes: StoredValue<HashMap<u64, Vec<u8>>> = StoredValue::new(HashMap::new());

        Effect::new(move |_| {
            let data_map = component_data.get();
            let mut typed_map = HashMap::new();
            let mut current_bytes: HashMap<u64, Vec<u8>> = HashMap::new();

            // Iterate through all entities and deserialize components of type T
            for ((entity_id, comp_name), bytes) in data_map.iter() {
                if comp_name == &component_name_str {
                    current_bytes.insert(*entity_id, bytes.clone());

                    // Deserialize the component
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

            // Compare with previous bytes to detect if THIS component type actually changed
            let changed = prev_bytes.with_value(|prev| *prev != current_bytes);

            if changed {
                // Update stored bytes for next comparison
                prev_bytes.set_value(current_bytes);

                // Update the typed signal and notify subscribers
                signal_clone.try_update_untracked(|val| *val = typed_map);
                signal_clone.notify();
            }
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
        // Track previous bytes to only update when this specific component changes
        let component_data = self.component_data;
        let registry = self.registry.clone();
        let component_name_str = component_name.to_string();
        let prev_bytes: StoredValue<HashMap<u64, Vec<u8>>> = StoredValue::new(HashMap::new());

        Effect::new(move |_| {
            let data_map = component_data.get();
            let mut typed_map = HashMap::new();
            let mut current_bytes: HashMap<u64, Vec<u8>> = HashMap::new();

            for ((entity_id, comp_name), bytes) in data_map.iter() {
                if comp_name == &component_name_str {
                    current_bytes.insert(*entity_id, bytes.clone());

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

            // Only update the store if this component type actually changed
            let changed = prev_bytes.with_value(|prev| *prev != current_bytes);

            if changed {
                prev_bytes.set_value(current_bytes);
                store_clone.write().clone_from(&typed_map);
            }
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

        // Get a reference to the incoming_messages signal
        let incoming_messages = self.incoming_messages;

        // Track the last bytes we processed to avoid duplicate processing
        // Use StoredValue instead of RwSignal to avoid reactive issues
        let last_bytes_hash = StoredValue::new(0u64);

        // Single effect that watches for new messages and deserializes them
        Effect::new(move |_| {
            // Get the current map (creates reactive dependency on the map itself)
            let messages_map = incoming_messages.get();

            // Check if we have a signal for this message type
            if let Some(bytes_signal) = messages_map.get(type_name) {
                // Get the bytes (creates reactive dependency on the signal)
                let bytes = bytes_signal.get();

                if bytes.is_empty() {
                    return;
                }

                // Simple hash to detect if bytes changed
                let bytes_hash = bytes.iter().fold(0u64, |acc, &b| acc.wrapping_add(b as u64).wrapping_mul(31));

                // Skip if we already processed these exact bytes
                if bytes_hash == last_bytes_hash.get_value() {
                    return;
                }

                // Update the hash BEFORE processing to prevent loops
                last_bytes_hash.set_value(bytes_hash);

                match bincode::serde::decode_from_slice::<T, _>(&bytes, bincode::config::standard()) {
                    Ok((deserialized, _)) => {
                        #[cfg(target_arch = "wasm32")]
                        leptos::logging::log!(
                            "[SyncContext] Deserialized message of type {}",
                            type_name
                        );
                        // Use try_update_untracked + notify to avoid reactive graph issues
                        // when updating signals inside Effects (per research/LESSONS_LEARNED.md)
                        write.try_update_untracked(|val| *val = deserialized);
                        write.notify();
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

        // Get a reference to the incoming_messages signal
        let incoming_messages = self.incoming_messages;

        // Track the last bytes we processed to avoid duplicate processing
        // Use StoredValue instead of RwSignal to avoid reactive issues
        let last_bytes_hash = StoredValue::new(0u64);

        // Single effect that watches for new messages and deserializes them
        Effect::new(move |_| {
            // Get the current map (creates reactive dependency on the map itself)
            let messages_map = incoming_messages.get();

            // Check if we have a signal for this message type
            if let Some(bytes_signal) = messages_map.get(type_name) {
                // Get the bytes (creates reactive dependency on the signal)
                let bytes = bytes_signal.get();

                if bytes.is_empty() {
                    return;
                }

                // Simple hash to detect if bytes changed
                let bytes_hash = bytes.iter().fold(0u64, |acc, &b| acc.wrapping_add(b as u64).wrapping_mul(31));

                // Skip if we already processed these exact bytes
                if bytes_hash == last_bytes_hash.get_value() {
                    return;
                }

                // Update the hash BEFORE processing to prevent loops
                last_bytes_hash.set_value(bytes_hash);

                match bincode::serde::decode_from_slice::<T, _>(&bytes, bincode::config::standard()) {
                    Ok((deserialized, _)) => {
                        #[cfg(target_arch = "wasm32")]
                        leptos::logging::log!(
                            "[SyncContext] Deserialized message of type {} for store",
                            type_name
                        );
                        // Use try_update_untracked + notify to avoid reactive graph issues
                        store_clone.try_update_untracked(|value| *value = deserialized);
                        store_clone.notify();
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

    // ============================================================================
    // Request/Response Methods
    // ============================================================================

    /// Send a request and track its state.
    ///
    /// This sends a request message to the server and returns a request ID that can
    /// be used to track the response. The request is wrapped with a correlation ID
    /// so responses can be matched.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use pl3xus_client::use_sync_context;
    /// use pl3xus_common::RequestMessage;
    ///
    /// #[derive(Clone, Serialize, Deserialize, Debug)]
    /// struct ListRobots;
    ///
    /// impl RequestMessage for ListRobots {
    ///     type ResponseMessage = Vec<RobotInfo>;
    /// }
    ///
    /// #[component]
    /// fn RobotList() -> impl IntoView {
    ///     let ctx = use_sync_context();
    ///     let (request_id, set_request_id) = signal(None::<u64>);
    ///
    ///     let fetch = move |_| {
    ///         let id = ctx.request(ListRobots);
    ///         set_request_id.set(Some(id));
    ///     };
    ///
    ///     view! { <button on:click=fetch>"Fetch Robots"</button> }
    /// }
    /// ```
    pub fn request<R>(&self, request: R) -> u64
    where
        R: pl3xus_common::RequestMessage,
    {
        use pl3xus_common::{NetworkPacket, Pl3xusMessage};
        use serde::{Serialize, Deserialize};

        // Internal wrapper for request with correlation ID
        #[derive(Serialize, Deserialize)]
        struct RequestInternal<T> {
            id: u64,
            request: T,
        }

        // Generate unique request ID
        let request_id = {
            let mut next_id = self.next_request_id.lock().unwrap();
            *next_id += 1;
            *next_id
        };

        // Track pending request
        let response_type = format!("ResponseInternal<{}>", R::ResponseMessage::type_name());
        self.requests.update(|map| {
            map.insert(request_id, RequestState {
                request_id,
                response_type: response_type.clone(),
                status: RequestStatus::Pending,
                response_bytes: None,
            });
        });

        // Wrap request with ID
        let wrapped = RequestInternal {
            id: request_id,
            request,
        };

        // Create NetworkPacket with the RequestInternal type name
        // This matches how pl3xus server expects to receive requests
        let type_name = format!("pl3xus::managers::network_request::RequestInternal<{}>", R::type_name());

        let data = match bincode::serde::encode_to_vec(&wrapped, bincode::config::standard()) {
            Ok(bytes) => bytes,
            Err(_e) => {
                #[cfg(target_arch = "wasm32")]
                leptos::logging::error!("[SyncContext::request] Failed to serialize request: {:?}", _e);

                // Mark as error
                self.requests.update(|map| {
                    if let Some(state) = map.get_mut(&request_id) {
                        state.status = RequestStatus::Error("Serialization failed".to_string());
                    }
                });
                return request_id;
            }
        };

        let packet = NetworkPacket {
            type_name,
            schema_hash: R::schema_hash(),
            data,
        };

        match bincode::serde::encode_to_vec(&packet, bincode::config::standard()) {
            Ok(bytes) => {
                #[cfg(target_arch = "wasm32")]
                leptos::logging::log!(
                    "[SyncContext::request] Sending request '{}' (type_name='{}') with id {} ({} bytes)",
                    R::request_name(),
                    packet.type_name,
                    request_id,
                    bytes.len()
                );
                (self.send)(&bytes);
            }
            Err(_e) => {
                #[cfg(target_arch = "wasm32")]
                leptos::logging::error!("[SyncContext::request] Failed to serialize packet: {:?}", _e);

                self.requests.update(|map| {
                    if let Some(state) = map.get_mut(&request_id) {
                        state.status = RequestStatus::Error("Packet serialization failed".to_string());
                    }
                });
            }
        }

        request_id
    }

    /// Handle a response from the server.
    ///
    /// Called by the provider when a ResponseInternal message is received.
    pub(crate) fn handle_request_response(&self, response_id: u64, response_bytes: Vec<u8>) {
        self.requests.update(|map| {
            if let Some(state) = map.get_mut(&response_id) {
                state.status = RequestStatus::Success;
                state.response_bytes = Some(response_bytes);
            }
        });

        #[cfg(target_arch = "wasm32")]
        leptos::logging::log!("[SyncContext] Request {} received response", response_id);
    }

    /// Get a read-only signal for tracking request states.
    pub fn requests(&self) -> ReadSignal<HashMap<u64, RequestState>> {
        self.requests.read_only()
    }

    /// Get the response for a completed request, deserializing it to the expected type.
    ///
    /// Returns None if the request is still pending or failed.
    pub fn get_response<R>(&self, request_id: u64) -> Option<R::ResponseMessage>
    where
        R: pl3xus_common::RequestMessage,
    {
        let requests = self.requests.get();
        let state = requests.get(&request_id)?;

        if state.status != RequestStatus::Success {
            return None;
        }

        let bytes = state.response_bytes.as_ref()?;

        match bincode::serde::decode_from_slice::<R::ResponseMessage, _>(bytes, bincode::config::standard()) {
            Ok((response, _)) => Some(response),
            Err(_e) => {
                #[cfg(target_arch = "wasm32")]
                leptos::logging::error!(
                    "[SyncContext::get_response] Failed to deserialize response: {:?}",
                    _e
                );
                None
            }
        }
    }

    // ============================================================================
    // Entity-Specific Subscription Methods
    // ============================================================================

    /// Subscribe to a specific entity's component as a signal.
    ///
    /// Unlike `subscribe_component` which returns all entities, this returns
    /// a signal for a single entity's component. The entity ID can be reactive
    /// (a closure), allowing you to switch which entity you're tracking.
    ///
    /// Returns a tuple of:
    /// - `ReadSignal<T>`: The component data (uses `T::default()` when entity doesn't exist)
    /// - `ReadSignal<bool>`: Whether the entity currently exists
    ///
    /// # Type Parameters
    ///
    /// - `T`: The component type to subscribe to. Must implement `SyncComponent`.
    ///
    /// # Arguments
    ///
    /// - `entity_id_fn`: A reactive closure that returns `Option<u64>`. When `None`,
    ///   the component will use default values and `exists` will be `false`.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use pl3xus_client::use_sync_entity_component;
    ///
    /// // Fixed entity ID (singleton pattern)
    /// let (exec, exists) = use_sync_entity_component::<ExecutionState>(|| Some(SYSTEM_ENTITY_ID));
    ///
    /// // Reactive entity ID from signal
    /// let selected_robot: RwSignal<Option<u64>> = ...;
    /// let (position, exists) = use_sync_entity_component::<Position>(move || selected_robot.get());
    ///
    /// // First entity of a type
    /// let entities = use_sync_component::<RobotState>();
    /// let (state, exists) = use_sync_entity_component::<RobotState>(move || {
    ///     entities.get().keys().next().copied()
    /// });
    /// ```
    pub fn subscribe_entity_component<T, F>(
        &self,
        entity_id_fn: F,
    ) -> (ReadSignal<T>, ReadSignal<bool>)
    where
        T: SyncComponent + Clone + Default + 'static,
        F: Fn() -> Option<u64> + Clone + 'static,
    {
        // Subscribe to the underlying component type
        let all_components = self.subscribe_component::<T>();

        // Create signals for the entity-specific data
        let (component_signal, set_component) = signal(T::default());
        let (exists_signal, set_exists) = signal(false);

        // Create an effect that watches both the entity_id and the component data
        let entity_id_fn_clone = entity_id_fn.clone();
        Effect::new(move |_| {
            let maybe_entity_id = entity_id_fn_clone();
            let components = all_components.get();

            match maybe_entity_id {
                Some(entity_id) => {
                    if let Some(component) = components.get(&entity_id) {
                        set_component.set(component.clone());
                        set_exists.set(true);
                    } else {
                        set_component.set(T::default());
                        set_exists.set(false);
                    }
                }
                None => {
                    set_component.set(T::default());
                    set_exists.set(false);
                }
            }
        });

        (component_signal.into(), exists_signal.into())
    }

    /// Subscribe to a specific entity's component as a Store for fine-grained reactivity.
    ///
    /// Unlike `subscribe_component_store` which returns `Store<HashMap<u64, T>>`, this
    /// returns `Store<T>` directly for a single entity, enabling fine-grained field-level
    /// reactivity using the `reactive_stores` crate.
    ///
    /// Returns a tuple of:
    /// - `Store<T>`: The component store (uses `T::default()` when entity doesn't exist)
    /// - `ReadSignal<bool>`: Whether the entity currently exists
    ///
    /// # Type Parameters
    ///
    /// - `T`: The component type to subscribe to. Must implement `SyncComponent` and derive `Store`.
    ///
    /// # Arguments
    ///
    /// - `entity_id_fn`: A reactive closure that returns `Option<u64>`. When `None`,
    ///   the store will contain default values and `exists` will be `false`.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use pl3xus_client::use_sync_entity_component_store;
    /// use reactive_stores::Store;
    ///
    /// #[derive(Clone, Default, Serialize, Deserialize, Store)]
    /// struct ExecutionState {
    ///     state: ProgramExecutionState,
    ///     can_start: bool,
    ///     can_pause: bool,
    ///     // ... other fields
    /// }
    ///
    /// // Subscribe to the single ExecutionState entity
    /// let entities = use_sync_component::<ExecutionState>();
    /// let (exec, exists) = use_sync_entity_component_store::<ExecutionState>(move || {
    ///     entities.get().keys().next().copied()
    /// });
    ///
    /// // Fine-grained field access - only re-renders when specific field changes
    /// let can_start = move || exec.can_start().get();
    /// let can_pause = move || exec.can_pause().get();
    /// ```
    #[cfg(feature = "stores")]
    pub fn subscribe_entity_component_store<T, F>(
        &self,
        entity_id_fn: F,
    ) -> (Store<T>, ReadSignal<bool>)
    where
        T: SyncComponent + Clone + Default + 'static,
        F: Fn() -> Option<u64> + Clone + 'static,
    {
        // Subscribe to the underlying component type
        let all_components = self.subscribe_component::<T>();

        // Create the store and exists signal
        let store = Store::new(T::default());
        let store_clone = store.clone();
        let (exists_signal, set_exists) = signal(false);

        // Create an effect that watches both the entity_id and the component data
        let entity_id_fn_clone = entity_id_fn.clone();
        Effect::new(move |_| {
            let maybe_entity_id = entity_id_fn_clone();
            let components = all_components.get();

            match maybe_entity_id {
                Some(entity_id) => {
                    if let Some(component) = components.get(&entity_id) {
                        // Update the store with the component data
                        store_clone.try_update_untracked(|value| *value = component.clone());
                        store_clone.notify();
                        set_exists.set(true);
                    } else {
                        store_clone.try_update_untracked(|value| *value = T::default());
                        store_clone.notify();
                        set_exists.set(false);
                    }
                }
                None => {
                    store_clone.try_update_untracked(|value| *value = T::default());
                    store_clone.notify();
                    set_exists.set(false);
                }
            }
        });

        (store, exists_signal.into())
    }
}

