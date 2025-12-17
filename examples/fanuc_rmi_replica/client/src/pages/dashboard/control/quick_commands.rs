//! Quick Commands panel for robot control (Initialize, Reset, Abort, Continue).

use leptos::prelude::*;
use pl3xus_client::{use_sync_context, use_sync_component};
use fanuc_replica_types::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = Date)]
    fn now() -> f64;
}

/// Quick Commands panel for robot control (Initialize, Reset, Abort, Continue).
#[component]
pub fn QuickCommandsPanel() -> impl IntoView {
    let ctx = use_sync_context();
    let status = use_sync_component::<RobotStatus>();
    let connection_state = use_sync_component::<ConnectionState>();

    // Local signal for slider value (synced with robot status when available)
    let (speed_override, set_speed_override) = signal(100u32);

    // Track when user is actively changing the value
    let (user_editing, set_user_editing) = signal(false);
    let (last_edit_time, set_last_edit_time) = signal(0.0f64);

    // Robot connected state
    let robot_connected = Memo::new(move |_| {
        connection_state.get().values().next()
            .map(|s| s.robot_connected)
            .unwrap_or(false)
    });

    // Sync with robot status when it changes
    Effect::new(move |_| {
        if let Some(s) = status.get().values().next() {
            if user_editing.get() {
                return;
            }
            let now_time = now();
            if now_time - last_edit_time.get() < 1000.0 {
                return;
            }
            set_speed_override.set(s.speed_override as u32);
        }
    });

    // Send override command when slider changes
    let send_override = {
        let ctx = ctx.clone();
        move |value: u32| {
            let clamped = value.min(100) as u8;
            set_last_edit_time.set(now());
            ctx.send(SetSpeedOverride { speed: clamped });
        }
    };
    let send_override = StoredValue::new(send_override);

    let init_click = {
        let ctx = ctx.clone();
        move |_| ctx.send(InitializeRobot { group_mask: Some(1) })
    };
    let reset_click = {
        let ctx = ctx.clone();
        move |_| ctx.send(ResetRobot)
    };
    let abort_click = {
        let ctx = ctx.clone();
        move |_| ctx.send(AbortMotion)
    };

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-2 shrink-0">
            <h3 class="text-[10px] font-semibold text-[#00d9ff] uppercase tracking-wide flex items-center mb-2">
                <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"/>
                </svg>
                "Quick Commands"
            </h3>
            <div class="flex gap-2 flex-wrap items-center">
                // Initialize button
                <button
                    class="bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] text-[9px] px-3 py-1.5 rounded hover:bg-[#22c55e30] flex items-center gap-1 disabled:opacity-50 disabled:cursor-not-allowed"
                    disabled=move || !robot_connected.get()
                    on:click=init_click
                >
                    <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 3v4M3 5h4M6 17v4m-2-2h4m5-16l2.286 6.857L21 12l-5.714 2.143L13 21l-2.286-6.857L5 12l5.714-2.143L13 3z"/>
                    </svg>
                    "Initialize"
                </button>
                // Reset button
                <button
                    class="bg-[#f59e0b20] border border-[#f59e0b40] text-[#f59e0b] text-[9px] px-3 py-1.5 rounded hover:bg-[#f59e0b30] flex items-center gap-1 disabled:opacity-50 disabled:cursor-not-allowed"
                    disabled=move || !robot_connected.get()
                    on:click=reset_click
                >
                    <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/>
                    </svg>
                    "Reset"
                </button>
                // Abort button
                <button
                    class="bg-[#ff444420] border border-[#ff444440] text-[#ff4444] text-[9px] px-3 py-1.5 rounded hover:bg-[#ff444430] flex items-center gap-1 disabled:opacity-50 disabled:cursor-not-allowed"
                    disabled=move || !robot_connected.get()
                    on:click=abort_click
                >
                    <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/>
                    </svg>
                    "Abort"
                </button>

                // Speed Override Slider
                <div class="flex items-center gap-2 ml-auto bg-[#1a1a1a] rounded px-2 py-1 border border-[#ffffff10]">
                    <svg class="w-3 h-3 text-[#00d9ff]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"/>
                    </svg>
                    <span class="text-[9px] text-gray-400 whitespace-nowrap">"Speed:"</span>
                    <input
                        type="range"
                        min="0"
                        max="100"
                        step="5"
                        class="w-20 h-1 bg-[#333] rounded-lg appearance-none cursor-pointer accent-[#00d9ff]"
                        prop:value=move || speed_override.get()
                        disabled=move || !robot_connected.get()
                        on:mousedown=move |_| set_user_editing.set(true)
                        on:touchstart=move |_| set_user_editing.set(true)
                        on:input=move |ev| {
                            set_last_edit_time.set(now());
                            if let Ok(val) = event_target_value(&ev).parse::<u32>() {
                                set_speed_override.set(val);
                            }
                        }
                        on:change=move |ev| {
                            set_user_editing.set(false);
                            if let Ok(val) = event_target_value(&ev).parse::<u32>() {
                                send_override.with_value(|f| f(val));
                            }
                        }
                    />
                    <span class="text-[10px] text-[#00d9ff] font-mono w-8 text-right">
                        {move || format!("{}%", speed_override.get())}
                    </span>
                </div>
            </div>
        </div>
    }
}

