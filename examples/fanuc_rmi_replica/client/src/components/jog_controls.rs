//! Jog controls component for manual robot movement.
//!
//! Syncs with server-owned JogSettingsState for default values.

use leptos::prelude::*;

use pl3xus_client::{use_sync_context, use_sync_component, EntityControl};
use fanuc_replica_types::*;
use crate::components::use_toast;

/// Jog controls for robot manual movement.
#[component]
pub fn JogControls() -> impl IntoView {
    let ctx = use_sync_context();
    let toast = use_toast();
    let control_state = use_sync_component::<EntityControl>();
    let jog_settings = use_sync_component::<JogSettingsState>();

    // Speed and step as string signals for text input (initialized from server state)
    let (speed_str, set_speed_str) = signal(String::new());
    let (step_str, set_step_str) = signal(String::new());

    // Rotation speed and step
    let (rot_speed_str, set_rot_speed_str) = signal(String::new());
    let (rot_step_str, set_rot_step_str) = signal(String::new());

    // Track if we've initialized from server state
    let (initialized, set_initialized) = signal(false);

    // Initialize from server state when it becomes available
    Effect::new(move |_| {
        if let Some(settings) = jog_settings.get().values().next() {
            // Only initialize once, don't overwrite user edits
            if !initialized.get_untracked() {
                set_speed_str.set(format!("{:.1}", settings.cartesian_jog_speed));
                set_step_str.set(format!("{:.1}", settings.cartesian_jog_step));
                set_rot_speed_str.set(format!("{:.1}", settings.rotation_jog_speed));
                set_rot_step_str.set(format!("{:.1}", settings.rotation_jog_step));
                set_initialized.set(true);
            }
        }
    });

    // Check if THIS client has control
    let has_control = move || {
        let my_id = ctx.my_connection_id.get();
        control_state.get().values().next()
            .map(|s| Some(s.client_id) == my_id)
            .unwrap_or(false)
    };

    let jog = move |axis: JogAxis, direction: JogDirection| {
        if !has_control() {
            toast.error("Cannot jog: you don't have control. Request control first.");
            return;
        }

        // Parse speed and step from string signals
        let (speed, step) = match axis {
            JogAxis::W | JogAxis::P | JogAxis::R => {
                let speed = rot_speed_str.get().parse::<f32>().unwrap_or(5.0);
                let step = rot_step_str.get().parse::<f32>().unwrap_or(1.0);
                (speed, step)
            }
            _ => {
                let speed = speed_str.get().parse::<f32>().unwrap_or(50.0);
                let step = step_str.get().parse::<f32>().unwrap_or(10.0);
                (speed, step)
            }
        };

        ctx.send(JogCommand {
            axis,
            direction,
            distance: step,
            speed,
        });
    };

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-2">
            <div class="flex items-center justify-between mb-2">
                <h2 class="text-[10px] font-semibold text-[#00d9ff] uppercase tracking-wide">"Jog Control"</h2>
                <Show when=move || !has_control()>
                    <span class="text-[8px] text-[#ff4444] bg-[#ff444420] px-1.5 py-0.5 rounded">"No Control"</span>
                </Show>
            </div>

            // Cartesian Settings (X/Y/Z)
            <div class="text-[8px] text-[#666666] mb-1">"Cartesian (X/Y/Z)"</div>
            <div class="grid grid-cols-2 gap-2 mb-2">
                <div class="bg-[#111111] rounded p-1.5">
                    <div class="text-[8px] text-[#666666] mb-1">"Speed (mm/s)"</div>
                    <NumberInput
                        value=speed_str
                        on_change=move |v| set_speed_str.set(v)
                        min=0.1
                        max=1000.0
                    />
                </div>
                <div class="bg-[#111111] rounded p-1.5">
                    <div class="text-[8px] text-[#666666] mb-1">"Step (mm)"</div>
                    <NumberInput
                        value=step_str
                        on_change=move |v| set_step_str.set(v)
                        min=0.1
                        max=500.0
                    />
                </div>
            </div>

            // Cartesian Directional Buttons
            <div class="grid grid-cols-3 gap-1 mb-3">
                <div></div>
                <JogButton label="Y+" jog=jog.clone() axis=JogAxis::Y direction=JogDirection::Positive disabled=Signal::derive(move || !has_control()) />
                <div></div>
                <JogButton label="X-" jog=jog.clone() axis=JogAxis::X direction=JogDirection::Negative disabled=Signal::derive(move || !has_control()) />
                <JogButton label="Z+" jog=jog.clone() axis=JogAxis::Z direction=JogDirection::Positive disabled=Signal::derive(move || !has_control()) />
                <JogButton label="X+" jog=jog.clone() axis=JogAxis::X direction=JogDirection::Positive disabled=Signal::derive(move || !has_control()) />
                <div></div>
                <JogButton label="Y-" jog=jog.clone() axis=JogAxis::Y direction=JogDirection::Negative disabled=Signal::derive(move || !has_control()) />
                <JogButton label="Z-" jog=jog.clone() axis=JogAxis::Z direction=JogDirection::Negative disabled=Signal::derive(move || !has_control()) />
            </div>

            // Rotation Settings (W/P/R)
            <div class="text-[8px] text-[#666666] mb-1">"Rotation (W/P/R)"</div>
            <div class="grid grid-cols-2 gap-2 mb-2">
                <div class="bg-[#111111] rounded p-1.5">
                    <div class="text-[8px] text-[#666666] mb-1">"Speed (°/s)"</div>
                    <NumberInput
                        value=rot_speed_str
                        on_change=move |v| set_rot_speed_str.set(v)
                        min=0.1
                        max=100.0
                    />
                </div>
                <div class="bg-[#111111] rounded p-1.5">
                    <div class="text-[8px] text-[#666666] mb-1">"Step (°)"</div>
                    <NumberInput
                        value=rot_step_str
                        on_change=move |v| set_rot_step_str.set(v)
                        min=0.1
                        max=180.0
                    />
                </div>
            </div>

            // Rotation Directional Buttons
            <div class="grid grid-cols-3 gap-1">
                <JogButton label="W-" jog=jog.clone() axis=JogAxis::W direction=JogDirection::Negative disabled=Signal::derive(move || !has_control()) />
                <JogButton label="P-" jog=jog.clone() axis=JogAxis::P direction=JogDirection::Negative disabled=Signal::derive(move || !has_control()) />
                <JogButton label="R-" jog=jog.clone() axis=JogAxis::R direction=JogDirection::Negative disabled=Signal::derive(move || !has_control()) />
                <JogButton label="W+" jog=jog.clone() axis=JogAxis::W direction=JogDirection::Positive disabled=Signal::derive(move || !has_control()) />
                <JogButton label="P+" jog=jog.clone() axis=JogAxis::P direction=JogDirection::Positive disabled=Signal::derive(move || !has_control()) />
                <JogButton label="R+" jog=jog.clone() axis=JogAxis::R direction=JogDirection::Positive disabled=Signal::derive(move || !has_control()) />
            </div>
        </div>
    }
}

