//! Top bar / header component.

#![allow(dead_code)]

use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

use pl3xus_client::{use_entity_component, use_sync_context, use_connection, use_query_keyed, use_message, use_request, ControlRequest, ControlResponse, EntityControl, ConnectionReadyState};
use fanuc_replica_plugins::*;
use crate::pages::dashboard::use_system_entity;

/// Top bar with connection status, robot info, and settings.
#[component]
pub fn TopBar() -> impl IntoView {
    let _ctx = use_sync_context();
    let connection = use_connection();
    let system_ctx = use_system_entity();

    // Subscribe to the robot's connection state (ConnectionState lives on robot entity, not system)
    let (connection_state, robot_exists) = use_entity_component::<ConnectionState, _>(move || system_ctx.robot_entity_id.get());

    // WebSocket connection state from connection
    let ws_connected = Signal::derive(move || connection.ready_state.get() == ConnectionReadyState::Open);
    let ws_connecting = Signal::derive(move || connection.ready_state.get() == ConnectionReadyState::Connecting);

    // Robot connection state from synced component (only valid if robot entity exists)
    let robot_connected = move || robot_exists.get() && connection_state.get().robot_connected;
    let robot_connecting = move || robot_exists.get() && connection_state.get().robot_connecting;
    let connected_robot_name = move || {
        if !robot_exists.get() { return None; }
        let state = connection_state.get();
        if state.robot_connected { Some(state.robot_name.clone()) } else { None }
    };

    // State for connection dropdown
    let (show_connection_menu, set_show_connection_menu) = signal(false);

    view! {
        <header class="h-9 bg-[#111111] border-b border-[#ffffff10] flex items-center px-3 shrink-0">
            // Logo and title
            <div class="flex items-center space-x-2">
                <div class="w-6 h-6 bg-[#00d9ff] rounded flex items-center justify-center">
                    <svg class="w-3.5 h-3.5 text-black" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z"/>
                    </svg>
                </div>
                <h1 class="text-xs font-semibold text-white">"FANUC RMI"</h1>
                <span class="text-[#555555] text-[10px]">"v2.0"</span>
            </div>

            // Spacer
            <div class="flex-1"></div>

            // Connection status indicators
            <div class="flex items-center space-x-4">
                // WebSocket status with reconnect
                <div class="relative">
                    <button
                        class="flex items-center space-x-1.5 px-2 py-1 rounded hover:bg-[#ffffff08] transition-colors"
                        on:click=move |_| set_show_connection_menu.update(|v| *v = !*v)
                    >
                        <div class={move || if ws_connected.get() {
                            "w-1.5 h-1.5 bg-[#00d9ff] rounded-full animate-pulse"
                        } else {
                            "w-1.5 h-1.5 bg-[#ff4444] rounded-full"
                        }}></div>
                        <span class="text-[10px] text-[#888888]">"WS"</span>
                        <svg class="w-2.5 h-2.5 text-[#666666]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"/>
                        </svg>
                    </button>

                    // Connection dropdown menu
                    <Show when=move || show_connection_menu.get()>
                        <WebSocketDropdown
                            ws_connected=ws_connected
                            ws_connecting=ws_connecting
                            set_show_connection_menu=set_show_connection_menu
                        />
                    </Show>
                </div>

                // Robot status indicator with name (only show when WebSocket connected)
                <Show when=move || ws_connected.get()>
                    <div class="flex items-center space-x-1.5">
                        <div class={move || {
                            if robot_connecting() {
                                "w-1.5 h-1.5 bg-[#ffaa00] rounded-full animate-pulse"
                            } else if robot_connected() {
                                "w-1.5 h-1.5 bg-[#22c55e] rounded-full animate-pulse"
                            } else {
                                "w-1.5 h-1.5 bg-[#444444] rounded-full"
                            }
                        }}></div>
                        <span class="text-[10px] text-[#888888]">
                            {move || {
                                if robot_connecting() {
                                    "Connecting...".to_string()
                                } else if let Some(name) = connected_robot_name() {
                                    name
                                } else if robot_connected() {
                                    "Robot".to_string()
                                } else {
                                    "No Robot".to_string()
                                }
                            }}
                        </span>
                    </div>
                </Show>

                // Control status button (only show when WebSocket connected)
                <Show when=move || ws_connected.get()>
                    <ControlButton/>
                </Show>

                // Settings button - shows quick settings popup (only show when WebSocket connected)
                <Show when=move || ws_connected.get()>
                    <QuickSettingsButton/>
                </Show>
            </div>
        </header>
    }
}

