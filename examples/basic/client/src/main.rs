//! Basic Leptos client example for pl3xus_client
//!
//! This example demonstrates:
//! - Using SyncProvider to connect to a Bevy server
//! - Using use_components hook to subscribe to component updates
//! - Displaying real-time entity data
//! - Integrating the DevTools widget
//!
//! Run the server first:
//!   cargo run -p pl3xus_client --example basic_server
//!
//! Then run this client:
//!   cd crates/pl3xus_client/examples/basic_client
//!   trunk serve --open

use pl3xus_client::{
    use_components, use_component_store, use_connection, use_sync_context, use_entity,
    ClientTypeRegistry, SyncProvider, MutationState,
};

#[cfg(target_arch = "wasm32")]
use pl3xus_client::devtools::DevTools;
use leptos::prelude::*;
use reactive_graph::traits::{Get, Read};
use reactive_stores::Store;

// Import shared component types (SyncComponent is already implemented in the shared crate)
use basic_types::{EntityName, Position, Velocity};

fn main() {
    console_error_panic_hook::set_once();
    _ = console_log::init_with_level(log::Level::Debug);

    leptos::mount::mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    // Build the client registry with DevTools support
    let registry = ClientTypeRegistry::builder()
        .register::<Position>()
        .register::<Velocity>()
        .register::<EntityName>()
        .with_devtools_support()
        .build();

    let ws_url = "ws://127.0.0.1:3000/sync";
    // TEMPORARY: Use different URL for DevTools to test if leptos-use is caching connections
    let devtools_url = "ws://127.0.0.1:3000/sync?devtools=true";

    // Tab state: "signals" or "stores"
    let (active_tab, set_active_tab) = signal("signals".to_string());

    view! {
        <SyncProvider url=ws_url.to_string() registry=registry.clone() auto_connect=true>
            <div class="min-h-screen w-screen bg-slate-950 text-slate-50 flex flex-col">
                <Header />
                <div class="flex-1 flex overflow-hidden">
                    <main class="flex-1 overflow-auto p-6">
                        <div class="max-w-6xl mx-auto space-y-6">
                            // Tab navigation
                            <div class="flex gap-2 border-b border-slate-800 pb-2">
                                <button
                                    class=move || if active_tab.get() == "signals" {
                                        "px-4 py-2 rounded-t bg-slate-800 text-emerald-400 font-medium text-sm"
                                    } else {
                                        "px-4 py-2 rounded-t bg-slate-900 text-slate-400 hover:text-slate-200 text-sm"
                                    }
                                    on:click=move |_| set_active_tab.set("signals".to_string())
                                >
                                    "Signals (Atomic)"
                                </button>
                                <button
                                    class=move || if active_tab.get() == "stores" {
                                        "px-4 py-2 rounded-t bg-slate-800 text-emerald-400 font-medium text-sm"
                                    } else {
                                        "px-4 py-2 rounded-t bg-slate-900 text-slate-400 hover:text-slate-200 text-sm"
                                    }
                                    on:click=move |_| set_active_tab.set("stores".to_string())
                                >
                                    "Stores (Fine-Grained)"
                                </button>
                            </div>

                            // Tab content
                            <Show
                                when=move || active_tab.get() == "signals"
                                fallback=move || view! { <EntityListWithStores /> }
                            >
                                <EntityList />
                            </Show>
                        </div>
                    </main>
                    <aside class="w-96 border-l border-slate-800 overflow-hidden">
                        {
                            #[cfg(target_arch = "wasm32")]
                            {
                                view! { <DevTools ws_url=devtools_url registry=registry /> }
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                view! { <div>"DevTools only available on WASM"</div> }
                            }
                        }
                    </aside>
                </div>
            </div>
        </SyncProvider>
    }
}

