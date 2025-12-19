//! Command Composer Modal - Create motion commands with a form interface.
//!
//! Provides a single-page form for composing motion commands with:
//! - Command type selection (Linear, Joint, Relative)
//! - Position inputs (X, Y, Z, W, P, R)
//! - Motion parameters (Speed, Termination)
//! - Frame/Tool selection

use leptos::prelude::*;
use pl3xus_client::{use_sync_context, use_sync_component};
use fanuc_replica_types::*;
use crate::pages::dashboard::context::{WorkspaceContext, RecentCommand};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = Date)]
    fn now() -> f64;
}

/// Command Composer Modal - Create motion commands with a form interface.
#[component]
pub fn CommandComposerModal() -> impl IntoView {
    let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let sync_ctx = use_sync_context();
    let position = use_sync_component::<RobotPosition>();

    // Form state - default to linear_absolute
    let (command_type, set_command_type) = signal("linear_absolute".to_string());
    let (command_name, set_command_name) = signal("".to_string());
    
    // Position inputs
    let (x, set_x) = signal("0.0".to_string());
    let (y, set_y) = signal("0.0".to_string());
    let (z, set_z) = signal("0.0".to_string());
    let (w, set_w) = signal("0.0".to_string());
    let (p, set_p) = signal("0.0".to_string());
    let (r, set_r) = signal("0.0".to_string());
    
    // Motion parameters
    let (speed, set_speed) = signal("100.0".to_string());
    let (term_type, set_term_type) = signal("FINE".to_string());
    let (uframe, set_uframe) = signal("0".to_string());
    let (utool, set_utool) = signal("0".to_string());

    // Load current position
    let load_current = move |_| {
        if let Some(pos) = position.get().values().next() {
            set_x.set(format!("{:.3}", pos.x));
            set_y.set(format!("{:.3}", pos.y));
            set_z.set(format!("{:.3}", pos.z));
            set_w.set(format!("{:.3}", pos.w));
            set_p.set(format!("{:.3}", pos.p));
            set_r.set(format!("{:.3}", pos.r));
        }
    };

    // Close modal
    let close = move |_| {
        ctx.show_composer.set(false);
    };

    // Add to recent commands
    let add_to_recent = move |_| {
        let name = if command_name.get().is_empty() {
            format!("{} Move", command_type.get().to_uppercase())
        } else {
            command_name.get()
        };
        
        let cmd = RecentCommand {
            id: now() as usize,
            name: name.clone(),
            command_type: command_type.get(),
            description: format!("X:{} Y:{} Z:{}", x.get(), y.get(), z.get()),
            x: x.get().parse().unwrap_or(0.0),
            y: y.get().parse().unwrap_or(0.0),
            z: z.get().parse().unwrap_or(0.0),
            w: w.get().parse().unwrap_or(0.0),
            p: p.get().parse().unwrap_or(0.0),
            r: r.get().parse().unwrap_or(0.0),
            speed: speed.get().parse().unwrap_or(100.0),
            term_type: term_type.get(),
            uframe: uframe.get().parse().unwrap_or(0),
            utool: utool.get().parse().unwrap_or(0),
        };
        
        ctx.recent_commands.update(|cmds| {
            cmds.insert(0, cmd);
            if cmds.len() > 20 {
                cmds.pop();
            }
        });
        
        ctx.show_composer.set(false);
    };

    // Execute command immediately
    let execute_now = {
        let sync_ctx = sync_ctx.clone();
        move |_| {
            let cmd_type = command_type.get();
            let x_val: f32 = x.get().parse().unwrap_or(0.0);
            let y_val: f32 = y.get().parse().unwrap_or(0.0);
            let z_val: f32 = z.get().parse().unwrap_or(0.0);
            let w_val: f32 = w.get().parse().unwrap_or(0.0);
            let p_val: f32 = p.get().parse().unwrap_or(0.0);
            let r_val: f32 = r.get().parse().unwrap_or(0.0);
            let speed_val: f32 = speed.get().parse().unwrap_or(100.0);

            match cmd_type.as_str() {
                "linear_absolute" => {
                    sync_ctx.send(MoveLinear {
                        x: x_val, y: y_val, z: z_val,
                        w: w_val, p: p_val, r: r_val,
                        speed: speed_val,
                        uframe: uframe.get().parse().ok(),
                        utool: utool.get().parse().ok(),
                    });
                }
                "linear_relative" => {
                    sync_ctx.send(MoveRelative {
                        dx: x_val, dy: y_val, dz: z_val,
                        dw: w_val, dp: p_val, dr: r_val,
                        speed: speed_val,
                    });
                }
                "joint_absolute" => {
                    sync_ctx.send(MoveJoint {
                        j1: x_val, j2: y_val, j3: z_val,
                        j4: w_val, j5: p_val, j6: r_val,
                        speed: speed_val,
                    });
                }
                "joint_relative" => {
                    // Joint relative - use same message but with relative values
                    sync_ctx.send(MoveJoint {
                        j1: x_val, j2: y_val, j3: z_val,
                        j4: w_val, j5: p_val, j6: r_val,
                        speed: speed_val,
                    });
                }
                _ => {}
            }

            ctx.show_composer.set(false);
        }
    };

    view! {
        <div class="fixed inset-0 bg-black/70 flex items-center justify-center z-50">
            <div class="bg-[#0d0d0d] border border-[#ffffff15] rounded-lg w-[600px] max-h-[90vh] flex flex-col">
                // Header
                <div class="flex items-center justify-between p-3 border-b border-[#ffffff08]">
                    <h2 class="text-sm font-semibold text-white flex items-center gap-2">
                        <svg class="w-4 h-4 text-[#00d9ff]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"/>
                        </svg>
                        "Command Composer"
                    </h2>
                    <button class="text-[#666666] hover:text-white" on:click=close>
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                        </svg>
                    </button>
                </div>

                // Content - scrollable
                <div class="flex-1 overflow-y-auto p-4 space-y-4">
                    <ComposerForm
                        command_type=command_type set_command_type=set_command_type
                        command_name=command_name set_command_name=set_command_name
                        x=x set_x=set_x y=y set_y=set_y z=z set_z=set_z
                        w=w set_w=set_w p=p set_p=set_p r=r set_r=set_r
                        speed=speed set_speed=set_speed
                        term_type=term_type set_term_type=set_term_type
                        uframe=uframe set_uframe=set_uframe
                        utool=utool set_utool=set_utool
                        on_load_current=load_current
                    />
                </div>

                // Footer
                <div class="flex justify-between p-3 border-t border-[#ffffff08]">
                    <button
                        class="bg-[#1a1a1a] border border-[#ffffff15] text-[#cccccc] text-[10px] px-4 py-1.5 rounded hover:bg-[#222222]"
                        on:click=close
                    >
                        "Cancel"
                    </button>
                    <div class="flex gap-2">
                        <button
                            class="bg-[#00d9ff20] border border-[#00d9ff40] text-[#00d9ff] text-[10px] px-4 py-1.5 rounded hover:bg-[#00d9ff30]"
                            on:click=add_to_recent
                        >
                            "+ Add to Recent"
                        </button>
                        <button
                            class="bg-[#22c55e] text-black text-[10px] px-4 py-1.5 rounded hover:bg-[#1ea34b] font-medium"
                            on:click=execute_now
                        >
                            "▶ Execute Now"
                        </button>
                    </div>
                </div>
            </div>
        </div>
    }
}

