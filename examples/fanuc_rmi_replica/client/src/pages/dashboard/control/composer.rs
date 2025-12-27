//! Command Composer Modal - Create motion commands with a form interface.
//!
//! Matches the original Fanuc_RMI_API web application implementation exactly.
//! Uses fanuc_rmi::dto types directly for sending commands.
//! Gets arm configuration from server-synced ActiveConfigState.
//! Uses targeted messages with authorization for motion commands.
//! Commands are sent to the System entity - server will route to the active robot.

use leptos::prelude::*;
use leptos::web_sys;
use wasm_bindgen::JsCast;
use pl3xus_client::{use_sync_context, use_entity_component};
use fanuc_replica_types::*;
use fanuc_rmi::dto::{SendPacket, Instruction, FrcLinearMotion, FrcLinearRelative, FrcJointMotion, Position, Configuration};
use fanuc_rmi::{SpeedType, TermType};
use crate::pages::dashboard::context::{WorkspaceContext, RecentCommand};
use crate::pages::dashboard::use_system_entity;

/// Instruction types available in the composer
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum InstructionType {
    LinearAbsolute,
    LinearRelative,
    JointAbsolute,
    JointRelative,
}

impl InstructionType {
    fn label(&self) -> &'static str {
        match self {
            Self::LinearAbsolute => "Linear Absolute",
            Self::LinearRelative => "Linear Relative",
            Self::JointAbsolute => "Joint Absolute",
            Self::JointRelative => "Joint Relative",
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::LinearAbsolute => "linear_abs",
            Self::LinearRelative => "linear_rel",
            Self::JointAbsolute => "joint_abs",
            Self::JointRelative => "joint_rel",
        }
    }

    fn is_cartesian(&self) -> bool {
        matches!(self, Self::LinearAbsolute | Self::LinearRelative)
    }

    fn is_absolute(&self) -> bool {
        matches!(self, Self::LinearAbsolute | Self::JointAbsolute)
    }
}