/// Validated text input for numeric values.
/// Uses type="text" to avoid issues with decimals and negative values.
#[component]
fn NumberInput(
    #[prop(into)] value: Signal<String>,
    on_change: impl Fn(String) + 'static,
    #[prop(default = 0.0)] min: f64,
    #[prop(default = f64::MAX)] max: f64,
) -> impl IntoView {
    let is_valid = move || {
        let v = value.get();
        if v.is_empty() {
            return true;
        }
        if let Ok(num) = v.parse::<f64>() {
            num >= min && num <= max
        } else {
            // Allow intermediate states like "-" or "." during typing
            v == "-" || v == "." || v == "-."
        }
    };

    view! {
        <input
            type="text"
            inputmode="decimal"
            class=move || format!(
                "w-full bg-[#0a0a0a] rounded px-1.5 py-1 text-white text-[11px] focus:outline-none text-right font-mono {}",
                if is_valid() {
                    "border border-[#ffffff08] focus:border-[#00d9ff]"
                } else {
                    "border-2 border-[#ff4444]"
                }
            )
            prop:value=value
            on:change=move |ev| {
                on_change(event_target_value(&ev));
            }
        />
    }
}

#[component]
fn JogButton<F>(label: &'static str, jog: F, axis: JogAxis, direction: JogDirection, disabled: Signal<bool>) -> impl IntoView
where F: Fn(JogAxis, JogDirection) + Clone + 'static {
    let do_jog = {
        let jog = jog.clone();
        move |_| jog(axis, direction)
    };

    view! {
        <button
            class=move || if disabled.get() {
                "bg-[#111111] border border-[#ffffff08] text-[#444444] font-semibold py-1.5 rounded text-center text-[10px] cursor-not-allowed opacity-50"
            } else {
                "bg-[#111111] hover:bg-[#00d9ff] border border-[#ffffff08] hover:border-[#00d9ff] text-white hover:text-black font-semibold py-1.5 rounded transition-colors text-center text-[10px]"
            }
            disabled=move || disabled.get()
            on:click=do_jog
        >
            {label}
        </button>
    }
}
