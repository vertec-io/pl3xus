//! Settings module - Robot and connection configuration.
//!
//! This module uses a two-panel layout:
//! - Left sidebar: Robot browser (list of saved robots) + System Settings
//! - Right panel: Robot-specific settings for the selected robot

use leptos::prelude::*;
use leptos::web_sys;

use pl3xus_client::use_request;
use fanuc_replica_types::*;
use crate::components::RobotCreationWizard;

/// Settings view with two-panel layout.
#[component]
pub fn SettingsView() -> impl IntoView {
    // Selected robot for editing
    let (selected_robot_id, set_selected_robot_id) = signal::<Option<i64>>(None);

    // Modal states
    let (show_add_robot, set_show_add_robot) = signal(false);
    let (show_delete_confirm, set_show_delete_confirm) = signal(false);
    let (robot_to_delete, set_robot_to_delete) = signal::<Option<(i64, String)>>(None);

    // Fetch robots
    let (fetch_robots, robots_state) = use_request::<ListRobotConnections>();

    // Track if we've loaded the robots
    let (has_loaded, set_has_loaded) = signal(false);

    // Load robot connections on mount (only once)
    {
        let fetch_robots = fetch_robots.clone();
        Effect::new(move |_| {
            // Only fetch once on mount
            if !has_loaded.get() {
                set_has_loaded.set(true);
                fetch_robots(ListRobotConnections);
            }
        });
    }

    // Get selected robot details
    let selected_robot = move || {
        let id = selected_robot_id.get()?;
        robots_state.get().data.as_ref()?.connections.iter().find(|r| r.id == id).cloned()
    };

    // Clone for closures
    let fetch_robots_for_settings = fetch_robots.clone();
    let fetch_robots_for_wizard = fetch_robots.clone();
    let fetch_robots_for_delete = fetch_robots.clone();

    view! {
        <div class="h-full flex flex-col">
            // Header
            <div class="h-8 border-b border-[#ffffff08] flex items-center px-3 shrink-0 bg-[#0d0d0d]">
                <h2 class="text-[11px] font-semibold text-white flex items-center">
                    <svg class="w-3.5 h-3.5 mr-1.5 text-[#00d9ff]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/>
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>
                    </svg>
                    "Settings"
                </h2>
            </div>

            // Main content - two panel layout
            <div class="flex-1 p-2 flex gap-2 min-h-0">
                // Left sidebar: Robot browser + System settings
                <RobotBrowser
                    robots_state=robots_state
                    selected_robot_id=selected_robot_id
                    set_selected_robot_id=set_selected_robot_id
                    set_show_add_robot=set_show_add_robot
                    set_show_delete_confirm=set_show_delete_confirm
                    set_robot_to_delete=set_robot_to_delete
                />

                // Right panel: Robot settings (or empty state)
                <RobotSettingsPanel
                    selected_robot=selected_robot
                    selected_robot_id=selected_robot_id
                    fetch_robots=fetch_robots_for_settings
                />
            </div>

            // Robot Creation Wizard
            <Show when=move || show_add_robot.get()>
                <RobotCreationWizard
                    on_close=move || set_show_add_robot.set(false)
                    on_created={
                        let fetch_robots = fetch_robots_for_wizard.clone();
                        move |id| {
                            set_show_add_robot.set(false);
                            set_selected_robot_id.set(Some(id));
                            fetch_robots(ListRobotConnections);
                        }
                    }
                />
            </Show>

            // Delete Confirmation Modal
            {
                let fetch_robots = fetch_robots_for_delete.clone();
                view! {
                    <Show when=move || show_delete_confirm.get()>
                        <DeleteConfirmModal
                            robot_to_delete=robot_to_delete
                            set_show_delete_confirm=set_show_delete_confirm
                            set_robot_to_delete=set_robot_to_delete
                            set_selected_robot_id=set_selected_robot_id
                            selected_robot_id=selected_robot_id
                            fetch_robots=fetch_robots.clone()
                        />
                    </Show>
                }
            }
        </div>
    }
}