/// Connection dropdown - shows robot connection status and allows connecting.
#[component]
fn ConnectionDropdown() -> impl IntoView {
    let system_ctx = use_system_entity();
    // ConnectionState lives on robot entity, not system entity
    let (connection_state, robot_exists) = use_entity_component::<ConnectionState, _>(move || system_ctx.robot_entity_id.get());
    let (dropdown_open, set_dropdown_open) = signal(false);

    // Use query that only fetches when dropdown is open
    // Server-side invalidation will auto-refetch if dropdown is open
    let robots_query = use_query_keyed::<ListRobotConnections, _>(move || {
        if dropdown_open.get() { Some(ListRobotConnections) } else { None }
    });

    let toggle_dropdown = move |_| {
        set_dropdown_open.set(!dropdown_open.get_untracked());
    };

    let is_robot_connected = move || robot_exists.get() && connection_state.get().robot_connected;
    let robot_addr = move || if robot_exists.get() { connection_state.get().robot_addr.clone() } else { String::new() };

    view! {
        <div class="relative">
            <button
                class="flex items-center gap-1.5 px-2 py-0.5 bg-[#ffffff05] rounded border border-[#ffffff05] hover:bg-[#ffffff10] cursor-pointer"
                on:click=toggle_dropdown
            >
                <div class=move || {
                    if is_robot_connected() { "w-1.5 h-1.5 rounded-full bg-[#22c55e] animate-pulse" }
                    else { "w-1.5 h-1.5 rounded-full bg-[#ef4444]" }
                }></div>
                <span class="text-[9px] text-[#cccccc] font-medium">
                    {move || if is_robot_connected() { robot_addr() } else { "Select Robot".to_string() }}
                </span>
                <span class="text-[8px] text-[#666666]">"▼"</span>
            </button>

            <Show when=move || dropdown_open.get()>
                <DropdownContent robots_query=robots_query set_dropdown_open=set_dropdown_open />
            </Show>
        </div>
    }
}

/// Dropdown content component.
#[allow(dead_code)]
#[component]
fn DropdownContent(
    robots_query: pl3xus_client::QueryHandle<ListRobotConnections>,
    set_dropdown_open: WriteSignal<bool>,
) -> impl IntoView {
    let ctx = use_sync_context();

    let quick_connect = {
        let ctx = ctx.clone();
        move |_| {
            ctx.send(ConnectToRobot {
                connection_id: None,
                addr: "127.0.0.1".to_string(),
                port: 16001,
                name: Some("CRX-10iA Simulator".to_string()),
            });
            set_dropdown_open.set(false);
        }
    };

    view! {
        <div class="absolute top-full left-0 mt-1 w-56 bg-[#1a1a1a] border border-[#ffffff15] rounded shadow-lg z-50">
            <div class="px-2 py-1.5 border-b border-[#ffffff10] text-[9px] text-[#666666] uppercase tracking-wide">"Saved Robots"</div>

            <Show when=move || robots_query.is_loading()>
                <div class="px-3 py-2 text-[10px] text-[#888888]">"Loading..."</div>
            </Show>

            <RobotConnectionList robots_query=robots_query set_dropdown_open=set_dropdown_open />

            <div class="border-t border-[#ffffff10] px-2 py-1.5">
                <button
                    class="w-full px-2 py-1 text-[9px] text-[#00d9ff] hover:bg-[#00d9ff10] rounded flex items-center gap-1.5"
                    on:click=quick_connect
                >
                    <span>"⚡"</span>
                    "Quick Connect (Simulator)"
                </button>
            </div>
        </div>
    }
}

/// Robot connection list component.
#[allow(dead_code)]
#[component]
fn RobotConnectionList(
    robots_query: pl3xus_client::QueryHandle<ListRobotConnections>,
    set_dropdown_open: WriteSignal<bool>,
) -> impl IntoView {
    let ctx = use_sync_context();

    view! {
        <Show when=move || robots_query.data().is_some()>
            {
                let ctx = ctx.clone();
                move || {
                    let robots = robots_query.data().map(|r| r.connections.clone()).unwrap_or_default();
                    let ctx = ctx.clone();

                    if robots.is_empty() {
                        view! {
                            <div class="px-3 py-2 text-[10px] text-[#666666] italic">
                                "No saved robots. Use Settings to add."
                            </div>
                        }.into_any()
                    } else {
                        robots.into_iter().map(|robot| {
                            let robot_clone = robot.clone();
                            let ctx = ctx.clone();
                            view! {
                                <button
                                    class="w-full px-3 py-1.5 text-left hover:bg-[#ffffff08] flex items-center gap-2"
                                    on:click=move |_| {
                                        ctx.send(ConnectToRobot {
                                            connection_id: Some(robot_clone.id),
                                            addr: robot_clone.ip_address.clone(),
                                            port: robot_clone.port,
                                            name: Some(robot_clone.name.clone()),
                                        });
                                        set_dropdown_open.set(false);
                                    }
                                >
                                    <div class="w-1.5 h-1.5 rounded-full bg-[#666666]"></div>
                                    <div class="flex-1">
                                        <div class="text-[10px] text-white">{robot.name.clone()}</div>
                                        <div class="text-[8px] text-[#666666]">
                                            {format!("{}:{}", robot.ip_address, robot.port)}
                                        </div>
                                    </div>
                                </button>
                            }
                        }).collect::<Vec<_>>().into_any()
                    }
                }
            }
        </Show>
    }
}

