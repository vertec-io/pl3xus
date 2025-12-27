//! Quick Commands panel for robot control (Initialize, Reset, Abort, Continue).
//!
//! This is a consolidated panel that combines:
//! - Quick action buttons (Initialize, Reset, Abort)
//! - Recent commands dropdown with Compose and Run buttons
//! - Speed override slider
//!
//! Layout is responsive - stacks vertically on smaller screens.
//!
//! ## Server-Driven Response Handling
//!
//! All robot commands use the targeted mutation pattern which requires control.
//! The server sends responses indicating success or failure, and the client
//! displays appropriate toast notifications based on those responses.

use leptos::prelude::*;
use pl3xus_client::{use_mutation_targeted, use_entity_component, use_sync_context};
use fanuc_replica_plugins::*;
use fanuc_rmi::dto::{SendPacket, Instruction, FrcLinearMotion, FrcLinearRelative, FrcJointMotion, Position, Configuration};
use fanuc_rmi::{SpeedType, TermType};
use crate::components::use_toast;
use crate::pages::dashboard::context::{WorkspaceContext, MessageDirection, MessageType, RecentCommand};
use crate::pages::dashboard::use_system_entity;

/// Helper function to create a motion packet from a RecentCommand.
fn create_motion_packet(cmd: &RecentCommand, active_config: &ActiveConfigState) -> SendPacket {
    let config = Configuration {
        u_tool_number: cmd.utool as i8,
        u_frame_number: cmd.uframe as i8,
        front: active_config.front as i8,
        up: active_config.up as i8,
        left: active_config.left as i8,
        flip: active_config.flip as i8,
        turn4: active_config.turn4 as i8,
        turn5: active_config.turn5 as i8,
        turn6: active_config.turn6 as i8,
    };
    let position = Position {
        x: cmd.x, y: cmd.y, z: cmd.z,
        w: cmd.w, p: cmd.p, r: cmd.r,
        ext1: 0.0, ext2: 0.0, ext3: 0.0,
    };
    let speed_type = SpeedType::MMSec;
    let term_type = if cmd.term_type == "FINE" { TermType::FINE } else { TermType::CNT };
    let term_value = if cmd.term_type == "FINE" { 0 } else { 100 };

    match cmd.command_type.as_str() {
        "linear_rel" => SendPacket::Instruction(Instruction::FrcLinearRelative(FrcLinearRelative {
            sequence_id: 0,
            configuration: config,
            position,
            speed_type,
            speed: cmd.speed,
            term_type,
            term_value,
        })),
        "linear_abs" => SendPacket::Instruction(Instruction::FrcLinearMotion(FrcLinearMotion {
            sequence_id: 0,
            configuration: config,
            position,
            speed_type,
            speed: cmd.speed,
            term_type,
            term_value,
        })),
        "joint_abs" | "joint_rel" => SendPacket::Instruction(Instruction::FrcJointMotion(FrcJointMotion {
            sequence_id: 0,
            configuration: config,
            position,
            speed_type,
            speed: cmd.speed,
            term_type,
            term_value,
        })),
        _ => SendPacket::Instruction(Instruction::FrcLinearRelative(FrcLinearRelative {
            sequence_id: 0,
            configuration: config,
            position,
            speed_type,
            speed: cmd.speed,
            term_type,
            term_value,
        })),
    }
}

