//! Command input section with recent commands and composer button.
//!
//! Matches original Fanuc_RMI_API implementation - uses fanuc_rmi::dto types directly.
//! Uses targeted messages with authorization for motion commands.
//! Commands are sent to the System entity - server will route to the active robot.

use leptos::prelude::*;
use pl3xus_client::{use_sync_context, use_entity_component};
use fanuc_replica_types::*;
use fanuc_rmi::dto::{SendPacket, Instruction, FrcLinearMotion, FrcLinearRelative, FrcJointMotion, Position, Configuration};
use fanuc_rmi::{SpeedType, TermType};
use crate::pages::dashboard::context::{WorkspaceContext, RecentCommand};
use crate::pages::dashboard::use_system_entity;

/// Helper function to create a motion packet from a RecentCommand.
/// Uses the active configuration from server-synced state for arm configuration.
/// Returns None if no robot is connected (can't create valid packet without config).
fn create_motion_packet(cmd: &RecentCommand, active_config: &ActiveConfigState) -> SendPacket {
    // Use active configuration values from server state
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
        // Both joint_abs and joint_rel use FrcJointMotion - the position determines absolute vs relative
        "joint_abs" | "joint_rel" => SendPacket::Instruction(Instruction::FrcJointMotion(FrcJointMotion {
            sequence_id: 0,
            configuration: config,
            position,
            speed_type,
            speed: cmd.speed,
            term_type,
            term_value,
        })),
        _ => {
            // Default to linear relative for unknown types
            SendPacket::Instruction(Instruction::FrcLinearRelative(FrcLinearRelative {
                sequence_id: 0,
                configuration: config,
                position,
                speed_type,
                speed: cmd.speed,
                term_type,
                term_value,
            }))
        }
    }
}

/// Command input section with recent commands and composer button
#[component]
pub fn CommandInputSection() -> impl IntoView {
    let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let sync_ctx = use_sync_context();
    let system_ctx = use_system_entity();

    // Subscribe to entity-specific components
    let (connection_state, _) = use_entity_component::<ConnectionState, _>(move || system_ctx.system_entity_id.get());
    let (active_config, _) = use_entity_component::<ActiveConfigState, _>(move || system_ctx.robot_entity_id.get());

    let recent_commands = ctx.recent_commands;
    let selected_cmd_id = ctx.selected_command_id;

    // Robot connected state
    let robot_connected = Memo::new(move |_| connection_state.get().robot_connected);

    // Get the Robot entity bits (for targeted motion commands)
    // Motion commands like SendPacket target the Robot entity
    let robot_entity_bits = move || system_ctx.robot_entity_id.get();

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
                    on:click={
                        let sync_ctx = sync_ctx.clone();
                        move |_| {
                            let Some(entity_bits) = robot_entity_bits() else {
                                ctx.add_error("Cannot run command: Robot entity not found".to_string());
                                return;
                            };
                            if let Some(idx) = selected_cmd_id.get() {
                                let cmds = recent_commands.get();
                                if let Some(cmd) = cmds.iter().find(|c| c.id == idx) {
                                    // Get active config from server state
                                    let cfg = active_config.get();
                                    // Create motion packet using fanuc_rmi::dto types
                                    let packet = create_motion_packet(cmd, &cfg);
                                    sync_ctx.send_targeted(entity_bits, packet);
                                }
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