/// WebSocket connection dropdown menu
#[component]
fn WebSocketDropdown(
    ws_connected: Signal<bool>,
    ws_connecting: Signal<bool>,
    set_show_connection_menu: WriteSignal<bool>,
) -> impl IntoView {
    let connection = use_connection();

    view! {
        <div class="absolute right-0 top-full mt-1 w-56 bg-[#1a1a1a] border border-[#ffffff15] rounded shadow-lg z-50">
            <div class="p-2">
                <div class="text-[9px] text-[#666666] uppercase tracking-wide mb-1">"WebSocket Server"</div>
                <div class="flex items-center justify-between mb-2">
                    <span class="text-[10px] text-[#aaaaaa]">
                        {move || {
                            if ws_connecting.get() {
                                "Connecting..."
                            } else if ws_connected.get() {
                                "Connected"
                            } else {
                                "Disconnected"
                            }
                        }}
                    </span>
                    <div class={move || {
                        if ws_connecting.get() {
                            "w-1.5 h-1.5 bg-[#ffaa00] rounded-full animate-pulse"
                        } else if ws_connected.get() {
                            "w-1.5 h-1.5 bg-[#00d9ff] rounded-full animate-pulse"
                        } else {
                            "w-1.5 h-1.5 bg-[#ff4444] rounded-full"
                        }
                    }}></div>
                </div>
                <div class="text-[9px] text-[#555555] mb-2">"ws://127.0.0.1:8083"</div>
                <button
                    class="w-full text-[9px] px-2 py-1 bg-[#00d9ff20] text-[#00d9ff] rounded hover:bg-[#00d9ff30] disabled:opacity-50 disabled:cursor-not-allowed"
                    disabled=move || ws_connected.get() || ws_connecting.get()
                    on:click={
                        let open = connection.open.clone();
                        move |_| {
                            open();
                            set_show_connection_menu.set(false);
                        }
                    }
                >
                    {move || {
                        if ws_connecting.get() {
                            "Connecting..."
                        } else if ws_connected.get() {
                            "Connected"
                        } else {
                            "Reconnect"
                        }
                    }}
                </button>
            </div>
        </div>
    }
}

/// Quick Settings button with popup - focused on robot connection switching
#[component]
fn QuickSettingsButton() -> impl IntoView {
    let ctx = use_sync_context();
    let _navigate = use_navigate();
    let system_ctx = use_system_entity();

    // Subscribe to entity-specific components
    // ConnectionState lives on robot entity, EntityControl lives on system entity
    let (connection_state, robot_exists) = use_entity_component::<ConnectionState, _>(move || system_ctx.robot_entity_id.get());
    let (control_state, _) = use_entity_component::<EntityControl, _>(move || system_ctx.system_entity_id.get());

    let (show_popup, set_show_popup) = signal(false);

    // Use query that only fetches when popup is open
    // Server-side invalidation will auto-refetch if popup is open
    let robots_query = use_query_keyed::<ListRobotConnections, _>(move || {
        if show_popup.get() { Some(ListRobotConnections) } else { None }
    });

    let robot_connected = move || robot_exists.get() && connection_state.get().robot_connected;
    let robot_connecting = move || robot_exists.get() && connection_state.get().robot_connecting;
    let connected_robot_name = move || {
        if !robot_exists.get() { return None; }
        let state = connection_state.get();
        if state.robot_connected { Some(state.robot_name.clone()) } else { None }
    };
    let active_connection_id = move || if robot_exists.get() { connection_state.get().active_connection_id } else { None };

    // Check if THIS client has control by comparing EntityControl.client_id with our own connection ID
    let has_control = move || {
        let my_id = ctx.my_connection_id.get();
        Some(control_state.get().client_id) == my_id
    };

    // Get the controlling client ID (if any)
    let controlling_client_id = move || -> Option<u32> {
        Some(control_state.get().client_id.id)
    };

    view! {
        <div class="relative">
            <button
                class="p-1 hover:bg-[#ffffff08] rounded transition-colors"
                on:click=move |_| set_show_popup.update(|v| *v = !*v)
                title="Quick Connect"
            >
                <svg class="w-3.5 h-3.5 text-[#888888] hover:text-[#00d9ff]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/>
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>
                </svg>
            </button>

            // Quick Connect popup
            <Show when=move || show_popup.get()>
                <QuickSettingsPopup
                    set_show_popup=set_show_popup
                    robots_query=robots_query
                    robot_connected=Signal::derive(robot_connected)
                    robot_connecting=Signal::derive(robot_connecting)
                    connected_robot_name=Signal::derive(connected_robot_name)
                    active_connection_id=Signal::derive(active_connection_id)
                    has_control=Signal::derive(has_control)
                    controlling_client_id=Signal::derive(controlling_client_id)
                />
            </Show>
        </div>
    }
}