/// Command Composer Modal - Create motion commands with a form interface.
#[component]
pub fn CommandComposerModal() -> impl IntoView {
    let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let sync_ctx = use_sync_context();
    let system_ctx = use_system_entity();

    // Subscribe to entity-specific components
    let (current_position, _) = use_entity_component::<RobotPosition, _>(move || system_ctx.robot_entity_id.get());
    let (current_joints, _) = use_entity_component::<JointAngles, _>(move || system_ctx.robot_entity_id.get());
    let (active_config, _) = use_entity_component::<ActiveConfigState, _>(move || system_ctx.robot_entity_id.get());

    // Get the Robot entity bits (for targeted motion commands)
    // Motion commands target the Robot entity
    let robot_entity_bits = move || system_ctx.robot_entity_id.get();

    // Instruction type - default to LinearRelative (matching original)
    let (instr_type, set_instr_type) = signal(InstructionType::LinearRelative);

    // Position/angle inputs (f64 for precision)
    let (x, set_x) = signal(0.0f64);
    let (y, set_y) = signal(0.0f64);
    let (z, set_z) = signal(0.0f64);
    let (w, set_w) = signal(0.0f64);
    let (p, set_p) = signal(0.0f64);
    let (r, set_r) = signal(0.0f64);

    // Joint angles (for joint moves)
    let (j1, set_j1) = signal(0.0f64);
    let (j2, set_j2) = signal(0.0f64);
    let (j3, set_j3) = signal(0.0f64);
    let (j4, set_j4) = signal(0.0f64);
    let (j5, set_j5) = signal(0.0f64);
    let (j6, set_j6) = signal(0.0f64);

    // Motion parameters
    let (speed, set_speed) = signal(100.0f64);
    let (term_type, set_term_type) = signal("FINE".to_string());

    // Track previous instruction type to detect changes
    let (prev_instr_type, set_prev_instr_type) = signal::<Option<InstructionType>>(None);

    // Update position and speed defaults when instruction type changes
    Effect::new(move |_| {
        let itype = instr_type.get();
        let prev = prev_instr_type.get_untracked();

        // Only update defaults when instruction type actually changes
        if prev == Some(itype) {
            return;
        }
        set_prev_instr_type.set(Some(itype));

        if itype.is_absolute() {
            // Absolute moves: load current position
            if itype.is_cartesian() {
                let pos = current_position.get_untracked();
                set_x.set(pos.x as f64);
                set_y.set(pos.y as f64);
                set_z.set(pos.z as f64);
                set_w.set(pos.w as f64);
                set_p.set(pos.p as f64);
                set_r.set(pos.r as f64);
            } else {
                // Joint absolute - load current joint angles
                let joints = current_joints.get_untracked();
                set_j1.set(joints.j1 as f64);
                set_j2.set(joints.j2 as f64);
                set_j3.set(joints.j3 as f64);
                set_j4.set(joints.j4 as f64);
                set_j5.set(joints.j5 as f64);
                set_j6.set(joints.j6 as f64);
            }
        } else {
            // Relative moves: default to zeros
            set_x.set(0.0);
            set_y.set(0.0);
            set_z.set(0.0);
            set_w.set(0.0);
            set_p.set(0.0);
            set_r.set(0.0);
            set_j1.set(0.0);
            set_j2.set(0.0);
            set_j3.set(0.0);
            set_j4.set(0.0);
            set_j5.set(0.0);
            set_j6.set(0.0);
        }
    });

    // Get active configuration for motion commands - from server-synced state
    let get_configuration = move || {
        let config = active_config.get_untracked();
        Configuration {
            u_tool_number: config.u_tool_number as i8,
            u_frame_number: config.u_frame_number as i8,
            front: config.front as i8,
            up: config.up as i8,
            left: config.left as i8,
            flip: config.flip as i8,
            turn4: config.turn4 as i8,
            turn5: config.turn5 as i8,
            turn6: config.turn6 as i8,
        }
    };

    // Create motion packet from current form values
    let create_packet = move || -> SendPacket {
        let itype = instr_type.get_untracked();
        let config = get_configuration();
        let term = if term_type.get_untracked() == "FINE" { TermType::FINE } else { TermType::CNT };
        let term_value = if term_type.get_untracked() == "FINE" { 0 } else { 100 };
        let spd = speed.get_untracked();

        match itype {
            InstructionType::LinearRelative => {
                SendPacket::Instruction(Instruction::FrcLinearRelative(FrcLinearRelative {
                    sequence_id: 0,
                    configuration: config,
                    position: Position {
                        x: x.get_untracked(), y: y.get_untracked(), z: z.get_untracked(),
                        w: w.get_untracked(), p: p.get_untracked(), r: r.get_untracked(),
                        ext1: 0.0, ext2: 0.0, ext3: 0.0,
                    },
                    speed_type: SpeedType::MMSec,
                    speed: spd,
                    term_type: term,
                    term_value,
                }))
            }
            InstructionType::LinearAbsolute => {
                SendPacket::Instruction(Instruction::FrcLinearMotion(FrcLinearMotion {
                    sequence_id: 0,
                    configuration: config,
                    position: Position {
                        x: x.get_untracked(), y: y.get_untracked(), z: z.get_untracked(),
                        w: w.get_untracked(), p: p.get_untracked(), r: r.get_untracked(),
                        ext1: 0.0, ext2: 0.0, ext3: 0.0,
                    },
                    speed_type: SpeedType::MMSec,
                    speed: spd,
                    term_type: term,
                    term_value,
                }))
            }
            InstructionType::JointAbsolute | InstructionType::JointRelative => {
                // Both joint types use FrcJointMotion with position
                SendPacket::Instruction(Instruction::FrcJointMotion(FrcJointMotion {
                    sequence_id: 0,
                    configuration: config,
                    position: Position {
                        x: j1.get_untracked(), y: j2.get_untracked(), z: j3.get_untracked(),
                        w: j4.get_untracked(), p: j5.get_untracked(), r: j6.get_untracked(),
                        ext1: 0.0, ext2: 0.0, ext3: 0.0,
                    },
                    speed_type: SpeedType::MMSec,
                    speed: spd,
                    term_type: term,
                    term_value,
                }))
            }
        }
    };

    // Apply command - add to recent AND select it (don't execute)
    let apply_command = move || {
        let itype = instr_type.get_untracked();
        let new_id = js_sys::Date::now() as usize;

        let (v1, v2, v3, v4, v5, v6) = if itype.is_cartesian() {
            (x.get_untracked(), y.get_untracked(), z.get_untracked(),
             w.get_untracked(), p.get_untracked(), r.get_untracked())
        } else {
            (j1.get_untracked(), j2.get_untracked(), j3.get_untracked(),
             j4.get_untracked(), j5.get_untracked(), j6.get_untracked())
        };

        // Get uframe/utool from server-synced active config
        let config = active_config.get_untracked();
        let uframe = config.u_frame_number as u8;
        let utool = config.u_tool_number as u8;

        let cmd = RecentCommand {
            id: new_id,
            name: format!("{} ({:.1}, {:.1}, {:.1})", itype.label(), v1, v2, v3),
            command_type: itype.code().to_string(),
            description: format!("{:.0} mm/s {}", speed.get_untracked(), term_type.get_untracked()),
            x: v1, y: v2, z: v3, w: v4, p: v5, r: v6,
            speed: speed.get_untracked(),
            term_type: term_type.get_untracked(),
            uframe,
            utool,
        };

        ctx.recent_commands.update(|cmds| {
            cmds.insert(0, cmd);
            while cmds.len() > 15 {
                cmds.pop();
            }
        });
        ctx.selected_command_id.set(Some(new_id));
        ctx.show_composer.set(false);
    };
    let apply_command = StoredValue::new(apply_command);

    // Close modal
    let close_modal = move |_| ctx.show_composer.set(false);

    // Execute command - apply + send to robot
    // Note: Console messages are logged by the server when it receives/processes commands
    let execute_command = {
        let sync_ctx = sync_ctx.clone();
        move || {
            let Some(entity_bits) = robot_entity_bits() else {
                leptos::logging::warn!("Cannot execute command: Robot entity not found");
                return;
            };

            // First apply (add to recent + select)
            apply_command.get_value()();

            // Send the DTO packet directly to server - pl3xus handles serialization
            let packet = create_packet();
            sync_ctx.send_targeted(entity_bits, packet);
        }
    };

    // Load current position button handler
    let load_current = move |_| {
        let itype = instr_type.get_untracked();
        if itype.is_cartesian() {
            let pos = current_position.get_untracked();
            set_x.set(pos.x as f64);
            set_y.set(pos.y as f64);
            set_z.set(pos.z as f64);
            set_w.set(pos.w as f64);
            set_p.set(pos.p as f64);
            set_r.set(pos.r as f64);
        } else {
            let joints = current_joints.get_untracked();
            set_j1.set(joints.j1 as f64);
            set_j2.set(joints.j2 as f64);
            set_j3.set(joints.j3 as f64);
            set_j4.set(joints.j4 as f64);
            set_j5.set(joints.j5 as f64);
            set_j6.set(joints.j6 as f64);
        }
    };

    // Set to zero button handler
    let set_to_zero = move |_| {
        set_x.set(0.0); set_y.set(0.0); set_z.set(0.0);
        set_w.set(0.0); set_p.set(0.0); set_r.set(0.0);
        set_j1.set(0.0); set_j2.set(0.0); set_j3.set(0.0);
        set_j4.set(0.0); set_j5.set(0.0); set_j6.set(0.0);
    };

    view! {
        <div class="fixed inset-0 bg-black/70 flex items-center justify-center z-50">
            <div class="bg-background border border-border/8 rounded-lg w-[600px] max-h-[90vh] flex flex-col">
                // Header
                <div class="flex items-center justify-between p-3 border-b border-border/8">
                    <h2 class="text-sm font-semibold text-white flex items-center gap-2">
                        <svg class="w-4 h-4 text-primary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"/>
                        </svg>
                        "Command Composer"
                    </h2>
                    <button class="text-muted-foreground hover:text-white" on:click=close_modal>
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                        </svg>
                    </button>
                </div>

                // Content - scrollable
                <div class="flex-1 overflow-y-auto p-4 space-y-4">
                    // Instruction Type Selection
                    <div class="space-y-2">
                        <label class="text-[10px] text-muted-foreground uppercase tracking-wide">"Motion Type"</label>
                        <select
                            class="w-full bg-card border border-primary/40 rounded px-3 py-2 text-[12px] text-white focus:border-primary focus:outline-none"
                            on:change=move |ev| {
                                let val = event_target_value(&ev);
                                let itype = match val.as_str() {
                                    "linear_abs" => InstructionType::LinearAbsolute,
                                    "linear_rel" => InstructionType::LinearRelative,
                                    "joint_abs" => InstructionType::JointAbsolute,
                                    "joint_rel" => InstructionType::JointRelative,
                                    _ => InstructionType::LinearRelative,
                                };
                                set_instr_type.set(itype);
                            }
                        >
                            <option value="linear_rel" selected=move || instr_type.get() == InstructionType::LinearRelative>"Linear Relative"</option>
                            <option value="linear_abs" selected=move || instr_type.get() == InstructionType::LinearAbsolute>"Linear Absolute"</option>
                            <option value="joint_abs" selected=move || instr_type.get() == InstructionType::JointAbsolute>"Joint Absolute"</option>
                            <option value="joint_rel" selected=move || instr_type.get() == InstructionType::JointRelative>"Joint Relative"</option>
                        </select>
                        <div class="text-[9px] text-muted-foreground">
                            {move || match instr_type.get() {
                                InstructionType::LinearAbsolute => "Move to absolute Cartesian position in a straight line",
                                InstructionType::LinearRelative => "Move by relative Cartesian offset in a straight line",
                                InstructionType::JointAbsolute => "Move to absolute joint angles",
                                InstructionType::JointRelative => "Move by relative joint angle offsets",
                            }}
                        </div>
                    </div>

                    // Position Inputs
                    <div class="space-y-2">
                        <div class="flex items-center justify-between">
                            <label class="text-[10px] text-muted-foreground uppercase tracking-wide">
                                {move || {
                                    let itype = instr_type.get();
                                    if !itype.is_cartesian() && !itype.is_absolute() { "Joint Offsets" }
                                    else if !itype.is_cartesian() { "Joint Angles" }
                                    else if !itype.is_absolute() { "Position Offset" }
                                    else { "Target Position" }
                                }}
                            </label>
                            <div class="flex gap-2">
                                <button
                                    class="text-[9px] text-primary hover:underline"
                                    on:click=set_to_zero
                                >
                                    "Set to Zero"
                                </button>
                                <button
                                    class="text-[9px] text-primary hover:underline"
                                    on:click=load_current
                                >
                                    "Load Current"
                                </button>
                            </div>
                        </div>

                        // Cartesian inputs
                        <Show when=move || instr_type.get().is_cartesian()>
                            <div class="grid grid-cols-6 gap-2">
                                <NumericInput label=move || if instr_type.get().is_absolute() { "X" } else { "ΔX" } value=x set_value=set_x unit="mm"/>
                                <NumericInput label=move || if instr_type.get().is_absolute() { "Y" } else { "ΔY" } value=y set_value=set_y unit="mm"/>
                                <NumericInput label=move || if instr_type.get().is_absolute() { "Z" } else { "ΔZ" } value=z set_value=set_z unit="mm"/>
                                <NumericInput label=move || if instr_type.get().is_absolute() { "W" } else { "ΔW" } value=w set_value=set_w unit="°"/>
                                <NumericInput label=move || if instr_type.get().is_absolute() { "P" } else { "ΔP" } value=p set_value=set_p unit="°"/>
                                <NumericInput label=move || if instr_type.get().is_absolute() { "R" } else { "ΔR" } value=r set_value=set_r unit="°"/>
                            </div>
                        </Show>

                        // Joint inputs
                        <Show when=move || !instr_type.get().is_cartesian()>
                            <div class="grid grid-cols-6 gap-2">
                                <NumericInput label=move || if instr_type.get().is_absolute() { "J1" } else { "ΔJ1" } value=j1 set_value=set_j1 unit="°"/>
                                <NumericInput label=move || if instr_type.get().is_absolute() { "J2" } else { "ΔJ2" } value=j2 set_value=set_j2 unit="°"/>
                                <NumericInput label=move || if instr_type.get().is_absolute() { "J3" } else { "ΔJ3" } value=j3 set_value=set_j3 unit="°"/>
                                <NumericInput label=move || if instr_type.get().is_absolute() { "J4" } else { "ΔJ4" } value=j4 set_value=set_j4 unit="°"/>
                                <NumericInput label=move || if instr_type.get().is_absolute() { "J5" } else { "ΔJ5" } value=j5 set_value=set_j5 unit="°"/>
                                <NumericInput label=move || if instr_type.get().is_absolute() { "J6" } else { "ΔJ6" } value=j6 set_value=set_j6 unit="°"/>
                            </div>
                        </Show>
                    </div>

                    // Motion Parameters
                    <div class="grid grid-cols-2 gap-3">
                        <div class="space-y-1">
                            <label class="text-[10px] text-muted-foreground uppercase tracking-wide">"Speed"</label>
                            <div class="flex">
                                <input
                                    type="text"
                                    inputmode="decimal"
                                    class="flex-1 bg-card border border-border/8 rounded-l px-2 py-1.5 text-[11px] text-white focus:border-primary focus:outline-none text-center"
                                    prop:value=move || format!("{:.1}", speed.get())
                                    on:input=move |ev| {
                                        if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                            set_speed.set(v);
                                        }
                                    }
                                />
                                <span class="bg-popover border border-l-0 border-border/8 rounded-r px-2 py-1.5 text-[9px] text-muted-foreground">"mm/s"</span>
                            </div>
                        </div>
                        <div class="space-y-1">
                            <label class="text-[10px] text-muted-foreground uppercase tracking-wide">"Termination"</label>
                            <select
                                class="w-full bg-card border border-border/8 rounded px-2 py-1.5 text-[11px] text-white focus:border-primary focus:outline-none"
                                prop:value=move || term_type.get()
                                on:change=move |ev| set_term_type.set(event_target_value(&ev))
                            >
                                <option value="FINE">"FINE"</option>
                                <option value="CNT100">"CNT100"</option>
                            </select>
                        </div>
                    </div>
                </div>

                // Footer
                <div class="flex justify-between p-3 border-t border-border/8">
                    <button
                        class="bg-popover border border-border/8 text-foreground text-[10px] px-4 py-1.5 rounded hover:bg-secondary"
                        on:click=close_modal
                    >
                        "Cancel"
                    </button>
                    <div class="flex gap-2">
                        <button
                            class="bg-primary text-white text-[10px] px-4 py-1.5 rounded hover:brightness-110"
                            on:click=move |_| apply_command.get_value()()
                        >
                            "Apply"
                        </button>
                        <button
                            class="bg-success text-white text-[10px] px-4 py-1.5 rounded hover:brightness-110 font-medium"
                            on:click=move |_| execute_command()
                        >
                            "▶ Execute Now"
                        </button>
                    </div>
                </div>
            </div>
        </div>
    }
}

