use std::collections::HashMap;
use std::fmt::Display;
use std::str::FromStr;

use leptos::prelude::*;
use leptos::html::Input;
use leptos::web_sys;

use crate::context::{MutationState, RequestState, RequestStatus, SyncConnection, SyncContext};
use crate::traits::SyncComponent;

#[cfg(feature = "stores")]
use reactive_stores::Store;

/// Hook to subscribe to all entities with a component type.
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
/// use pl3xus_client::{use_components, SyncComponent};
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
///     let positions = use_components::<Position>();
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
pub fn use_components<T: SyncComponent + Clone + Default + 'static>() -> ReadSignal<HashMap<u64, T>> {
    let ctx = expect_context::<SyncContext>();
    ctx.subscribe_component::<T>()
}

/// Deprecated: Use [`use_components`] instead.
#[deprecated(since = "0.2.0", note = "Use use_components instead")]
pub fn use_sync_component<T: SyncComponent + Clone + Default + 'static>() -> ReadSignal<HashMap<u64, T>> {
    use_components::<T>()
}

/// Hook to subscribe to a component type with client-side filtering.
///
/// This returns a signal containing a HashMap of entity_id -> component,
/// filtered by the provided predicate function. The filter runs on the client
/// side whenever the component data updates.
///
/// The filter receives both the entity ID and the component reference, allowing
/// filtering by entity ID, component properties, or both.
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
/// use pl3xus_client::{use_components_where, SyncComponent};
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
///     let filtered_positions = use_components_where::<Position, _>(
///         |_entity_id, pos| pos.x > 100.0
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
///
/// #[component]
/// fn SpecificEntityPosition(target_id: u64) -> impl IntoView {
///     // Filter by specific entity ID
///     let position = use_components_where::<Position, _>(
///         move |entity_id, _| entity_id == target_id
///     );
///
///     view! {
///         {move || position.get().values().next().map(|p| format!("x: {}, y: {}", p.x, p.y))}
///     }
/// }
/// ```
pub fn use_components_where<T, F>(
    filter: F,
) -> Signal<HashMap<u64, T>>
where
    T: SyncComponent + Clone + Default + 'static,
    F: Fn(u64, &T) -> bool + Send + Sync + 'static,
{
    let all_components = use_components::<T>();

    Signal::derive(move || {
        all_components.get()
            .into_iter()
            .filter(|(entity_id, component)| filter(*entity_id, component))
            .collect::<HashMap<u64, T>>()
    })
}

/// Deprecated: Use [`use_components_where`] instead.
#[deprecated(since = "0.2.0", note = "Use use_components_where instead")]
pub fn use_sync_component_where<T, F>(
    filter: F,
) -> Signal<HashMap<u64, T>>
where
    T: SyncComponent + Clone + Default + 'static,
    F: Fn(u64, &T) -> bool + Send + Sync + 'static,
{
    use_components_where(filter)
}

/// Hook to subscribe to a single entity's component by static entity ID.
///
/// This is a convenience helper that creates a derived signal for accessing
/// a specific entity's component. It's useful when you know the entity ID
/// and want to reactively access its component data.
///
/// This is equivalent to manually creating a derived signal from
/// `use_components`, but more ergonomic.
///
/// # Panics
///
/// Panics if called outside of a `SyncProvider` context.
///
/// # Example
///
/// ```rust,ignore
/// use pl3xus_client::{use_entity, use_sync_context, SyncComponent};
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
///     let server_config = use_entity::<MicrowaveConfig>(entity_id);
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
pub fn use_entity<T: SyncComponent + Clone + Default + 'static>(
    entity_id: u64,
) -> Signal<Option<T>> {
    let all_components = use_components::<T>();

    Signal::derive(move || {
        all_components.get().get(&entity_id).cloned()
    })
}

/// Deprecated: Use [`use_entity`] instead.
#[deprecated(since = "0.2.0", note = "Use use_entity instead")]
pub fn use_sync_entity<T: SyncComponent + Clone + Default + 'static>(
    entity_id: u64,
) -> Signal<Option<T>> {
    use_entity::<T>(entity_id)
}