/// Quick Settings popup content
#[component]
fn QuickSettingsPopup(
    set_show_popup: WriteSignal<bool>,
    robots_query: pl3xus_client::QueryHandle<ListRobotConnections>,
    robot_connected: Signal<bool>,
    robot_connecting: Signal<bool>,
    connected_robot_name: Signal<Option<String>>,
    active_connection_id: Signal<Option<i64>>,
    has_control: Signal<bool>,
    controlling_client_id: Signal<Option<u32>>,
) -> impl IntoView {
    let ctx = use_sync_context();
    let navigate = use_navigate();

    view! {
        <div class="absolute right-0 top-full mt-1 w-72 bg-[#1a1a1a] border border-[#ffffff15] rounded-lg shadow-lg z-50">
            // Header
            <div class="flex items-center justify-between p-2 border-b border-[#ffffff10]">
                <span class="text-[10px] font-semibold text-[#00d9ff]">"Quick Connect"</span>
                <button
                    class="text-[#666666] hover:text-white"
                    on:click=move |_| set_show_popup.set(false)
                >
                    <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                    </svg>
                </button>
            </div>

            // Current Status Section
            <div class="p-2 border-b border-[#ffffff10] space-y-1.5">
                // Connection status
                <div class="flex items-center justify-between">
                    <span class="text-[9px] text-[#666666] uppercase">"Robot"</span>
                    {move || {
                        if robot_connecting.get() {
                            view! {
                                <div class="flex items-center gap-1.5">
                                    <div class="w-1.5 h-1.5 bg-[#ffaa00] rounded-full animate-pulse"></div>
                                    <span class="text-[10px] text-[#ffaa00] font-medium">"Connecting..."</span>
                                </div>
                            }.into_any()
                        } else if robot_connected.get() {
                            let name = connected_robot_name.get().unwrap_or_else(|| "Connected".to_string());
                            view! {
                                <div class="flex items-center gap-1.5">
                                    <div class="w-1.5 h-1.5 bg-[#22c55e] rounded-full animate-pulse"></div>
                                    <span class="text-[10px] text-[#22c55e] font-medium">{name}</span>
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <div class="flex items-center gap-1.5">
                                    <div class="w-1.5 h-1.5 bg-[#666666] rounded-full"></div>
                                    <span class="text-[10px] text-[#888888]">"Not Connected"</span>
                                </div>
                            }.into_any()
                        }
                    }}
                </div>
                // Control status
                <div class="flex items-center justify-between">
                    <span class="text-[9px] text-[#666666] uppercase">"Control"</span>
                    {move || {
                        let my_id = ctx.my_connection_id.get();
                        if has_control.get() {
                            view! {
                                <span class="text-[10px] text-[#00d9ff] font-medium">"You have control"</span>
                            }.into_any()
                        } else if let Some(client_id) = controlling_client_id.get() {
                            // Another client has control
                            let is_me = my_id.map(|id| id.id == client_id).unwrap_or(false);
                            if is_me {
                                // This shouldn't happen, but handle it gracefully
                                view! {
                                    <span class="text-[10px] text-[#00d9ff] font-medium">"You have control"</span>
                                }.into_any()
                            } else {
                                view! {
                                    <span class="text-[10px] text-[#f59e0b]">{format!("Client {} has control", client_id)}</span>
                                }.into_any()
                            }
                        } else {
                            view! {
                                <span class="text-[10px] text-[#888888]">"No control"</span>
                            }.into_any()
                        }
                    }}
                </div>
            </div>

            // Saved Connections List
            <div class="p-2 border-b border-[#ffffff10]">
                <div class="text-[9px] text-[#666666] uppercase tracking-wide mb-1.5">"Saved Robots"</div>
                <div class="space-y-1 max-h-48 overflow-y-auto">
                    <SavedConnectionsList
                        robots_query=robots_query
                        active_connection_id=active_connection_id
                        has_control=has_control
                        robot_connecting=robot_connecting
                        set_show_popup=set_show_popup
                    />
                </div>
            </div>

            // Control actions
            <ControlActions has_control=has_control />

            // Link to full settings
            <div class="p-2">
                <button
                    class="w-full text-[9px] text-[#888888] hover:text-[#00d9ff] flex items-center justify-center gap-1"
                    on:click={
                        let nav = navigate.clone();
                        move |_| {
                            nav("/settings", Default::default());
                            set_show_popup.set(false);
                        }
                    }
                >
                    "Manage connections in "
                    <span class="underline">"Settings"</span>
                    <svg class="w-2.5 h-2.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"/>
                    </svg>
                </button>
            </div>
        </div>
    }
}