#[component]
fn Header() -> impl IntoView {
    let connection = use_connection();
    let ready_state = connection.ready_state;

    let status_text = move || match ready_state.get() {
        leptos_use::core::ConnectionReadyState::Connecting => "Connecting...",
        leptos_use::core::ConnectionReadyState::Open => "Connected",
        leptos_use::core::ConnectionReadyState::Closing => "Closing...",
        leptos_use::core::ConnectionReadyState::Closed => "Disconnected",
    };

    let status_color = move || match ready_state.get() {
        leptos_use::core::ConnectionReadyState::Open => "bg-emerald-500",
        leptos_use::core::ConnectionReadyState::Connecting => "bg-yellow-500",
        _ => "bg-red-500",
    };

    view! {
        <header class="border-b border-slate-800 bg-slate-900/80 backdrop-blur px-6 py-4">
            <div class="flex items-center justify-between">
                <div>
                    <h1 class="text-lg font-semibold tracking-tight">"Pl3xus Client - Basic Example"</h1>
                    <p class="text-xs text-slate-400">"Real-time entity synchronization with Bevy ECS"</p>
                </div>
                <div class="flex items-center gap-2">
                    <div class=move || format!("w-2 h-2 rounded-full {}", status_color())></div>
                    <span class="text-xs text-slate-400">{status_text}</span>
                </div>
            </div>
        </header>
    }
}

#[component]
fn EntityList() -> impl IntoView {
    // Subscribe to component updates
    let positions = use_components::<Position>();
    let velocities = use_components::<Velocity>();
    let names = use_components::<EntityName>();

    // Debug: Log positions signal content
    Effect::new(move |_| {
        let pos_map = positions.get();
        leptos::logging::log!("[EntityList] Positions signal updated: {} entities", pos_map.len());
        for (entity_id, pos) in pos_map.iter() {
            leptos::logging::log!("[EntityList] Entity {}: Position({}, {})", entity_id, pos.x, pos.y);
        }
    });

    view! {
        <div class="space-y-4">
            <h2 class="text-xl font-semibold">"Entities"</h2>
            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {move || {
                leptos::logging::log!("[EntityList] Rendering entity cards for {} entities", positions.get().len());
            }}
                <For
                    each=move || {
                        let mut entity_ids: Vec<_> = positions.get()
                            .keys()
                            .copied()
                            .collect();
                        entity_ids.sort(); // Sort for stable ordering
                        leptos::logging::log!("[EntityList] For loop: {} entity IDs", entity_ids.len());
                        entity_ids
                    }
                    key=|entity_id| *entity_id
                    children=move |entity_id| {
                        view! { <EntityCard entity_id=entity_id /> }
                    }
                />
            </div>
        </div>
    }
}

#[component]
fn EntityCard(entity_id: u64) -> impl IntoView {
    let positions = use_components::<Position>();
    let velocities = use_components::<Velocity>();
    let names = use_components::<EntityName>();

    let position = move || positions.get().get(&entity_id).cloned();
    let velocity = move || velocities.get().get(&entity_id).cloned();
    let name = move || names.get().get(&entity_id).cloned();

    view! {
        <div class="bg-slate-900 border border-slate-800 rounded-lg p-4 space-y-2">
            <div class="flex items-center justify-between">
                <h3 class="font-medium text-sm">
                    {move || name().map(|n| n.name).unwrap_or_else(|| format!("Entity {}", entity_id))}
                </h3>
                <span class="text-xs text-slate-500">
                    "ID: " {entity_id}
                </span>
            </div>

            <div class="space-y-1 text-xs">
                <div class="flex items-center justify-between">
                    <span class="text-slate-400">"Position:"</span>
                    <span class="font-mono">
                        {move || position().map(|p| format!("({:.1}, {:.1})", p.x, p.y)).unwrap_or_else(|| "N/A".to_string())}
                    </span>
                </div>

                <div class="flex items-center justify-between">
                    <span class="text-slate-400">"Velocity:"</span>
                    <span class="font-mono">
                        {move || velocity().map(|v| format!("({:.1}, {:.1})", v.x, v.y)).unwrap_or_else(|| "N/A".to_string())}
                    </span>
                </div>
            </div>

            /* Editable Position (demonstrates use_components_write hook) */
            <EditablePosition entity_id=entity_id />

            /* Visual representation */
            <div class="mt-3 h-32 bg-slate-950 rounded border border-slate-700 relative overflow-hidden">
                {move || {
                    position().map(|pos| {
                        // Map position to canvas coordinates (assuming -200 to 200 range)
                        let x_percent = ((pos.x + 200.0) / 400.0 * 100.0).clamp(0.0, 100.0);
                        let y_percent = ((pos.y + 200.0) / 400.0 * 100.0).clamp(0.0, 100.0);

                        view! {
                            <div
                                class="absolute w-3 h-3 bg-emerald-500 rounded-full -translate-x-1/2 -translate-y-1/2"
                                style:left=format!("{}%", x_percent)
                                style:top=format!("{}%", y_percent)
                            ></div>
                        }
                    })
                }}
            </div>
        </div>
    }
}