/// Hook to subscribe to a single entity's component with a reactive entity ID.
///
/// This is similar to [`use_entity`], but accepts a reactive getter for the
/// entity ID instead of a static value. This is useful when the target entity
/// can change at runtime (e.g., currently selected robot).
///
/// # Arguments
///
/// * `entity_id_fn` - A closure that returns `Option<u64>`. When `None`, the
///   signal returns `None`. When `Some(id)`, returns the component for that entity.
///
/// # Panics
///
/// Panics if called outside of a `SyncProvider` context.
///
/// # Example
///
/// ```rust,ignore
/// use pl3xus_client::{use_entity_reactive, SyncComponent};
/// use leptos::prelude::*;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Clone, Default, Serialize, Deserialize)]
/// struct RobotPosition {
///     x: f32,
///     y: f32,
///     z: f32,
/// }
///
/// #[component]
/// fn RobotPositionDisplay() -> impl IntoView {
///     // Stored in context or parent component
///     let selected_robot_id = use_context::<RwSignal<Option<u64>>>()
///         .expect("selected_robot_id in context");
///
///     // Reactively get position for the currently selected robot
///     let position = use_entity_reactive::<RobotPosition, _>(
///         move || selected_robot_id.get()
///     );
///
///     view! {
///         {move || match position.get() {
///             Some(pos) => view! {
///                 <div>
///                     <p>"X: " {pos.x}</p>
///                     <p>"Y: " {pos.y}</p>
///                     <p>"Z: " {pos.z}</p>
///                 </div>
///             }.into_any(),
///             None => view! { <p>"No robot selected"</p> }.into_any(),
///         }}
///     }
/// }
/// ```
pub fn use_entity_reactive<T, F>(
    entity_id_fn: F,
) -> Signal<Option<T>>
where
    T: SyncComponent + Clone + Default + 'static,
    F: Fn() -> Option<u64> + Send + Sync + 'static,
{
    let all_components = use_components::<T>();

    Signal::derive(move || {
        entity_id_fn().and_then(|id| all_components.get().get(&id).cloned())
    })
}

/// Deprecated: Use [`use_entity_reactive`] instead.
#[deprecated(since = "0.2.0", note = "Use use_entity_reactive instead")]
pub fn use_sync_entity_reactive<T, F>(
    entity_id_fn: F,
) -> Signal<Option<T>>
where
    T: SyncComponent + Clone + Default + 'static,
    F: Fn() -> Option<u64> + Send + Sync + 'static,
{
    use_entity_reactive(entity_id_fn)
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
/// use pl3xus_client::use_connection;
/// use leptos_use::core::ConnectionReadyState;
///
/// #[component]
/// fn ConnectionStatus() -> impl IntoView {
///     let connection = use_connection();
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
pub fn use_connection() -> SyncConnection {
    let ctx = expect_context::<SyncContext>();
    ctx.connection()
}

/// Deprecated: Use [`use_connection`] instead.
#[deprecated(since = "0.2.0", note = "Use use_connection instead")]
pub fn use_sync_connection() -> SyncConnection {
    use_connection()
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
/// use pl3xus_client::{use_component_store, SyncComponent};
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
///     let positions = use_component_store::<Position>();
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
pub fn use_component_store<T: SyncComponent + Clone + Default + 'static>() -> Store<HashMap<u64, T>> {
    let ctx = expect_context::<SyncContext>();
    ctx.subscribe_component_store::<T>()
}

/// Deprecated: Use [`use_component_store`] instead.
#[cfg(feature = "stores")]
#[deprecated(since = "0.2.0", note = "Use use_component_store instead")]
pub fn use_sync_component_store<T: SyncComponent + Clone + Default + 'static>() -> Store<HashMap<u64, T>> {
    use_component_store::<T>()
}

/// Hook to subscribe to a specific entity's component as a signal.
///
/// Unlike `use_components` which returns all entities as a HashMap, this returns
/// a signal for a single entity's component. The entity ID can be reactive (a closure),
/// allowing you to switch which entity you're tracking.
///
/// Returns a tuple of:
/// - `ReadSignal<T>`: The component data (uses `T::default()` when entity doesn't exist)
/// - `ReadSignal<bool>`: Whether the entity currently exists
///
/// # Panics
///
/// Panics if called outside of a `SyncProvider` context.
///
/// # Example
///
/// ```rust,ignore
/// use pl3xus_client::use_entity_component;
///
/// // Fixed entity ID (singleton pattern)
/// let (exec, exists) = use_entity_component::<ExecutionState>(|| Some(SYSTEM_ENTITY_ID));
///
/// // Reactive entity ID from signal
/// let selected_robot: RwSignal<Option<u64>> = ...;
/// let (position, exists) = use_entity_component::<Position>(move || selected_robot.get());
///
/// // First entity of a type (true singleton)
/// let entities = use_components::<ExecutionState>();
/// let (exec, exists) = use_entity_component::<ExecutionState>(move || {
///     entities.get().keys().next().copied()
/// });
/// ```
pub fn use_entity_component<T, F>(entity_id_fn: F) -> (ReadSignal<T>, ReadSignal<bool>)
where
    T: SyncComponent + Clone + Default + 'static,
    F: Fn() -> Option<u64> + Clone + 'static,
{
    let ctx = expect_context::<SyncContext>();
    ctx.subscribe_entity_component::<T, F>(entity_id_fn)
}

/// Deprecated: Use [`use_entity_component`] instead.
#[deprecated(since = "0.2.0", note = "Use use_entity_component instead")]
pub fn use_sync_entity_component<T, F>(entity_id_fn: F) -> (ReadSignal<T>, ReadSignal<bool>)
where
    T: SyncComponent + Clone + Default + 'static,
    F: Fn() -> Option<u64> + Clone + 'static,
{
    use_entity_component(entity_id_fn)
}