/// Numeric input field component for f64 values
/// Uses type="text" with inputmode="decimal" for better decimal/negative handling
#[component]
fn NumericInput<L>(
    label: L,
    value: ReadSignal<f64>,
    set_value: WriteSignal<f64>,
    #[prop(into)] unit: &'static str,
) -> impl IntoView
where
    L: Fn() -> &'static str + Send + Sync + 'static,
{
    // Local string state for editing
    let (text, set_text) = signal(format!("{:.3}", value.get_untracked()));
    // Track if user is actively editing (focused)
    let (is_editing, set_is_editing) = signal(false);
    // Track the last value we synced to, to detect external changes
    let (last_synced_value, set_last_synced_value) = signal(value.get_untracked());

    // Only sync text when value changes externally (not from our own input)
    Effect::new(move |_| {
        let v = value.get();
        let last = last_synced_value.get_untracked();
        // Only update text if not editing AND value changed externally
        if !is_editing.get_untracked() && (v - last).abs() > 0.0001 {
            set_text.set(format!("{:.3}", v));
            set_last_synced_value.set(v);
        }
    });

    view! {
        <div class="space-y-1">
            <label class="text-[9px] text-muted-foreground text-center block">{label}</label>
            <div class="flex">
                <input
                    type="text"
                    inputmode="decimal"
                    class="flex-1 min-w-0 bg-card border border-border/8 rounded-l px-1 py-1 text-[10px] text-white focus:outline-none focus:border-primary text-center"
                    prop:value=move || text.get()
                    on:focus=move |_| set_is_editing.set(true)
                    on:blur=move |_| {
                        set_is_editing.set(false);
                        // On blur, format the value nicely
                        let v = value.get_untracked();
                        set_text.set(format!("{:.3}", v));
                        set_last_synced_value.set(v);
                    }
                    on:keydown=move |ev: web_sys::KeyboardEvent| {
                        // Enter key applies the value by blurring the input
                        if ev.key() == "Enter" {
                            if let Some(target) = ev.target() {
                                if let Ok(el) = target.dyn_into::<web_sys::HtmlElement>() {
                                    let _ = el.blur();
                                }
                            }
                        }
                    }
                    on:input=move |ev| {
                        let s = event_target_value(&ev);
                        set_text.set(s.clone());
                        if let Ok(v) = s.parse::<f64>() {
                            set_value.set(v);
                            set_last_synced_value.set(v);
                        }
                    }
                />
                <span class="bg-popover border border-l-0 border-border/8 rounded-r px-1 py-1 text-[8px] text-muted-foreground">{unit}</span>
            </div>
        </div>
    }
}
