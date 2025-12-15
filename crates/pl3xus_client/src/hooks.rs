use std::collections::HashMap;
use std::fmt::Display;
use std::str::FromStr;

use leptos::prelude::*;
use leptos::html::Input;
use leptos::web_sys;

use crate::context::{MutationState, SyncConnection, SyncContext};
use crate::traits::SyncComponent;

#[cfg(feature = "stores")]
use reactive_stores::Store;

/// Hook to subscribe to a component type.
///
/// This returns a signal containing a HashMap of entity_id -> component.
/// The subscription is automatically managed - it will be created when the
/// component mounts and cleaned up when it unmounts.
///
/// Multiple calls to this hook with the same component type will share the
/// same underlying subscription (deduplication).
///
/// # Panics
///
/// Panics if called outside of a `SyncProvider` context.
///
/// # Example
///
/// ```rust,ignore
/// use pl3xus_client::{use_sync_component, SyncComponent};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Clone, Default, Serialize, Deserialize)]
/// struct Position {
///     x: f32,
///     y: f32,
/// }
///
/// // SyncComponent is automatically implemented!
///
/// #[component]
/// fn PositionList() -> impl IntoView {
///     let positions = use_sync_component::<Position>();
///
///     view! {
///         <ul>
///             <For
///                 each=move || positions.get().into_iter()
///                 key=|(id, _)| *id
///                 children=|(id, pos)| {
///                     view! {
///                         <li>{format!("Entity {}: ({}, {})", id, pos.x, pos.y)}</li>
///                     }
///                 }
///             />
///         </ul>
///     }
/// }
/// ```
pub fn use_sync_component<T: SyncComponent + Clone + Default + 'static>() -> ReadSignal<HashMap<u64, T>> {
    let ctx = expect_context::<SyncContext>();
    ctx.subscribe_component::<T>()
}

/// Hook to subscribe to a component type with client-side filtering.
///
/// This returns a signal containing a HashMap of entity_id -> component,
/// filtered by the provided predicate function. The filter runs on the client
/// side whenever the component data updates.
///
/// This provides an ergonomic API without requiring server-side query parsing
/// or DSL complexity. The filter is type-safe and can use any Rust expression.
///
/// # Performance
///
/// The filter runs on every update of the underlying component data. For most
/// use cases this is very fast (< 1μs per entity). If you have thousands of
/// entities and performance becomes an issue, consider using server-side
/// query filtering (future feature).
///
/// # Panics
///
/// Panics if called outside of a `SyncProvider` context.
///
/// # Example
///
/// ```rust,ignore
/// use pl3xus_client::{use_sync_component_where, SyncComponent};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Clone, Default, Serialize, Deserialize)]
/// struct Position {
///     x: f32,
///     y: f32,
/// }
///
/// #[component]
/// fn FilteredPositionList() -> impl IntoView {
///     // Only show positions where x > 100.0
///     let filtered_positions = use_sync_component_where::<Position, _>(
///         |pos| pos.x > 100.0
///     );
///
///     view! {
///         <ul>
///             <For
///                 each=move || filtered_positions.get().into_iter()
///                 key=|(id, _)| *id
///                 children=|(id, pos)| {
///                     view! {
///                         <li>{format!("Entity {}: ({}, {})", id, pos.x, pos.y)}</li>
///                     }
///                 }
///             />
///         </ul>
///     }
/// }
/// ```
pub fn use_sync_component_where<T, F>(
    filter: F,
) -> Signal<HashMap<u64, T>>
where
    T: SyncComponent + Clone + Default + 'static,
    F: Fn(&T) -> bool + Send + Sync + 'static,
{
    let all_components = use_sync_component::<T>();

    Signal::derive(move || {
        all_components.get()
            .into_iter()
            .filter(|(_, component)| filter(component))
            .collect::<HashMap<u64, T>>()
    })
}

