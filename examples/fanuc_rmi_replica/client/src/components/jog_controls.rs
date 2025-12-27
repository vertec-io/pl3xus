//! Jog controls component for manual robot movement.
//!
//! Displays and allows editing of the server's JogSettingsState values.
//! Jog buttons send axis/direction - the server uses its JogSettingsState
//! for speed and step values. Settings are editable inline - press Enter to
//! submit changes, or blur to discard.

use leptos::prelude::*;
use leptos::ev::KeyboardEvent;
use leptos::web_sys;
use wasm_bindgen::JsCast;

use pl3xus_client::{use_sync_context, use_entity_component, use_mut_component, ComponentMutationState, EntityControl};
use fanuc_replica_types::*;
use crate::components::use_toast;
use crate::pages::dashboard::context::use_system_entity;

/// Jog controls for robot manual movement.
///
/// Uses the new targeted message pattern - jog commands are sent with the
/// Robot entity as the target. The server's authorization middleware
/// verifies that this client has control before processing the command.
///
/// Speed and step values are editable. Press Enter to send mutation to server,
/// or blur/Escape to discard changes and revert to server value.
#[component]
pub fn JogControls() -> impl IntoView {
    let ctx = use_sync_context();
    let toast = use_toast();
    let system_ctx = use_system_entity();

    // Subscribe to entity-specific components
    let (control_state, _) = use_entity_component::<EntityControl, _>(move || system_ctx.system_entity_id.get());

    // Use mut_component for jog settings - allows mutations
    let jog_handle = use_mut_component::<JogSettingsState, _>(move || system_ctx.robot_entity_id.get());
    let jog_settings = jog_handle.value;

    // Watch mutation_state for errors and show toast
    Effect::new(move |prev: Option<ComponentMutationState>| {
        let state = jog_handle.mutation_state.get();
        // Only toast on transition TO error (not every render)
        if let ComponentMutationState::Error(ref msg) = state {
            if !matches!(prev.as_ref(), Some(ComponentMutationState::Error(_))) {
                toast.error(format!("Jog settings update denied: {}", msg));
            }
        }
        state
    });

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
        <div class="bg-background rounded border border-border/8 p-2">
            <div class="flex items-center justify-between mb-2">
                <h2 class="text-[10px] font-semibold text-primary uppercase tracking-wide">"Jog Control"</h2>
                <Show when=move || !has_control()>
                    <span class="text-[8px] text-destructive bg-destructive/20 px-1.5 py-0.5 rounded">"No Control"</span>
                </Show>
            </div>

            // Cartesian Settings (X/Y/Z) - Editable
            <div class="text-[8px] text-muted-foreground mb-1">"Cartesian (X/Y/Z)"</div>
            <div class="grid grid-cols-2 gap-2 mb-2">
                <div class="bg-card rounded p-1.5">
                    <div class="text-[8px] text-muted-foreground mb-1">"Speed (mm/s)"</div>
                    <JogSettingInput
                        server_value=Signal::derive(move || jog_settings.get().cartesian_jog_speed)
                        on_submit=move |val| {
                            let mut settings = jog_settings.get();
                            settings.cartesian_jog_speed = val;
                            jog_handle.mutate(settings);
                        }
                        disabled=Signal::derive(move || !has_control())
                    />
                </div>
                <div class="bg-card rounded p-1.5">
                    <div class="text-[8px] text-muted-foreground mb-1">"Step (mm)"</div>
                    <JogSettingInput
                        server_value=Signal::derive(move || jog_settings.get().cartesian_jog_step)
                        on_submit=move |val| {
                            let mut settings = jog_settings.get();
                            settings.cartesian_jog_step = val;
                            jog_handle.mutate(settings);
                        }
                        disabled=Signal::derive(move || !has_control())
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

            // Rotation Settings (W/P/R) - Editable
            <div class="text-[8px] text-muted-foreground mb-1">"Rotation (W/P/R)"</div>
            <div class="grid grid-cols-2 gap-2 mb-2">
                <div class="bg-card rounded p-1.5">
                    <div class="text-[8px] text-muted-foreground mb-1">"Speed (°/s)"</div>
                    <JogSettingInput
                        server_value=Signal::derive(move || jog_settings.get().rotation_jog_speed)
                        on_submit=move |val| {
                            let mut settings = jog_settings.get();
                            settings.rotation_jog_speed = val;
                            jog_handle.mutate(settings);
                        }
                        disabled=Signal::derive(move || !has_control())
                    />
                </div>
                <div class="bg-card rounded p-1.5">
                    <div class="text-[8px] text-muted-foreground mb-1">"Step (°)"</div>
                    <JogSettingInput
                        server_value=Signal::derive(move || jog_settings.get().rotation_jog_step)
                        on_submit=move |val| {
                            let mut settings = jog_settings.get();
                            settings.rotation_jog_step = val;
                            jog_handle.mutate(settings);
                        }
                        disabled=Signal::derive(move || !has_control())
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

/// Editable input for a jog setting value.
/// - Shows server value by default
/// - User can edit, press Enter to submit
/// - Blur or Escape reverts to server value
/// - Disabled when client doesn't have control
#[component]
fn JogSettingInput<F>(
    #[prop(into)] server_value: Signal<f64>,
    on_submit: F,
    #[prop(into)] disabled: Signal<bool>,
) -> impl IntoView
where
    F: Fn(f64) + Clone + 'static,
{
    let (local_value, set_local_value) = signal(String::new());
    let (is_editing, set_is_editing) = signal(false);

    // Display either local value (when editing) or server value
    let display_value = move || {
        if is_editing.get() {
            local_value.get()
        } else {
            format!("{:.1}", server_value.get())
        }
    };

    let on_focus = move |_| {
        if disabled.get() {
            return;
        }
        set_local_value.set(format!("{:.1}", server_value.get()));
        set_is_editing.set(true);
    };

    let on_blur = move |_| {
        // Discard changes on blur
        set_is_editing.set(false);
    };

    let on_submit_clone = on_submit.clone();
    let on_keydown = move |ev: KeyboardEvent| {
        match ev.key().as_str() {
            "Enter" => {
                ev.prevent_default();
                if let Ok(val) = local_value.get().parse::<f64>() {
                    on_submit_clone(val);
                }
                set_is_editing.set(false);
                // Blur the input
                if let Some(target) = ev.target() {
                    if let Ok(el) = target.dyn_into::<web_sys::HtmlElement>() {
                        let _ = el.blur();
                    }
                }
            }
            "Escape" => {
                set_is_editing.set(false);
                // Blur the input
                if let Some(target) = ev.target() {
                    if let Ok(el) = target.dyn_into::<web_sys::HtmlElement>() {
                        let _ = el.blur();
                    }
                }
            }
            _ => {}
        }
    };

    let input_class = move || {
        if disabled.get() {
            "w-full bg-background rounded px-1.5 py-1 text-muted-foreground text-[11px] text-right font-mono border border-border/8 cursor-not-allowed"
        } else {
            "w-full bg-background rounded px-1.5 py-1 text-foreground text-[11px] text-right font-mono border border-border/8 focus:border-primary focus:outline-none"
        }
    };

    view! {
        <input
            type="text"
            class=input_class
            prop:value=display_value
            disabled=move || disabled.get()
            on:focus=on_focus
            on:blur=on_blur
            on:input=move |ev| set_local_value.set(event_target_value(&ev))
            on:keydown=on_keydown
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
                "bg-card border border-border/8 text-muted font-semibold py-1.5 rounded text-center text-[10px] cursor-not-allowed opacity-50"
            } else {
                "bg-card hover:bg-primary border border-border/8 hover:border-primary text-foreground hover:text-primary-foreground font-semibold py-1.5 rounded transition-colors text-center text-[10px]"
            }
            disabled=move || disabled.get()
            on:click=do_jog
        >
            {label}
        </button>
    }
}