/// Robot browser - left sidebar with list of saved robots and system settings.
#[component]
fn RobotBrowser(
    robots_state: Signal<pl3xus_client::UseRequestState<RobotConnectionsResponse>>,
    selected_robot_id: ReadSignal<Option<i64>>,
    set_selected_robot_id: WriteSignal<Option<i64>>,
    set_show_add_robot: WriteSignal<bool>,
    set_show_delete_confirm: WriteSignal<bool>,
    set_robot_to_delete: WriteSignal<Option<(i64, String)>>,
) -> impl IntoView {
    view! {
        <div class="w-56 flex flex-col gap-2 shrink-0">
            // Robot Connections section
            <div class="bg-[#0d0d0d] border border-[#ffffff08] rounded flex flex-col flex-1 min-h-0">
                // Header
                <div class="h-7 border-b border-[#ffffff08] flex items-center justify-between px-2 shrink-0">
                    <span class="text-[9px] font-semibold text-[#888888] uppercase tracking-wide">"Robot Connections"</span>
                    <button
                        class="text-[8px] px-1.5 py-0.5 bg-[#00d9ff20] text-[#00d9ff] rounded hover:bg-[#00d9ff30]"
                        on:click=move |_| set_show_add_robot.set(true)
                    >
                        "+ Add"
                    </button>
                </div>

                // Robot list
                <div class="flex-1 overflow-y-auto p-1.5 space-y-1">
                    {move || {
                        let state = robots_state.get();
                        if state.is_loading() {
                            view! {
                                <div class="text-[9px] text-[#555555] text-center py-4">"Loading..."</div>
                            }.into_any()
                        } else if let Some(data) = state.data.as_ref() {
                            if data.connections.is_empty() {
                                view! {
                                    <div class="text-[9px] text-[#555555] text-center py-4">
                                        "No saved robots"<br/>
                                        "Click + Add to create one"
                                    </div>
                                }.into_any()
                            } else {
                                data.connections.iter().map(|robot| {
                                    let robot_id = robot.id;
                                    let robot_name = robot.name.clone();
                                    let robot_ip = robot.ip_address.clone();
                                    let robot_port = robot.port;
                                    let is_selected = move || selected_robot_id.get() == Some(robot_id);

                                    view! {
                                        <div
                                            class=move || format!(
                                                "p-1.5 rounded cursor-pointer group {}",
                                                if is_selected() { "bg-[#00d9ff15] border border-[#00d9ff30]" } else { "hover:bg-[#ffffff05] border border-transparent" }
                                            )
                                            on:click=move |_| set_selected_robot_id.set(Some(robot_id))
                                        >
                                            <div class="flex items-center justify-between">
                                                <div class="flex items-center gap-1.5 min-w-0">
                                                    <div class="w-1.5 h-1.5 rounded-full bg-[#666666] shrink-0"></div>
                                                    <div class="min-w-0">
                                                        <div class="text-[9px] text-white font-medium truncate">{robot_name.clone()}</div>
                                                        <div class="text-[8px] text-[#555555] truncate">{format!("{}:{}", robot_ip, robot_port)}</div>
                                                    </div>
                                                </div>
                                                <button
                                                    class="text-[8px] px-1 py-0.5 text-[#ff4444] opacity-0 group-hover:opacity-100 hover:bg-[#ff444410] rounded"
                                                    title="Delete"
                                                    on:click=move |ev| {
                                                        ev.stop_propagation();
                                                        set_robot_to_delete.set(Some((robot_id, robot_name.clone())));
                                                        set_show_delete_confirm.set(true);
                                                    }
                                                >
                                                    "×"
                                                </button>
                                            </div>
                                        </div>
                                    }
                                }).collect::<Vec<_>>().into_any()
                            }
                        } else {
                            view! {
                                <div class="text-[9px] text-[#555555] text-center py-4">"Failed to load"</div>
                            }.into_any()
                        }
                    }}
                </div>
            </div>

            // System Settings section
            <SystemSettingsPanel />
        </div>
    }
}

/// System settings panel - version info and reset database.
#[component]
fn SystemSettingsPanel() -> impl IntoView {
    let (confirm_reset, set_confirm_reset) = signal(false);
    let (reset_database, reset_state) = use_request::<ResetDatabase>();
    let reset_database = StoredValue::new(reset_database);

    // Watch for reset completion
    Effect::new(move |_| {
        if let Some(data) = reset_state.get().data.as_ref() {
            if data.success {
                // Reload the page to reflect database reset
                if let Some(window) = web_sys::window() {
                    let _ = window.location().reload();
                }
            }
            set_confirm_reset.set(false);
        }
    });

    view! {
        <div class="bg-[#0d0d0d] border border-[#ffffff08] rounded">
            <div class="h-7 border-b border-[#ffffff08] flex items-center px-2">
                <span class="text-[9px] font-semibold text-[#888888] uppercase tracking-wide">"System"</span>
            </div>
            <div class="p-2 space-y-2">
                // Version info
                <div class="text-[8px]">
                    <span class="text-[#555555]">"Version: "</span>
                    <span class="text-[#888888] font-mono">"0.8.0"</span>
                </div>
                <div class="text-[8px]">
                    <span class="text-[#555555]">"RMI Protocol: "</span>
                    <span class="text-[#888888] font-mono">"v5+"</span>
                </div>

                // Reset database button with inline confirmation
                <div class="pt-2 border-t border-[#ffffff08]">
                    <Show
                        when=move || confirm_reset.get()
                        fallback=move || view! {
                            <button
                                class="w-full text-[8px] px-2 py-1 bg-[#ff444410] border border-[#ff444420] text-[#ff4444] rounded hover:bg-[#ff444420]"
                                on:click=move |_| set_confirm_reset.set(true)
                            >
                                "Reset Database"
                            </button>
                        }
                    >
                        <div class="space-y-1">
                            <p class="text-[8px] text-[#ff4444]">"Delete all data?"</p>
                            <div class="flex gap-1">
                                <button
                                    class="flex-1 text-[8px] px-2 py-1 bg-[#ff4444] text-white rounded hover:bg-[#ff5555]"
                                    on:click=move |_| {
                                        reset_database.get_value()(ResetDatabase);
                                    }
                                >
                                    "Yes"
                                </button>
                                <button
                                    class="flex-1 text-[8px] px-2 py-1 bg-[#1a1a1a] border border-[#ffffff08] text-[#888888] rounded hover:text-white"
                                    on:click=move |_| set_confirm_reset.set(false)
                                >
                                    "No"
                                </button>
                            </div>
                        </div>
                    </Show>
                </div>
            </div>
        </div>
    }
}