/// Hook to subscribe to a single entity's component.
///
/// This is a convenience helper that creates a derived signal for accessing
/// a specific entity's component. It's useful when you know the entity ID
/// and want to reactively access its component data.
///
/// This is equivalent to manually creating a derived signal from
/// `use_sync_component`, but more ergonomic.
///
/// # Panics
///
/// Panics if called outside of a `SyncProvider` context.
///
/// # Example
///
/// ```rust,ignore
/// use pl3xus_client::{use_sync_entity, use_sync_context, SyncComponent};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Clone, Default, Serialize, Deserialize)]
/// struct MicrowaveConfig {
///     power_enabled: bool,
///     watts: f32,
/// }
///
/// #[component]
/// fn MicrowaveControls(entity_id: u64) -> impl IntoView {
///     let ctx = use_sync_context();
///     let server_config = use_sync_entity::<MicrowaveConfig>(entity_id);
///
///     // Toggle power using direct mutation
///     let toggle_power = move |_| {
///         if let Some(config) = server_config.get_untracked() {
///             ctx.mutate(entity_id, MicrowaveConfig {
///                 power_enabled: !config.power_enabled,
///                 ..config
///             });
///         }
///     };
///
///     view! {
///         <button on:click=toggle_power>
///             {move || if server_config.get().map(|c| c.power_enabled).unwrap_or(false) {
///                 "Power On"
///             } else {
///                 "Power Off"
///             }}
///         </button>
///     }
/// }
/// ```
pub fn use_sync_entity<T: SyncComponent + Clone + Default + 'static>(
    entity_id: u64,
) -> Signal<Option<T>> {
    let all_components = use_sync_component::<T>();

    Signal::derive(move || {
        all_components.get().get(&entity_id).cloned()
    })
}

/// Hook to access the WebSocket connection control interface.
///
/// This allows you to manually control the WebSocket connection (open/close)
/// and check the connection state.
///
/// # Panics
///
/// Panics if called outside of a `SyncProvider` context.
///
/// # Example
///
/// ```rust,ignore
/// use pl3xus_client::use_sync_connection;
/// use leptos_use::core::ConnectionReadyState;
///
/// #[component]
/// fn ConnectionStatus() -> impl IntoView {
///     let connection = use_sync_connection();
///
///     let status_text = move || {
///         match connection.ready_state.get() {
///             ConnectionReadyState::Connecting => "Connecting...",
///             ConnectionReadyState::Open => "Connected",
///             ConnectionReadyState::Closing => "Closing...",
///             ConnectionReadyState::Closed => "Disconnected",
///         }
///     };
///
///     let is_connected = move || {
///         connection.ready_state.get() == ConnectionReadyState::Open
///     };
///
///     view! {
///         <div>
///             <p>"Status: " {status_text}</p>
///             <button
///                 on:click=move |_| (connection.open)()
///                 disabled=is_connected
///             >
///                 "Connect"
///             </button>
///             <button
///                 on:click=move |_| (connection.close)()
///                 disabled=move || !is_connected()
///             >
///                 "Disconnect"
///             </button>
///         </div>
///     }
/// }
/// ```
pub fn use_sync_connection() -> SyncConnection {
    let ctx = expect_context::<SyncContext>();
    ctx.connection()
}

/// Hook to subscribe to a component type with fine-grained reactivity using stores.
///
/// This returns a `Store<HashMap<u64, T>>` that provides fine-grained reactive access
/// to individual entity fields. Unlike signals which are atomic, stores allow you to
/// reactively access nested fields without triggering updates for unrelated data.
///
/// The subscription is automatically managed - it will be created when the component
/// mounts and cleaned up when it unmounts.
///
/// Multiple calls to this hook with the same component type will share the same
/// underlying subscription (deduplication).
///
/// # Panics
///
/// Panics if called outside of a `SyncProvider` context.
///
/// # Example
///
/// ```rust,ignore
/// use pl3xus_client::{use_sync_component_store, SyncComponent};
/// use reactive_stores::Store;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Clone, Default, Serialize, Deserialize, Store)]
/// struct Position {
///     x: f32,
///     y: f32,
/// }
///
/// // SyncComponent is automatically implemented!
///
/// #[component]
/// fn PositionList() -> impl IntoView {
///     let positions = use_sync_component_store::<Position>();
///
///     view! {
///         <For
///             each=move || positions.read().keys().copied().collect::<Vec<_>>()
///             key=|id| *id
///             let:entity_id
///         >
///             {move || {
///                 // Fine-grained: only updates when this specific entity's position changes
///                 let pos = positions.read().get(&entity_id).cloned();
///                 view! {
///                     <li>{format!("Entity {}: {:?}", entity_id, pos)}</li>
///                 }
///             }}
///         </For>
///     }
/// }
/// ```
#[cfg(feature = "stores")]
pub fn use_sync_component_store<T: SyncComponent + Clone + Default + 'static>() -> Store<HashMap<u64, T>> {
    let ctx = expect_context::<SyncContext>();
    ctx.subscribe_component_store::<T>()
}