/// Hook to subscribe to a specific entity's component as a Store for fine-grained reactivity.
///
/// Unlike `use_component_store` which returns `Store<HashMap<u64, T>>`, this returns
/// `Store<T>` directly for a single entity, enabling fine-grained field-level reactivity
/// using the `reactive_stores` crate.
///
/// Returns a tuple of:
/// - `Store<T>`: The component store (uses `T::default()` when entity doesn't exist)
/// - `ReadSignal<bool>`: Whether the entity currently exists
///
/// # Panics
///
/// Panics if called outside of a `SyncProvider` context.
///
/// # Example
///
/// ```rust,ignore
/// use pl3xus_client::use_entity_component_store;
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
/// // Subscribe to the first (and only) ExecutionState entity
/// let entities = use_components::<ExecutionState>();
/// let (exec, exists) = use_entity_component_store::<ExecutionState, _>(move || {
///     entities.get().keys().next().copied()
/// });
///
/// // Fine-grained field access - only re-renders when specific field changes
/// let can_start = move || exec.can_start().get();
/// let can_pause = move || exec.can_pause().get();
/// ```
#[cfg(feature = "stores")]
pub fn use_entity_component_store<T, F>(entity_id_fn: F) -> (Store<T>, ReadSignal<bool>)
where
    T: SyncComponent + Clone + Default + 'static,
    F: Fn() -> Option<u64> + Clone + 'static,
{
    let ctx = expect_context::<SyncContext>();
    ctx.subscribe_entity_component_store::<T, F>(entity_id_fn)
}

/// Deprecated: Use [`use_entity_component_store`] instead.
#[cfg(feature = "stores")]
#[deprecated(since = "0.2.0", note = "Use use_entity_component_store instead")]
pub fn use_sync_entity_component_store<T, F>(entity_id_fn: F) -> (Store<T>, ReadSignal<bool>)
where
    T: SyncComponent + Clone + Default + 'static,
    F: Fn() -> Option<u64> + Clone + 'static,
{
    use_entity_component_store(entity_id_fn)
}

/// Hook to access the SyncContext directly.
///
/// This provides access to the full SyncContext API, including mutation methods.
/// Most users should use the more specific hooks like `use_components` or
/// `use_mutations` instead.
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

