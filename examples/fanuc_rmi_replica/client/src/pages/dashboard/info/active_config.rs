//! Active Configuration Panel - Shows loaded config and arm config.

use leptos::prelude::*;
use pl3xus_client::use_sync_component;
use fanuc_replica_types::{ConnectionState, ActiveConfigState, RobotConfiguration};

/// Active Configuration Panel - Shows loaded config and arm config (read-only display)
#[component]
pub fn ActiveConfigurationPanel() -> impl IntoView {
    let connection_state = use_sync_component::<ConnectionState>();
    let active_config = use_sync_component::<ActiveConfigState>();

    let robot_connected = Memo::new(move |_| {
        connection_state.get().values().next()
            .map(|s| s.robot_connected)
            .unwrap_or(false)
    });

    // TODO: Get robot_configs from a request once ListConfigurations is wired up
    let robot_configs: RwSignal<Vec<RobotConfiguration>> = RwSignal::new(Vec::new());

    // Get active config values
    let config = Memo::new(move |_| {
        active_config.get().values().next().cloned().unwrap_or_default()
    });

    // Modal state for save confirmation
    let (show_save_modal, set_show_save_modal) = signal(false);

    view! {
        <Show when=move || robot_connected.get()>
            <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-3 shrink-0">
                <h3 class="text-[10px] font-semibold text-[#00d9ff] mb-2 uppercase tracking-wide flex items-center group">
                    <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/>
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>
                    </svg>
                    "Active Configuration"
                </h3>

                <div class="grid grid-cols-2 gap-4">
                    // Left side - Configuration selector
                    <div class="space-y-2">
                        // Configuration dropdown with Revert button
                        <div class="flex items-center gap-2">
                            <label class="text-[9px] text-[#666666] w-16">"Loaded From:"</label>
                            <select
                                class="flex-1 bg-[#111111] border border-[#ffffff15] rounded px-2 py-1 text-[10px] text-white"
                                on:change=move |_ev| {
                                    // TODO: ws.load_configuration(id)
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
                                    class="px-2 py-1 text-[9px] bg-[#ffaa0020] text-[#ffaa00] border border-[#ffaa00] rounded hover:bg-[#ffaa0030]"
                                    on:click=move |_| {
                                        // TODO: ws.load_configuration to revert
                                    }
                                    title="Revert to saved configuration"
                                >
                                    "Revert"
                                </button>
                            </Show>
                            // Save button
                            <Show when=move || { config.get().changes_count > 0 }>
                                <button
                                    class="px-2 py-1 text-[9px] bg-[#22c55e20] text-[#22c55e] border border-[#22c55e] rounded hover:bg-[#22c55e30]"
                                    on:click=move |_| set_show_save_modal.set(true)
                                    title="Save current configuration to database"
                                >
                                    "Save"
                                </button>
                            </Show>
                        </div>

                        // Current UFrame/UTool display (read-only)
                        <div class="bg-[#111111] rounded p-2 border border-[#ffffff08]">
                            <div class="text-[9px] text-[#666666] mb-1">"Active Frame/Tool"</div>
                            <div class="flex gap-4 text-[10px]">
                                <div class="flex items-center gap-1">
                                    <span class="text-[#555555]">"UFrame:"</span>
                                    <span class="text-[#00d9ff] font-medium">{move || config.get().u_frame_number}</span>
                                </div>
                                <div class="flex items-center gap-1">
                                    <span class="text-[#555555]">"UTool:"</span>
                                    <span class="text-[#00d9ff] font-medium">{move || config.get().u_tool_number}</span>
                                </div>
                            </div>
                            <div class="text-[8px] text-[#555555] mt-1">
                                "Use panels below to change"
                            </div>
                        </div>
                    </div>

                    // Right side - Arm Configuration (read-only)
                    <div class="bg-[#111111] rounded p-2 border border-[#ffffff08]">
                        <div class="text-[9px] text-[#666666] mb-1">"Arm Configuration"</div>
                        <div class="grid grid-cols-2 gap-x-3 gap-y-0.5 text-[9px]">
                            <div class="flex justify-between">
                                <span class="text-[#555555]">"Front/Back:"</span>
                                <span class="text-white">{move || if config.get().front == 1 { "Front" } else { "Back" }}</span>
                            </div>
                            <div class="flex justify-between">
                                <span class="text-[#555555]">"Up/Down:"</span>
                                <span class="text-white">{move || if config.get().up == 1 { "Up" } else { "Down" }}</span>
                            </div>
                            <div class="flex justify-between">
                                <span class="text-[#555555]">"Left/Right:"</span>
                                <span class="text-white">{move || if config.get().left == 1 { "Left" } else { "Right" }}</span>
                            </div>
                            <div class="flex justify-between">
                                <span class="text-[#555555]">"Flip:"</span>
                                <span class="text-white">{move || if config.get().flip == 1 { "Flip" } else { "NoFlip" }}</span>
                            </div>
                            <div class="flex justify-between col-span-2">
                                <span class="text-[#555555]">"Turn (J4/J5/J6):"</span>
                                <span class="text-white font-mono">{move || {
                                    let c = config.get();
                                    format!("{}/{}/{}", c.turn4, c.turn5, c.turn6)
                                }}</span>
                            </div>
                        </div>
                    </div>
                </div>

                // Save Configuration Modal (placeholder)
                <Show when=move || show_save_modal.get()>
                    <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
                        on:click=move |_| set_show_save_modal.set(false)
                    >
                        <div class="bg-[#0a0a0a] border border-[#ffffff15] rounded-lg p-6 max-w-md"
                            on:click=move |e| e.stop_propagation()
                        >
                            <h3 class="text-sm font-semibold text-white mb-3">"Save Configuration"</h3>
                            <p class="text-xs text-[#888888] mb-4">"Save modal - to be implemented"</p>
                            <button
                                class="px-3 py-1.5 text-[10px] bg-[#1a1a1a] border border-[#ffffff08] text-[#888888] rounded"
                                on:click=move |_| set_show_save_modal.set(false)
                            >"Close"</button>
                        </div>
                    </div>
                </Show>
            </div>
        </Show>
    }
}