/// Hook to access the SyncContext directly.
///
/// This provides access to the full SyncContext API, including mutation methods.
/// Most users should use the more specific hooks like `use_sync_component` or
/// `use_sync_mutations` instead.
///
/// # Panics
///
/// Panics if called outside of a `SyncProvider` context.
///
/// # Example
///
/// ```rust,ignore
/// use pl3xus_client::use_sync_context;
///
/// #[component]
/// fn MutatePosition() -> impl IntoView {
///     let ctx = use_sync_context();
///
///     let update_position = move |_| {
///         let new_pos = Position { x: 10.0, y: 20.0 };
///         let request_id = ctx.mutate(entity_id, new_pos);
///     };
///
///     view! {
///         <button on:click=update_position>"Update Position"</button>
///     }
/// }
/// ```
pub fn use_sync_context() -> SyncContext {
    expect_context::<SyncContext>()
}

/// Hook to access mutation state tracking.
///
/// This returns a read-only signal containing all mutation states, allowing
/// components to reactively track the status of mutations (pending, success, error).
///
/// # Panics
///
/// Panics if called outside of a `SyncProvider` context.
///
/// # Example
///
/// ```rust,ignore
/// use pl3xus_client::{use_sync_context, use_sync_mutations};
/// use pl3xus_sync::MutationStatus;
///
/// #[component]
/// fn MutateWithFeedback() -> impl IntoView {
///     let ctx = use_sync_context();
///     let mutations = use_sync_mutations();
///     let (last_request_id, set_last_request_id) = signal(None::<u64>);
///
///     let update_position = move |_| {
///         let new_pos = Position { x: 10.0, y: 20.0 };
///         let request_id = ctx.mutate(entity_id, new_pos);
///         set_last_request_id.set(Some(request_id));
///     };
///
///     let status_text = move || {
///         last_request_id.get().and_then(|id| {
///             mutations.get().get(&id).map(|state| {
///                 match &state.status {
///                     None => "Pending...".to_string(),
///                     Some(MutationStatus::Ok) => "Success!".to_string(),
///                     Some(status) => format!("Error: {:?}", status),
///                 }
///             })
///         })
///     };
///
///     view! {
///         <div>
///             <button on:click=update_position>"Update Position"</button>
///             {move || status_text().unwrap_or_default()}
///         </div>
///     }
/// }
/// ```
pub fn use_sync_mutations() -> ReadSignal<HashMap<u64, MutationState>> {
    let ctx = expect_context::<SyncContext>();
    ctx.mutations()
}