#[component]
fn EntityListWithStores() -> impl IntoView {
    // Subscribe to component updates using stores for fine-grained reactivity
    let positions = use_component_store::<Position>();
    let velocities = use_component_store::<Velocity>();
    let names = use_component_store::<EntityName>();

    // Debug: Log when the store updates
    Effect::new(move |_| {
        let pos_map = positions.read();
        leptos::logging::log!("[EntityListWithStores] Positions store updated: {} entities", pos_map.len());
    });

    view! {
        <div class="space-y-4">
            <div class="flex items-center justify-between">
                <h2 class="text-xl font-semibold">"Entities (Store-based)"</h2>
                <div class="text-xs text-slate-400 bg-slate-900 px-3 py-1 rounded">
                    "Fine-grained reactivity with reactive_stores"
                </div>
            </div>
            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                <For
                    each=move || {
                        let mut entity_ids: Vec<_> = positions.read()
                            .keys()
                            .copied()
                            .collect();
                        entity_ids.sort(); // Sort for stable ordering
                        entity_ids
                    }
                    key=|entity_id| *entity_id
                    children=move |entity_id| {
                        view! { <EntityCardWithStore entity_id=entity_id positions=positions velocities=velocities names=names /> }
                    }
                />
            </div>
        </div>
    }
}

#[component]
fn EntityCardWithStore(
    entity_id: u64,
    positions: Store<std::collections::HashMap<u64, Position>>,
    velocities: Store<std::collections::HashMap<u64, Velocity>>,
    names: Store<std::collections::HashMap<u64, EntityName>>,
) -> impl IntoView {
    // Fine-grained reactivity: only updates when this specific entity's data changes
    let position = move || positions.read().get(&entity_id).cloned();
    let velocity = move || velocities.read().get(&entity_id).cloned();
    let name = move || names.read().get(&entity_id).cloned();

    view! {
        <div class="bg-slate-900 border border-slate-800 rounded-lg p-4 space-y-2">
            <div class="flex items-center justify-between">
                <h3 class="font-medium text-sm">
                    {move || name().map(|n| n.name).unwrap_or_else(|| format!("Entity {}", entity_id))}
                </h3>
                <span class="text-xs text-slate-500">
                    "ID: " {entity_id}
                </span>
            </div>

            <div class="space-y-1 text-xs">
                <div class="flex items-center justify-between">
                    <span class="text-slate-400">"Position:"</span>
                    <span class="font-mono">
                        {move || position().map(|p| format!("({:.1}, {:.1})", p.x, p.y)).unwrap_or_else(|| "N/A".to_string())}
                    </span>
                </div>

                <div class="flex items-center justify-between">
                    <span class="text-slate-400">"Velocity:"</span>
                    <span class="font-mono">
                        {move || velocity().map(|v| format!("({:.1}, {:.1})", v.x, v.y)).unwrap_or_else(|| "N/A".to_string())}
                    </span>
                </div>
            </div>

            /* Visual representation */
            <div class="mt-3 h-32 bg-slate-950 rounded border border-slate-700 relative overflow-hidden">
                {move || {
                    position().map(|pos| {
                        // Map position to canvas coordinates (assuming -200 to 200 range)
                        let x_percent = ((pos.x + 200.0) / 400.0 * 100.0).clamp(0.0, 100.0);
                        let y_percent = ((pos.y + 200.0) / 400.0 * 100.0).clamp(0.0, 100.0);

                        view! {
                            <div
                                class="absolute w-3 h-3 bg-blue-500 rounded-full -translate-x-1/2 -translate-y-1/2"
                                style:left=format!("{}%", x_percent)
                                style:top=format!("{}%", y_percent)
                            ></div>
                        }
                    })
                }}
            </div>

            <div class="text-xs text-slate-500 italic mt-2">
                "Using Store for fine-grained reactivity"
            </div>
        </div>
    }
}

