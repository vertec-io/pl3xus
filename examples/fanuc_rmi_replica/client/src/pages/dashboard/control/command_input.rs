//! Command input section with recent commands and composer button.

use leptos::prelude::*;
use pl3xus_client::use_sync_component;
use fanuc_replica_types::*;
use crate::pages::dashboard::context::{WorkspaceContext, CommandLogEntry, CommandStatus};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    type Date;
    #[wasm_bindgen(constructor)]
    fn new() -> Date;
    #[wasm_bindgen(method, js_name = toLocaleTimeString)]
    fn to_locale_time_string(this: &Date, locale: &str) -> String;
}

/// Command input section with recent commands and composer button
#[component]
pub fn CommandInputSection() -> impl IntoView {
    let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let connection_state = use_sync_component::<ConnectionState>();
    
    let recent_commands = ctx.recent_commands;
    let selected_cmd_id = ctx.selected_command_id;

    // Robot connected state
    let robot_connected = Memo::new(move |_| {
        connection_state.get().values().next()
            .map(|s| s.robot_connected)
            .unwrap_or(false)
    });

    // Program running state (TODO: get from synced component)
    let controls_disabled = move || !robot_connected.get();

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-2 shrink-0">
            <div class="flex items-center justify-between mb-1.5">
                <h3 class="text-[10px] font-semibold text-[#00d9ff] uppercase tracking-wide flex items-center">
                    <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"/>
                    </svg>
                    "Recent Commands"
                </h3>
                <button
                    class="text-[8px] text-[#666666] hover:text-[#ff4444]"
                    on:click=move |_| {
                        recent_commands.set(Vec::new());
                        selected_cmd_id.set(None);
                    }
                    title="Clear all recent commands"
                >
                    "Clear"
                </button>
            </div>
            <div class="flex gap-1">
                <select
                    class="flex-1 bg-[#111111] border border-[#ffffff08] rounded px-2 py-1 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                    prop:value=move || selected_cmd_id.get().map(|id| id.to_string()).unwrap_or_default()
                    on:change=move |ev| {
                        let val = event_target_value(&ev);
                        selected_cmd_id.set(val.parse().ok());
                    }
                >
                    <option value="">{move || {
                        let count = recent_commands.get().len();
                        if count == 0 {
                            "No recent commands - use Composer to create".to_string()
                        } else {
                            format!("Select from {} recent commands...", count)
                        }
                    }}</option>
                    {move || recent_commands.get().into_iter().map(|cmd| {
                        view! {
                            <option value={cmd.id.to_string()}>
                                {format!("{} - {}", cmd.name, cmd.description)}
                            </option>
                        }
                    }).collect_view()}
                </select>
                <button
                    class=move || if controls_disabled() {
                        "bg-[#111111] border border-[#ffffff08] text-[#555555] text-[9px] px-3 py-1 rounded cursor-not-allowed"
                    } else {
                        "bg-[#00d9ff20] border border-[#00d9ff40] text-[#00d9ff] text-[9px] px-3 py-1 rounded hover:bg-[#00d9ff30]"
                    }
                    disabled=controls_disabled
                    on:click=move |_| {
                        if !controls_disabled() {
                            ctx.show_composer.set(true);
                        }
                    }
                    title=move || if controls_disabled() { "Disabled: Not connected" } else { "Create new command" }
                >
                    "+ Compose"
                </button>
                <button
                    class={move || format!(
                        "text-[9px] px-3 py-1 rounded transition-colors {}",
                        if controls_disabled() || selected_cmd_id.get().is_none() {
                            "bg-[#111111] border border-[#ffffff08] text-[#555555] cursor-not-allowed"
                        } else {
                            "bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] hover:bg-[#22c55e30]"
                        }
                    )}
                    disabled=move || controls_disabled() || selected_cmd_id.get().is_none()
                    on:click=move |_| {
                        if let Some(idx) = selected_cmd_id.get() {
                            let cmds = recent_commands.get();
                            if let Some(cmd) = cmds.iter().find(|c| c.id == idx) {
                                // Log the command
                                ctx.command_log.update(|log| {
                                    log.push(CommandLogEntry {
                                        timestamp: Date::new().to_locale_time_string("en-US"),
                                        command: cmd.name.clone(),
                                        status: CommandStatus::Pending,
                                    });
                                });
                                // TODO: Send command to robot via pl3xus
                            }
                        }
                    }
                    title=move || if controls_disabled() { "Disabled: Not connected" } else { "Run selected command" }
                >
                    "▶ Run"
                </button>
            </div>
        </div>
    }
}

