//! Joint Jog Panel - Individual joint jogging controls.
//!
//! Provides up/down buttons for each joint (J1-J6) with configurable step and speed.
//! Syncs with server-owned JogSettingsState for default values.
//! Uses targeted messages with authorization for jog commands.
//! Commands are sent to the System entity - server will route to the active robot.

use leptos::prelude::*;
use pl3xus_client::{use_sync_context, use_entity_component};
use fanuc_replica_types::*;
use crate::pages::dashboard::use_system_entity;

/// Joint Jog Panel - Jog individual joints with up/down buttons.
#[component]
pub fn JointJogPanel() -> impl IntoView {
    let ctx = use_sync_context();
    let system_ctx = use_system_entity();

    // Subscribe to entity-specific components
    let (joint_angles, _) = use_entity_component::<JointAngles, _>(move || system_ctx.robot_entity_id.get());
    let (connection_state, _) = use_entity_component::<ConnectionState, _>(move || system_ctx.system_entity_id.get());
    let (jog_settings, _) = use_entity_component::<JogSettingsState, _>(move || system_ctx.robot_entity_id.get());

    // Local string state for inputs (initialized from server state)
    let (speed_str, set_speed_str) = signal(String::new());
    let (step_str, set_step_str) = signal(String::new());
    let (initialized, set_initialized) = signal(false);

    // Initialize from server state when it becomes available
    Effect::new(move |_| {
        let settings = jog_settings.get();
        // Only initialize once, don't overwrite user edits
        if !initialized.get_untracked() && settings.joint_jog_speed > 0.0 {
            set_speed_str.set(format!("{:.1}", settings.joint_jog_speed));
            set_step_str.set(format!("{:.1}", settings.joint_jog_step));
            set_initialized.set(true);
        }
    });

    // Robot connected state
    let robot_connected = Memo::new(move |_| connection_state.get().robot_connected);

    // Get the Robot entity bits (for targeted robot commands)
    // Jog commands target the Robot entity, not the System
    let robot_entity_bits = move || system_ctx.robot_entity_id.get();

    // Controls disabled when not connected
    let controls_disabled = move || !robot_connected.get();

    // Get current joint angles
    let get_angles = move || Some(joint_angles.get());

    // Send joint jog command for a specific joint
    let send_joint_jog = {
        let ctx = ctx.clone();
        move |joint_index: usize, direction: f32| {
            if controls_disabled() {
                return;
            }
            let Some(entity_bits) = robot_entity_bits() else {
                leptos::logging::warn!("Cannot jog: Robot entity not found");
                return;
            };
            let step: f32 = step_str.get_untracked().parse().unwrap_or(1.0) * direction;

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

            ctx.send_targeted(entity_bits, JogRobot {
                axis,
                distance: step,
                speed: speed_str.get_untracked().parse().unwrap_or(10.0),
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
                // Step and Speed inputs (text inputs for better decimal/negative handling)
                <div class="flex items-center gap-2">
                    <div class="flex items-center gap-1">
                        <label class="text-[8px] text-[#666666]">"Step:"</label>
                        <input
                            type="text"
                            inputmode="decimal"
                            class="w-12 bg-[#111111] border border-[#ffffff08] rounded px-1 py-0.5 text-white text-[9px] focus:border-[#00d9ff] focus:outline-none text-center"
                            prop:value=move || step_str.get()
                            on:input=move |ev| set_step_str.set(event_target_value(&ev))
                        />
                        <span class="text-[8px] text-[#666666]">"°"</span>
                    </div>
                    <div class="flex items-center gap-1">
                        <label class="text-[8px] text-[#666666]">"Speed:"</label>
                        <input
                            type="text"
                            inputmode="decimal"
                            class="w-12 bg-[#111111] border border-[#ffffff08] rounded px-1 py-0.5 text-white text-[9px] focus:border-[#00d9ff] focus:outline-none text-center"
                            prop:value=move || speed_str.get()
                            on:input=move |ev| set_speed_str.set(event_target_value(&ev))
                        />
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
        </div>
    }
}

/// Individual joint button with up/down controls and angle display.
#[component]
fn JointButton(
    joint_name: String,
    #[prop(into)] angle: Signal<f32>,
    #[prop(into)] disabled: Signal<bool>,
    on_jog: impl Fn(f32) + 'static + Clone,
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
                on:click=move |_| on_jog_up(1.0)
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
                on:click=move |_| on_jog_down(-1.0)
                title=format!("{} -", joint_name)
            >
                "▼"
            </button>
        </div>
    }
}