/// Hook to get a callback for sending targeted messages to a specific entity.
///
/// This returns a callback that sends a message wrapped in `TargetedMessage<T>`.
/// On the server, the message will be processed by the authorization middleware
/// and converted to an `AuthorizedTargetedMessage<T>` if the client has control of the
/// target entity.
///
/// # Type Parameters
///
/// - `T`: The message type (must implement `Pl3xusMessage`)
///
/// # Panics
///
/// Panics if called outside of a `SyncProvider` context.
///
/// # Example
///
/// ```rust,ignore
/// use pl3xus_client::use_send_targeted;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Clone, Serialize, Deserialize)]
/// struct JogCommand {
///     axis: u8,
///     direction: i8,
/// }
///
/// #[component]
/// fn JogControls(entity_bits: u64) -> impl IntoView {
///     let send_jog = use_send_targeted::<JogCommand>(entity_bits);
///
///     let jog_x_plus = move |_| {
///         send_jog(JogCommand { axis: 0, direction: 1 });
///     };
///
///     let jog_x_minus = move |_| {
///         send_jog(JogCommand { axis: 0, direction: -1 });
///     };
///
///     view! {
///         <button on:click=jog_x_plus>"X+"</button>
///         <button on:click=jog_x_minus>"X-"</button>
///     }
/// }
/// ```
pub fn use_send_targeted<T>(entity_bits: u64) -> impl Fn(T) + Clone
where
    T: serde::Serialize + pl3xus_common::Pl3xusMessage + Clone + 'static,
{
    let ctx = expect_context::<SyncContext>();
    move |message: T| {
        ctx.send_targeted(entity_bits, message);
    }
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
/// use pl3xus_client::{use_sync_context, use_mutations};
/// use pl3xus_sync::MutationStatus;
///
/// #[component]
/// fn MutateWithFeedback() -> impl IntoView {
///     let ctx = use_sync_context();
///     let mutations = use_mutations();
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
pub fn use_mutations() -> ReadSignal<HashMap<u64, MutationState>> {
    let ctx = expect_context::<SyncContext>();
    ctx.mutations()
}

/// Deprecated: Use [`use_mutations`] instead.
#[deprecated(since = "0.2.0", note = "Use use_mutations instead")]
pub fn use_sync_mutations() -> ReadSignal<HashMap<u64, MutationState>> {
    use_mutations()
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
/// use pl3xus_client::{use_field_editor, SyncComponent};
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
///         use_field_editor(
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
pub fn use_field_editor<T, F, A, M>(
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
    let all_components = use_components::<T>();

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

/// Deprecated: Use [`use_field_editor`] instead.
#[deprecated(since = "0.2.0", note = "Use use_field_editor instead")]
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
    use_field_editor(entity_id, field_accessor, field_mutator)
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
/// use pl3xus_client::{use_untracked, SyncComponent};
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
///     let (full_log, latest_message) = use_untracked::<ServerLog, ServerLogMessage>(
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
pub fn use_untracked<TFull, TIncremental, F>(
    append_fn: F,
) -> (Signal<TFull>, Signal<Option<TIncremental>>)
where
    TFull: SyncComponent + Clone + Default + 'static,
    TIncremental: SyncComponent + Clone + Default + 'static,
    F: Fn(&mut TFull, TIncremental) + Send + Sync + Clone + 'static,
{
    // Subscribe to full state (HashMap<u64, TFull>)
    let full_components = use_components::<TFull>();

    // Subscribe to incremental updates (HashMap<u64, TIncremental>)
    let incremental_components = use_components::<TIncremental>();

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

/// Deprecated: Use [`use_untracked`] instead.
#[deprecated(since = "0.2.0", note = "Use use_untracked instead")]
pub fn use_sync_untracked<TFull, TIncremental, F>(
    append_fn: F,
) -> (Signal<TFull>, Signal<Option<TIncremental>>)
where
    TFull: SyncComponent + Clone + Default + 'static,
    TIncremental: SyncComponent + Clone + Default + 'static,
    F: Fn(&mut TFull, TIncremental) + Send + Sync + Clone + 'static,
{
    use_untracked(append_fn)
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
/// use pl3xus_client::use_message;
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
///     let notification = use_message::<Notification>();
///
///     view! {
///         <div class="notification">
///             {move || notification.get().message}
///         </div>
///     }
/// }
/// ```
pub fn use_message<T>() -> ReadSignal<T>
where
    T: SyncComponent + Clone + Default + 'static,
{
    let ctx = expect_context::<SyncContext>();
    ctx.subscribe_message::<T>()
}

/// Deprecated: Use [`use_message`] instead.
#[deprecated(since = "0.2.0", note = "Use use_message instead")]
pub fn use_sync_message<T>() -> ReadSignal<T>
where
    T: SyncComponent + Clone + Default + 'static,
{
    use_message::<T>()
}

/// Hook to subscribe to arbitrary Pl3xusMessage broadcasts using a Store.
///
/// This provides fine-grained reactivity for message fields, similar to
/// `use_component_store` but for broadcast messages.
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
/// use pl3xus_client::use_message_store;
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
///     let stats = use_message_store::<ServerStats>();
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
pub fn use_message_store<T>() -> Store<T>
where
    T: SyncComponent + Clone + Default + 'static,
{
    let ctx = expect_context::<SyncContext>();
    ctx.subscribe_message_store::<T>()
}

/// Deprecated: Use [`use_message_store`] instead.
#[cfg(feature = "stores")]
#[deprecated(since = "0.2.0", note = "Use use_message_store instead")]
pub fn use_sync_message_store<T>() -> Store<T>
where
    T: SyncComponent + Clone + Default + 'static,
{
    use_message_store::<T>()
}


/// Hook to send a request and get a reactive signal for the response.
///
/// This provides a simple way to make request/response calls to the server
/// with reactive state tracking. The hook returns a tuple of:
/// - A function to trigger the request
/// - A reactive signal with the current state (loading, data, error)
///
/// # Type Parameters
///
/// - `R`: The request type (must implement `RequestMessage`)
///
/// # Panics
///
/// Panics if called outside of a `SyncProvider` context.
///
/// # Example
///
/// ```rust,ignore
/// use pl3xus_client::use_request;
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
///     let (fetch, state) = use_request::<ListRobots>();
///
///     Effect::new(move |_| {
///         // Fetch on mount
///         fetch(ListRobots);
///     });
///
///     view! {
///         <Show when=move || state.get().is_loading()>
///             <p>"Loading..."</p>
///         </Show>
///         <Show when=move || state.get().data.is_some()>
///             <ul>
///                 {move || state.get().data.unwrap_or_default().iter().map(|r| view! {
///                     <li>{&r.name}</li>
///                 }).collect::<Vec<_>>()}
///             </ul>
///         </Show>
///     }
/// }
/// ```
pub fn use_request<R>() -> (
    impl Fn(R) + Clone,
    Signal<UseRequestState<R::ResponseMessage>>,
)
where
    R: pl3xus_common::RequestMessage + Clone + 'static,
{
    let ctx = expect_context::<SyncContext>();

    // Track the current request ID
    let current_request_id = RwSignal::new(None::<u64>);

    // Derive state from the context's request tracking
    let state = {
        let ctx = ctx.clone();
        Signal::derive(move || {
            let request_id = current_request_id.get();

            match request_id {
                None => UseRequestState {
                    is_loading: false,
                    data: None,
                    error: None,
                },
                Some(id) => {
                    let requests = ctx.requests.get();
                    match requests.get(&id) {
                        None => UseRequestState {
                            is_loading: false,
                            data: None,
                            error: Some("Request not found".to_string()),
                        },
                        Some(req_state) => {
                            match &req_state.status {
                                RequestStatus::Pending => UseRequestState {
                                    is_loading: true,
                                    data: None,
                                    error: None,
                                },
                                RequestStatus::Success => {
                                    let data = ctx.get_response::<R>(id);
                                    UseRequestState {
                                        is_loading: false,
                                        data,
                                        error: None,
                                    }
                                }
                                RequestStatus::Error(e) => UseRequestState {
                                    is_loading: false,
                                    data: None,
                                    error: Some(e.clone()),
                                },
                            }
                        }
                    }
                }
            }
        })
    };

    // Create the fetch function
    let fetch = move |request: R| {
        #[cfg(target_arch = "wasm32")]
        leptos::logging::log!("[use_request] fetch called for request type: {}", R::request_name());
        let id = ctx.request(request);
        #[cfg(target_arch = "wasm32")]
        leptos::logging::log!("[use_request] request sent with id: {}", id);
        current_request_id.set(Some(id));
    };

    (fetch, state)
}

/// State for a request/response cycle.
#[derive(Clone, Debug)]
pub struct UseRequestState<T> {
    /// Whether the request is currently in flight
    pub is_loading: bool,
    /// The response data (if successful)
    pub data: Option<T>,
    /// Error message (if failed)
    pub error: Option<String>,
}

impl<T> UseRequestState<T> {
    /// Returns true if the request is currently loading.
    pub fn is_loading(&self) -> bool {
        self.is_loading
    }

    /// Returns true if no request has been made yet (initial state).
    pub fn is_idle(&self) -> bool {
        !self.is_loading && self.data.is_none() && self.error.is_none()
    }

    /// Returns true if the request completed successfully.
    pub fn is_success(&self) -> bool {
        self.data.is_some()
    }

    /// Returns true if the request failed.
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }
}

/// Hook to access request state tracking directly.
///
/// This returns a read-only signal containing all request states.
/// Use this if you need more control than `use_request` provides.
///
/// # Panics
///
/// Panics if called outside of a `SyncProvider` context.
pub fn use_request_state() -> ReadSignal<HashMap<u64, RequestState>> {
    let ctx = expect_context::<SyncContext>();
    ctx.requests()
}

/// Hook for sending requests with a response handler callback.
///
/// This is a convenience hook that wraps `use_request` and sets up
/// an Effect that calls your handler exactly once per response.
/// The handler receives `Result<&ResponseMessage, &str>`.
///
/// Returns just the send function (not the state signal).
///
/// # Panics
///
/// Panics if called outside of a `SyncProvider` context.
///
/// # Example
///
/// ```rust,ignore
/// let load = use_request_with_handler::<LoadProgram, _>(move |result| {
///     match result {
///         Ok(r) if r.success => toast.success("Program loaded"),
///         Ok(r) => toast.error(format!("Failed: {}", r.error.as_deref().unwrap_or(""))),
///         Err(e) => toast.error(format!("Error: {e}")),
///     }
/// });
///
/// load(LoadProgram { program_id: 42 });
/// ```
pub fn use_request_with_handler<R, F>(handler: F) -> impl Fn(R) + Clone
where
    R: pl3xus_common::RequestMessage + Clone + 'static,
    F: Fn(Result<&R::ResponseMessage, &str>) + Clone + 'static,
{
    let (send, state) = use_request::<R>();

    // Track whether the current response has been processed
    let processed = RwSignal::new(false);

    // Set up the Effect that calls the handler exactly once per response
    Effect::new(move |_| {
        let current_state = state.get();

        // Reset processed flag when a new request starts loading
        if current_state.is_loading() {
            processed.set(false);
            return;
        }

        // Skip if idle (no request made yet) or already processed
        if current_state.is_idle() || processed.get_untracked() {
            return;
        }

        // Mark as processed before calling handler
        processed.set(true);

        // Call the handler with the result
        if let Some(ref error) = current_state.error {
            handler(Err(error.as_str()));
        } else if let Some(ref data) = current_state.data {
            handler(Ok(data));
        }
    });

    send
}

/// Hook for sending targeted requests to specific entities.
///
/// Returns a tuple of:
/// - A function to send the request (takes entity_bits and request)
/// - A reactive signal containing the current request state
///
/// This is similar to `use_request` but for targeted requests that are
/// directed at a specific entity. The server will authorize the request
/// based on whether the client has control of the target entity.
///
/// # Panics
///
/// Panics if called outside of a `SyncProvider` context.
///
/// # Example
///
/// ```rust,ignore
/// use pl3xus_client::use_targeted_request;
/// use pl3xus_common::RequestMessage;
///
/// #[derive(Clone, Serialize, Deserialize, Debug)]
/// struct SetSpeedOverride { value: f32 }
///
/// #[derive(Clone, Serialize, Deserialize, Debug)]
/// struct SetSpeedOverrideResponse { success: bool }
///
/// impl RequestMessage for SetSpeedOverride {
///     type ResponseMessage = SetSpeedOverrideResponse;
/// }
///
/// #[component]
/// fn SpeedControl(entity_bits: u64) -> impl IntoView {
///     let (send, state) = use_targeted_request::<SetSpeedOverride>();
///
///     let set_speed = move |_| {
///         send(entity_bits, SetSpeedOverride { value: 50.0 });
///     };
///
///     view! {
///         <button on:click=set_speed disabled=move || state.get().is_loading>
///             "Set Speed"
///         </button>
///         <Show when=move || state.get().is_success()>
///             <p>"Speed updated!"</p>
///         </Show>
///     }
/// }
/// ```
pub fn use_targeted_request<R>() -> (
    impl Fn(u64, R) + Clone,
    Signal<UseRequestState<R::ResponseMessage>>,
)
where
    R: pl3xus_common::RequestMessage + Clone + 'static,
{
    let ctx = expect_context::<SyncContext>();

    // Track the current request ID
    let current_request_id = RwSignal::new(None::<u64>);

    // Derive state from the context's request tracking
    let state = {
        let ctx = ctx.clone();
        Signal::derive(move || {
            let request_id = current_request_id.get();

            match request_id {
                None => UseRequestState {
                    is_loading: false,
                    data: None,
                    error: None,
                },
                Some(id) => {
                    let requests = ctx.requests.get();
                    match requests.get(&id) {
                        None => UseRequestState {
                            is_loading: false,
                            data: None,
                            error: Some("Request not found".to_string()),
                        },
                        Some(req_state) => {
                            match &req_state.status {
                                RequestStatus::Pending => UseRequestState {
                                    is_loading: true,
                                    data: None,
                                    error: None,
                                },
                                RequestStatus::Success => {
                                    let data = ctx.get_response::<R>(id);
                                    UseRequestState {
                                        is_loading: false,
                                        data,
                                        error: None,
                                    }
                                }
                                RequestStatus::Error(e) => UseRequestState {
                                    is_loading: false,
                                    data: None,
                                    error: Some(e.clone()),
                                },
                            }
                        }
                    }
                }
            }
        })
    };

    // Create the send function
    let send = move |entity_bits: u64, request: R| {
        #[cfg(target_arch = "wasm32")]
        leptos::logging::log!(
            "[use_targeted_request] sending request type: {} to entity: {}",
            R::request_name(),
            entity_bits
        );
        let id = ctx.targeted_request(entity_bits, request);
        #[cfg(target_arch = "wasm32")]
        leptos::logging::log!("[use_targeted_request] request sent with id: {}", id);
        current_request_id.set(Some(id));
    };

    (send, state)
}

/// Hook for sending targeted requests with a response handler callback.
///
/// This is a convenience wrapper around `use_targeted_request` that automatically
/// sets up response handling with proper deduplication. The handler is called
/// exactly once per response, avoiding duplicate processing when Effects re-run.
///
/// # Arguments
///
/// * `handler` - A callback that receives `Result<&ResponseMessage, &str>` where:
///   - `Ok(response)` - The request succeeded (transport-level), check `response.success` for business logic
///   - `Err(error)` - Transport-level error message
///
/// # Returns
///
/// A send function that takes `(entity_id: u64, request: R)`.
///
/// # Example
///
/// ```rust,ignore
/// use pl3xus_client::use_targeted_request_with_handler;
///
/// let toast = use_toast();
///
/// let send_abort = use_targeted_request_with_handler::<AbortMotion, _>(move |result| {
///     match result {
///         Ok(response) if response.success => toast.warning("Motion aborted"),
///         Ok(response) => toast.error(format!("Denied: {}", response.error.as_deref().unwrap_or(""))),
///         Err(e) => toast.error(format!("Failed: {}", e)),
///     }
/// });
///
/// // Later, send the request:
/// send_abort(entity_id, AbortMotion);
/// ```
pub fn use_targeted_request_with_handler<R, F>(handler: F) -> impl Fn(u64, R) + Clone
where
    R: pl3xus_common::RequestMessage + Clone + 'static,
    F: Fn(Result<&R::ResponseMessage, &str>) + Clone + 'static,
{
    let (send, state) = use_targeted_request::<R>();

    // Track whether the current response has been processed
    let processed = RwSignal::new(false);

    // Set up the Effect that calls the handler exactly once per response
    Effect::new(move |_| {
        let current_state = state.get();

        // Reset processed flag when a new request starts loading
        if current_state.is_loading() {
            processed.set(false);
            return;
        }

        // Skip if idle (no request made yet) or already processed
        if current_state.is_idle() || processed.get_untracked() {
            return;
        }

        // Mark as processed before calling handler
        processed.set(true);

        // Call the handler with the result
        if let Some(ref error) = current_state.error {
            handler(Err(error.as_str()));
        } else if let Some(ref data) = current_state.data {
            handler(Ok(data));
        }
    });

    send
}

// ============================================================================
// MUTATION API (TanStack Query-inspired)
// ============================================================================

/// Handle returned by `use_mutation` for sending mutations and accessing state.
///
/// This provides an ergonomic API for mutations (write operations) with:
/// - A `send` method to trigger the mutation
/// - State accessors (`is_loading`, `is_idle`, `is_success`, `is_error`)
/// - Access to response data and errors
///
/// The handle is `Copy`, so it can be used directly in multiple closures without cloning.
///
/// # Example
///
/// ```rust,ignore
/// let create = use_mutation::<CreateProgram>(move |result| {
///     match result {
///         Ok(r) if r.success => toast.success("Created!"),
///         Ok(r) => toast.error(format!("Failed: {}", r.error())),
///         Err(e) => toast.error(format!("Error: {e}")),
///     }
/// });
///
/// // Send the mutation
/// create.send(CreateProgram { name: "test".into() });
///
/// // Check state
/// if create.is_loading() { /* show spinner */ }
/// ```
pub struct MutationHandle<R>
where
    R: pl3xus_common::RequestMessage + Clone + 'static,
{
    send_fn: StoredValue<Box<dyn Fn(R) + Send + Sync>>,
    state: Signal<UseRequestState<R::ResponseMessage>>,
}

// Manual Clone/Copy implementations because StoredValue is Copy
// but derive(Copy) doesn't work with Box<dyn Fn>
impl<R> Clone for MutationHandle<R>
where
    R: pl3xus_common::RequestMessage + Clone + 'static,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<R> Copy for MutationHandle<R> where R: pl3xus_common::RequestMessage + Clone + 'static {}

impl<R> MutationHandle<R>
where
    R: pl3xus_common::RequestMessage + Clone + 'static,
{
    /// Send the mutation request.
    pub fn send(&self, request: R) {
        self.send_fn.with_value(|f| f(request));
    }

    /// Returns true if the mutation is currently in flight.
    pub fn is_loading(&self) -> bool {
        self.state.get().is_loading()
    }

    /// Returns true if no mutation has been sent yet.
    pub fn is_idle(&self) -> bool {
        self.state.get().is_idle()
    }

    /// Returns true if the last mutation completed successfully (transport-level).
    pub fn is_success(&self) -> bool {
        self.state.get().is_success()
    }

    /// Returns true if the last mutation failed (transport-level).
    pub fn is_error(&self) -> bool {
        self.state.get().is_error()
    }

    /// Get the response data from the last successful mutation.
    pub fn data(&self) -> Option<R::ResponseMessage> {
        self.state.get().data.clone()
    }

    /// Get the error message from the last failed mutation.
    pub fn error(&self) -> Option<String> {
        self.state.get().error.clone()
    }

    /// Get the raw state signal for advanced use cases.
    pub fn state(&self) -> Signal<UseRequestState<R::ResponseMessage>> {
        self.state
    }
}

/// Hook for mutations (write operations) with a response handler.
///
/// This is the primary hook for sending mutations to the server. It provides:
/// - Automatic response deduplication (handler called exactly once per response)
/// - Ergonomic `MutationHandle` with state accessors
/// - Type-safe request/response handling
///
/// # Arguments
///
/// * `handler` - Callback receiving `Result<&ResponseMessage, &str>`:
///   - `Ok(response)` - Transport succeeded, check `response.success` for business logic
///   - `Err(error)` - Transport-level error
///
/// # Returns
///
/// A `MutationHandle` with `send()` method and state accessors.
///
/// # Panics
///
/// Panics if called outside of a `SyncProvider` context.
///
/// # Example
///
/// ```rust,ignore
/// use pl3xus_client::use_mutation;
///
/// let toast = use_toast();
///
/// let load = use_mutation::<LoadProgram>(move |result| {
///     match result {
///         Ok(r) if r.success => toast.success("Program loaded"),
///         Ok(r) => toast.error(format!("Failed: {}", r.error.as_deref().unwrap_or(""))),
///         Err(e) => toast.error(format!("Error: {e}")),
///     }
/// });
///
/// // Send the mutation
/// load.send(LoadProgram { program_id: 42 });
///
/// // Access state
/// view! {
///     <button disabled=move || load.is_loading()>
///         "Load"
///     </button>
/// }
/// ```
pub fn use_mutation<R>(
    handler: impl Fn(Result<&R::ResponseMessage, &str>) + Clone + 'static,
) -> MutationHandle<R>
where
    R: pl3xus_common::RequestMessage + Clone + 'static,
{
    let (send, state) = use_request::<R>();

    // Track whether the current response has been processed
    let processed = RwSignal::new(false);

    // Set up the Effect that calls the handler exactly once per response
    Effect::new(move |_| {
        let current_state = state.get();

        // Reset processed flag when a new request starts loading
        if current_state.is_loading() {
            processed.set(false);
            return;
        }

        // Skip if idle (no request made yet) or already processed
        if current_state.is_idle() || processed.get_untracked() {
            return;
        }

        // Mark as processed before calling handler
        processed.set(true);

        // Call the handler with the result
        if let Some(ref error) = current_state.error {
            handler(Err(error.as_str()));
        } else if let Some(ref data) = current_state.data {
            handler(Ok(data));
        }
    });

    // Store the send function in a StoredValue to make the handle Copy
    let send_fn = StoredValue::new(Box::new(send) as Box<dyn Fn(R) + Send + Sync>);

    MutationHandle { send_fn, state }
}

/// Handle returned by `use_mutation_targeted` for sending entity-targeted mutations.
///
/// Similar to `MutationHandle` but the `send` method takes an entity ID.
///
/// The handle is `Copy`, so it can be used directly in multiple closures without cloning.
///
/// # Example
///
/// ```rust,ignore
/// let set_speed = use_mutation_targeted::<SetSpeedOverride>(move |result| {
///     match result {
///         Ok(r) if r.success => toast.success("Speed set"),
///         Ok(r) => toast.error(format!("Denied: {}", r.error())),
///         Err(e) => toast.error(format!("Error: {e}")),
///     }
/// });
///
/// // Send to specific entity
/// set_speed.send(entity_id, SetSpeedOverride { value: 50.0 });
/// ```
pub struct TargetedMutationHandle<R>
where
    R: pl3xus_common::RequestMessage + Clone + 'static,
{
    send_fn: StoredValue<Box<dyn Fn(u64, R) + Send + Sync>>,
    state: Signal<UseRequestState<R::ResponseMessage>>,
}

// Manual Clone/Copy implementations because StoredValue is Copy
// but derive(Copy) doesn't work with Box<dyn Fn>
impl<R> Clone for TargetedMutationHandle<R>
where
    R: pl3xus_common::RequestMessage + Clone + 'static,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<R> Copy for TargetedMutationHandle<R> where R: pl3xus_common::RequestMessage + Clone + 'static {}

impl<R> TargetedMutationHandle<R>
where
    R: pl3xus_common::RequestMessage + Clone + 'static,
{
    /// Send the mutation request to a specific entity.
    pub fn send(&self, entity_id: u64, request: R) {
        self.send_fn.with_value(|f| f(entity_id, request));
    }

    /// Returns true if the mutation is currently in flight.
    pub fn is_loading(&self) -> bool {
        self.state.get().is_loading()
    }

    /// Returns true if no mutation has been sent yet.
    pub fn is_idle(&self) -> bool {
        self.state.get().is_idle()
    }

    /// Returns true if the last mutation completed successfully (transport-level).
    pub fn is_success(&self) -> bool {
        self.state.get().is_success()
    }

    /// Returns true if the last mutation failed (transport-level).
    pub fn is_error(&self) -> bool {
        self.state.get().is_error()
    }

    /// Get the response data from the last successful mutation.
    pub fn data(&self) -> Option<R::ResponseMessage> {
        self.state.get().data.clone()
    }

    /// Get the error message from the last failed mutation.
    pub fn error(&self) -> Option<String> {
        self.state.get().error.clone()
    }

    /// Get the raw state signal for advanced use cases.
    pub fn state(&self) -> Signal<UseRequestState<R::ResponseMessage>> {
        self.state
    }
}

/// Hook for entity-targeted mutations with a response handler.
///
/// This is the primary hook for sending mutations to specific entities. It provides:
/// - Entity-targeted requests with server-side authorization
/// - Automatic response deduplication (handler called exactly once per response)
/// - Ergonomic `TargetedMutationHandle` with state accessors
///
/// # Arguments
///
/// * `handler` - Callback receiving `Result<&ResponseMessage, &str>`:
///   - `Ok(response)` - Transport succeeded, check `response.success` for business logic
///   - `Err(error)` - Transport-level error (including authorization failures)
///
/// # Returns
///
/// A `TargetedMutationHandle` with `send(entity_id, request)` method and state accessors.
///
/// # Panics
///
/// Panics if called outside of a `SyncProvider` context.
///
/// # Example
///
/// ```rust,ignore
/// use pl3xus_client::use_mutation_targeted;
///
/// let toast = use_toast();
///
/// let abort = use_mutation_targeted::<AbortMotion>(move |result| {
///     match result {
///         Ok(r) if r.success => toast.warning("Motion aborted"),
///         Ok(r) => toast.error(format!("Denied: {}", r.error.as_deref().unwrap_or(""))),
///         Err(e) => toast.error(format!("Error: {e}")),
///     }
/// });
///
/// // Send to specific entity
/// abort.send(robot_entity_id, AbortMotion);
///
/// // Access state
/// view! {
///     <button disabled=move || abort.is_loading()>
///         "Abort"
///     </button>
/// }
/// ```
pub fn use_mutation_targeted<R>(
    handler: impl Fn(Result<&R::ResponseMessage, &str>) + Clone + 'static,
) -> TargetedMutationHandle<R>
where
    R: pl3xus_common::RequestMessage + Clone + 'static,
{
    let (send, state) = use_targeted_request::<R>();

    // Track whether the current response has been processed
    let processed = RwSignal::new(false);

    // Set up the Effect that calls the handler exactly once per response
    Effect::new(move |_| {
        let current_state = state.get();

        // Reset processed flag when a new request starts loading
        if current_state.is_loading() {
            processed.set(false);
            return;
        }

        // Skip if idle (no request made yet) or already processed
        if current_state.is_idle() || processed.get_untracked() {
            return;
        }

        // Mark as processed before calling handler
        processed.set(true);

        // Call the handler with the result
        if let Some(ref error) = current_state.error {
            handler(Err(error.as_str()));
        } else if let Some(ref data) = current_state.data {
            handler(Ok(data));
        }
    });

    // Store the send function in a StoredValue to make the handle Copy
    let send_fn = StoredValue::new(Box::new(send) as Box<dyn Fn(u64, R) + Send + Sync>);

    TargetedMutationHandle { send_fn, state }
}