/// Composer form with all input fields
#[component]
fn ComposerForm(
    command_type: ReadSignal<String>,
    set_command_type: WriteSignal<String>,
    command_name: ReadSignal<String>,
    set_command_name: WriteSignal<String>,
    x: ReadSignal<String>,
    set_x: WriteSignal<String>,
    y: ReadSignal<String>,
    set_y: WriteSignal<String>,
    z: ReadSignal<String>,
    set_z: WriteSignal<String>,
    w: ReadSignal<String>,
    set_w: WriteSignal<String>,
    p: ReadSignal<String>,
    set_p: WriteSignal<String>,
    r: ReadSignal<String>,
    set_r: WriteSignal<String>,
    speed: ReadSignal<String>,
    set_speed: WriteSignal<String>,
    term_type: ReadSignal<String>,
    set_term_type: WriteSignal<String>,
    uframe: ReadSignal<String>,
    set_uframe: WriteSignal<String>,
    utool: ReadSignal<String>,
    set_utool: WriteSignal<String>,
    on_load_current: impl Fn(leptos::ev::MouseEvent) + 'static,
) -> impl IntoView {
    let is_joint = Memo::new(move |_| command_type.get().starts_with("joint"));
    let is_relative = Memo::new(move |_| command_type.get().ends_with("_relative"));

    view! {
        // Command Type Selection - Dropdown with clear options like original
        <div class="space-y-2">
            <label class="text-[10px] text-[#888888] uppercase tracking-wide">"Motion Type"</label>
            <select
                class="w-full bg-[#111111] border border-[#00d9ff40] rounded px-3 py-2 text-[12px] text-white focus:border-[#00d9ff] focus:outline-none"
                prop:value=move || command_type.get()
                on:change=move |ev| set_command_type.set(event_target_value(&ev))
            >
                <option value="linear_absolute">"Linear Absolute (L)"</option>
                <option value="linear_relative">"Linear Relative (L REL)"</option>
                <option value="joint_absolute">"Joint Absolute (J)"</option>
                <option value="joint_relative">"Joint Relative (J REL)"</option>
            </select>
            <div class="text-[9px] text-[#666666]">
                {move || match command_type.get().as_str() {
                    "linear_absolute" => "Move to absolute Cartesian position in a straight line",
                    "linear_relative" => "Move by relative Cartesian offset in a straight line",
                    "joint_absolute" => "Move to absolute joint angles",
                    "joint_relative" => "Move by relative joint angle offsets",
                    _ => "",
                }}
            </div>
        </div>

        // Command Name
        <div class="space-y-1">
            <label class="text-[10px] text-[#888888] uppercase tracking-wide">"Command Name (optional)"</label>
            <input
                type="text"
                placeholder="e.g., Pick Position 1"
                class="w-full bg-[#111111] border border-[#ffffff08] rounded px-3 py-2 text-[11px] text-white focus:border-[#00d9ff] focus:outline-none"
                prop:value=move || command_name.get()
                on:input=move |ev| set_command_name.set(event_target_value(&ev))
            />
        </div>

        // Position Inputs
        <div class="space-y-2">
            <div class="flex items-center justify-between">
                <label class="text-[10px] text-[#888888] uppercase tracking-wide">
                    {move || {
                        if is_joint.get() && is_relative.get() { "Joint Offsets" }
                        else if is_joint.get() { "Joint Angles" }
                        else if is_relative.get() { "Position Offset" }
                        else { "Target Position" }
                    }}
                </label>
                <button
                    class="text-[9px] text-[#00d9ff] hover:underline"
                    on:click=on_load_current
                >
                    {move || if is_relative.get() { "Set to Zero" } else { "Load Current Position" }}
                </button>
            </div>
            <div class="grid grid-cols-6 gap-2">
                <PositionInput label=move || if is_joint.get() { if is_relative.get() { "ΔJ1" } else { "J1" } } else { if is_relative.get() { "ΔX" } else { "X" } } value=x set_value=set_x unit=move || if is_joint.get() { "°" } else { "mm" }/>
                <PositionInput label=move || if is_joint.get() { if is_relative.get() { "ΔJ2" } else { "J2" } } else { if is_relative.get() { "ΔY" } else { "Y" } } value=y set_value=set_y unit=move || if is_joint.get() { "°" } else { "mm" }/>
                <PositionInput label=move || if is_joint.get() { if is_relative.get() { "ΔJ3" } else { "J3" } } else { if is_relative.get() { "ΔZ" } else { "Z" } } value=z set_value=set_z unit=move || if is_joint.get() { "°" } else { "mm" }/>
                <PositionInput label=move || if is_joint.get() { if is_relative.get() { "ΔJ4" } else { "J4" } } else { if is_relative.get() { "ΔW" } else { "W" } } value=w set_value=set_w unit=move || if is_joint.get() { "°" } else { "°" }/>
                <PositionInput label=move || if is_joint.get() { if is_relative.get() { "ΔJ5" } else { "J5" } } else { if is_relative.get() { "ΔP" } else { "P" } } value=p set_value=set_p unit=move || if is_joint.get() { "°" } else { "°" }/>
                <PositionInput label=move || if is_joint.get() { if is_relative.get() { "ΔJ6" } else { "J6" } } else { if is_relative.get() { "ΔR" } else { "R" } } value=r set_value=set_r unit=move || if is_joint.get() { "°" } else { "°" }/>
            </div>
        </div>

        // Motion Parameters
        <div class="grid grid-cols-4 gap-3">
            <div class="space-y-1">
                <label class="text-[10px] text-[#888888] uppercase tracking-wide">"Speed"</label>
                <div class="flex">
                    <input
                        type="text"
                        inputmode="decimal"
                        class="flex-1 bg-[#111111] border border-[#ffffff08] rounded-l px-2 py-1.5 text-[11px] text-white focus:border-[#00d9ff] focus:outline-none text-center"
                        prop:value=move || speed.get()
                        on:input=move |ev| set_speed.set(event_target_value(&ev))
                    />
                    <span class="bg-[#1a1a1a] border border-l-0 border-[#ffffff08] rounded-r px-2 py-1.5 text-[9px] text-[#666666]">"mm/s"</span>
                </div>
            </div>
            <div class="space-y-1">
                <label class="text-[10px] text-[#888888] uppercase tracking-wide">"Termination"</label>
                <select
                    class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[11px] text-white focus:border-[#00d9ff] focus:outline-none"
                    prop:value=move || term_type.get()
                    on:change=move |ev| set_term_type.set(event_target_value(&ev))
                >
                    <option value="FINE">"FINE"</option>
                    <option value="CNT100">"CNT100"</option>
                    <option value="CNT50">"CNT50"</option>
                    <option value="CNT0">"CNT0"</option>
                </select>
            </div>
            <div class="space-y-1">
                <label class="text-[10px] text-[#888888] uppercase tracking-wide">"UFrame"</label>
                <input
                    type="text"
                    inputmode="numeric"
                    class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[11px] text-white focus:border-[#00d9ff] focus:outline-none text-center"
                    prop:value=move || uframe.get()
                    on:input=move |ev| set_uframe.set(event_target_value(&ev))
                />
            </div>
            <div class="space-y-1">
                <label class="text-[10px] text-[#888888] uppercase tracking-wide">"UTool"</label>
                <input
                    type="text"
                    inputmode="numeric"
                    class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[11px] text-white focus:border-[#00d9ff] focus:outline-none text-center"
                    prop:value=move || utool.get()
                    on:input=move |ev| set_utool.set(event_target_value(&ev))
                />
            </div>
        </div>
    }
}

/// Position input field component
/// Uses type="text" with inputmode="decimal" for better decimal/negative handling
#[component]
fn PositionInput<L, U>(
    label: L,
    value: ReadSignal<String>,
    set_value: WriteSignal<String>,
    unit: U,
) -> impl IntoView
where
    L: Fn() -> &'static str + Send + Sync + 'static,
    U: Fn() -> &'static str + Send + Sync + 'static,
{
    // Validation for numeric input
    let is_valid = move || {
        let v = value.get();
        if v.is_empty() {
            return true;
        }
        if v.parse::<f64>().is_ok() {
            return true;
        }
        // Allow intermediate states like "-" or "." during typing
        v == "-" || v == "." || v == "-."
    };

    view! {
        <div class="space-y-1">
            <label class="text-[9px] text-[#666666] text-center block">{label}</label>
            <div class="flex">
                <input
                    type="text"
                    inputmode="decimal"
                    class=move || format!(
                        "flex-1 min-w-0 bg-[#111111] rounded-l px-1 py-1 text-[10px] text-white focus:outline-none text-center {}",
                        if is_valid() {
                            "border border-[#ffffff08] focus:border-[#00d9ff]"
                        } else {
                            "border-2 border-[#ff4444]"
                        }
                    )
                    prop:value=move || value.get()
                    on:input=move |ev| set_value.set(event_target_value(&ev))
                />
                <span class="bg-[#1a1a1a] border border-l-0 border-[#ffffff08] rounded-r px-1 py-1 text-[8px] text-[#666666]">{unit}</span>
            </div>
        </div>
    }
}