/// Saved connections list for quick settings popup
#[component]
fn SavedConnectionsList(
    robots_query: pl3xus_client::QueryHandle<ListRobotConnections>,
    active_connection_id: Signal<Option<i64>>,
    has_control: Signal<bool>,
    robot_connecting: Signal<bool>,
    set_show_popup: WriteSignal<bool>,
) -> impl IntoView {
    let ctx = use_sync_context();

    // Use request/response pattern for ConnectToRobot
    let (send_connect, connect_state) = use_request::<ConnectToRobot>();

    // Track which connection we're trying to connect to (for showing loading state per-button)
    let (connecting_to_id, set_connecting_to_id) = signal::<Option<i64>>(None);

    // Track error message to display
    let (error_message, set_error_message) = signal::<Option<String>>(None);

    // Effect to handle response state changes
    // Only process when we're actively connecting (connecting_to_id is set)
    Effect::new(move |_| {
        // Only process responses when we're actively waiting for one
        if connecting_to_id.get().is_none() {
            return;
        }

        let state = connect_state.get();
        if let Some(response) = state.data {
            if response.success {
                // Success! Close the popup
                set_show_popup.set(false);
                set_connecting_to_id.set(None);
                set_error_message.set(None);
            } else if let Some(err) = response.error {
                // Server returned an error in the response
                set_error_message.set(Some(err));
                set_connecting_to_id.set(None);
            }
        } else if let Some(err) = state.error {
            // Network/transport error
            set_error_message.set(Some(err));
            set_connecting_to_id.set(None);
        }
    });

    view! {
        // Show error message if any
        {move || error_message.get().map(|err| view! {
            <div class="text-[9px] text-[#ff4444] bg-[#ff444420] p-1.5 rounded mb-2">
                {err}
            </div>
        })}

        {move || {
            if robots_query.is_loading() {
                return view! {
                    <div class="text-[10px] text-[#555555] italic py-2 text-center">
                        "Loading..."
                    </div>
                }.into_any();
            }

            let connections = robots_query.data().map(|r| r.connections.clone()).unwrap_or_default();
            if connections.is_empty() {
                view! {
                    <div class="text-[10px] text-[#555555] italic py-2 text-center">
                        "No saved connections"
                    </div>
                }.into_any()
            } else {
                let ctx = ctx.clone();
                connections.into_iter().map(|conn| {
                    let conn_id = conn.id;
                    let conn_name = conn.name.clone();
                    let conn_addr = format!("{}:{}", conn.ip_address, conn.port);
                    let is_active = move || active_connection_id.get() == Some(conn_id);
                    // Check if we're currently connecting to THIS specific connection
                    let is_connecting_to_this = move || connecting_to_id.get() == Some(conn_id);
                    // Require control to connect - control = ownership of the apparatus/system
                    let can_connect = move || has_control.get() && !is_active() && !robot_connecting.get() && connecting_to_id.get().is_none();
                    let ctx = ctx.clone();
                    let send_connect = send_connect.clone();

                    view! {
                        <div class={move || {
                            let base = "flex items-center justify-between p-1.5 rounded";
                            if is_active() {
                                format!("{} bg-[#22c55e15] border border-[#22c55e40]", base)
                            } else if is_connecting_to_this() {
                                format!("{} bg-[#00d9ff10] border border-[#00d9ff40]", base)
                            } else {
                                format!("{} bg-[#ffffff05] hover:bg-[#ffffff08]", base)
                            }
                        }}>
                            <div class="flex-1 min-w-0">
                                <div class="text-[10px] text-white font-medium truncate">{conn_name}</div>
                                <div class="text-[9px] text-[#666666]">{conn_addr}</div>
                            </div>
                            {move || {
                                let ctx = ctx.clone();
                                let send_connect = send_connect.clone();
                                if is_active() {
                                    // Show disconnect button for active connection
                                    view! {
                                        <button
                                            class="text-[8px] px-2 py-0.5 bg-[#ff444420] text-[#ff4444] rounded hover:bg-[#ff444430] disabled:opacity-50 disabled:cursor-not-allowed"
                                            disabled=move || !has_control.get() || robot_connecting.get()
                                            title=move || if has_control.get() { "Disconnect" } else { "Need control to disconnect" }
                                            on:click=move |_| {
                                                ctx.send(DisconnectRobot);
                                            }
                                        >
                                            "Disconnect"
                                        </button>
                                    }.into_any()
                                } else if is_connecting_to_this() {
                                    // Show connecting state for this specific connection
                                    view! {
                                        <span class="text-[8px] px-2 py-0.5 text-[#00d9ff] animate-pulse">
                                            "Connecting..."
                                        </span>
                                    }.into_any()
                                } else {
                                    // Show connect button for other connections - requires control
                                    view! {
                                        <button
                                            class="text-[8px] px-2 py-0.5 bg-[#00d9ff20] text-[#00d9ff] rounded hover:bg-[#00d9ff30] disabled:opacity-50 disabled:cursor-not-allowed"
                                            disabled=move || !can_connect()
                                            title=move || {
                                                if robot_connecting.get() || connecting_to_id.get().is_some() {
                                                    "Another connection in progress"
                                                } else if !has_control.get() {
                                                    "Need control to connect"
                                                } else {
                                                    "Connect to this robot"
                                                }
                                            }
                                            on:click=move |_| {
                                                // Clear any previous error
                                                set_error_message.set(None);
                                                // Track which connection we're connecting to
                                                set_connecting_to_id.set(Some(conn_id));
                                                // Send the request (don't close popup - wait for response)
                                                send_connect(ConnectToRobot {
                                                    connection_id: Some(conn_id),
                                                    addr: String::new(),
                                                    port: 0,
                                                    name: None,
                                                });
                                            }
                                        >
                                            "Connect"
                                        </button>
                                    }.into_any()
                                }
                            }}
                        </div>
                    }
                }).collect::<Vec<_>>().into_any()
            }
        }}
    }
}

