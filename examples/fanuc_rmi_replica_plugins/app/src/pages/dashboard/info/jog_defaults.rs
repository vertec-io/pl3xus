//! Jog Defaults Panel - Configure per-robot jog speed and step defaults.

use leptos::prelude::*;
use pl3xus_client::{use_sync_context, use_entity_component, use_mut_component, EntityControl};
use fanuc_replica_plugins::{ConnectionState, JogSettingsState};
use super::NumberInput;
use crate::pages::dashboard::use_system_entity;

/// Jog Defaults Panel - Configure per-robot jog speed and step defaults
#[component]
pub fn JogDefaultsPanel() -> impl IntoView {
    let ctx = use_sync_context();
    let system_ctx = use_system_entity();

    // Subscribe to entity-specific components
    // All these components live on the robot entity
    let (connection_state, robot_exists) = use_entity_component::<ConnectionState, _>(move || system_ctx.robot_entity_id.get());

    // Subscribe to entity control on the System entity (control is at hierarchy level)
    let (control_state, _) = use_entity_component::<EntityControl, _>(move || system_ctx.system_entity_id.get());

    // Use the new mutation hook for jog settings - provides read + write + mutation state
    let jog_handle = use_mut_component::<JogSettingsState, _>(move || system_ctx.robot_entity_id.get());
    // Extract the value signal for use in closures (ReadSignal is Copy)
    let jog_value = jog_handle.value;

    let robot_connected = Memo::new(move |_| robot_exists.get() && connection_state.get().robot_connected);

    // Check if THIS client has control (using System entity)
    let has_control = Memo::new(move |_| {
        let my_id = ctx.my_connection_id.get();
        let state = control_state.get();
        Some(state.client_id) == my_id
    });

    // Local state for editing
    let (cart_speed, set_cart_speed) = signal(String::new());
    let (cart_step, set_cart_step) = signal(String::new());
    let (joint_speed, set_joint_speed) = signal(String::new());
    let (joint_step, set_joint_step) = signal(String::new());
    let (has_changes, set_has_changes) = signal(false);

    // Initialize from synced jog settings
    Effect::new(move || {
        let settings = jog_value.get();
        if settings.cartesian_jog_speed > 0.0 {
            set_cart_speed.set(format!("{:.1}", settings.cartesian_jog_speed));
            set_cart_step.set(format!("{:.1}", settings.cartesian_jog_step));
            set_joint_speed.set(format!("{:.1}", settings.joint_jog_speed));
            set_joint_step.set(format!("{:.1}", settings.joint_jog_step));
            set_has_changes.set(false);
        }
    });

    // Check if edited values differ from synced settings
    let check_changes = move || {
        let settings = jog_value.get_untracked();
        let cs = cart_speed.get().parse::<f64>().unwrap_or(0.0);
        let cst = cart_step.get().parse::<f64>().unwrap_or(0.0);
        let js = joint_speed.get().parse::<f64>().unwrap_or(0.0);
        let jst = joint_step.get().parse::<f64>().unwrap_or(0.0);

        let changed = (cs - settings.cartesian_jog_speed).abs() > 0.01
            || (cst - settings.cartesian_jog_step).abs() > 0.01
            || (js - settings.joint_jog_speed).abs() > 0.01
            || (jst - settings.joint_jog_step).abs() > 0.01;
        set_has_changes.set(changed);
    };

    view! {
        <Show when=move || robot_connected.get()>
            <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-3 shrink-0">
                <div class="flex items-center justify-between mb-2">
                    <h3 class="text-[10px] font-semibold text-[#00d9ff] uppercase tracking-wide flex items-center group">
                        <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"/>
                        </svg>
                        "Jog Defaults"
                    </h3>
                    <Show when=move || !has_control.get()>
                        <span class="text-[8px] text-[#ff4444] bg-[#ff444420] px-1.5 py-0.5 rounded">"No Control"</span>
                    </Show>
                </div>

                <div class="grid grid-cols-2 gap-4">
                    // Cartesian Jog Defaults
                    <div class="bg-[#111111] rounded p-2 border border-[#ffffff08]">
                        <div class="text-[9px] text-[#666666] mb-1.5">"Cartesian Jog"</div>
                        <div class="grid grid-cols-2 gap-2">
                            <div>
                                <label class="text-[8px] text-[#555555] block mb-0.5">"Speed (mm/s)"</label>
                                <NumberInput
                                    value=Signal::derive(move || cart_speed.get())
                                    on_input=move |val: String| {
                                        set_cart_speed.set(val);
                                        check_changes();
                                    }
                                    min=0.1
                                    max=1000.0
                                    disabled=Signal::derive(move || !has_control.get())
                                />
                            </div>
                            <div>
                                <label class="text-[8px] text-[#555555] block mb-0.5">"Step (mm)"</label>
                                <NumberInput
                                    value=Signal::derive(move || cart_step.get())
                                    on_input=move |val: String| {
                                        set_cart_step.set(val);
                                        check_changes();
                                    }
                                    min=0.1
                                    max=100.0
                                    disabled=Signal::derive(move || !has_control.get())
                                />
                            </div>
                        </div>
                    </div>

                    // Joint Jog Defaults
                    <div class="bg-[#111111] rounded p-2 border border-[#ffffff08]">
                        <div class="text-[9px] text-[#666666] mb-1.5">"Joint Jog"</div>
                        <div class="grid grid-cols-2 gap-2">
                            <div>
                                <label class="text-[8px] text-[#555555] block mb-0.5">"Speed (°/s)"</label>
                                <NumberInput
                                    value=Signal::derive(move || joint_speed.get())
                                    on_input=move |val: String| {
                                        set_joint_speed.set(val);
                                        check_changes();
                                    }
                                    min=0.1
                                    max=100.0
                                    disabled=Signal::derive(move || !has_control.get())
                                />
                            </div>
                            <div>
                                <label class="text-[8px] text-[#555555] block mb-0.5">"Step (°)"</label>
                                <NumberInput
                                    value=Signal::derive(move || joint_step.get())
                                    on_input=move |val: String| {
                                        set_joint_step.set(val);
                                        check_changes();
                                    }
                                    min=0.1
                                    max=90.0
                                    disabled=Signal::derive(move || !has_control.get())
                                />
                            </div>
                        </div>
                    </div>
                </div>

                // Apply button row
                <Show when=move || has_changes.get()>
                    <div class="flex justify-end mt-2 gap-2">
                        <button
                            class="px-3 py-1 text-[9px] bg-[#1a1a1a] border border-[#ffffff08] text-[#888888] rounded hover:text-white"
                            on:click=move |_| {
                                // Reset to synced settings
                                let settings = jog_value.get();
                                set_cart_speed.set(format!("{:.1}", settings.cartesian_jog_speed));
                                set_cart_step.set(format!("{:.1}", settings.cartesian_jog_step));
                                set_joint_speed.set(format!("{:.1}", settings.joint_jog_speed));
                                set_joint_step.set(format!("{:.1}", settings.joint_jog_step));
                                set_has_changes.set(false);
                            }
                        >
                            "Cancel"
                        </button>
                        <button
                            class="px-3 py-1 text-[9px] bg-[#ffaa0020] text-[#ffaa00] border border-[#ffaa00] rounded hover:bg-[#ffaa0030]"
                            on:click=move |_| {
                                // Build new settings from edited values
                                let current = jog_value.get();
                                let new_settings = JogSettingsState {
                                    cartesian_jog_speed: cart_speed.get().parse().unwrap_or(current.cartesian_jog_speed),
                                    cartesian_jog_step: cart_step.get().parse().unwrap_or(current.cartesian_jog_step),
                                    joint_jog_speed: joint_speed.get().parse().unwrap_or(current.joint_jog_speed),
                                    joint_jog_step: joint_step.get().parse().unwrap_or(current.joint_jog_step),
                                    rotation_jog_speed: current.rotation_jog_speed,
                                    rotation_jog_step: current.rotation_jog_step,
                                };
                                // Send mutation to server
                                jog_handle.mutate(new_settings);
                                set_has_changes.set(false);
                            }
                            title="Apply these values to the active jog settings"
                        >
                            "Apply"
                        </button>
                    </div>
                </Show>
            </div>
        </Show>
    }
}

