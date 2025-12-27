//! Quick Commands panel for robot control (Initialize, Reset, Abort, Continue).
//!
//! Uses targeted mutations with authorization for robot commands.
//! Commands are sent to the Robot entity and return responses.
//!
//! ## Server-Driven Response Handling
//!
//! All robot commands use the targeted mutation pattern which requires control.
//! The server sends responses indicating success or failure, and the client
//! displays appropriate toast notifications based on those responses.
//!
//! The UI never lies to the user - it only shows success when the server confirms.

use leptos::prelude::*;
use pl3xus_client::{use_mutation_targeted, use_entity_component};
use fanuc_replica_types::*;
use crate::components::use_toast;
use crate::pages::dashboard::context::{WorkspaceContext, MessageDirection, MessageType};
use crate::pages::dashboard::use_system_entity;

/// Quick Commands panel for robot control (Initialize, Reset, Abort, Continue).
#[component]
pub fn QuickCommandsPanel() -> impl IntoView {
    let ws_ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let toast = use_toast();
    let system_ctx = use_system_entity();

    // Subscribe to entity-specific components
    // RobotStatus and ConnectionState both live on the robot entity
    let (status, _) = use_entity_component::<RobotStatus, _>(move || system_ctx.robot_entity_id.get());
    let (connection_state, robot_exists) = use_entity_component::<ConnectionState, _>(move || system_ctx.robot_entity_id.get());

    // Track when user is actively dragging the slider
    let (user_editing, set_user_editing) = signal(false);

    // Robot connected state (only true if robot entity exists AND is connected)
    let robot_connected = Memo::new(move |_| robot_exists.get() && connection_state.get().robot_connected);

    // Get the Robot entity bits (for targeted robot commands)
    let robot_entity_bits = move || system_ctx.robot_entity_id.get();

    // Read speed override directly from synced RobotStatus component
    let (pending_speed, set_pending_speed) = signal::<Option<u32>>(None);

    // Get server speed value
    let server_speed = move || status.get().speed_override as u32;

    // Display value: use pending value while editing or waiting for sync, otherwise use server value
    let display_speed = move || {
        if user_editing.get() {
            if let Some(pending) = pending_speed.get() {
                return pending;
            }
        }
        if let Some(pending) = pending_speed.get() {
            return pending;
        }
        server_speed()
    };

    // =========================================================================
    // Targeted Mutation Hooks (TanStack Query-inspired API)
    //
    // use_mutation_targeted handles response deduplication internally
    // - The handler is called exactly once per response
    // - Returns a MutationHandle with .send() and state accessors
    // =========================================================================

    // SetSpeedOverride - mutation handler clears pending_speed on failure
    let speed_override = use_mutation_targeted::<SetSpeedOverride>(move |result| {
        match result {
            Ok(r) if r.success => {
                // Success - pending_speed will be cleared when synced component updates
            }
            Ok(r) => {
                set_pending_speed.set(None);
                toast.error(format!("Speed denied: {}", r.error.as_deref().unwrap_or("No control")));
            }
            Err(e) => {
                set_pending_speed.set(None);
                toast.error(format!("Failed to set speed: {e}"));
            }
        }
    });

    // InitializeRobot - use the clean mutation API
    let initialize = use_mutation_targeted::<InitializeRobot>(move |result| {
        match result {
            Ok(r) if r.success => {
                toast.success("Robot initialized");
                ws_ctx.add_console_message("Robot initialized".to_string(), MessageDirection::Received, MessageType::Response);
            }
            Ok(r) => toast.error(format!("Initialize denied: {}", r.error.as_deref().unwrap_or("No control"))),
            Err(e) => toast.error(format!("Initialize failed: {e}")),
        }
    });

    // ResetRobot
    let reset = use_mutation_targeted::<ResetRobot>(move |result| {
        match result {
            Ok(r) if r.success => {
                toast.success("Robot reset");
                ws_ctx.add_console_message("Robot reset".to_string(), MessageDirection::Received, MessageType::Response);
            }
            Ok(r) => toast.error(format!("Reset denied: {}", r.error.as_deref().unwrap_or("No control"))),
            Err(e) => toast.error(format!("Reset failed: {e}")),
        }
    });

    // AbortMotion
    let abort = use_mutation_targeted::<AbortMotion>(move |result| {
        match result {
            Ok(r) if r.success => {
                toast.warning("Motion aborted");
                ws_ctx.add_console_message("Motion aborted".to_string(), MessageDirection::Received, MessageType::Response);
            }
            Ok(r) => toast.error(format!("Abort denied: {}", r.error.as_deref().unwrap_or("No control"))),
            Err(e) => toast.error(format!("Abort failed: {e}")),
        }
    });

    // Effect to clear pending value when server catches up
    Effect::new(move |_| {
        let server = server_speed();
        let pending = pending_speed.get_untracked();
        if let Some(pending_val) = pending {
            if server == pending_val {
                set_pending_speed.set(None);
            }
        }
    });

    // Send override command when slider changes
    // MutationHandle is Copy, so no StoredValue needed
    let send_override = move |value: u32| {
        let Some(entity_bits) = robot_entity_bits() else {
            toast.error("Cannot send speed: Robot not found");
            return;
        };
        let clamped = value.min(100) as u8;
        speed_override.send(entity_bits, SetSpeedOverride { speed: clamped });
    };
    let send_override = StoredValue::new(send_override);

    // Click handlers using the new mutation API
    let init_click = move |_| {
        let Some(entity_bits) = robot_entity_bits() else {
            toast.error("Cannot initialize: Robot not found");
            return;
        };
        ws_ctx.add_console_message("Initialize Robot".to_string(), MessageDirection::Sent, MessageType::Command);
        initialize.send(entity_bits, InitializeRobot { group_mask: Some(1) });
    };
    let reset_click = move |_| {
        let Some(entity_bits) = robot_entity_bits() else {
            toast.error("Cannot reset: Robot not found");
            return;
        };
        ws_ctx.add_console_message("Reset Robot".to_string(), MessageDirection::Sent, MessageType::Command);
        reset.send(entity_bits, ResetRobot);
    };
    let abort_click = move |_| {
        let Some(entity_bits) = robot_entity_bits() else {
            toast.error("Cannot abort: Robot not found");
            return;
        };
        ws_ctx.add_console_message("Abort Motion".to_string(), MessageDirection::Sent, MessageType::Command);
        abort.send(entity_bits, AbortMotion);
    };

    view! {
        <div class="bg-card backdrop-blur-theme rounded-theme border border-border shadow-theme p-2 shrink-0 transition-all duration-300">
            <h3 class="text-[10px] font-semibold text-primary uppercase tracking-wide flex items-center mb-2">
                <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"/>
                </svg>
                "Quick Commands"
            </h3>
            <div class="flex gap-2 flex-wrap items-center">
                // Initialize button
                <button
                    class="bg-success text-white text-[9px] px-3 py-1.5 rounded hover:brightness-110 flex items-center gap-1 disabled:opacity-50 disabled:cursor-not-allowed"
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
                    class="bg-warning text-white text-[9px] px-3 py-1.5 rounded hover:brightness-110 flex items-center gap-1 disabled:opacity-50 disabled:cursor-not-allowed"
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
                    class="bg-destructive text-white text-[9px] px-3 py-1.5 rounded hover:brightness-110 flex items-center gap-1 disabled:opacity-50 disabled:cursor-not-allowed"
                    disabled=move || !robot_connected.get()
                    on:click=abort_click
                >
                    <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/>
                    </svg>
                    "Abort"
                </button>

                // Speed Override Slider
                <div class="flex items-center gap-2 ml-auto bg-popover rounded px-2 py-1 border border-border/6">
                    <svg class="w-3 h-3 text-primary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"/>
                    </svg>
                    <span class="text-[9px] text-gray-400 whitespace-nowrap">"Speed:"</span>
                    <input
                        type="range"
                        min="0"
                        max="100"
                        step="5"
                        class="w-20 h-1 bg-muted rounded-lg appearance-none cursor-pointer accent-primary"
                        prop:value=move || display_speed()
                        disabled=move || !robot_connected.get()
                        on:mousedown=move |_| set_user_editing.set(true)
                        on:touchstart=move |_| set_user_editing.set(true)
                        on:input=move |ev| {
                            // While dragging, update pending value for smooth visual feedback
                            if let Ok(val) = event_target_value(&ev).parse::<u32>() {
                                set_pending_speed.set(Some(val));
                            }
                        }
                        on:change=move |ev| {
                            // On release, send command and clear pending state
                            // The display will revert to server value (synced component)
                            // If command succeeds, server will update the synced component
                            // If command fails (e.g., no control), slider shows server value
                            set_user_editing.set(false);
                            // set_pending_speed.set(None);
                            if let Ok(val) = event_target_value(&ev).parse::<u32>() {
                                send_override.with_value(|f| f(val));
                            }
                        }
                    />
                    <span class="text-[10px] text-primary font-mono w-8 text-right">
                        {move || format!("{}%", display_speed())}
                    </span>
                </div>
            </div>
        </div>
    }
}