/// Demonstrates direct mutations for editable fields
#[component]
fn EditablePosition(entity_id: u64) -> impl IntoView {
    // Get the sync context for mutations
    let ctx = use_sync_context();

    // Get the current position using use_entity
    let position = use_entity::<Position>(entity_id);

    // Local state for editing
    let (local_x, set_local_x) = signal(String::new());
    let (local_y, set_local_y) = signal(String::new());
    let (mutation_status, set_mutation_status) = signal::<Option<MutationState>>(None);

    // Initialize local state from server value
    Effect::new(move |_| {
        if let Some(pos) = position.get() {
            set_local_x.set(pos.x.to_string());
            set_local_y.set(pos.y.to_string());
        }
    });

    // Mutation handler for X
    let ctx_x = ctx.clone();
    let mutate_x = move |_| {
        if let Ok(x) = local_x.get().parse::<f32>() {
            if let Some(mut pos) = position.get() {
                pos.x = x;
                let request_id = ctx_x.mutate(entity_id, pos);
                set_mutation_status.set(Some(MutationState {
                    request_id,
                    status: None,
                    message: None,
                }));
            }
        }
    };

    // Mutation handler for Y
    let ctx_y = ctx.clone();
    let mutate_y = move |_| {
        if let Ok(y) = local_y.get().parse::<f32>() {
            if let Some(mut pos) = position.get() {
                pos.y = y;
                let request_id = ctx_y.mutate(entity_id, pos);
                set_mutation_status.set(Some(MutationState {
                    request_id,
                    status: None,
                    message: None,
                }));
            }
        }
    };

    view! {
        <div class="mt-3 pt-3 border-t border-slate-800">
            <div class="text-xs text-slate-400 mb-2">"Edit Position (Direct Mutations):"</div>
            <div class="flex gap-2">
                <div class="flex-1">
                    <label class="text-[10px] text-slate-500">"X:"</label>
                    <input
                        type="number"
                        step="0.1"
                        class="w-full bg-slate-950 border border-slate-700 rounded px-2 py-1 text-xs focus:outline-none focus:ring-1 focus:ring-emerald-500"
                        prop:value=move || local_x.get()
                        on:input=move |ev| set_local_x.set(event_target_value(&ev))
                        on:blur=mutate_x
                    />
                </div>
                <div class="flex-1">
                    <label class="text-[10px] text-slate-500">"Y:"</label>
                    <input
                        type="number"
                        step="0.1"
                        class="w-full bg-slate-950 border border-slate-700 rounded px-2 py-1 text-xs focus:outline-none focus:ring-1 focus:ring-emerald-500"
                        prop:value=move || local_y.get()
                        on:input=move |ev| set_local_y.set(event_target_value(&ev))
                        on:blur=mutate_y
                    />
                </div>
            </div>
            <div class="text-[10px] text-slate-500 mt-1 italic">
                "Changes are sent to server on blur"
            </div>
            {move || mutation_status.get().and_then(|s| s.message.clone()).map(|msg| view! {
                <div class="text-[10px] text-red-400 mt-1">{msg}</div>
            })}
        </div>
    }
}