/// Robot settings panel - displays and allows editing of selected robot settings.
#[component]
fn RobotSettingsPanel(
    selected_robot: impl Fn() -> Option<RobotConnection> + Send + Sync + 'static,
    selected_robot_id: ReadSignal<Option<i64>>,
    fetch_robots: impl Fn(ListRobotConnections) + Clone + Send + Sync + 'static,
) -> impl IntoView {
    // Form fields for connection details
    let (edit_name, set_edit_name) = signal(String::new());
    let (edit_desc, set_edit_desc) = signal(String::new());
    let (edit_ip, set_edit_ip) = signal(String::new());
    let (edit_port, set_edit_port) = signal(String::new());

    // Form fields for motion defaults
    let (edit_speed, set_edit_speed) = signal("100.0".to_string());
    let (edit_speed_type, set_edit_speed_type) = signal("mmSec".to_string());
    let (edit_term_type, set_edit_term_type) = signal("CNT".to_string());

    // Form fields for orientation defaults
    let (edit_w, set_edit_w) = signal("0.0".to_string());
    let (edit_p, set_edit_p) = signal("0.0".to_string());
    let (edit_r, set_edit_r) = signal("0.0".to_string());

    // Form fields for jog defaults
    let (edit_cart_speed, set_edit_cart_speed) = signal("10.0".to_string());
    let (edit_cart_step, set_edit_cart_step) = signal("1.0".to_string());
    let (edit_joint_speed, set_edit_joint_speed) = signal("0.1".to_string());
    let (edit_joint_step, set_edit_joint_step) = signal("0.25".to_string());

    // Track changes and save status
    let (has_changes, set_has_changes) = signal(false);
    let (save_status, set_save_status) = signal::<Option<String>>(None);

    // Configuration management
    let (configurations, set_configurations) = signal::<Vec<RobotConfiguration>>(Vec::new());
    let (selected_config_id, set_selected_config_id) = signal::<Option<i64>>(None);
    let (show_config_form, set_show_config_form) = signal(false);
    let (editing_config_id, set_editing_config_id) = signal::<Option<i64>>(None);

    // Configuration form fields
    let (config_name, set_config_name) = signal(String::new());
    let (config_uframe, set_config_uframe) = signal("0".to_string());
    let (config_utool, set_config_utool) = signal("0".to_string());
    let (config_front, set_config_front) = signal("0".to_string());
    let (config_up, set_config_up) = signal("0".to_string());
    let (config_left, set_config_left) = signal("0".to_string());
    let (config_flip, set_config_flip) = signal("0".to_string());
    let (config_turn4, set_config_turn4) = signal("0".to_string());
    let (config_turn5, set_config_turn5) = signal("0".to_string());
    let (config_turn6, set_config_turn6) = signal("0".to_string());
    let (config_is_default, set_config_is_default) = signal(false);

    // Update request
    let (update_robot, _update_state) = use_request::<UpdateRobotConnection>();
    let (fetch_configs, configs_state) = use_request::<GetRobotConfigurations>();

    // Configuration CRUD requests
    let (create_config, create_config_state) = use_request::<CreateConfiguration>();
    let (update_config, update_config_state) = use_request::<UpdateConfiguration>();
    let (delete_config, delete_config_state) = use_request::<DeleteConfiguration>();
    let (set_default_config, set_default_config_state) = use_request::<SetDefaultConfiguration>();

    // Configuration delete confirmation
    let (show_delete_config_confirm, set_show_delete_config_confirm) = signal(false);
    let (config_to_delete, set_config_to_delete) = signal::<Option<i64>>(None);
    let (config_error_message, set_config_error_message) = signal::<Option<String>>(None);
    let (is_saving_config, set_is_saving_config) = signal(false);

    // Store in StoredValue so they can be accessed inside closures
    let fetch_robots = StoredValue::new(fetch_robots);
    let update_robot = StoredValue::new(update_robot);
    let selected_robot = StoredValue::new(selected_robot);
    let fetch_configs = StoredValue::new(fetch_configs);
    let create_config = StoredValue::new(create_config);
    let update_config = StoredValue::new(update_config);
    let delete_config = StoredValue::new(delete_config);
    let set_default_config = StoredValue::new(set_default_config);

    // Track last loaded robot ID to avoid re-fetching
    let (last_loaded_robot_id, set_last_loaded_robot_id) = signal::<Option<i64>>(None);

    // Load robot data when selection changes
    Effect::new(move |_| {
        // Only track selected_robot_id changes, not the full robot data
        let current_id = selected_robot_id.get();
        let last_id = last_loaded_robot_id.get_untracked();

        // Only proceed if the ID actually changed
        if current_id != last_id {
            set_last_loaded_robot_id.set(current_id);

            if let Some(robot) = selected_robot.with_value(|f| f()) {
                set_edit_name.set(robot.name.clone());
                set_edit_desc.set(robot.description.clone().unwrap_or_default());
                set_edit_ip.set(robot.ip_address.clone());
                set_edit_port.set(robot.port.to_string());
                // Motion defaults
                set_edit_speed.set(robot.default_speed.to_string());
                set_edit_speed_type.set(robot.default_speed_type.clone());
                set_edit_term_type.set(robot.default_term_type.clone());
                set_edit_w.set(robot.default_w.to_string());
                set_edit_p.set(robot.default_p.to_string());
                set_edit_r.set(robot.default_r.to_string());
                // Jog defaults
                set_edit_cart_speed.set(robot.default_cartesian_jog_speed.to_string());
                set_edit_cart_step.set(robot.default_cartesian_jog_step.to_string());
                set_edit_joint_speed.set(robot.default_joint_jog_speed.to_string());
                set_edit_joint_step.set(robot.default_joint_jog_step.to_string());
                set_has_changes.set(false);
                set_save_status.set(None);
                // Load configurations for this robot
                fetch_configs.with_value(|f| f(GetRobotConfigurations { robot_connection_id: robot.id }));
            }
        }
    });

    // Track last received configs to avoid re-setting
    let (last_configs_version, set_last_configs_version) = signal::<usize>(0);

    // Update configurations when response arrives
    Effect::new(move |_| {
        let state = configs_state.get();
        if let Some(response) = &state.data {
            // Use a version counter to track changes
            let new_version = last_configs_version.get_untracked() + 1;
            set_last_configs_version.set(new_version);
            set_configurations.set(response.configurations.clone());
        }
    });

    // Handle create configuration response
    Effect::new(move |_| {
        let state = create_config_state.get();
        if let Some(response) = &state.data {
            set_is_saving_config.set(false);
            if response.success {
                set_show_config_form.set(false);
                set_config_error_message.set(None);
                // Refresh configurations
                if let Some(robot_id) = selected_robot_id.get_untracked() {
                    fetch_configs.with_value(|f| f(GetRobotConfigurations { robot_connection_id: robot_id }));
                }
            } else {
                set_config_error_message.set(response.error.clone());
            }
        }
    });

    // Handle update configuration response
    Effect::new(move |_| {
        let state = update_config_state.get();
        if let Some(response) = &state.data {
            set_is_saving_config.set(false);
            if response.success {
                set_show_config_form.set(false);
                set_config_error_message.set(None);
                // Refresh configurations
                if let Some(robot_id) = selected_robot_id.get_untracked() {
                    fetch_configs.with_value(|f| f(GetRobotConfigurations { robot_connection_id: robot_id }));
                }
            } else {
                set_config_error_message.set(response.error.clone());
            }
        }
    });

    // Handle delete configuration response
    Effect::new(move |_| {
        let state = delete_config_state.get();
        if let Some(response) = &state.data {
            if response.success {
                set_show_delete_config_confirm.set(false);
                set_config_to_delete.set(None);
                // Refresh configurations
                if let Some(robot_id) = selected_robot_id.get_untracked() {
                    fetch_configs.with_value(|f| f(GetRobotConfigurations { robot_connection_id: robot_id }));
                }
            }
        }
    });

    // Handle set default configuration response
    Effect::new(move |_| {
        let state = set_default_config_state.get();
        if let Some(response) = &state.data {
            if response.success {
                // Refresh configurations to show updated default
                if let Some(robot_id) = selected_robot_id.get_untracked() {
                    fetch_configs.with_value(|f| f(GetRobotConfigurations { robot_connection_id: robot_id }));
                }
            }
        }
    });

    view! {
        <div class="flex-1 bg-[#0a0a0a] rounded border border-[#ffffff08] flex flex-col min-h-0">
            {move || {
                if let Some(robot) = selected_robot.with_value(|f| f()) {
                    let robot_id = robot.id;
                    let robot_name = robot.name.clone();

                    view! {
                        // Header with robot name
                        <div class="flex items-center justify-between p-3 border-b border-[#ffffff08]">
                            <h3 class="text-[11px] font-semibold text-white flex items-center">
                                <svg class="w-4 h-4 mr-2 text-[#00d9ff]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z"/>
                                </svg>
                                "Robot Settings: "
                                <span class="text-[#00d9ff] ml-1">{robot_name}</span>
                            </h3>
                            <div class="flex items-center gap-2">
                                {move || save_status.get().map(|s| view! {
                                    <span class="text-[9px] text-[#22c55e]">{s}</span>
                                })}
                                <button
                                    class=move || format!(
                                        "text-[9px] px-3 py-1 rounded transition-colors {}",
                                        if has_changes.get() {
                                            "bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] hover:bg-[#22c55e30]"
                                        } else {
                                            "bg-[#111111] border border-[#ffffff08] text-[#555555]"
                                        }
                                    )
                                    disabled=move || !has_changes.get()
                                    on:click=move |_| {
                                        let port: i32 = edit_port.get().parse().unwrap_or(60008);
                                        update_robot.with_value(|f| f(UpdateRobotConnection {
                                            id: robot_id,
                                            name: Some(edit_name.get()),
                                            description: Some(edit_desc.get()),
                                            ip_address: Some(edit_ip.get()),
                                            port: Some(port),
                                        }));
                                        set_has_changes.set(false);
                                        set_save_status.set(Some("✓ Saved".to_string()));
                                        fetch_robots.with_value(|f| f(ListRobotConnections));
                                    }
                                >
                                    "Save Changes"
                                </button>
                            </div>
                        </div>

                        // Settings content
                        <div class="flex-1 overflow-y-auto p-3 space-y-4">
                            // Connection Details
                            <div>
                                <h4 class="text-[10px] font-semibold text-[#888888] mb-2 uppercase tracking-wide">"Connection Details"</h4>
                                <div class="grid grid-cols-2 gap-3">
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"Name"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                                            prop:value=move || edit_name.get()
                                            on:input=move |ev| {
                                                set_edit_name.set(event_target_value(&ev));
                                                set_has_changes.set(true);
                                            }
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"Description"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                                            placeholder="Optional"
                                            prop:value=move || edit_desc.get()
                                            on:input=move |ev| {
                                                set_edit_desc.set(event_target_value(&ev));
                                                set_has_changes.set(true);
                                            }
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"IP Address"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                            prop:value=move || edit_ip.get()
                                            on:input=move |ev| {
                                                set_edit_ip.set(event_target_value(&ev));
                                                set_has_changes.set(true);
                                            }
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"Port"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                            prop:value=move || edit_port.get()
                                            on:input=move |ev| {
                                                set_edit_port.set(event_target_value(&ev));
                                                set_has_changes.set(true);
                                            }
                                        />
                                    </div>
                                </div>
                            </div>

                            // Motion Defaults
                            <div>
                                <h4 class="text-[10px] font-semibold text-[#888888] mb-2 uppercase tracking-wide">"Motion Defaults"</h4>
                                <div class="grid grid-cols-3 gap-3">
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"Default Speed"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                            prop:value=move || edit_speed.get()
                                            on:input=move |ev| {
                                                set_edit_speed.set(event_target_value(&ev));
                                                set_has_changes.set(true);
                                            }
                                            placeholder="100.0"
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"Speed Type"</label>
                                        <select
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                                            on:change=move |ev| {
                                                set_edit_speed_type.set(event_target_value(&ev));
                                                set_has_changes.set(true);
                                            }
                                        >
                                            <option value="mmSec" selected=move || edit_speed_type.get() == "mmSec">"mm/sec (Linear)"</option>
                                            <option value="InchMin" selected=move || edit_speed_type.get() == "InchMin">"0.1 inch/min"</option>
                                            <option value="Time" selected=move || edit_speed_type.get() == "Time">"0.1 seconds"</option>
                                            <option value="mSec" selected=move || edit_speed_type.get() == "mSec">"milliseconds"</option>
                                        </select>
                                    </div>
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"Termination"</label>
                                        <select
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                                            on:change=move |ev| {
                                                set_edit_term_type.set(event_target_value(&ev));
                                                set_has_changes.set(true);
                                            }
                                        >
                                            <option value="CNT" selected=move || edit_term_type.get() == "CNT">"CNT"</option>
                                            <option value="FINE" selected=move || edit_term_type.get() == "FINE">"FINE"</option>
                                        </select>
                                    </div>
                                </div>
                            </div>

                            // Orientation Defaults
                            <div>
                                <h4 class="text-[10px] font-semibold text-[#888888] mb-2 uppercase tracking-wide">"Orientation Defaults (W, P, R)"</h4>
                                <div class="grid grid-cols-3 gap-3">
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"W (deg)"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                            prop:value=move || edit_w.get()
                                            on:input=move |ev| {
                                                set_edit_w.set(event_target_value(&ev));
                                                set_has_changes.set(true);
                                            }
                                            placeholder="0.0"
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"P (deg)"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                            prop:value=move || edit_p.get()
                                            on:input=move |ev| {
                                                set_edit_p.set(event_target_value(&ev));
                                                set_has_changes.set(true);
                                            }
                                            placeholder="0.0"
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"R (deg)"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                            prop:value=move || edit_r.get()
                                            on:input=move |ev| {
                                                set_edit_r.set(event_target_value(&ev));
                                                set_has_changes.set(true);
                                            }
                                            placeholder="0.0"
                                        />
                                    </div>
                                </div>
                            </div>

                            // Jog Defaults
                            <div>
                                <h4 class="text-[10px] font-semibold text-[#888888] mb-2 uppercase tracking-wide">"Jog Defaults"</h4>
                                <div class="grid grid-cols-2 gap-3">
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"Cartesian Jog Speed"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                            prop:value=move || edit_cart_speed.get()
                                            on:input=move |ev| {
                                                set_edit_cart_speed.set(event_target_value(&ev));
                                                set_has_changes.set(true);
                                            }
                                            placeholder="10.0"
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"Cartesian Jog Step"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                            prop:value=move || edit_cart_step.get()
                                            on:input=move |ev| {
                                                set_edit_cart_step.set(event_target_value(&ev));
                                                set_has_changes.set(true);
                                            }
                                            placeholder="1.0"
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"Joint Jog Speed"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                            prop:value=move || edit_joint_speed.get()
                                            on:input=move |ev| {
                                                set_edit_joint_speed.set(event_target_value(&ev));
                                                set_has_changes.set(true);
                                            }
                                            placeholder="0.1"
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"Joint Jog Step"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                            prop:value=move || edit_joint_step.get()
                                            on:input=move |ev| {
                                                set_edit_joint_step.set(event_target_value(&ev));
                                                set_has_changes.set(true);
                                            }
                                            placeholder="0.25"
                                        />
                                    </div>
                                </div>
                            </div>

                            // Configurations Section
                            <div>
                                <div class="flex items-center justify-between mb-2">
                                    <h4 class="text-[10px] font-semibold text-[#888888] uppercase tracking-wide">"Robot Configurations"</h4>
                                    <button
                                        class="text-[8px] px-2 py-0.5 bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] rounded hover:bg-[#22c55e30]"
                                        on:click=move |_| {
                                            set_editing_config_id.set(None);
                                            set_config_name.set(String::new());
                                            set_config_uframe.set("0".to_string());
                                            set_config_utool.set("0".to_string());
                                            set_config_front.set("0".to_string());
                                            set_config_up.set("0".to_string());
                                            set_config_left.set("0".to_string());
                                            set_config_flip.set("0".to_string());
                                            set_config_turn4.set("0".to_string());
                                            set_config_turn5.set("0".to_string());
                                            set_config_turn6.set("0".to_string());
                                            set_config_is_default.set(false);
                                            set_show_config_form.set(true);
                                        }
                                    >
                                        "+ Add Configuration"
                                    </button>
                                </div>

                                // Configuration list
                                <div class="space-y-1.5">
                                    <For
                                        each=move || configurations.get()
                                        key=|config| config.id
                                        children=move |config| {
                                            let config_id = config.id;
                                            let is_selected = move || selected_config_id.get() == Some(config_id);

                                            // Helper to get current config data from signal
                                            let get_config = move || {
                                                configurations.get()
                                                    .into_iter()
                                                    .find(|c| c.id == config_id)
                                            };

                                            view! {
                                                <div
                                                    class=move || {
                                                        if is_selected() {
                                                            "bg-[#00d9ff10] border border-[#00d9ff40] rounded p-2 cursor-pointer hover:bg-[#00d9ff15]"
                                                        } else {
                                                            "bg-[#111111] border border-[#ffffff08] rounded p-2 cursor-pointer hover:bg-[#ffffff05]"
                                                        }
                                                    }
                                                    on:click=move |_| {
                                                        if is_selected() {
                                                            set_selected_config_id.set(None);
                                                        } else {
                                                            set_selected_config_id.set(Some(config_id));
                                                        }
                                                    }
                                                >
                                                    <div class="flex items-center justify-between">
                                                        <div class="flex-1 min-w-0">
                                                            <div class="flex items-center gap-1.5">
                                                                <span class="text-[9px] text-white font-medium">
                                                                    {move || get_config().map(|c| c.name.clone()).unwrap_or_default()}
                                                                </span>
                                                                {move || {
                                                                    if let Some(cfg) = get_config() {
                                                                        if cfg.is_default {
                                                                            view! {
                                                                                <span class="text-[8px] px-1.5 py-0.5 bg-[#fbbf2420] border border-[#fbbf2440] text-[#fbbf24] rounded">"DEFAULT"</span>
                                                                            }.into_any()
                                                                        } else {
                                                                            view! { <span></span> }.into_any()
                                                                        }
                                                                    } else {
                                                                        view! { <span></span> }.into_any()
                                                                    }
                                                                }}
                                                            </div>
                                                            <div class="text-[8px] text-[#666666] mt-0.5 font-mono">
                                                                {move || {
                                                                    get_config()
                                                                        .map(|c| format!("UFrame: {} | UTool: {}", c.u_frame_number, c.u_tool_number))
                                                                        .unwrap_or_default()
                                                                }}
                                                            </div>
                                                        </div>
                                                        <div class="flex gap-1 ml-2">
                                                            <button
                                                                class="text-[8px] px-1.5 py-0.5 text-[#00d9ff] hover:bg-[#00d9ff10] rounded"
                                                                title="Edit"
                                                                on:click=move |ev| {
                                                                    ev.stop_propagation();
                                                                    if let Some(cfg) = get_config() {
                                                                        set_editing_config_id.set(Some(config_id));
                                                                        set_config_name.set(cfg.name.clone());
                                                                        set_config_uframe.set(cfg.u_frame_number.to_string());
                                                                        set_config_utool.set(cfg.u_tool_number.to_string());
                                                                        set_config_front.set(cfg.front.to_string());
                                                                        set_config_up.set(cfg.up.to_string());
                                                                        set_config_left.set(cfg.left.to_string());
                                                                        set_config_flip.set(cfg.flip.to_string());
                                                                        set_config_turn4.set(cfg.turn4.to_string());
                                                                        set_config_turn5.set(cfg.turn5.to_string());
                                                                        set_config_turn6.set(cfg.turn6.to_string());
                                                                        set_config_is_default.set(cfg.is_default);
                                                                        set_show_config_form.set(true);
                                                                    }
                                                                }
                                                            >
                                                                "✎"
                                                            </button>
                                                            // Set as Default button (only for non-default configs)
                                                            <Show when=move || !get_config().map(|c| c.is_default).unwrap_or(false)>
                                                                <button
                                                                    class="text-[8px] px-1.5 py-0.5 text-[#fbbf24] hover:bg-[#fbbf2410] rounded"
                                                                    title="Set as Default"
                                                                    on:click=move |ev| {
                                                                        ev.stop_propagation();
                                                                        set_default_config.with_value(|f| f(SetDefaultConfiguration { id: config_id }));
                                                                    }
                                                                >
                                                                    "⭐"
                                                                </button>
                                                            </Show>
                                                            <button
                                                                class="text-[8px] px-1.5 py-0.5 text-[#ff4444] hover:bg-[#ff444410] rounded"
                                                                title="Delete"
                                                                on:click=move |ev| {
                                                                    ev.stop_propagation();
                                                                    set_config_to_delete.set(Some(config_id));
                                                                    set_show_delete_config_confirm.set(true);
                                                                }
                                                            >
                                                                "×"
                                                            </button>
                                                        </div>
                                                    </div>

                                                    // Configuration details (shown when selected)
                                                    <Show when=is_selected>
                                                        <div class="mt-2 pt-2 border-t border-[#ffffff08] space-y-1.5">
                                                            {move || {
                                                                if let Some(cfg) = get_config() {
                                                                    view! {
                                                                        <div class="grid grid-cols-3 gap-2 text-[8px]">
                                                                            <div>
                                                                                <span class="text-[#666666]">"Front: "</span>
                                                                                <span class="text-white font-mono">{cfg.front}</span>
                                                                            </div>
                                                                            <div>
                                                                                <span class="text-[#666666]">"Up: "</span>
                                                                                <span class="text-white font-mono">{cfg.up}</span>
                                                                            </div>
                                                                            <div>
                                                                                <span class="text-[#666666]">"Left: "</span>
                                                                                <span class="text-white font-mono">{cfg.left}</span>
                                                                            </div>
                                                                            <div>
                                                                                <span class="text-[#666666]">"Flip: "</span>
                                                                                <span class="text-white font-mono">{cfg.flip}</span>
                                                                            </div>
                                                                            <div>
                                                                                <span class="text-[#666666]">"Turn4: "</span>
                                                                                <span class="text-white font-mono">{cfg.turn4}</span>
                                                                            </div>
                                                                            <div>
                                                                                <span class="text-[#666666]">"Turn5: "</span>
                                                                                <span class="text-white font-mono">{cfg.turn5}</span>
                                                                            </div>
                                                                            <div>
                                                                                <span class="text-[#666666]">"Turn6: "</span>
                                                                                <span class="text-white font-mono">{cfg.turn6}</span>
                                                                            </div>
                                                                        </div>
                                                                    }.into_any()
                                                                } else {
                                                                    view! { <div></div> }.into_any()
                                                                }
                                                            }}
                                                        </div>
                                                    </Show>
                                                </div>
                                            }
                                        }
                                    />
                                    {move || if configurations.get().is_empty() {
                                        view! {
                                            <div class="text-[8px] text-[#555555] text-center py-4 bg-[#111111] border border-[#ffffff08] rounded">
                                                "No configurations"<br/>
                                                "Click + Add to create one"
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! { <div></div> }.into_any()
                                    }}
                                </div>
                            </div>

                            // Quick Connect Button
                            <div class="mt-4 pt-4 border-t border-[#ffffff08]">
                                <button
                                    class="w-full text-[10px] px-4 py-2.5 bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] rounded-lg hover:bg-[#22c55e30] font-medium flex items-center justify-center gap-2"
                                    on:click=move |_| {
                                        // TODO: Navigate to connect page or trigger connection
                                        // For now, just log
                                        web_sys::console::log_1(&format!("Connect to robot: {}", robot_id).into());
                                    }
                                >
                                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z"/>
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                                    </svg>
                                    "Connect to this Robot"
                                </button>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    // Empty state
                    view! {
                        <div class="flex-1 flex items-center justify-center">
                            <div class="text-center">
                                <svg class="w-12 h-12 mx-auto mb-3 text-[#333333]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z"/>
                                </svg>
                                <p class="text-[11px] text-[#555555]">"Select a robot connection"</p>
                                <p class="text-[9px] text-[#444444] mt-1">"to view and edit its settings"</p>
                            </div>
                        </div>
                    }.into_any()
                }
            }}

            // Configuration Form Modal
            <Show when=move || show_config_form.get()>
                <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
                    <div class="bg-[#0a0a0a] border border-[#ffffff20] rounded-lg p-6 w-[500px] max-h-[80vh] overflow-y-auto">
                        <h3 class="text-[12px] font-semibold text-white mb-4">
                            {move || if editing_config_id.get().is_some() {
                                "Edit Configuration"
                            } else {
                                "New Configuration"
                            }}
                        </h3>

                        <div class="space-y-3">
                            // Configuration Name
                            <div>
                                <label class="block text-[#666666] text-[9px] mb-1">"Configuration Name"</label>
                                <input
                                    type="text"
                                    class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                                    prop:value=move || config_name.get()
                                    on:input=move |ev| set_config_name.set(event_target_value(&ev))
                                    placeholder="e.g., Default Config, Welding Setup"
                                />
                            </div>

                            // UFrame and UTool
                            <div class="grid grid-cols-2 gap-3">
                                <div>
                                    <label class="block text-[#666666] text-[9px] mb-1">"User Frame (UFrame)"</label>
                                    <input
                                        type="text"
                                        class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                        prop:value=move || config_uframe.get()
                                        on:input=move |ev| set_config_uframe.set(event_target_value(&ev))
                                        placeholder="0"
                                    />
                                </div>
                                <div>
                                    <label class="block text-[#666666] text-[9px] mb-1">"User Tool (UTool)"</label>
                                    <input
                                        type="text"
                                        class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                        prop:value=move || config_utool.get()
                                        on:input=move |ev| set_config_utool.set(event_target_value(&ev))
                                        placeholder="0"
                                    />
                                </div>
                            </div>

                            // Arm Configuration
                            <div>
                                <label class="block text-[#666666] text-[9px] mb-1">"Arm Configuration"</label>
                                <div class="grid grid-cols-3 gap-2">
                                    <div>
                                        <label class="block text-[#555555] text-[8px] mb-0.5">"Front"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1 text-[9px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                            prop:value=move || config_front.get()
                                            on:input=move |ev| set_config_front.set(event_target_value(&ev))
                                            placeholder="0"
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-[#555555] text-[8px] mb-0.5">"Up"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1 text-[9px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                            prop:value=move || config_up.get()
                                            on:input=move |ev| set_config_up.set(event_target_value(&ev))
                                            placeholder="0"
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-[#555555] text-[8px] mb-0.5">"Left"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1 text-[9px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                            prop:value=move || config_left.get()
                                            on:input=move |ev| set_config_left.set(event_target_value(&ev))
                                            placeholder="0"
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-[#555555] text-[8px] mb-0.5">"Flip"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1 text-[9px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                            prop:value=move || config_flip.get()
                                            on:input=move |ev| set_config_flip.set(event_target_value(&ev))
                                            placeholder="0"
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-[#555555] text-[8px] mb-0.5">"Turn4"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1 text-[9px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                            prop:value=move || config_turn4.get()
                                            on:input=move |ev| set_config_turn4.set(event_target_value(&ev))
                                            placeholder="0"
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-[#555555] text-[8px] mb-0.5">"Turn5"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1 text-[9px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                            prop:value=move || config_turn5.get()
                                            on:input=move |ev| set_config_turn5.set(event_target_value(&ev))
                                            placeholder="0"
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-[#555555] text-[8px] mb-0.5">"Turn6"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1 text-[9px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                            prop:value=move || config_turn6.get()
                                            on:input=move |ev| set_config_turn6.set(event_target_value(&ev))
                                            placeholder="0"
                                        />
                                    </div>
                                </div>
                            </div>

                            // Set as Default checkbox
                            <div class="flex items-center gap-2">
                                <input
                                    type="checkbox"
                                    id="config-is-default"
                                    class="w-3.5 h-3.5 bg-[#111111] border border-[#ffffff08] rounded"
                                    prop:checked=move || config_is_default.get()
                                    on:change=move |ev| set_config_is_default.set(event_target_checked(&ev))
                                />
                                <label for="config-is-default" class="text-[9px] text-[#888888]">"Set as default configuration"</label>
                            </div>
                        </div>

                        // Error message
                        {move || config_error_message.get().map(|msg| view! {
                            <div class="p-2 bg-[#ff444420] border border-[#ff444440] rounded text-[9px] text-[#ff4444] mb-3">
                                {msg}
                            </div>
                        })}

                        // Action buttons
                        <div class="flex gap-2 mt-4">
                            <button
                                class=move || format!(
                                    "flex-1 text-[10px] px-4 py-2 rounded font-medium {}",
                                    if is_saving_config.get() {
                                        "bg-[#00d9ff80] text-[#0a0a0a80] cursor-wait"
                                    } else {
                                        "bg-[#00d9ff] text-[#0a0a0a] hover:bg-[#00e5ff]"
                                    }
                                )
                                disabled=move || is_saving_config.get()
                                on:click=move |_| {
                                    let robot_id = match selected_robot_id.get() {
                                        Some(id) => id,
                                        None => return,
                                    };

                                    set_is_saving_config.set(true);
                                    set_config_error_message.set(None);

                                    if let Some(config_id) = editing_config_id.get() {
                                        // Update existing configuration
                                        update_config.with_value(|f| f(UpdateConfiguration {
                                            id: config_id,
                                            name: Some(config_name.get()),
                                            is_default: Some(config_is_default.get()),
                                            u_frame_number: config_uframe.get().parse().ok(),
                                            u_tool_number: config_utool.get().parse().ok(),
                                            front: config_front.get().parse().ok(),
                                            up: config_up.get().parse().ok(),
                                            left: config_left.get().parse().ok(),
                                            flip: config_flip.get().parse().ok(),
                                            turn4: config_turn4.get().parse().ok(),
                                            turn5: config_turn5.get().parse().ok(),
                                            turn6: config_turn6.get().parse().ok(),
                                        }));
                                    } else {
                                        // Create new configuration
                                        create_config.with_value(|f| f(CreateConfiguration {
                                            robot_connection_id: robot_id,
                                            name: config_name.get(),
                                            is_default: config_is_default.get(),
                                            u_frame_number: config_uframe.get().parse().unwrap_or(0),
                                            u_tool_number: config_utool.get().parse().unwrap_or(0),
                                            front: config_front.get().parse().unwrap_or(0),
                                            up: config_up.get().parse().unwrap_or(0),
                                            left: config_left.get().parse().unwrap_or(0),
                                            flip: config_flip.get().parse().unwrap_or(0),
                                            turn4: config_turn4.get().parse().unwrap_or(0),
                                            turn5: config_turn5.get().parse().unwrap_or(0),
                                            turn6: config_turn6.get().parse().unwrap_or(0),
                                        }));
                                    }
                                }
                            >
                                {move || {
                                    if is_saving_config.get() {
                                        "Saving..."
                                    } else if editing_config_id.get().is_some() {
                                        "Update"
                                    } else {
                                        "Create"
                                    }
                                }}
                            </button>
                            <button
                                class="flex-1 text-[10px] px-4 py-2 bg-[#1a1a1a] border border-[#ffffff08] text-[#888888] rounded hover:text-white"
                                on:click=move |_| {
                                    set_show_config_form.set(false);
                                    set_editing_config_id.set(None);
                                    set_config_name.set(String::new());
                                    set_config_uframe.set("0".to_string());
                                    set_config_utool.set("0".to_string());
                                    set_config_front.set("0".to_string());
                                    set_config_up.set("0".to_string());
                                    set_config_left.set("0".to_string());
                                    set_config_flip.set("0".to_string());
                                    set_config_turn4.set("0".to_string());
                                    set_config_turn5.set("0".to_string());
                                    set_config_turn6.set("0".to_string());
                                    set_config_is_default.set(false);
                                }
                            >
                                "Cancel"
                            </button>
                        </div>
                    </div>
                </div>
            </Show>

            // Delete Configuration Confirmation Modal
            <Show when=move || show_delete_config_confirm.get()>
                <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
                    <div class="bg-[#0a0a0a] border border-[#ff444440] rounded-lg p-6 w-[400px]">
                        <h3 class="text-[12px] font-semibold text-[#ff4444] mb-3 flex items-center">
                            <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/>
                            </svg>
                            "Delete Configuration"
                        </h3>
                        <p class="text-[10px] text-[#888888] mb-4">
                            "Are you sure you want to delete this configuration? This action cannot be undone."
                        </p>

                        <div class="flex gap-2">
                            <button
                                class="flex-1 text-[10px] px-4 py-2 bg-[#ff4444] text-white rounded hover:bg-[#ff5555] font-medium"
                                on:click=move |_| {
                                    if let Some(id) = config_to_delete.get() {
                                        delete_config.with_value(|f| f(DeleteConfiguration { id }));
                                    }
                                }
                            >
                                "Delete"
                            </button>
                            <button
                                class="flex-1 text-[10px] px-4 py-2 bg-[#1a1a1a] border border-[#ffffff08] text-[#888888] rounded hover:text-white"
                                on:click=move |_| {
                                    set_show_delete_config_confirm.set(false);
                                    set_config_to_delete.set(None);
                                }
                            >
                                "Cancel"
                            </button>
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    }
}

/// Modal for confirming robot deletion.
#[component]
fn DeleteConfirmModal(
    robot_to_delete: ReadSignal<Option<(i64, String)>>,
    set_show_delete_confirm: WriteSignal<bool>,
    set_robot_to_delete: WriteSignal<Option<(i64, String)>>,
    set_selected_robot_id: WriteSignal<Option<i64>>,
    selected_robot_id: ReadSignal<Option<i64>>,
    fetch_robots: impl Fn(ListRobotConnections) + Clone + Send + Sync + 'static,
) -> impl IntoView {
    let (delete_robot, _delete_state) = use_request::<DeleteRobotConnection>();
    let (is_deleting, set_is_deleting) = signal(false);

    view! {
        <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
            <div class="bg-[#0d0d0d] border border-[#ff444440] rounded-lg p-4 w-80 shadow-xl">
                <h3 class="text-[12px] font-semibold text-[#ff4444] mb-3 flex items-center">
                    <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/>
                    </svg>
                    "Delete Robot Connection"
                </h3>

                <p class="text-[10px] text-[#888888] mb-4">
                    "Are you sure you want to delete "
                    <span class="text-white font-medium">
                        "\""
                        {move || robot_to_delete.get().map(|(_, name)| name).unwrap_or_default()}
                        "\""
                    </span>
                    "? This action cannot be undone."
                </p>

                <div class="flex gap-2">
                    <button
                        class="flex-1 text-[10px] px-4 py-2 bg-[#ff4444] text-white rounded hover:bg-[#ff5555] font-medium disabled:opacity-50 disabled:cursor-not-allowed"
                        disabled=move || is_deleting.get()
                        on:click={
                            let fetch_robots = fetch_robots.clone();
                            move |_| {
                                if let Some((id, _)) = robot_to_delete.get() {
                                    set_is_deleting.set(true);
                                    delete_robot(DeleteRobotConnection { id });

                                    // Clear selection if we deleted the selected robot
                                    if selected_robot_id.get() == Some(id) {
                                        set_selected_robot_id.set(None);
                                    }

                                    // Refresh the list
                                    fetch_robots(ListRobotConnections);
                                }
                                set_show_delete_confirm.set(false);
                                set_robot_to_delete.set(None);
                                set_is_deleting.set(false);
                            }
                        }
                    >
                        {move || if is_deleting.get() { "Deleting..." } else { "Delete" }}
                    </button>
                    <button
                        class="flex-1 text-[10px] px-4 py-2 bg-[#1a1a1a] border border-[#ffffff08] text-[#888888] rounded hover:text-white"
                        on:click=move |_| {
                            set_show_delete_confirm.set(false);
                            set_robot_to_delete.set(None);
                        }
                    >
                        "Cancel"
                    </button>
                </div>
            </div>
        </div>
    }
}