/// Hook for creating editable fields with Enter-to-apply, blur-to-revert UX.
///
/// This hook implements the NodeRef + Effect + focus tracking pattern to achieve:
/// - Focus retention through server updates
/// - User input preservation while focused
/// - Enter key to apply mutation
/// - Blur (click away) to revert to server value
///
/// Unlike `use_controlled_input`, this hook uses manual DOM updates via NodeRef
/// to avoid reactive property binding that would steal focus. It also implements
/// the Enter-to-apply, blur-to-revert UX pattern that provides clear feedback
/// to users about when mutations are applied.
///
/// # Type Parameters
///
/// - `T`: The component type (must implement `SyncComponent`)
/// - `F`: The field type (must implement `Display + FromStr`)
///
/// # Arguments
///
/// - `entity_id`: The entity to edit
/// - `field_accessor`: A function that extracts the field value from the component
/// - `field_mutator`: A function that creates a new component with the field updated
///
/// # Returns
///
/// A tuple of:
/// - `NodeRef<Input>`: Reference to the input element (use with `node_ref=`)
/// - `RwSignal<bool>`: Focus state (use with `on:focus` and `on:blur`)
/// - `String`: Initial value for the input (use with `value=`)
/// - `impl Fn(web_sys::KeyboardEvent)`: Keydown handler (use with `on:keydown`)
/// - `impl Fn()`: Blur handler (use with `on:blur`)
///
/// # Example
///
/// ```rust,ignore
/// use pl3xus_client::{use_sync_field_editor, SyncComponent};
/// use leptos::prelude::*;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Clone, Default, Serialize, Deserialize)]
/// struct Position {
///     x: f32,
///     y: f32,
/// }
///
/// // SyncComponent is automatically implemented!
///
/// #[component]
/// fn PositionEditor(entity_id: u64) -> impl IntoView {
///     let (input_ref, is_focused, initial_value, on_keydown, on_blur_handler) =
///         use_sync_field_editor(
///             entity_id,
///             |pos: &Position| pos.x,
///             |pos: &Position, new_x: f32| Position { x: new_x, y: pos.y },
///         );
///
///     view! {
///         <input
///             node_ref=input_ref
///             type="number"
///             value=initial_value
///             on:focus=move |_| is_focused.set(true)
///             on:blur=move |_| {
///                 is_focused.set(false);
///                 on_blur_handler();
///             }
///             on:keydown=on_keydown
///         />
///     }
/// }
/// ```
pub fn use_sync_field_editor<T, F, A, M>(
    entity_id: u64,
    field_accessor: A,
    field_mutator: M,
) -> (
    NodeRef<Input>,
    RwSignal<bool>,
    String,
    impl Fn(web_sys::KeyboardEvent) + Clone,
    impl Fn() + Clone,
)
where
    T: SyncComponent + Clone + Default + 'static,
    F: Display + FromStr + Clone + PartialEq + 'static,
    A: Fn(&T) -> F + Clone + 'static,
    M: Fn(&T, F) -> T + Clone + 'static,
{
    let ctx = expect_context::<SyncContext>();

    // Subscribe to all instances of this component type
    let all_components = use_sync_component::<T>();

    // Create NodeRef for direct DOM access
    let input_ref = NodeRef::<Input>::new();

    // Track focus state
    let is_focused = RwSignal::new(false);

    // Get initial value
    let initial_value = all_components
        .get_untracked()
        .get(&entity_id)
        .map(|component| field_accessor(component).to_string())
        .unwrap_or_default();

    // Effect to update input when server value changes (only when NOT focused)
    {
        let input_ref = input_ref.clone();
        let field_accessor = field_accessor.clone();

        Effect::new(move |_| {
            if let Some(component) = all_components.get().get(&entity_id) {
                let server_value = field_accessor(component).to_string();

                // Only update DOM if input is NOT focused
                if !is_focused.get_untracked() {
                    if let Some(input) = input_ref.get() {
                        // Only update if value actually changed
                        if input.value() != server_value {
                            input.set_value(&server_value);
                        }
                    }
                }
            }
        });
    }

    // Create blur handler (reverts to server value)
    let on_blur_handler = {
        let input_ref = input_ref.clone();
        let field_accessor = field_accessor.clone();

        move || {
            if let Some(component) = all_components.get_untracked().get(&entity_id) {
                let server_value = field_accessor(component).to_string();
                if let Some(input) = input_ref.get_untracked() {
                    input.set_value(&server_value);
                }
            }
        }
    };

    // Create keydown handler (applies mutation on Enter)
    let on_keydown = {
        let input_ref = input_ref.clone();

        move |ev: web_sys::KeyboardEvent| {
            if ev.key() == "Enter" {
                // Get current component value
                if let Some(component) = all_components.get_untracked().get(&entity_id) {
                    // Get input value
                    if let Some(input) = input_ref.get_untracked() {
                        let raw_value = input.value();

                        // Parse the value
                        if let Ok(parsed_value) = raw_value.parse::<F>() {
                            // Create updated component
                            let updated_component = field_mutator(component, parsed_value);

                            // Send mutation
                            ctx.mutate(entity_id, updated_component);

                            // Blur the input to trigger revert (in case server rejects)
                            let _ = input.blur();
                        }
                    }
                }
            }
        }
    };

    (
        input_ref,
        is_focused,
        initial_value,
        on_keydown,
        on_blur_handler,
    )
}

