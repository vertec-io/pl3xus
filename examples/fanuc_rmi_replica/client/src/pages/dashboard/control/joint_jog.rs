//! Joint Jog Panel - Individual joint jogging controls.
//!
//! Displays the server's JogSettingsState values (read-only).
//! Jog buttons only send axis/direction - the server uses its own JogSettingsState
//! for speed and step values. This ensures jog settings are tied to the robot
//! entity, not the client, so any client that takes control uses the same settings.
//!
//! To change jog settings, use the Configuration panel's Jog Defaults section.

use leptos::prelude::*;
use pl3xus_client::{use_sync_context, use_entity_component};
use fanuc_replica_types::*;
use crate::pages::dashboard::use_system_entity;

/// Joint Jog Panel - Jog individual joints with up/down buttons.
///
/// Speed and step values are displayed from the server's JogSettingsState
/// but are read-only here. The server uses its own settings when processing
/// jog commands. To change settings, use the Configuration panel.
#[component]
pub fn JointJogPanel() -> impl IntoView {
    let ctx = use_sync_context();
    let system_ctx = use_system_entity();

    // Subscribe to entity-specific components
    // All these components live on the robot entity
    let (joint_angles, _) = use_entity_component::<JointAngles, _>(move || system_ctx.robot_entity_id.get());
    let (connection_state, robot_exists) = use_entity_component::<ConnectionState, _>(move || system_ctx.robot_entity_id.get());
    let (jog_settings, _) = use_entity_component::<JogSettingsState, _>(move || system_ctx.robot_entity_id.get());

    // Robot connected state (only true if robot entity exists AND is connected)
    let robot_connected = Memo::new(move |_| robot_exists.get() && connection_state.get().robot_connected);

    // Get the Robot entity bits (for targeted robot commands)
    // Jog commands target the Robot entity, not the System
    let robot_entity_bits = move || system_ctx.robot_entity_id.get();

    // Controls disabled when not connected
    let controls_disabled = move || !robot_connected.get();

    // Get current joint angles
    let get_angles = move || Some(joint_angles.get());

    // Send joint jog command for a specific joint
    // Only sends axis and direction - server uses its own JogSettingsState for speed/step
    let send_joint_jog = {
        let ctx = ctx.clone();
        move |joint_index: usize, direction: JogDirection| {
            if controls_disabled() {
                return;
            }
            let Some(entity_bits) = robot_entity_bits() else {
                leptos::logging::warn!("Cannot jog: Robot entity not found");
                return;
            };

            // Create jog command for the specific joint
            let axis = match joint_index {
                0 => JogAxis::J1,
                1 => JogAxis::J2,
                2 => JogAxis::J3,
                3 => JogAxis::J4,
                4 => JogAxis::J5,
                5 => JogAxis::J6,
                _ => return,
            };

            // Send targeted message with only axis and direction
            // Server uses its own JogSettingsState for speed/step values
            ctx.send_targeted(entity_bits, JogCommand {
                axis,
                direction,
            });
        }
    };
    let send_joint_jog = StoredValue::new(send_joint_jog);

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-2">
            <div class="flex items-center justify-between mb-2">
                <h3 class="text-[10px] font-semibold text-[#00d9ff] uppercase tracking-wide flex items-center">
                    <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/>
                    </svg>
                    "Joint Jog"
                </h3>
                // Step and Speed display (read-only from server's JogSettingsState)
                <div class="flex items-center gap-2">
                    <div class="flex items-center gap-1">
                        <span class="text-[8px] text-[#666666]">"Step:"</span>
                        <span class="w-12 bg-[#0a0a0a] border border-[#ffffff08] rounded px-1 py-0.5 text-white text-[9px] text-center font-mono">
                            {move || format!("{:.1}", jog_settings.get().joint_jog_step)}
                        </span>
                        <span class="text-[8px] text-[#666666]">"°"</span>
                    </div>
                    <div class="flex items-center gap-1">
                        <span class="text-[8px] text-[#666666]">"Speed:"</span>
                        <span class="w-12 bg-[#0a0a0a] border border-[#ffffff08] rounded px-1 py-0.5 text-white text-[9px] text-center font-mono">
                            {move || format!("{:.1}", jog_settings.get().joint_jog_speed)}
                        </span>
                        <span class="text-[8px] text-[#666666]">"°/s"</span>
                    </div>
                </div>
            </div>

            // Show warning when controls are disabled
            <Show when=controls_disabled>
                <div class="text-[9px] text-[#ff8800] mb-1 text-center">
                    "⚠ Disabled: Not connected"
                </div>
            </Show>

            // Joint buttons grid - 6 columns for J1-J6
            <div class="grid grid-cols-6 gap-1">
                {(0..6).map(|i| {
                    let joint_name = format!("J{}", i + 1);
                    let get_angle = move || {
                        get_angles().map(|a| match i {
                            0 => a.j1,
                            1 => a.j2,
                            2 => a.j3,
                            3 => a.j4,
                            4 => a.j5,
                            5 => a.j6,
                            _ => 0.0,
                        }).unwrap_or(0.0)
                    };
                    view! {
                        <JointButton
                            joint_name=joint_name
                            angle=Signal::derive(get_angle)
                            disabled=Signal::derive(controls_disabled)
                            on_jog=move |dir| send_joint_jog.with_value(|f| f(i, dir))
                        />
                    }
                }).collect_view()}
            </div>

            // Link to Configuration panel
            <div class="mt-2 text-center">
                <a href="/dashboard/info" class="text-[8px] text-[#00d9ff] hover:underline">"Edit settings in Configuration →"</a>
            </div>
        </div>
    }
}

/// Individual joint button with up/down controls and angle display.
#[component]
fn JointButton(
    joint_name: String,
    #[prop(into)] angle: Signal<f32>,
    #[prop(into)] disabled: Signal<bool>,
    on_jog: impl Fn(JogDirection) + 'static + Clone,
) -> impl IntoView {
    let on_jog_up = on_jog.clone();
    let on_jog_down = on_jog;

    let button_class = move || {
        if disabled.get() {
            "w-full bg-[#0a0a0a] border border-[#ffffff08] text-[#444444] py-1 rounded cursor-not-allowed text-[10px]"
        } else {
            "w-full bg-[#111111] hover:bg-[#00d9ff] border border-[#ffffff08] hover:border-[#00d9ff] text-white hover:text-black py-1 rounded transition-colors text-[10px]"
        }
    };

    view! {
        <div class="flex flex-col items-center">
            <button
                class=button_class
                disabled=disabled
                on:click=move |_| on_jog_up(JogDirection::Positive)
                title=format!("{} +", joint_name)
            >
                "▲"
            </button>
            <div class="py-1 text-center w-full bg-[#111111] border-x border-[#ffffff08]">
                <div class="text-[9px] text-[#00d9ff] font-semibold">{joint_name.clone()}</div>
                <div class="text-[10px] text-white font-mono">
                    {move || format!("{:.2}°", angle.get())}
                </div>
            </div>
            <button
                class=button_class
                disabled=disabled
                on:click=move |_| on_jog_down(JogDirection::Negative)
                title=format!("{} -", joint_name)
            >
                "▼"
            </button>
        </div>
    }
}

