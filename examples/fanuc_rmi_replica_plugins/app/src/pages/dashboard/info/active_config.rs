//! Active Configuration Panel - Shows loaded config and arm config.
//!
//! Features:
//! - Shows currently loaded configuration with changes indicator
//! - Dropdown to switch between saved configurations
//! - Revert button to reload the current configuration (discarding changes)
//! - Save button to persist changes to database (update existing or save as new)

use leptos::prelude::*;
use pl3xus_client::{use_entity_component, use_query_keyed, use_mutation};
use fanuc_replica_plugins::{ConnectionState, ActiveConfigState, GetRobotConfigurations, LoadConfiguration, SaveCurrentConfiguration};
use crate::components::use_toast;
use crate::pages::dashboard::use_system_entity;

/// Active Configuration Panel - Shows loaded config and arm config (read-only display)
#[component]
pub fn ActiveConfigurationPanel() -> impl IntoView {
    let toast = use_toast();
    let system_ctx = use_system_entity();

    // Subscribe to entity-specific components
    // ConnectionState lives on robot entity, not system entity
    let (connection_state, robot_exists) = use_entity_component::<ConnectionState, _>(move || system_ctx.robot_entity_id.get());
    let (active_config, _) = use_entity_component::<ActiveConfigState, _>(move || system_ctx.robot_entity_id.get());

    let robot_connected = Memo::new(move |_| robot_exists.get() && connection_state.get().robot_connected);

    // Get active connection ID (only valid when robot exists)
    let active_connection_id = Memo::new(move |_| if robot_exists.get() { connection_state.get().active_connection_id } else { None });

    // Query for robot configurations - auto-fetches when robot is connected
    let configs_query = use_query_keyed::<GetRobotConfigurations, _>(move || {
        if robot_connected.get() {
            active_connection_id.get().map(|id| GetRobotConfigurations { robot_connection_id: id })
        } else {
            None
        }
    });

    // LoadConfiguration mutation with error handling
    let load_config = use_mutation::<LoadConfiguration>(move |result| {
        match result {
            Ok(r) if r.success => {} // Silent success - config syncs automatically
            Ok(r) => toast.error(format!("Load config failed: {}", r.error.as_deref().unwrap_or(""))),
            Err(e) => toast.error(format!("Config error: {e}")),
        }
    });

    // Derive robot_configs from query data
    let robot_configs = Memo::new(move |_| {
        configs_query.data()
            .map(|r| r.configurations.clone())
            .unwrap_or_default()
    });

    // Get active config values
    let config = Memo::new(move |_| active_config.get());

    // Modal state for save confirmation
    let (show_save_modal, set_show_save_modal) = signal(false);
    let (save_as_new, set_save_as_new) = signal(false);
    let (new_config_name, set_new_config_name) = signal(String::new());
    let (is_saving, set_is_saving) = signal(false);

    // SaveCurrentConfiguration mutation
    let save_config = use_mutation::<SaveCurrentConfiguration>(move |result| {
        set_is_saving.set(false);
        match result {
            Ok(r) if r.success => {
                set_show_save_modal.set(false);
                set_save_as_new.set(false);
                set_new_config_name.set(String::new());
                toast.success(format!("Configuration '{}' saved", r.configuration_name.as_deref().unwrap_or("Unknown")));
            }
            Ok(r) => toast.error(format!("Save failed: {}", r.error.as_deref().unwrap_or(""))),
            Err(e) => toast.error(format!("Save error: {e}")),
        }
    });

    view! {
        <Show when=move || robot_connected.get()>
            <div class="bg-background rounded border border-border/8 p-3 shrink-0">
                <h3 class="text-[10px] font-semibold text-primary mb-2 uppercase tracking-wide flex items-center justify-between">
                    <span class="flex items-center">
                        <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/>
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>
                        </svg>
                        "Active Configuration"
                    </span>
                    // Modified indicator
                    <Show when=move || { config.get().changes_count > 0 }>
                        <span class="flex items-center gap-1 text-[9px] text-warning bg-[#ffaa0015] border border-warning/40 px-2 py-0.5 rounded font-normal">
                            <svg class="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                                <path fill-rule="evenodd" d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z" clip-rule="evenodd"/>
                            </svg>
                            {move || format!("Modified ({})", config.get().changes_count)}
                        </span>
                    </Show>
                </h3>

                <div class="grid grid-cols-2 gap-4">
                    // Left side - Configuration selector
                    <div class="space-y-2">
                        // Configuration dropdown with Revert button
                        <div class="flex items-center gap-2">
                            <label class="text-[9px] text-muted-foreground w-16">"Loaded From:"</label>
                            <select
                                class="flex-1 bg-card border border-border/15 rounded px-2 py-1 text-[10px] text-foreground"
                                on:change=move |ev| {
                                    let value = event_target_value(&ev);
                                    if let Ok(id) = value.parse::<i64>() {
                                        load_config.send(LoadConfiguration { configuration_id: id });
                                    }
                                }
                            >
                                {move || robot_configs.get().into_iter().map(|cfg| {
                                    let id = cfg.id;
                                    let name = cfg.name.clone();
                                    let is_selected = move || config.get().loaded_from_id == Some(id);
                                    view! {
                                        <option value={id.to_string()} selected=is_selected>
                                            {name}
                                        </option>
                                    }
                                }).collect_view()}
                            </select>
                            // Revert button (only show if changes have been made)
                            <Show when=move || { config.get().changes_count > 0 }>
                                <button
                                    class="px-2 py-1 text-[9px] bg-[#ffaa0020] text-warning border border-warning rounded hover:bg-warning/20"
                                    on:click=move |_| {
                                        if let Some(id) = config.get().loaded_from_id {
                                            load_config.send(LoadConfiguration { configuration_id: id });
                                        }
                                    }
                                    title="Revert to saved configuration"
                                >
                                    "Revert"
                                </button>
                            </Show>
                            // Save button
                            <Show when=move || { config.get().changes_count > 0 }>
                                <button
                                    class="px-2 py-1 text-[9px] bg-[#22c55e20] text-success border border-success rounded hover:bg-success/20"
                                    on:click=move |_| set_show_save_modal.set(true)
                                    title="Save current configuration to database"
                                >
                                    "Save"
                                </button>
                            </Show>
                        </div>

                        // Current UFrame/UTool display (read-only)
                        <div class="bg-card rounded p-2 border border-border/8">
                            <div class="text-[9px] text-muted-foreground mb-1">"Active Frame/Tool"</div>
                            <div class="flex gap-4 text-[10px]">
                                <div class="flex items-center gap-1">
                                    <span class="text-muted-foreground">"UFrame:"</span>
                                    <span class="text-primary font-medium">{move || config.get().u_frame_number}</span>
                                </div>
                                <div class="flex items-center gap-1">
                                    <span class="text-muted-foreground">"UTool:"</span>
                                    <span class="text-primary font-medium">{move || config.get().u_tool_number}</span>
                                </div>
                            </div>
                            <div class="text-[8px] text-muted-foreground mt-1">
                                "Use panels below to change"
                            </div>
                        </div>
                    </div>

                    // Right side - Arm Configuration (read-only)
                    <div class="bg-card rounded p-2 border border-border/8">
                        <div class="text-[9px] text-muted-foreground mb-1">"Arm Configuration"</div>
                        <div class="grid grid-cols-2 gap-x-3 gap-y-0.5 text-[9px]">
                            <div class="flex justify-between">
                                <span class="text-muted-foreground">"Front/Back:"</span>
                                <span class="text-foreground">{move || if config.get().front == 1 { "Front" } else { "Back" }}</span>
                            </div>
                            <div class="flex justify-between">
                                <span class="text-muted-foreground">"Up/Down:"</span>
                                <span class="text-foreground">{move || if config.get().up == 1 { "Up" } else { "Down" }}</span>
                            </div>
                            <div class="flex justify-between">
                                <span class="text-muted-foreground">"Left/Right:"</span>
                                <span class="text-foreground">{move || if config.get().left == 1 { "Left" } else { "Right" }}</span>
                            </div>
                            <div class="flex justify-between">
                                <span class="text-muted-foreground">"Flip:"</span>
                                <span class="text-foreground">{move || if config.get().flip == 1 { "Flip" } else { "NoFlip" }}</span>
                            </div>
                            <div class="flex justify-between col-span-2">
                                <span class="text-muted-foreground">"Turn (J4/J5/J6):"</span>
                                <span class="text-foreground font-mono">{move || {
                                    let c = config.get();
                                    format!("{}/{}/{}", c.turn4, c.turn5, c.turn6)
                                }}</span>
                            </div>
                        </div>
                    </div>
                </div>

                // Save Configuration Modal
                <Show when=move || show_save_modal.get()>
                    <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
                        on:click=move |_| {
                            if !is_saving.get() {
                                set_show_save_modal.set(false);
                                set_save_as_new.set(false);
                                set_new_config_name.set(String::new());
                            }
                        }
                    >
                        <div class="bg-background border border-border/15 rounded-lg p-6 w-96 max-h-[80vh] overflow-y-auto"
                            on:click=move |e| e.stop_propagation()
                        >
                            <h3 class="text-sm font-semibold text-foreground mb-3">"Save Configuration"</h3>

                            // Show change log summary
                            <div class="mb-4">
                                <p class="text-xs text-muted-foreground mb-2">
                                    {move || format!("{} change(s) since loading:", config.get().changes_count)}
                                </p>
                                <div class="bg-card border border-border/15 rounded p-2 max-h-32 overflow-y-auto">
                                    {move || {
                                        let changes = config.get().change_log.clone();
                                        if changes.is_empty() {
                                            view! {
                                                <p class="text-[10px] text-muted-foreground italic">"No changes recorded"</p>
                                            }.into_any()
                                        } else {
                                            changes.into_iter().map(|change| {
                                                view! {
                                                    <div class="flex items-center text-[10px] py-0.5 border-b border-border/8 last:border-0">
                                                        <span class="text-muted-foreground w-16">{change.field_name}</span>
                                                        <span class="text-warning mx-1">{change.old_value}</span>
                                                        <svg class="w-3 h-3 text-muted-foreground mx-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 7l5 5m0 0l-5 5m5-5H6"/>
                                                        </svg>
                                                        <span class="text-success">{change.new_value}</span>
                                                    </div>
                                                }
                                            }).collect_view().into_any()
                                        }
                                    }}
                                </div>
                            </div>

                            // Show current config name if updating existing
                            <Show when=move || !save_as_new.get() && config.get().loaded_from_id.is_some()>
                                <div class="mb-4">
                                    <p class="text-xs text-muted-foreground mb-2">
                                        "Update existing configuration:"
                                    </p>
                                    <div class="bg-card border border-border/15 rounded px-3 py-2 text-sm text-foreground">
                                        {move || config.get().loaded_from_name.clone().unwrap_or_else(|| "Unknown".to_string())}
                                    </div>
                                </div>
                            </Show>

                            // Save as new option
                            <Show when=move || save_as_new.get() || config.get().loaded_from_id.is_none()>
                                <div class="mb-4">
                                    <label class="text-xs text-muted-foreground block mb-1">"New configuration name:"</label>
                                    <input
                                        type="text"
                                        class="w-full bg-card border border-border/15 rounded px-3 py-2 text-sm text-foreground focus:outline-none focus:border-primary"
                                        placeholder="Enter configuration name"
                                        prop:value=move || new_config_name.get()
                                        on:input=move |ev| set_new_config_name.set(event_target_value(&ev))
                                        disabled=move || is_saving.get()
                                    />
                                </div>
                            </Show>

                            // Toggle between update and save as new (only if we have a loaded config)
                            <Show when=move || config.get().loaded_from_id.is_some()>
                                <div class="mb-4">
                                    <label class="flex items-center gap-2 text-xs text-muted-foreground cursor-pointer">
                                        <input
                                            type="checkbox"
                                            class="rounded border-border/15"
                                            prop:checked=move || save_as_new.get()
                                            on:change=move |ev| set_save_as_new.set(event_target_checked(&ev))
                                            disabled=move || is_saving.get()
                                        />
                                        "Save as new configuration"
                                    </label>
                                </div>
                            </Show>

                            // Action buttons
                            <div class="flex justify-end gap-2">
                                <button
                                    class="px-3 py-1.5 text-[10px] bg-popover border border-border/8 text-muted-foreground rounded hover:bg-muted disabled:opacity-50"
                                    on:click=move |_| {
                                        set_show_save_modal.set(false);
                                        set_save_as_new.set(false);
                                        set_new_config_name.set(String::new());
                                    }
                                    disabled=move || is_saving.get()
                                >"Cancel"</button>
                                <button
                                    class="px-3 py-1.5 text-[10px] bg-success/20 text-success border border-success rounded hover:bg-success/30 disabled:opacity-50"
                                    on:click=move |_| {
                                        let name = if save_as_new.get() || config.get().loaded_from_id.is_none() {
                                            let n = new_config_name.get();
                                            if n.trim().is_empty() {
                                                toast.error("Please enter a configuration name");
                                                return;
                                            }
                                            Some(n)
                                        } else {
                                            None
                                        };
                                        set_is_saving.set(true);
                                        save_config.send(SaveCurrentConfiguration { name });
                                    }
                                    disabled=move || is_saving.get() || (save_as_new.get() && new_config_name.get().trim().is_empty())
                                >
                                    {move || if is_saving.get() { "Saving..." } else { "Save" }}
                                </button>
                            </div>
                        </div>
                    </div>
                </Show>
            </div>
        </Show>
    }
}

