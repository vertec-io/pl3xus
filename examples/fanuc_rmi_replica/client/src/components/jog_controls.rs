//! Jog controls component for manual robot movement.
//!
//! Displays the server's JogSettingsState values (read-only).
//! Jog buttons only send axis/direction - the server uses its own JogSettingsState
//! for speed and step values. This ensures jog settings are tied to the robot
//! entity, not the client, so any client that takes control uses the same settings.
//!
//! To change jog settings, use the Configuration panel's Jog Defaults section.

use leptos::prelude::*;

use pl3xus_client::{use_sync_context, use_entity_component, EntityControl};
use fanuc_replica_types::*;
use crate::components::use_toast;
use crate::pages::dashboard::context::use_system_entity;

/// Jog controls for robot manual movement.
///
/// Uses the new targeted message pattern - jog commands are sent with the
/// Robot entity as the target. The server's authorization middleware
/// verifies that this client has control before processing the command.
///
/// Speed and step values are displayed from the server's JogSettingsState
/// but are read-only here. The server uses its own settings when processing
/// jog commands. To change settings, use the Configuration panel.
#[component]
pub fn JogControls() -> impl IntoView {
    let ctx = use_sync_context();
    let toast = use_toast();
    let system_ctx = use_system_entity();

    // Subscribe to entity-specific components
    let (control_state, _) = use_entity_component::<EntityControl, _>(move || system_ctx.system_entity_id.get());
    let (jog_settings, _) = use_entity_component::<JogSettingsState, _>(move || system_ctx.robot_entity_id.get());

    // Get the Robot entity bits (for targeted jog commands)
    let robot_entity_bits = move || system_ctx.robot_entity_id.get();

    // Check if THIS client has control (using System entity)
    let has_control = move || {
        let my_id = ctx.my_connection_id.get();
        let state = control_state.get();
        Some(state.client_id) == my_id
    };

    let jog = move |axis: JogAxis, direction: JogDirection| {
        // Get the target entity (Robot)
        let Some(entity_bits) = robot_entity_bits() else {
            toast.error("Cannot jog: Robot entity not found.");
            return;
        };

        // Note: We still check has_control() client-side for immediate UI feedback,
        // but the server will also verify via authorization middleware.
        if !has_control() {
            toast.error("Cannot jog: you don't have control. Request control first.");
            return;
        }

        // Send targeted message with only axis and direction
        // Server uses its own JogSettingsState for speed/step values
        ctx.send_targeted(entity_bits, JogCommand {
            axis,
            direction,
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

            // Cartesian Settings (X/Y/Z) - Read-only display from server
            <div class="text-[8px] text-[#666666] mb-1">"Cartesian (X/Y/Z)"</div>
            <div class="grid grid-cols-2 gap-2 mb-2">
                <div class="bg-[#111111] rounded p-1.5">
                    <div class="text-[8px] text-[#666666] mb-1">"Speed (mm/s)"</div>
                    <ReadOnlyValue value=Signal::derive(move || format!("{:.1}", jog_settings.get().cartesian_jog_speed)) />
                </div>
                <div class="bg-[#111111] rounded p-1.5">
                    <div class="text-[8px] text-[#666666] mb-1">"Step (mm)"</div>
                    <ReadOnlyValue value=Signal::derive(move || format!("{:.1}", jog_settings.get().cartesian_jog_step)) />
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

            // Rotation Settings (W/P/R) - Read-only display from server
            <div class="text-[8px] text-[#666666] mb-1">"Rotation (W/P/R)"</div>
            <div class="grid grid-cols-2 gap-2 mb-2">
                <div class="bg-[#111111] rounded p-1.5">
                    <div class="text-[8px] text-[#666666] mb-1">"Speed (°/s)"</div>
                    <ReadOnlyValue value=Signal::derive(move || format!("{:.1}", jog_settings.get().rotation_jog_speed)) />
                </div>
                <div class="bg-[#111111] rounded p-1.5">
                    <div class="text-[8px] text-[#666666] mb-1">"Step (°)"</div>
                    <ReadOnlyValue value=Signal::derive(move || format!("{:.1}", jog_settings.get().rotation_jog_step)) />
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

            // Link to Configuration panel
            <div class="mt-2 text-center">
                <a href="/dashboard/info" class="text-[8px] text-[#00d9ff] hover:underline">"Edit settings in Configuration →"</a>
            </div>
        </div>
    }
}

/// Read-only display of a value (styled like an input but not editable).
#[component]
fn ReadOnlyValue(#[prop(into)] value: Signal<String>) -> impl IntoView {
    view! {
        <div class="w-full bg-[#0a0a0a] rounded px-1.5 py-1 text-white text-[11px] text-right font-mono border border-[#ffffff08]">
            {value}
        </div>
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