/// Hook for "untracked" synchronization pattern.
///
/// This hook implements the pattern where:
/// 1. Initial sync: Client receives full state from server
/// 2. Incremental updates: Client receives individual updates and appends them locally
///
/// This is useful for scenarios like logging where:
/// - Server sends full log history on connection
/// - Server broadcasts individual log messages thereafter
/// - Client appends messages to local copy without re-syncing entire log
///
/// # Type Parameters
///
/// - `TFull`: The full state type (e.g., `ServerLog` containing `VecDeque<ServerLogMessage>`)
/// - `TIncremental`: The incremental update type (e.g., `ServerLogMessage`)
///
/// # Arguments
///
/// - `append_fn`: Function to append an incremental update to the full state
///
/// # Returns
///
/// A tuple of:
/// - `Signal<TFull>`: The full state (initialized from server, then updated locally)
/// - `Signal<Option<TIncremental>>`: The latest incremental update (for triggering effects)
///
/// # Panics
///
/// Panics if called outside of a `SyncProvider` context.
///
/// # Example
///
/// ```rust,ignore
/// use pl3xus_client::{use_sync_untracked, SyncComponent};
/// use serde::{Deserialize, Serialize};
/// use std::collections::VecDeque;
///
/// #[derive(Clone, Default, Serialize, Deserialize)]
/// struct ServerLog {
///     messages: VecDeque<ServerLogMessage>,
/// }
///
/// #[derive(Clone, Default, Serialize, Deserialize)]
/// struct ServerLogMessage {
///     timestamp: u64,
///     level: String,
///     message: String,
/// }
///
/// #[component]
/// fn LogViewer() -> impl IntoView {
///     let (full_log, latest_message) = use_sync_untracked::<ServerLog, ServerLogMessage>(
///         |log, msg| log.messages.push_back(msg),
///     );
///
///     // Auto-scroll to bottom when new message arrives
///     Effect::new(move |_| {
///         if latest_message.get().is_some() {
///             scroll_to_bottom();
///         }
///     });
///
///     view! {
///         <div class="log-viewer">
///             <For
///                 each=move || full_log.get().messages.iter().cloned().collect::<Vec<_>>()
///                 key=|msg| msg.timestamp
///                 children=|msg| view! {
///                     <div class=format!("log-{}", msg.level)>
///                         {format!("[{}] {}", msg.timestamp, msg.message)}
///                     </div>
///                 }
///             />
///         </div>
///     }
/// }
/// ```
pub fn use_sync_untracked<TFull, TIncremental, F>(
    append_fn: F,
) -> (Signal<TFull>, Signal<Option<TIncremental>>)
where
    TFull: SyncComponent + Clone + Default + 'static,
    TIncremental: SyncComponent + Clone + Default + 'static,
    F: Fn(&mut TFull, TIncremental) + Send + Sync + Clone + 'static,
{
    // Subscribe to full state (HashMap<u64, TFull>)
    let full_components = use_sync_component::<TFull>();

    // Subscribe to incremental updates (HashMap<u64, TIncremental>)
    let incremental_components = use_sync_component::<TIncremental>();

    // Create local state for the full data
    let local_full = RwSignal::new(TFull::default());

    // Track the latest incremental update
    let (latest_incremental, set_latest_incremental) = signal(None::<TIncremental>);

    // Track if we've received the initial full sync
    let has_initial_sync = RwSignal::new(false);

    // Effect: Initialize from full state (runs once on initial sync)
    Effect::new(move |_| {
        let full_map = full_components.get();

        // If we haven't initialized yet and we have data, initialize
        if !has_initial_sync.get_untracked() && !full_map.is_empty() {
            // Take the first (and should be only) full state
            if let Some((_, full_state)) = full_map.iter().next() {
                local_full.set(full_state.clone());
                has_initial_sync.set(true);
            }
        }
    });

    // Effect: Append incremental updates
    Effect::new(move |_| {
        let incremental_map = incremental_components.get();

        // Only process incremental updates after initial sync
        if has_initial_sync.get_untracked() {
            // Process all incremental updates
            for (_, incremental) in incremental_map.iter() {
                local_full.update(|full| {
                    append_fn(full, incremental.clone());
                });
                set_latest_incremental.set(Some(incremental.clone()));
            }
        }
    });

    // Return read-only signal for full state and latest incremental
    (local_full.into(), latest_incremental.into())
}