/// Control actions section for quick settings popup
#[component]
fn ControlActions(has_control: Signal<bool>) -> impl IntoView {
    let ctx = use_sync_context();
    let system_ctx = use_system_entity();

    // Get the System entity ID from the context (provided by DesktopLayout)
    // Control requests target the System entity
    let system_entity_bits = move || -> Option<u64> {
        system_ctx.system_entity_id.get()
    };

    view! {
        {move || {
            let ctx = ctx.clone();
            if !has_control.get() {
                view! {
                    <div class="p-2 border-b border-[#ffffff10]">
                        <button
                            class="w-full text-[9px] px-3 py-1.5 bg-[#00d9ff20] border border-[#00d9ff40] text-[#00d9ff] rounded hover:bg-[#00d9ff30]"
                            on:click=move |_| {
                                if let Some(entity_bits) = system_entity_bits() {
                                    ctx.send(ControlRequest::Take(entity_bits));
                                }
                            }
                        >
                            "Request Control"
                        </button>
                    </div>
                }.into_any()
            } else {
                view! {
                    <div class="p-2 border-b border-[#ffffff10]">
                        <button
                            class="w-full text-[9px] px-3 py-1.5 bg-[#ff444420] border border-[#ff444440] text-[#ff4444] rounded hover:bg-[#ff444430]"
                            on:click=move |_| {
                                if let Some(entity_bits) = system_entity_bits() {
                                    ctx.send(ControlRequest::Release(entity_bits));
                                }
                            }
                        >
                            "Release Control"
                        </button>
                    </div>
                }.into_any()
            }
        }}
    }
}

/// Control button - Request/Release control of the System entity
///
/// The System entity is the root of the hierarchy. Clients request control
/// of the System to gain control over all child entities (robots, etc.).
#[component]
fn ControlButton() -> impl IntoView {
    let ctx = use_sync_context();
    let toast = crate::components::use_toast();
    let system_ctx = use_system_entity();

    // Subscribe to entity-specific control state
    let (control_state, _) = use_entity_component::<EntityControl, _>(move || system_ctx.system_entity_id.get());

    // Get the System entity ID from the context (provided by DesktopLayout)
    // Control requests target the System entity
    let system_entity_bits = move || -> Option<u64> {
        system_ctx.system_entity_id.get()
    };

    // Check if THIS client has control by comparing EntityControl.client_id with our own connection ID
    // Use the System entity (from context) to check control status
    let has_control = move || {
        let my_id = ctx.my_connection_id.get();
        Some(control_state.get().client_id) == my_id
    };

    // Check if another client has control
    let other_has_control = move || {
        let my_id = ctx.my_connection_id.get();
        let state = control_state.get();
        // Someone has control and it's not us
        state.client_id.id != 0 && Some(state.client_id) != my_id
    };

    view! {
        <button
            class=move || if has_control() {
                "bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] text-[8px] px-2 py-0.5 rounded hover:bg-[#22c55e30] flex items-center gap-1"
            } else if other_has_control() {
                // Another client has control - show red/warning style
                "bg-[#ff444420] border border-[#ff444440] text-[#ff4444] text-[8px] px-2 py-0.5 rounded hover:bg-[#ff444430] flex items-center gap-1"
            } else {
                "bg-[#f59e0b20] border border-[#f59e0b40] text-[#f59e0b] text-[8px] px-2 py-0.5 rounded hover:bg-[#f59e0b30] flex items-center gap-1"
            }
            on:click={
                let ctx = ctx.clone();
                let toast = toast.clone();
                move |_| {
                    leptos::logging::log!("[ControlButton] Button clicked");
                    let Some(entity_bits) = system_entity_bits() else {
                        leptos::logging::log!("[ControlButton] No System entity found");
                        toast.error("System not ready - please wait");
                        return;
                    };
                    leptos::logging::log!("[ControlButton] System entity bits: {}", entity_bits);

                    if has_control() {
                        leptos::logging::log!("[ControlButton] Releasing control");
                        ctx.send(ControlRequest::Release(entity_bits));
                        // Toast will be shown by ControlResponseHandler when server responds
                    } else {
                        leptos::logging::log!("[ControlButton] Requesting control");
                        ctx.send(ControlRequest::Take(entity_bits));
                        // Toast will be shown by ControlResponseHandler when server responds
                    }
                }
            }
            title=move || if has_control() {
                "You have control. Click to release.".to_string()
            } else if other_has_control() {
                "Another client has control. Click to request.".to_string()
            } else {
                "Request control of the apparatus".to_string()
            }
        >
            {move || if has_control() {
                view! {
                    <svg class="w-2.5 h-2.5" fill="currentColor" viewBox="0 0 24 24">
                        <path d="M12 1L3 5v6c0 5.55 3.84 10.74 9 12 5.16-1.26 9-6.45 9-12V5l-9-4zm-2 16l-4-4 1.41-1.41L10 14.17l6.59-6.59L18 9l-8 8z"/>
                    </svg>
                    "IN CONTROL"
                }.into_any()
            } else if other_has_control() {
                view! {
                    <svg class="w-2.5 h-2.5" fill="currentColor" viewBox="0 0 24 24">
                        <path d="M12 1L3 5v6c0 5.55 3.84 10.74 9 12 5.16-1.26 9-6.45 9-12V5l-9-4z"/>
                    </svg>
                    "CONTROLLED"
                }.into_any()
            } else {
                view! {
                    <svg class="w-2.5 h-2.5" fill="currentColor" viewBox="0 0 24 24">
                        <path d="M12 1L3 5v6c0 5.55 3.84 10.74 9 12 5.16-1.26 9-6.45 9-12V5l-9-4z"/>
                    </svg>
                    "REQUEST CONTROL"
                }.into_any()
            }}
        </button>
    }
}

