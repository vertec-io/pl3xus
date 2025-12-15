//! DevTools UI components
//!
//! This module contains the DevTools widget and related UI components.

use crate::client_type_registry::ClientTypeRegistry;
use crate::devtools::sync::{DevtoolsSync, use_sync};

use pl3xus_common::codec::Pl3xusBincodeCodec;
use pl3xus_common::NetworkPacket;
use leptos::prelude::*;
use leptos::html::Input;
use leptos::web_sys::console;
use leptos_use::{
    core::ConnectionReadyState,
    use_websocket_with_options,
    DummyEncoder,
    UseWebSocketOptions,
    UseWebSocketReturn,
};
use reactive_graph::traits::{Get, Update};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;

use pl3xus_sync::{
    SerializableEntity,
    SyncClientMessage,
    SyncItem,
    SyncServerMessage,
    SubscriptionRequest,
};

    fn entity_label(id: u64, components: &HashMap<String, JsonValue>) -> String {
        for value in components.values() {
            if let JsonValue::Object(obj) = value {
                if let Some(JsonValue::String(name)) =
                    obj.get("name").or_else(|| obj.get("label"))
                {
                    return format!("{name} · #{id}");
                }
            }
        }
        format!("Entity #{id}")
    }

    fn parse_number_like(original: &serde_json::Number, text: &str) -> Option<serde_json::Number> {
        if original.is_i64() {
            text.parse::<i64>().ok().map(serde_json::Number::from)
        } else if original.is_u64() {
            text.parse::<u64>().ok().map(serde_json::Number::from)
        } else {
            text
                .parse::<f64>()
                .ok()
                .and_then(serde_json::Number::from_f64)
        }
    }

    fn apply_field_update(
        entities: RwSignal<HashMap<u64, HashMap<String, JsonValue>>>,
        sync: RwSignal<DevtoolsSync>,
        entity_bits: u64,
        component_type: String,
        field_name: String,
        new_value: JsonValue,
    ) {
        let mut updated_component: Option<JsonValue> = None;

        entities.update(|map| {
            if let Some(components) = map.get_mut(&entity_bits) {
                if let Some(component_value) = components.get_mut(&component_type) {
                    match component_value {
                        JsonValue::Object(obj) => {
                            obj.insert(field_name.clone(), new_value.clone());
                        }
                        _ => {
                            *component_value = new_value.clone();
                        }
                    }
                    updated_component = Some(component_value.clone());
                }
            }
        });

        if let Some(component_json) = updated_component {
            sync.get().mutate(
                SerializableEntity { bits: entity_bits },
                component_type,
                component_json,
            );
        }
    }

    fn component_editor(
        entity_bits: u64,
        component_type: String,
        entities: RwSignal<HashMap<u64, HashMap<String, JsonValue>>>,
        sync: RwSignal<DevtoolsSync>,
    ) -> impl IntoView {
        let component_type_for_fields = component_type.clone();

        // Get field names only (stable keys for For component)
        // Use .get_untracked() to avoid creating reactive dependency that would cause For to re-run
        let field_names = move || {
            entities
                .get_untracked()
                .get(&entity_bits)
                .and_then(|components| components.get(&component_type_for_fields))
                .and_then(|value| {
                    if let JsonValue::Object(obj) = value {
                        let mut keys: Vec<String> = obj.keys().cloned().collect();
                        keys.sort();
                        Some(keys)
                    } else {
                        None
                    }
                })
                .unwrap_or_default()
        };

        view! {
            <div class="space-y-2">
                <For
                    each=field_names
                    key=|name| name.clone()
                    children=move |field_name: String| {
                        // Get the initial field value to determine the field type
                        let initial_field_value = entities
                            .get_untracked()
                            .get(&entity_bits)
                            .and_then(|c| c.get(&component_type))
                            .and_then(|v| {
                                if let JsonValue::Object(obj) = v {
                                    obj.get(&field_name).cloned()
                                } else {
                                    None
                                }
                            });

                        // Render different field types
                        match initial_field_value {
                            Some(JsonValue::Bool(initial_bool)) => {
                                // Boolean fields: use checkbox with immediate updates
                                let checkbox_ref = NodeRef::<Input>::new();

                                // Effect to update checkbox when server value changes
                                Effect::new({
                                    let field_name = field_name.clone();
                                    let checkbox_ref = checkbox_ref.clone();
                                    let component_type = component_type.clone();

                                    move |_| {
                                        if let Some(value) = entities.get()
                                            .get(&entity_bits)
                                            .and_then(|c| c.get(&component_type))
                                            .and_then(|v| {
                                                if let JsonValue::Object(obj) = v {
                                                    obj.get(&field_name).and_then(|v| v.as_bool())
                                                } else {
                                                    None
                                                }
                                            })
                                        {
                                            if let Some(checkbox) = checkbox_ref.get() {
                                                checkbox.set_checked(value);
                                            }
                                        }
                                    }
                                });

                                let component_type_for_handler = component_type.clone();
                                let field_name_for_handler = field_name.clone();

                                view! {
                                    <div class="flex items-center justify-between gap-2">
                                        <span class="text-[11px] text-slate-300">{field_name.clone()}</span>
                                        <input
                                            node_ref=checkbox_ref
                                            type="checkbox"
                                            class="h-3 w-3 rounded border-slate-600 bg-slate-950"
                                            prop:checked=initial_bool
                                            on:input=move |ev| {
                                                let value = event_target_checked(&ev);
                                                apply_field_update(
                                                    entities,
                                                    sync,
                                                    entity_bits,
                                                    component_type_for_handler.clone(),
                                                    field_name_for_handler.clone(),
                                                    JsonValue::Bool(value),
                                                );
                                            }
                                        />
                                    </div>
                                }.into_any()
                            }
                            Some(JsonValue::Number(initial_num)) => {
                                // Number fields: use text input with focus tracking
                                let input_ref = NodeRef::<Input>::new();
                                let is_focused = RwSignal::new(false);

                                // Effect to update input when server value changes (only when NOT focused)
                                Effect::new({
                                    let field_name = field_name.clone();
                                    let input_ref = input_ref.clone();
                                    let component_type = component_type.clone();

                                    move |_| {
                                        if let Some(value) = entities.get()
                                            .get(&entity_bits)
                                            .and_then(|c| c.get(&component_type))
                                            .and_then(|v| {
                                                if let JsonValue::Object(obj) = v {
                                                    obj.get(&field_name).and_then(|v| v.as_number())
                                                } else {
                                                    None
                                                }
                                            })
                                        {
                                            // Only update DOM if input is NOT focused
                                            if !is_focused.get_untracked() {
                                                if let Some(input) = input_ref.get() {
                                                    let new_value = value.to_string();
                                                    // Only update if value actually changed
                                                    if input.value() != new_value {
                                                        input.set_value(&new_value);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                });

                                let initial_value = initial_num.to_string();
                                let component_type_for_blur = component_type.clone();
                                let field_name_for_blur = field_name.clone();
                                let component_type_for_keydown = component_type.clone();
                                let field_name_for_keydown = field_name.clone();

                                view! {
                                    <div class="space-y-1">
                                        <div class="text-[11px] text-slate-300">{field_name.clone()}</div>
                                        <input
                                            node_ref=input_ref
                                            class="w-full rounded-md bg-slate-950/70 border border-slate-700 px-2 py-1 text-[11px] focus:outline-none focus:ring-1 focus:ring-indigo-500 focus:border-indigo-500"
                                            value=initial_value
                                            on:focus=move |_| is_focused.set(true)
                                            on:blur=move |_| {
                                                is_focused.set(false);
                                                // On blur: revert to latest server value
                                                if let Some(server_value) = entities.get_untracked()
                                                    .get(&entity_bits)
                                                    .and_then(|c| c.get(&component_type_for_blur))
                                                    .and_then(|v| {
                                                        if let JsonValue::Object(obj) = v {
                                                            obj.get(&field_name_for_blur).and_then(|v| v.as_number())
                                                        } else {
                                                            None
                                                        }
                                                    })
                                                {
                                                    if let Some(input) = input_ref.get_untracked() {
                                                        input.set_value(&server_value.to_string());
                                                    }
                                                }
                                            }
                                            on:keydown=move |ev| {
                                                // On Enter: apply mutation
                                                if ev.key() == "Enter" {
                                                    let raw = event_target_value(&ev);
                                                    if let Some(num) = parse_number_like(&initial_num, &raw) {
                                                        apply_field_update(
                                                            entities,
                                                            sync,
                                                            entity_bits,
                                                            component_type_for_keydown.clone(),
                                                            field_name_for_keydown.clone(),
                                                            JsonValue::Number(num),
                                                        );
                                                        // Blur the input to trigger revert (in case server rejects)
                                                        if let Some(input) = input_ref.get_untracked() {
                                                            let _ = input.blur();
                                                        }
                                                    }
                                                }
                                            }
                                        />
                                    </div>
                                }.into_any()
                            }
                            Some(JsonValue::String(initial_str)) => {
                                // String fields: use text input with focus tracking
                                let input_ref = NodeRef::<Input>::new();
                                let is_focused = RwSignal::new(false);

                                // Effect to update input when server value changes (only when NOT focused)
                                Effect::new({
                                    let field_name = field_name.clone();
                                    let input_ref = input_ref.clone();
                                    let component_type = component_type.clone();

                                    move |_| {
                                        if let Some(value) = entities.get()
                                            .get(&entity_bits)
                                            .and_then(|c| c.get(&component_type))
                                            .and_then(|v| {
                                                if let JsonValue::Object(obj) = v {
                                                    obj.get(&field_name).and_then(|v| v.as_str())
                                                } else {
                                                    None
                                                }
                                            })
                                        {
                                            // Only update DOM if input is NOT focused
                                            if !is_focused.get_untracked() {
                                                if let Some(input) = input_ref.get() {
                                                    // Only update if value actually changed
                                                    if input.value() != value {
                                                        input.set_value(value);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                });

                                let component_type_for_blur = component_type.clone();
                                let field_name_for_blur = field_name.clone();
                                let component_type_for_keydown = component_type.clone();
                                let field_name_for_keydown = field_name.clone();

                                view! {
                                    <div class="space-y-1">
                                        <div class="text-[11px] text-slate-300">{field_name.clone()}</div>
                                        <input
                                            node_ref=input_ref
                                            class="w-full rounded-md bg-slate-950/70 border border-slate-700 px-2 py-1 text-[11px] focus:outline-none focus:ring-1 focus:ring-indigo-500 focus:border-indigo-500"
                                            value=initial_str
                                            on:focus=move |_| is_focused.set(true)
                                            on:blur=move |_| {
                                                is_focused.set(false);
                                                // On blur: revert to latest server value
                                                if let Some(server_value) = entities.get_untracked()
                                                    .get(&entity_bits)
                                                    .and_then(|c| c.get(&component_type_for_blur))
                                                    .and_then(|v| {
                                                        if let JsonValue::Object(obj) = v {
                                                            obj.get(&field_name_for_blur).and_then(|v| v.as_str())
                                                        } else {
                                                            None
                                                        }
                                                    })
                                                {
                                                    if let Some(input) = input_ref.get_untracked() {
                                                        input.set_value(server_value);
                                                    }
                                                }
                                            }
                                            on:keydown=move |ev| {
                                                // On Enter: apply mutation
                                                if ev.key() == "Enter" {
                                                    let raw = event_target_value(&ev);
                                                    apply_field_update(
                                                        entities,
                                                        sync,
                                                        entity_bits,
                                                        component_type_for_keydown.clone(),
                                                        field_name_for_keydown.clone(),
                                                        JsonValue::String(raw),
                                                    );
                                                    // Blur the input to trigger revert (in case server rejects)
                                                    if let Some(input) = input_ref.get_untracked() {
                                                        let _ = input.blur();
                                                    }
                                                }
                                            }
                                        />
                                    </div>
                                }.into_any()
                            }
                            Some(other) => {
                                // Other types: read-only JSON display
                                let json = serde_json::to_string_pretty(&other).unwrap_or_default();
                                view! {
                                    <div class="space-y-1">
                                        <div class="text-[11px] text-slate-300">{field_name.clone()}</div>
                                        <pre class="mt-0.5 bg-slate-950/60 border border-slate-800 rounded p-1 font-mono text-[10px] whitespace-pre-wrap break-all">{json}</pre>
                                    </div>
                                }.into_any()
                            }
                            None => {
                                // Field not found
                                view! {
                                    <div class="space-y-1">
                                        <div class="text-[11px] text-slate-300">{field_name.clone()}</div>
                                        <div class="text-[10px] text-slate-500">"(field not found)"</div>
                                    </div>
                                }.into_any()
                            }
                        }
                    }
                />
            </div>
        }
    }

    /// Display mode for the DevTools component
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum DevToolsMode {
        /// Floating widget in lower-left corner (default)
        Widget,
        /// Full-page embedded view
        Embedded,
    }

    impl Default for DevToolsMode {
        fn default() -> Self {
            Self::Widget
        }
    }

    /// High-level DevTools surface: given a WebSocket URL speaking the
    /// `pl3xus_sync` wire protocol, render a modern Tailwind-powered
    /// inspector + mutation console.
    ///
    /// # Parameters
    /// - `ws_url`: WebSocket URL to connect to
    /// - `registry`: Type registry for deserializing component data
    /// - `mode`: Display mode (Widget or Embedded). Defaults to Widget.
    #[component]
    pub fn DevTools(
        ws_url: &'static str,
        registry: Arc<ClientTypeRegistry>,
        #[prop(optional)] mode: DevToolsMode,
    ) -> impl IntoView {
        // Check if DevTools support is enabled in the registry
        let devtools_support_enabled = registry.is_devtools_support_enabled();
        if !devtools_support_enabled {
            console::error_1(&"[DevTools] ERROR: ClientTypeRegistry was not built with .with_devtools_support()! DevTools will not function correctly. Please add .with_devtools_support() to your registry builder.".into());
        }

        // Connection + debug state
        let (last_incoming, set_last_incoming) = signal(String::new());
        let (last_error, set_last_error) = signal(Option::<String>::None);
        let (message_expanded, set_message_expanded) = signal(false);
        let (message_flash, set_message_flash) = signal(false);

        // Widget state (for floating mode)
        let (widget_expanded, set_widget_expanded) = signal(false);

        // Live entity/component view built from incoming SyncBatch items.
        let entities = RwSignal::new(HashMap::<u64, HashMap<String, JsonValue>>::new());

        // Client-side subscription tracking so we can render and cancel them.
        let next_subscription_id = RwSignal::new(0_u64);
        let subscriptions = RwSignal::new(Vec::<SubscriptionRequest>::new());
        let selected_entity = RwSignal::new(None::<u64>);
        let auto_subscription_id = RwSignal::new(None::<u64>);


        // Use Pl3xusBincodeCodec to receive NetworkPacket directly
        // This gives us better error handling and debugging
        let UseWebSocketReturn { ready_state, message: raw_message, send: raw_send, open, close, .. } =
            use_websocket_with_options::<
                NetworkPacket,
                NetworkPacket,
                Pl3xusBincodeCodec,
                (),
                DummyEncoder,
            >(
                ws_url,
                UseWebSocketOptions::default()
                    .immediate(true)  // Auto-connect immediately
                    .on_open(move |_| {
                        console::log_1(&"[DevTools] WebSocket opened!".into());
                        set_last_error.set(None);
                    })
                    .on_error(move |e| {
                        console::error_1(&format!("[DevTools] WebSocket error: {e:?}").into());
                        set_last_error.set(Some(format!("{e:?}")));
                    }),
            );

        // Unwrap NetworkPacket and deserialize to SyncServerMessage
        let message = Signal::derive(move || {
            raw_message.with(|packet_opt| {
                console::log_1(&format!("[DevTools] raw_message signal fired, packet present: {}", packet_opt.is_some()).into());

                packet_opt.as_ref().and_then(|packet| {
                    console::log_1(&format!("[DevTools] Received NetworkPacket: type_name={}, schema_hash={}, data_len={}", packet.type_name, packet.schema_hash, packet.data.len()).into());

                    // Use bincode v2 serde API with standard config
                    match bincode::serde::decode_from_slice(&packet.data, bincode::config::standard()) {
                        Ok((msg, _)) => {
                            console::log_1(&format!("[DevTools] Successfully deserialized SyncServerMessage").into());
                            Some(msg)
                        },
                        Err(e) => {
                            console::error_1(&format!("[DevTools] Failed to deserialize SyncServerMessage from NetworkPacket: {:?}", e).into());
                            console::error_1(&format!("[DevTools] NetworkPacket: type_name={}, schema_hash={}, data_len={}", packet.type_name, packet.schema_hash, packet.data.len()).into());
                            set_last_error.set(Some(format!("Deserialization error: {:?}", e)));
                            None
                        }
                    }
                })
            })
        });

        // Wrap send to serialize SyncClientMessage into NetworkPacket
        let send = move |msg: &SyncClientMessage| {
            let packet = NetworkPacket {
                type_name: std::any::type_name::<SyncClientMessage>().to_string(),
                schema_hash: 0, // TODO: compute proper schema hash
                data: bincode::serde::encode_to_vec(msg, bincode::config::standard()).unwrap(),
            };
            raw_send(&packet);
        };

        // General sync hook powered by the WebSocket transport.
        let sync = {
            let registry_clone = registry.clone();
            let s = use_sync(move |msg: SyncClientMessage| {
                send(&msg);
            }, registry_clone);
            RwSignal::new(s)
        };

        // Provide the DevtoolsSync via context so other components can use it
        provide_context(sync.get_untracked());

        // React to incoming server messages: update mutation state and
        // maintain a simple entity/component projection.
        {
            let sync = sync;
            let entities = entities;
            let set_last_incoming = set_last_incoming;
            let set_message_flash = set_message_flash;
            let registry = registry.clone();
            Effect::new(move |_| {
                message.with(|msg| {
                    if let Some(msg) = msg {
                        if let Ok(json) = serde_json::to_string_pretty(msg) {
                            set_last_incoming.set(json);
                        }

                        // Trigger flash animation
                        set_message_flash.set(true);
                        set_timeout(move || {
                            set_message_flash.set(false);
                        }, std::time::Duration::from_millis(300));

                        sync.get().handle_server_message(msg);
                        if let SyncServerMessage::SyncBatch(batch) = msg {
                            entities.update(|map| {
                                for item in &batch.items {
                                    match item {
                                        SyncItem::Snapshot { entity, component_type, value, .. }
                                        | SyncItem::Update { entity, component_type, value, .. } => {
                                            // Use the type registry to deserialize component data
                                            match registry.deserialize_to_json(component_type, value) {
                                                Ok(json_value) => {
                                                    map.entry(entity.bits)
                                                        .or_default()
                                                        .insert(component_type.clone(), json_value);
                                                }
                                                Err(e) => {
                                                    console::error_1(&format!("[DevTools] Failed to deserialize component '{}': {}", component_type, e).into());
                                                }
                                            }
                                        }
                                        SyncItem::ComponentRemoved { entity, component_type, .. } => {
                                            if let Some(entry) = map.get_mut(&entity.bits) {
                                                entry.remove(component_type);
                                                if entry.is_empty() {
                                                    map.remove(&entity.bits);
                                                }
                                            }
                                        }
                                        SyncItem::EntityRemoved { entity, .. } => {
                                            map.remove(&entity.bits);
                                        }
                                    }
                                }
                            });
                        }
                    }
                });
            });
        }
        // Automatically subscribe to all components once the WebSocket is open.
        {
            let sync = sync;
            let entities = entities;
            let subscriptions = subscriptions;
            let selected_entity = selected_entity;
            let auto_subscription_id = auto_subscription_id;
            let next_subscription_id = next_subscription_id;
            Effect::new(move |_| {
                let state = ready_state.get();
                if state == ConnectionReadyState::Open && auto_subscription_id.get().is_none() {
                    let id = next_subscription_id.get() + 1;
                    next_subscription_id.set(id);
                    let req = SubscriptionRequest { subscription_id: id, component_type: "*".to_string(), entity: None };
                    sync.get().send_raw(SyncClientMessage::Subscription(req.clone()));
                    auto_subscription_id.set(Some(id));
                    subscriptions.update(|subs| subs.push(req));
                } else if state != ConnectionReadyState::Open && auto_subscription_id.get().is_some() {
                    auto_subscription_id.set(None);
                    subscriptions.update(|subs| subs.clear());
                    entities.set(HashMap::new());
                    selected_entity.set(None);
                }
            });
        }


        let connection_label = move || match ready_state.get() {
            ConnectionReadyState::Connecting => "Connecting",
            ConnectionReadyState::Open => "Open",
            ConnectionReadyState::Closing => "Closing",
            ConnectionReadyState::Closed => "Closed",
        };

        // View mode: true = tree view, false = flat view
        let tree_view_mode = RwSignal::new(true);

        // Track which entities are expanded in tree view (entity_id -> is_expanded)
        // Default to expanded for all entities
        let expanded_entities = RwSignal::new(HashMap::<u64, bool>::new());

        let sorted_entities = move || {
            let mut v: Vec<_> = entities.get().into_iter().collect();
            v.sort_by_key(|(id, _)| *id);
            v
        };

        // Build tree structure from ParentEntity and ChildEntities components
        let entity_tree = move || {
            let all_entities = entities.get();
            let mut roots = Vec::new();
            let mut children_map: HashMap<u64, Vec<u64>> = HashMap::new();

            // First pass: collect all parent-child relationships
            for (entity_id, components) in &all_entities {
                // Check if this entity has a ParentEntity component
                if let Some(JsonValue::Object(parent_comp)) = components.get("ParentEntity") {
                    if let Some(JsonValue::Number(parent_bits)) = parent_comp.get("parent_bits") {
                        if let Some(parent_id) = parent_bits.as_u64() {
                            children_map.entry(parent_id).or_default().push(*entity_id);
                        }
                    }
                }
            }

            // Second pass: find root entities (those without ParentEntity)
            for (entity_id, components) in &all_entities {
                if !components.contains_key("ParentEntity") {
                    roots.push(*entity_id);
                }
            }

            // Sort roots and children for consistent ordering
            roots.sort();
            for children in children_map.values_mut() {
                children.sort();
            }

            (roots, children_map)
        };

        // The selected entity ID - this is the only thing we memoize to prevent
        // recreating the entire inspector view when the user clicks a different entity.
        // We DON'T memoize the component data because that would create a reactive
        // dependency on the entities signal, causing the Memo to recompute on every update.
        let selected_id = Memo::new(move |_| {
            let id = selected_entity.get();
            leptos::logging::log!("🔍 selected_id Memo recomputed: {:?}", id);
            id
        });

        // Render based on mode
        match mode {
            DevToolsMode::Embedded => {
                // Full-page embedded view with fixed viewport height
                view! {
            <div class="fixed inset-0 w-full h-full bg-gradient-to-b from-slate-950 via-slate-900 to-slate-950 text-slate-50 flex flex-col overflow-hidden">
                <header class="border-b border-white/5 bg-slate-900/80 backdrop-blur-sm shadow-sm px-4 py-3 flex items-center justify-between flex-shrink-0">
                    <div>
                        <h1 class="text-lg font-semibold tracking-tight">"Pl3xus DevTools"</h1>
                        <p class="text-xs text-slate-400">"Realtime ECS inspector & mutation console"</p>
                    </div>
                    <div class="flex items-center gap-3 text-xs">
                        <span class="px-2 py-1 rounded-full border border-slate-700 bg-slate-900">
                            {move || format!("{} · {}", connection_label(), ws_url)}
                        </span>
                        <button
                            class="px-3 py-1 rounded bg-emerald-500 text-slate-950 font-medium disabled:opacity-50"
                            on:click=move |_| open()
                            disabled=move || ready_state.get() == ConnectionReadyState::Open
                        >"Connect"</button>
                        <button
                            class="px-3 py-1 rounded bg-slate-700 text-slate-50 disabled:opacity-50"
                            on:click=move |_| close()
                            disabled=move || ready_state.get() != ConnectionReadyState::Open
                        >"Disconnect"</button>
                    </div>
                </header>

                // Error banner when DevTools support is not enabled
                <Show when=move || !devtools_support_enabled>
                    <div class="bg-red-900/80 border-b border-red-700 px-4 py-3 flex items-start gap-3 flex-shrink-0">
                        <svg class="w-5 h-5 text-red-400 flex-shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"></path>
                        </svg>
                        <div class="flex-1">
                            <h3 class="text-sm font-semibold text-red-200">"DevTools Support Not Enabled"</h3>
                            <p class="text-xs text-red-300 mt-1">
                                "The ClientTypeRegistry was not built with "
                                <code class="px-1 py-0.5 bg-red-950/50 rounded font-mono text-red-200">".with_devtools_support()"</code>
                                ". DevTools will not function correctly. Please update your registry builder:"
                            </p>
                            <pre class="mt-2 text-[10px] font-mono bg-red-950/50 border border-red-800 rounded p-2 text-red-200">
"let registry = ClientTypeRegistry::builder()
    .register::<YourComponent>()
    .with_devtools_support()  // ← Add this line
    .build();"
                            </pre>
                        </div>
                    </div>
                </Show>

                <main class="flex-1 overflow-hidden grid grid-cols-12 gap-4 p-4 min-h-0">
                    <section class="col-span-3 flex flex-col gap-3 min-h-0">
                        <div class="rounded-2xl border border-white/5 bg-slate-900/70 backdrop-blur-sm shadow-lg shadow-black/40 p-3 flex flex-col min-h-0 h-full">
                            <div class="flex items-center justify-between mb-2 flex-shrink-0">
                                <h2 class="text-sm font-semibold text-slate-100">"World"</h2>
                                <div class="flex items-center gap-2">
                                    <button
                                        class="px-2 py-1 text-[10px] rounded border border-white/10 bg-slate-800/50 hover:bg-slate-700/50 transition-colors"
                                        on:click=move |_| tree_view_mode.update(|mode| *mode = !*mode)
                                    >
                                        {move || if tree_view_mode.get() { "Tree View" } else { "Flat View" }}
                                    </button>
                                    <Show when=move || tree_view_mode.get()>
                                        <button
                                            class="px-2 py-1 text-[10px] rounded border border-white/10 bg-slate-800/50 hover:bg-slate-700/50 transition-colors"
                                            on:click=move |_| {
                                                let (_, children_map) = entity_tree();

                                                // Check if all entities with children are expanded
                                                let all_expanded = expanded_entities.with(|map| {
                                                    children_map.keys().all(|id| map.get(id).copied().unwrap_or(true))
                                                });

                                                // Toggle all
                                                expanded_entities.update(|map| {
                                                    for entity_id in children_map.keys() {
                                                        map.insert(*entity_id, !all_expanded);
                                                    }
                                                });
                                            }
                                        >
                                            {move || {
                                                let (_, children_map) = entity_tree();
                                                let all_expanded = expanded_entities.with(|map| {
                                                    children_map.keys().all(|id| map.get(id).copied().unwrap_or(true))
                                                });
                                                if all_expanded { "Collapse All" } else { "Expand All" }
                                            }}
                                        </button>
                                    </Show>
                                    <span class="text-[11px] text-slate-400">
                                        {move || format!("{} entities", entities.get().len())}
                                    </span>
                                </div>
                            </div>
                            <div class="flex-1 overflow-y-auto space-y-1 text-xs min-h-0">
                                <Show
                                    when=move || !entities.get().is_empty()
                                    fallback=move || view! {
                                        <div class="text-[11px] text-slate-500">
                                            "No entities yet. Connect a Bevy app with Pl3xusSyncPlugin."
                                        </div>
                                    }
                                >
                                    <Show
                                        when=move || tree_view_mode.get()
                                        fallback=move || view! {
                                            // Flat view
                                            <For
                                                each=sorted_entities
                                                key=|(id, _)| *id
                                                children=move |(id, components): (u64, HashMap<String, JsonValue>)| {
                                                    let label = entity_label(id, &components);
                                                    let selected_entity = selected_entity;
                                                    view! {
                                                        <button
                                                            class=move || {
                                                        let is_selected = selected_entity.get() == Some(id);
                                                        let base = "w-full text-left px-2 py-1.5 rounded-md border transition-colors";
                                                        if is_selected {
                                                            format!("{base} bg-indigo-600/80 border-indigo-500 text-slate-50")
                                                        } else {
                                                            format!("{base} bg-slate-900/40 border-slate-800 text-slate-300 hover:bg-slate-800/70")
                                                        }
                                                    }
                                                    on:click=move |_| selected_entity.set(Some(id))
                                                >
                                                    <div class="flex items-center justify-between gap-2">
                                                        <span class="truncate text-[11px] font-medium">{label}</span>
                                                        <span class="text-[10px] text-slate-400">
                                                            {format!("{} comps", components.len())}
                                                        </span>
                                                    </div>
                                                    <div class="text-[10px] text-slate-500 font-mono mt-0.5">
                                                        "#"{id}
                                                    </div>
                                                </button>
                                            }
                                        }
                                    />
                                        }
                                    >
                                        // Tree view - render entities hierarchically with accordion
                                        {move || {
                                            let (roots, children_map) = entity_tree();
                                            let all_entities = entities.get();
                                            let expanded = expanded_entities;

                                            // Recursive function to render entity and its children
                                            fn render_entity_tree(
                                                entity_id: u64,
                                                components: &HashMap<String, JsonValue>,
                                                children_map: &HashMap<u64, Vec<u64>>,
                                                all_entities: &HashMap<u64, HashMap<String, JsonValue>>,
                                                selected_entity: RwSignal<Option<u64>>,
                                                expanded_entities: RwSignal<HashMap<u64, bool>>,
                                                depth: usize,
                                            ) -> Vec<AnyView> {
                                                let mut views = Vec::new();
                                                let label = entity_label(entity_id, components);
                                                let has_children = children_map.contains_key(&entity_id);

                                                // Check if this entity is expanded (default to true)
                                                let is_expanded = expanded_entities.with(|map| {
                                                    map.get(&entity_id).copied().unwrap_or(true)
                                                });

                                                // Render this entity with expand/collapse icon if it has children
                                                let entity_view = view! {
                                                    <div class="flex items-center gap-1">
                                                        {if has_children {
                                                            view! {
                                                                <button
                                                                    class="flex-shrink-0 w-4 h-4 flex items-center justify-center text-slate-400 hover:text-slate-200 transition-colors"
                                                                    on:click=move |e| {
                                                                        e.stop_propagation();
                                                                        expanded_entities.update(|map| {
                                                                            let current = map.get(&entity_id).copied().unwrap_or(true);
                                                                            map.insert(entity_id, !current);
                                                                        });
                                                                    }
                                                                >
                                                                    <span class="text-[10px]">
                                                                        {move || if expanded_entities.with(|map| map.get(&entity_id).copied().unwrap_or(true)) { "▼" } else { "▶" }}
                                                                    </span>
                                                                </button>
                                                            }.into_any()
                                                        } else {
                                                            view! {
                                                                <div class="w-4"></div>
                                                            }.into_any()
                                                        }}
                                                        <button
                                                            class=move || {
                                                                let is_selected = selected_entity.get() == Some(entity_id);
                                                                let base = "flex-1 text-left px-2 py-1.5 rounded-md border transition-colors";
                                                                if is_selected {
                                                                    format!("{base} bg-indigo-600/80 border-indigo-500 text-slate-50")
                                                                } else {
                                                                    format!("{base} bg-slate-900/40 border-slate-800 text-slate-300 hover:bg-slate-800/70")
                                                                }
                                                            }
                                                            on:click=move |_| selected_entity.set(Some(entity_id))
                                                        >
                                                            <div class="flex items-center justify-between gap-2">
                                                                <span class="truncate text-[11px] font-medium">{label}</span>
                                                                <span class="text-[10px] text-slate-400">
                                                                    {format!("{} comps", components.len())}
                                                                </span>
                                                            </div>
                                                            <div class="text-[10px] text-slate-500 font-mono mt-0.5">
                                                                "#"{entity_id}
                                                            </div>
                                                        </button>
                                                    </div>
                                                }.into_any();
                                                views.push(entity_view);

                                                // Render children recursively only if expanded
                                                if is_expanded {
                                                    if let Some(children) = children_map.get(&entity_id) {
                                                        for child_id in children {
                                                            if let Some(child_components) = all_entities.get(child_id) {
                                                                // Wrap children in a container with left margin for indentation
                                                                let child_views = view! {
                                                                    <div class="ml-4">
                                                                        {render_entity_tree(
                                                                            *child_id,
                                                                            child_components,
                                                                            children_map,
                                                                            all_entities,
                                                                            selected_entity,
                                                                            expanded_entities,
                                                                            depth + 1,
                                                                        )}
                                                                    </div>
                                                                }.into_any();
                                                                views.push(child_views);
                                                            }
                                                        }
                                                    }
                                                }

                                                views
                                            }

                                            // Render all root entities and their trees
                                            let mut all_views = Vec::new();
                                            for root_id in roots {
                                                if let Some(root_components) = all_entities.get(&root_id) {
                                                    let tree_views = render_entity_tree(
                                                        root_id,
                                                        root_components,
                                                        &children_map,
                                                        &all_entities,
                                                        selected_entity,
                                                        expanded,
                                                        0,
                                                    );
                                                    all_views.extend(tree_views);
                                                }
                                            }

                                            all_views
                                        }}
                                    </Show>
                                </Show>
                            </div>
                        </div>
                    </section>

                    <section class="col-span-6 rounded-2xl border border-white/5 bg-slate-900/70 backdrop-blur-sm shadow-lg shadow-black/40 p-4 flex flex-col min-h-0">
                        <h2 class="text-sm font-semibold text-slate-100 mb-2 flex-shrink-0">"Inspector"</h2>
                        <div class="flex-1 overflow-y-auto min-h-0">
                            <Show
                                when=move || selected_id.get().is_some()
                                fallback=move || view! {
                                    <div class="text-[11px] text-slate-500">
                                        "Select an entity from the left to inspect and edit its components."
                                    </div>
                                }
                            >
                                {move || {
                                    // Look up the selected entity ID
                                    let Some(id) = selected_id.get() else {
                                        return ().into_view().into_any();
                                    };

                                    // Get the entity label (use get_untracked to avoid recreating the entire view)
                                    let label = entities.get_untracked()
                                        .get(&id)
                                        .map(|components| entity_label(id, components))
                                        .unwrap_or_else(|| format!("Entity #{}", id));

                                    // Create a Memo that tracks only component type names (keys) for this entity
                                    // This will update when component types are added/removed, but NOT when values change
                                    let component_types = Memo::new(move |_| {
                                        entities
                                            .get()
                                            .get(&id)
                                            .map(|components| {
                                                let mut types: Vec<String> = components.keys().cloned().collect();
                                                types.sort();
                                                types
                                            })
                                            .unwrap_or_default()
                                    });

                                    view! {
                                        <div class="flex flex-col gap-3 text-xs">
                                            <div class="flex items-center justify-between">
                                                <div>
                                                    <div class="text-[11px] uppercase tracking-wide text-slate-500">"Entity"</div>
                                                    <div class="text-sm font-semibold text-slate-50">{label}</div>
                                                </div>
                                                <div class="text-[10px] text-slate-500 font-mono">
                                                    "#"{id}
                                                </div>
                                            </div>
                                            <div class="border-t border-slate-800 pt-3 space-y-3">
                                            <For
                                                each=move || component_types.get()
                                                key=|ty: &String| ty.clone()
                                                children=move |ty: String| {
                                                    let entities_for = entities;
                                                    let sync_for = sync;
                                                    let ty_for = ty.clone();
                                                    let ty_for_value = ty.clone();
                                                    let id_for = id;

                                                    let body: AnyView = {
                                                        // Check if this is an Object component (editable) or other (read-only)
                                                        // Use get_untracked() to avoid creating a reactive dependency
                                                        // We only need to check this once when the component is first rendered
                                                        let is_object = entities_for
                                                            .get_untracked()
                                                            .get(&id_for)
                                                            .and_then(|comps| comps.get(&ty_for_value))
                                                            .map(|v| matches!(v, JsonValue::Object(_)))
                                                            .unwrap_or(false);

                                                        if is_object {
                                                            component_editor(id_for, ty_for.clone(), entities_for, sync_for)
                                                                .into_view()
                                                                .into_any()
                                                        } else {
                                                            // For non-object components, create a reactive signal for the value
                                                            let component_value = move || {
                                                                entities_for
                                                                    .get()
                                                                    .get(&id_for)
                                                                    .and_then(|comps| comps.get(&ty_for_value))
                                                                    .cloned()
                                                            };

                                                            view! {
                                                                <pre class="mt-1 bg-slate-950/60 border border-slate-800 rounded p-1 font-mono text-[10px] whitespace-pre-wrap break-all">
                                                                    {move || {
                                                                        component_value()
                                                                            .and_then(|v| serde_json::to_string_pretty(&v).ok())
                                                                            .unwrap_or_default()
                                                                    }}
                                                                </pre>
                                                            }.into_any()
                                                        }
                                                    };
                                                    view! {
                                                        <div class="border border-slate-800 rounded-md p-2 space-y-1">
                                                            <div class="flex items-center justify-between">
                                                                <div class="text-[11px] text-indigo-300 font-medium">{ty.clone()}</div>
                                                            </div>
                                                            {body}
                                                        </div>
                                                    }
                                                }
                                            />
                                            </div>
                                        </div>
                                    }.into_any()
                                }}
                            </Show>
                        </div>
                    </section>

                    <section class="col-span-3 flex flex-col gap-3 min-h-0">
                        <div class="rounded-2xl border border-white/5 bg-slate-900/70 backdrop-blur-sm shadow-lg shadow-black/40 p-3 text-xs space-y-1 flex-shrink-0">
                            <div class="flex items-center justify-between">
                                <span class="font-semibold">"Status"</span>
                                <span class="text-slate-400">{move || format!("{:?}", ready_state.get())}</span>
                            </div>
                            <div class="mt-1 text-[11px] text-slate-400">
                                {move || {
                                    if let Some(id) = auto_subscription_id.get() {
                                        format!("Wildcard subscription · #{}", id)
                                    } else {
                                        "No active subscriptions".to_string()
                                    }
                                }}
                            </div>
                            <div class="mt-1 text-[11px] text-slate-400">
                                {move || format!("Entities mirrored · {}", entities.get().len())}
                            </div>
                            <Show
                                when=move || last_error.get().is_some()
                                fallback=|| view! { <></> }
                            >
                                <div class="mt-1 text-red-400">{move || last_error.get().unwrap_or_default()}</div>
                            </Show>
                        </div>
                        <div class="rounded-2xl border border-white/5 bg-slate-900/70 backdrop-blur-sm shadow-lg shadow-black/40 p-3 flex flex-col min-h-0 flex-1">
                            <button
                                class="flex items-center justify-between w-full text-left group flex-shrink-0"
                                on:click=move |_| set_message_expanded.update(|v| *v = !*v)
                            >
                                <div class="flex items-center gap-2">
                                    <h2 class="text-sm font-semibold text-slate-100">"Server Messages"</h2>
                                    <div
                                        class=move || {
                                            let base = "w-2 h-2 rounded-full transition-all duration-300";
                                            if message_flash.get() {
                                                format!("{base} bg-green-400 shadow-lg shadow-green-400/50")
                                            } else {
                                                format!("{base} bg-slate-700")
                                            }
                                        }
                                    ></div>
                                </div>
                                <span class="text-slate-400 text-xs group-hover:text-slate-300 transition-colors">
                                    {move || if message_expanded.get() { "▼" } else { "▶" }}
                                </span>
                            </button>
                            <Show
                                when=move || message_expanded.get()
                                fallback=|| view! { <></> }
                            >
                                <div class="mt-2 flex-1 overflow-y-auto min-h-0 h-full">
                                    <pre class="text-[10px] font-mono bg-slate-950/60 border border-slate-800 rounded p-2 whitespace-pre-wrap break-all h-full">
                                        {move || last_incoming.get()}
                                    </pre>
                                </div>
                            </Show>
                        </div>
                    </section>
                </main>
            </div>
        }.into_any()
            }
            DevToolsMode::Widget => {
                // Floating widget mode
                let open_in_new_tab = move |_| {
                    // Open DevTools in a new tab/window with ?devtools=1 query param
                    if let Some(window) = leptos::web_sys::window() {
                        let _ = window.open_with_url_and_target(
                            "?devtools=1",
                            "_blank"
                        );
                    }
                };

                view! {
                    <div>
                        // Floating widget button (collapsed state)
                        <Show
                            when=move || !widget_expanded.get()
                            fallback=|| view! { <></> }
                        >
                            <button
                                class=move || {
                                    let base = "fixed bottom-4 left-4 z-50 flex items-center gap-2 px-3 py-2 text-white rounded-full shadow-lg hover:shadow-xl transition-all duration-200 hover:scale-105 border border-white/20";
                                    if devtools_support_enabled {
                                        format!("{base} bg-gradient-to-r from-indigo-600 to-purple-600")
                                    } else {
                                        format!("{base} bg-gradient-to-r from-red-600 to-red-700")
                                    }
                                }
                                on:click=move |_| set_widget_expanded.set(true)
                            >
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z"></path>
                                </svg>
                                <span class="text-xs font-semibold">"DevTools"</span>
                                <Show when=move || !devtools_support_enabled>
                                    <span class="px-1.5 py-0.5 bg-white/30 rounded-full text-[10px] font-bold">
                                        "!"
                                    </span>
                                </Show>
                                <Show when=move || devtools_support_enabled && !entities.get().is_empty()>
                                    <span class="px-1.5 py-0.5 bg-white/20 rounded-full text-[10px] font-bold">
                                        {move || entities.get().len()}
                                    </span>
                                </Show>
                            </button>
                        </Show>

                        // Modal overlay (expanded state)
                        <Show
                            when=move || widget_expanded.get()
                            fallback=|| view! { <></> }
                        >
                            <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
                                <div class="relative w-[95vw] h-[90vh] max-w-[1800px] rounded-2xl shadow-2xl overflow-hidden border border-white/10">
                                    // Render the full DevTools UI inside the modal
                                    // Call DevTools recursively with Embedded mode
                                    <DevTools ws_url=ws_url registry=registry.clone() mode=DevToolsMode::Embedded />

                                    // Action buttons at bottom-left (away from Connect button)
                                    <div class="absolute bottom-4 left-4 z-10 flex gap-2">
                                        <button
                                            class="px-3 py-1.5 bg-slate-800/90 hover:bg-slate-700/90 text-slate-200 rounded-lg text-xs font-medium transition-colors border border-white/10 flex items-center gap-1.5 shadow-lg"
                                            on:click=open_in_new_tab
                                        >
                                            <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"></path>
                                            </svg>
                                            "Open in New Tab"
                                        </button>
                                        <button
                                            class="px-3 py-1.5 bg-slate-800/90 hover:bg-slate-700/90 text-slate-200 rounded-lg text-xs font-medium transition-colors border border-white/10 shadow-lg"
                                            on:click=move |_| set_widget_expanded.set(false)
                                        >
                                            "✕ Close"
                                        </button>
                                    </div>
                                </div>
                            </div>
                        </Show>
                    </div>
                }.into_any()
            }
        }
    }