/// Hook to subscribe to arbitrary Pl3xusMessage broadcasts from the server.
///
/// This is for one-way broadcast messages (e.g., notifications, events, video frames)
/// that are not part of the component sync system. Unlike component subscriptions which
/// track entity state, message subscriptions receive broadcast messages.
///
/// # Type Parameters
///
/// - `T`: The message type. Must implement `SyncComponent` (which provides type_name).
///
/// # Returns
///
/// Returns a `ReadSignal<T>` that updates whenever a message of type `T` is received.
///
/// # Panics
///
/// Panics if called outside of a `SyncProvider` context.
///
/// # Example
///
/// ```rust,ignore
/// use pl3xus_client::use_sync_message;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Clone, Default, Serialize, Deserialize)]
/// struct Notification {
///     message: String,
///     level: String,
/// }
///
/// #[component]
/// fn NotificationBanner() -> impl IntoView {
///     let notification = use_sync_message::<Notification>();
///
///     view! {
///         <div class="notification">
///             {move || notification.get().message}
///         </div>
///     }
/// }
/// ```
pub fn use_sync_message<T>() -> ReadSignal<T>
where
    T: SyncComponent + Clone + Default + 'static,
{
    let ctx = expect_context::<SyncContext>();
    ctx.subscribe_message::<T>()
}

/// Hook to subscribe to arbitrary Pl3xusMessage broadcasts using a Store.
///
/// This provides fine-grained reactivity for message fields, similar to
/// `use_sync_component_store` but for broadcast messages.
///
/// # Type Parameters
///
/// - `T`: The message type. Must implement `SyncComponent` (which provides type_name).
///
/// # Returns
///
/// Returns a `Store<T>` that updates whenever a message of type `T` is received.
///
/// # Panics
///
/// Panics if called outside of a `SyncProvider` context.
///
/// # Example
///
/// ```rust,ignore
/// use pl3xus_client::use_sync_message_store;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Clone, Default, Serialize, Deserialize)]
/// struct ServerStats {
///     cpu_usage: f32,
///     memory_usage: f32,
///     active_connections: u32,
/// }
///
/// #[component]
/// fn StatsDisplay() -> impl IntoView {
///     let stats = use_sync_message_store::<ServerStats>();
///
///     view! {
///         <div>
///             <p>"CPU: " {move || stats.cpu_usage().get()}</p>
///             <p>"Memory: " {move || stats.memory_usage().get()}</p>
///             <p>"Connections: " {move || stats.active_connections().get()}</p>
///         </div>
///     }
/// }
/// ```
#[cfg(feature = "stores")]
pub fn use_sync_message_store<T>() -> Store<T>
where
    T: SyncComponent + Clone + Default + 'static,
{
    let ctx = expect_context::<SyncContext>();
    ctx.subscribe_message_store::<T>()
}