/// Consolidated Commands panel combining quick actions, recent commands, and speed override.
#[component]
pub fn QuickCommandsPanel() -> impl IntoView {
    let ws_ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let ctx = ws_ctx.clone();
    let sync_ctx = use_sync_context();
    let toast = use_toast();
    let system_ctx = use_system_entity();

    // Subscribe to entity-specific components
    let (status, _) = use_entity_component::<RobotStatus, _>(move || system_ctx.robot_entity_id.get());
    let (connection_state, robot_exists) = use_entity_component::<ConnectionState, _>(move || system_ctx.robot_entity_id.get());
    let (active_config, _) = use_entity_component::<ActiveConfigState, _>(move || system_ctx.robot_entity_id.get());

    // Recent commands from workspace context
    let recent_commands = ctx.recent_commands;
    let selected_cmd_id = ctx.selected_command_id;
    let show_composer = ctx.show_composer;

    // Track when user is actively dragging the slider
    let (user_editing, set_user_editing) = signal(false);

    // Robot connected state (only true if robot entity exists AND is connected)
    let robot_connected = Memo::new(move |_| robot_exists.get() && connection_state.get().robot_connected);
    let controls_disabled = move || !robot_connected.get();

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

    // Run command handler
    let run_command = {
        let sync_ctx = sync_ctx.clone();
        move || {
            let Some(entity_bits) = robot_entity_bits() else {
                ctx.add_error("Cannot run command: Robot entity not found".to_string());
                return;
            };
            if let Some(idx) = selected_cmd_id.get() {
                let cmds = recent_commands.get();
                if let Some(cmd) = cmds.iter().find(|c| c.id == idx) {
                    let cfg = active_config.get();
                    let packet = create_motion_packet(cmd, &cfg);
                    sync_ctx.send_targeted(entity_bits, packet);
                }
            }
        }
    };
    let run_command = StoredValue::new(run_command);

    // Speed adjustment handlers
    let adjust_speed = move |delta: i32| {
        let current = display_speed() as i32;
        let new_val = (current + delta).clamp(0, 100) as u32;
        set_pending_speed.set(Some(new_val));
        send_override.with_value(|f| f(new_val));
    };

    view! {
        <div class="bg-background rounded border border-border/8 p-2 shrink-0">
            // Single row: Action buttons + Speed control + Recent Commands
            <div class="flex flex-wrap gap-2 items-center">
                // Left group: Initialize, Reset, Abort buttons
                <div class="flex gap-1 items-center shrink-0">
                    <button
                        class="bg-[#22c55e20] border border-[#22c55e40] text-success text-[9px] px-2 py-1 rounded hover:bg-success/20 flex items-center gap-1 disabled:opacity-50 disabled:cursor-not-allowed"
                        disabled=controls_disabled
                        on:click=init_click
                        title="Initialize robot"
                    >
                        <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 3v4M3 5h4M6 17v4m-2-2h4m5-16l2.286 6.857L21 12l-5.714 2.143L13 21l-2.286-6.857L5 12l5.714-2.143L13 3z"/>
                        </svg>
                        "Initialize"
                    </button>
                    <button
                        class="bg-[#f59e0b20] border border-[#f59e0b40] text-warning text-[9px] px-2 py-1 rounded hover:bg-warning/20 flex items-center gap-1 disabled:opacity-50 disabled:cursor-not-allowed"
                        disabled=controls_disabled
                        on:click=reset_click
                        title="Reset robot"
                    >
                        <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/>
                        </svg>
                        "Reset"
                    </button>
                    <button
                        class="bg-destructive/15 border border-destructive/25 text-destructive text-[9px] px-2 py-1 rounded hover:bg-destructive/20 flex items-center gap-1 disabled:opacity-50 disabled:cursor-not-allowed"
                        disabled=controls_disabled
                        on:click=abort_click
                        title="Abort motion"
                    >
                        <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                        </svg>
                        "Abort"
                    </button>
                </div>

                // Separator
                <div class="w-px h-5 bg-border/20 shrink-0"/>

                // Speed Override - compact width matching action buttons
                <div class="flex items-center gap-1 bg-popover/50 rounded px-1.5 py-0.5 border border-border/5 shrink-0">
                    <svg class="w-3 h-3 text-primary shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"/>
                    </svg>
                    <button
                        class="text-[8px] text-muted-foreground hover:text-foreground px-1 py-0.5 rounded hover:bg-card disabled:opacity-50 disabled:cursor-not-allowed"
                        disabled=controls_disabled
                        on:click=move |_| adjust_speed(-5)
                        title="Decrease speed by 5%"
                    >
                        "-5"
                    </button>
                    <input
                        type="range"
                        min="0"
                        max="100"
                        step="5"
                        class="w-20 h-1 bg-[#333] rounded-lg appearance-none cursor-pointer accent-primary"
                        prop:value=move || display_speed()
                        disabled=controls_disabled
                        on:mousedown=move |_| set_user_editing.set(true)
                        on:touchstart=move |_| set_user_editing.set(true)
                        on:input=move |ev| {
                            if let Ok(val) = event_target_value(&ev).parse::<u32>() {
                                set_pending_speed.set(Some(val));
                            }
                        }
                        on:change=move |ev| {
                            set_user_editing.set(false);
                            if let Ok(val) = event_target_value(&ev).parse::<u32>() {
                                send_override.with_value(|f| f(val));
                            }
                        }
                    />
                    <button
                        class="text-[8px] text-muted-foreground hover:text-foreground px-1 py-0.5 rounded hover:bg-card disabled:opacity-50 disabled:cursor-not-allowed"
                        disabled=controls_disabled
                        on:click=move |_| adjust_speed(5)
                        title="Increase speed by 5%"
                    >
                        "+5"
                    </button>
                    <span class="text-[10px] text-primary font-mono w-7 text-right shrink-0">
                        {move || format!("{}%", display_speed())}
                    </span>
                </div>

                // Separator
                <div class="w-px h-5 bg-border/20 shrink-0"/>

                // Right group: Recent commands dropdown + Compose + Run
                <div class="flex gap-1 items-center flex-1 min-w-0">
                    <select
                        class="flex-1 min-w-0 bg-card border border-border/8 rounded px-2 py-1 text-[9px] text-foreground focus:border-primary focus:outline-none truncate"
                        prop:value=move || selected_cmd_id.get().map(|id| id.to_string()).unwrap_or_default()
                        on:change=move |ev| {
                            let val = event_target_value(&ev);
                            selected_cmd_id.set(val.parse().ok());
                        }
                    >
                        <option value="">{move || {
                            let count = recent_commands.get().len();
                            if count == 0 { "No commands".to_string() } else { format!("{} commands", count) }
                        }}</option>
                        {move || recent_commands.get().into_iter().map(|cmd| {
                            view! { <option value={cmd.id.to_string()}>{cmd.name.clone()}</option> }
                        }).collect_view()}
                    </select>
                    <button
                        class=move || if controls_disabled() {
                            "bg-card border border-border/8 text-muted-foreground text-[9px] px-2 py-1 rounded cursor-not-allowed"
                        } else {
                            "bg-[#00d9ff20] border border-[#00d9ff40] text-primary text-[9px] px-2 py-1 rounded hover:bg-primary/20"
                        }
                        disabled=controls_disabled
                        on:click=move |_| if !controls_disabled() { show_composer.set(true); }
                        title="Compose new command"
                    >
                        "+"
                    </button>
                    <button
                        class=move || if controls_disabled() || selected_cmd_id.get().is_none() {
                            "bg-card border border-border/8 text-muted-foreground text-[9px] px-2 py-1 rounded cursor-not-allowed"
                        } else {
                            "bg-[#22c55e20] border border-[#22c55e40] text-success text-[9px] px-2 py-1 rounded hover:bg-success/20"
                        }
                        disabled=move || controls_disabled() || selected_cmd_id.get().is_none()
                        on:click=move |_| run_command.with_value(|f| f())
                        title="Run selected command"
                    >
                        "▶"
                    </button>
                </div>
            </div>
        </div>
    }
}