/// Component that listens for ControlResponse messages and shows appropriate feedback.
///
/// This is a "headless" component that doesn't render anything visible, but handles
/// the server's response to control requests.
#[component]
pub fn ControlResponseHandler() -> impl IntoView {
    use pl3xus_common::ControlResponseKind;

    let toast = crate::components::use_toast();
    let control_response = use_message::<ControlResponse>();

    // Track the last sequence number we processed to avoid duplicate toasts
    // Use StoredValue instead of RwSignal to avoid any reactive issues
    let last_sequence = StoredValue::new(0u64);

    // Effect to handle control responses
    Effect::new(move |_| {
        let response = control_response.get();

        // Skip if sequence is 0 (default state) or same as last processed
        if response.sequence == 0 {
            return;
        }

        // Check last processed WITHOUT creating a reactive dependency
        let last = last_sequence.get_value();
        if response.sequence == last {
            return;
        }

        // Update last processed first to avoid duplicate processing
        last_sequence.set_value(response.sequence);

        match &response.kind {
            ControlResponseKind::None => {}
            ControlResponseKind::Taken => {
                toast.success("You now have control of the robot");
            }
            ControlResponseKind::Released => {
                toast.info("Control released");
            }
            ControlResponseKind::AlreadyControlled { by_client } => {
                toast.warning(format!("Control denied - robot is controlled by client {}", by_client));
            }
            ControlResponseKind::NotControlled => {
                toast.info("Robot is not currently controlled");
            }
            ControlResponseKind::ControlRequested { by_client } => {
                toast.warning(format!("Client {} is requesting control of the robot", by_client));
            }
            ControlResponseKind::Error(msg) => {
                toast.error(format!("Control error: {}", msg));
            }
        }
    });

    // This component doesn't render anything visible
    view! {}
}



/// Component that watches connection state changes and shows toast notifications.
///
/// This is a "headless" component that detects when:
/// - Connection attempt fails (was connecting, now disconnected)
/// - Connection succeeds (was connecting, now connected)
/// - Connection lost (was connected, now disconnected)
#[component]
pub fn ConnectionStateHandler() -> impl IntoView {
    let toast = crate::components::use_toast();
    let system_ctx = use_system_entity();

    // Subscribe to robot's connection state (ConnectionState lives on robot entity)
    let (connection_state, robot_exists) = use_entity_component::<ConnectionState, _>(move || system_ctx.robot_entity_id.get());

    // Track previous state to detect transitions
    // (robot_existed, was_connecting, was_connected, robot_name)
    let prev_state = StoredValue::new((false, false, false, String::new()));

    Effect::new(move |_| {
        let exists = robot_exists.get();
        let current = connection_state.get();

        let (is_connecting, is_connected, robot_name) = if exists {
            (current.robot_connecting, current.robot_connected, current.robot_name.clone())
        } else {
            (false, false, String::new())
        };

        let (prev_existed, was_connecting, was_connected, prev_name) = prev_state.get_value();

        // Detect state transitions (only when robot entity exists or just disappeared)
        if prev_existed && exists {
            // Robot entity exists in both states - check connection state transitions
            if was_connecting && !is_connecting && !is_connected {
                // Was connecting, now not connecting and not connected = connection failed
                toast.error("Failed to connect to robot");
            } else if was_connecting && is_connected {
                // Was connecting, now connected = success
                let name = if robot_name.is_empty() { "Robot".to_string() } else { robot_name.clone() };
                toast.success(format!("Connected to {}", name));
            } else if was_connected && !is_connected && !is_connecting {
                // Was connected, now disconnected = connection lost or intentional disconnect
                let name = if prev_name.is_empty() { "Robot".to_string() } else { prev_name };
                toast.info(format!("Disconnected from {}", name));
            }
        } else if prev_existed && !exists && was_connected {
            // Robot entity was despawned while connected - show disconnect message
            let name = if prev_name.is_empty() { "Robot".to_string() } else { prev_name };
            toast.info(format!("Disconnected from {}", name));
        }

        // Update previous state
        prev_state.set_value((exists, is_connecting, is_connected, robot_name));
    });

    view! {}
}

/// Component that listens for ProgramNotification messages from the server.
///
/// This is a "headless" component that handles server-broadcast notifications
/// about program events (completion, errors, etc.) and shows appropriate toasts.
/// All connected clients receive these notifications simultaneously.
#[component]
pub fn ProgramNotificationHandler() -> impl IntoView {
    use fanuc_replica_plugins::{ProgramNotification, ProgramNotificationKind};

    let toast = crate::components::use_toast();
    let notification = use_message::<ProgramNotification>();

    // Track the last sequence number we processed to avoid duplicate toasts
    let last_sequence = StoredValue::new(0u64);

    Effect::new(move |_| {
        let notif = notification.get();

        // Skip if sequence is 0 (default state) or same as last processed
        if notif.sequence == 0 {
            return;
        }

        let last = last_sequence.get_value();
        if notif.sequence == last {
            return;
        }

        // Update last processed first to avoid duplicate processing
        last_sequence.set_value(notif.sequence);

        match &notif.kind {
            ProgramNotificationKind::None => {}
            ProgramNotificationKind::Completed { program_name, total_instructions } => {
                toast.success(format!(
                    "✅ Program '{}' completed ({} instructions)",
                    program_name, total_instructions
                ));
            }
            ProgramNotificationKind::Stopped { program_name, at_line } => {
                toast.info(format!(
                    "⏹️ Program '{}' stopped at line {}",
                    program_name, at_line
                ));
            }
            ProgramNotificationKind::Error { program_name, at_line, error_message } => {
                toast.error(format!(
                    "❌ Program '{}' error at line {}: {}",
                    program_name, at_line, error_message
                ));
            }
        }
    });

    view! {}
}

/// Component that listens for ConsoleLogEntry messages from the server.
///
/// This is a "headless" component that handles server-broadcast console messages
/// and adds them to the local console display. All connected clients receive
/// these messages simultaneously.
#[component]
pub fn ConsoleLogHandler() -> impl IntoView {
    use fanuc_replica_plugins::{ConsoleLogEntry, ConsoleDirection, ConsoleMsgType};
    use crate::pages::dashboard::context::{WorkspaceContext, MessageDirection, MessageType, ConsoleMessage};

    let ctx = use_context::<WorkspaceContext>();
    let console_entry = use_message::<ConsoleLogEntry>();

    // Track the last timestamp_ms we processed to avoid duplicate entries
    let last_timestamp = StoredValue::new(0u64);

    Effect::new(move |_| {
        let entry = console_entry.get();

        // Skip if this is the same entry we already processed
        if entry.timestamp_ms == 0 || entry.timestamp_ms == last_timestamp.get_value() {
            return;
        }

        // Update last processed first to avoid duplicate processing
        last_timestamp.set_value(entry.timestamp_ms);

        // TODO: Remove the client types and just use the server types directly.
        // Convert server types to client types
        let direction = match entry.direction {
            ConsoleDirection::Sent => MessageDirection::Sent,
            ConsoleDirection::Received => MessageDirection::Received,
            ConsoleDirection::System => MessageDirection::System,
        };
        let msg_type = match entry.msg_type {
            ConsoleMsgType::Command => MessageType::Command,
            ConsoleMsgType::Response => MessageType::Response,
            ConsoleMsgType::Error => MessageType::Error,
            ConsoleMsgType::Status => MessageType::Status,
            ConsoleMsgType::Config => MessageType::Config,
        };

        // Add to console if context is available
        // Use update_untracked to avoid reactive graph issues when updating
        // signals inside Effects (per LESSONS_LEARNED.md)
        if let Some(ctx) = &ctx {
            ctx.console_messages.try_update_untracked(|msgs| {
                msgs.push(ConsoleMessage {
                    timestamp: entry.timestamp.clone(),
                    timestamp_ms: entry.timestamp_ms,
                    content: entry.content.clone(),
                    direction,
                    msg_type: msg_type.clone(),
                    sequence_id: entry.sequence_id,
                });
                // Keep only last 500 messages
                if msgs.len() > 500 {
                    msgs.remove(0);
                }
            });
            ctx.console_messages.notify();

            // Also add to error log if it's an error
            if matches!(msg_type, MessageType::Error) {
                ctx.error_log.try_update_untracked(|errors| {
                    errors.push(entry.content.clone());
                    if errors.len() > 100 {
                        errors.remove(0);
                    }
                });
                ctx.error_log.notify();
            }
        }
    });

    view! {}
}

/// Component that listens for ServerNotification messages and shows appropriate feedback.
///
/// This is a "headless" component that handles server-sent notifications
/// (e.g., authorization denials) and displays them as toasts.
#[component]
pub fn ServerNotificationHandler() -> impl IntoView {
    use pl3xus_client::{NotificationLevel, ServerNotification};
    use crate::components::{use_toast, ToastType};

    let toast = use_toast();
    let notification = use_message::<ServerNotification>();

    // Track the last sequence number we processed to avoid duplicate toasts
    let last_sequence = StoredValue::new(0u64);

    Effect::new(move |_| {
        let notif = notification.get();

        // Skip if sequence is 0 (default state) or same as last processed
        if notif.sequence == 0 {
            return;
        }

        let last = last_sequence.get_value();
        if notif.sequence == last {
            return;
        }

        // Update last processed first to avoid duplicate processing
        last_sequence.set_value(notif.sequence);

        // Convert notification level to toast type
        let toast_type = match notif.level {
            NotificationLevel::Info => ToastType::Info,
            NotificationLevel::Success => ToastType::Success,
            NotificationLevel::Warning => ToastType::Warning,
            NotificationLevel::Error => ToastType::Error,
        };

        // Format message with context if available
        let message = if let Some(ctx) = &notif.context {
            format!("{} ({})", notif.message, ctx)
        } else {
            notif.message.clone()
        };

        toast.show(message, toast_type);
    });

    view! {}
}
