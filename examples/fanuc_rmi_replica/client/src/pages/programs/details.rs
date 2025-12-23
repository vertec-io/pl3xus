//! Program details panel component.

use leptos::prelude::*;
use leptos::either::Either;
use pl3xus_client::use_mutation;
use fanuc_replica_types::{ProgramDetail, UpdateProgramSettings};
use crate::components::use_toast;

/// Program details panel
#[component]
pub fn ProgramDetails(
    current_program: RwSignal<Option<ProgramDetail>>,
    #[allow(unused_variables)]
    #[prop(into)] selected_program_id: Signal<Option<i64>>,
    on_select: impl Fn(Option<i64>) + 'static + Clone + Send,
    set_show_csv_upload: WriteSignal<bool>,
    set_show_open_modal: WriteSignal<bool>,
    set_show_new_program: WriteSignal<bool>,
) -> impl IntoView {
    // Editable position signals - X, Y, Z, W, P, R for start and end
    let (start_x, set_start_x) = signal(String::new());
    let (start_y, set_start_y) = signal(String::new());
    let (start_z, set_start_z) = signal(String::new());
    let (start_w, set_start_w) = signal(String::new());
    let (start_p, set_start_p) = signal(String::new());
    let (start_r, set_start_r) = signal(String::new());
    let (end_x, set_end_x) = signal(String::new());
    let (end_y, set_end_y) = signal(String::new());
    let (end_z, set_end_z) = signal(String::new());
    let (end_w, set_end_w) = signal(String::new());
    let (end_p, set_end_p) = signal(String::new());
    let (end_r, set_end_r) = signal(String::new());
    let (move_speed, set_move_speed) = signal(String::new());
    // Termination settings - empty until loaded from server (never show fake defaults)
    let (term_type, set_term_type) = signal(String::new());
    let (term_value, set_term_value) = signal(String::new());
    let (settings_modified, set_settings_modified) = signal(false);

    // Track current program ID and instruction count to detect changes
    let (current_prog_id, set_current_prog_id) = signal::<Option<i64>>(None);
    let (current_inst_count, set_current_inst_count) = signal::<usize>(0);

    // Toast context for validation errors
    let toast = use_toast();

    // Mutation for updating settings with error handling
    let update_settings = use_mutation::<UpdateProgramSettings>(move |result| {
        match result {
            Ok(r) if r.success => toast.success("Settings saved"),
            Ok(r) => toast.error(format!("Save failed: {}", r.error.as_deref().unwrap_or(""))),
            Err(e) => toast.error(format!("Error: {e}")),
        }
    });

    // Sync signals when program changes or is re-fetched with new data
    Effect::new(move |_| {
        leptos::logging::log!("[ProgramDetails] Effect running, checking current_program");
        if let Some(prog) = current_program.get() {
            let id_changed = current_prog_id.get() != Some(prog.id);
            let inst_count_changed = current_inst_count.get() != prog.instructions.len();
            leptos::logging::log!(
                "[ProgramDetails] Program: {} (id={}), instructions: {}, id_changed: {}, inst_count_changed: {}",
                prog.name, prog.id, prog.instructions.len(), id_changed, inst_count_changed
            );

            // Update when ID changes OR when instruction count changes (i.e. after CSV upload)
            // but only if settings haven't been modified by user
            if id_changed || (inst_count_changed && !settings_modified.get()) {
                leptos::logging::log!("[ProgramDetails] Updating local signals");
                set_current_prog_id.set(Some(prog.id));
                set_current_inst_count.set(prog.instructions.len());
                set_start_x.set(prog.start_x.map(|v| format!("{:.2}", v)).unwrap_or_default());
                set_start_y.set(prog.start_y.map(|v| format!("{:.2}", v)).unwrap_or_default());
                set_start_z.set(prog.start_z.map(|v| format!("{:.2}", v)).unwrap_or_default());
                set_start_w.set(prog.start_w.map(|v| format!("{:.2}", v)).unwrap_or_default());
                set_start_p.set(prog.start_p.map(|v| format!("{:.2}", v)).unwrap_or_default());
                set_start_r.set(prog.start_r.map(|v| format!("{:.2}", v)).unwrap_or_default());
                set_end_x.set(prog.end_x.map(|v| format!("{:.2}", v)).unwrap_or_default());
                set_end_y.set(prog.end_y.map(|v| format!("{:.2}", v)).unwrap_or_default());
                set_end_z.set(prog.end_z.map(|v| format!("{:.2}", v)).unwrap_or_default());
                set_end_w.set(prog.end_w.map(|v| format!("{:.2}", v)).unwrap_or_default());
                set_end_p.set(prog.end_p.map(|v| format!("{:.2}", v)).unwrap_or_default());
                set_end_r.set(prog.end_r.map(|v| format!("{:.2}", v)).unwrap_or_default());
                // Required fields come directly from DB - no fallbacks, values are always present
                set_move_speed.set(format!("{:.0}", prog.move_speed));
                set_term_type.set(prog.default_term_type.clone());
                set_term_value.set(prog.default_term_value.to_string());
                set_settings_modified.set(false);
            }
        } else {
            leptos::logging::log!("[ProgramDetails] No current_program");
        }
    });

    view! {
        <div class="flex-1 bg-[#0a0a0a] rounded border border-[#ffffff08] flex flex-col overflow-hidden">
            {move || {
                if let Some(prog) = current_program.get() {
                    let prog_id = prog.id;
                    let prog_name = prog.name.clone();
                    let prog_desc = prog.description.clone().unwrap_or_default();
                    let line_count = prog.instructions.len();
                    let instructions_for_table = prog.instructions.clone();

                    Either::Left(view! {
                        <div class="h-full flex flex-col">
                            // Header
                            <div class="p-3 border-b border-[#ffffff08]">
                                <div class="flex items-start justify-between">
                                    <div>
                                        <h2 class="text-sm font-semibold text-white">{prog_name}</h2>
                                        <p class="text-[#666666] text-[9px] mt-0.5">{prog_desc}</p>
                                    </div>
                                    <div class="flex gap-1">
                                        <button
                                            class="bg-[#00d9ff20] border border-[#00d9ff40] text-[#00d9ff] text-[9px] px-2 py-1 rounded hover:bg-[#00d9ff30]"
                                            on:click=move |_| set_show_csv_upload.set(true)
                                        >
                                            "⬆ Upload CSV"
                                        </button>
                                        <button
                                            class="bg-[#ff444420] border border-[#ff444440] text-[#ff4444] text-[9px] px-2 py-1 rounded hover:bg-[#ff444430]"
                                            on:click={
                                                let on_select = on_select.clone();
                                                move |_| {
                                                    on_select(None);
                                                    current_program.set(None);
                                                }
                                            }
                                        >
                                            "Close"
                                        </button>
                                    </div>
                                </div>
                            </div>

                            // Metadata
                            <div class="px-3 pt-3 pb-2">
                                <div>
                                    <div class="text-[8px] text-[#555555] uppercase">"Instructions"</div>
                                    <div class="text-[11px] text-white font-mono">{line_count}" lines"</div>
                                </div>
                            </div>

                            // Start Position
                            <PositionInputRow
                                label="Start Position"
                                hint="(approach before toolpath)"
                                x=start_x set_x=set_start_x
                                y=start_y set_y=set_start_y
                                z=start_z set_z=set_start_z
                                w=start_w set_w=set_start_w
                                p=start_p set_p=set_start_p
                                r=start_r set_r=set_start_r
                                set_modified=set_settings_modified
                            />

                            // End Position
                            <PositionInputRow
                                label="End Position"
                                hint="(retreat after toolpath)"
                                x=end_x set_x=set_end_x
                                y=end_y set_y=set_end_y
                                z=end_z set_z=set_end_z
                                w=end_w set_w=set_end_w
                                p=end_p set_p=set_end_p
                                r=end_r set_r=set_end_r
                                set_modified=set_settings_modified
                            />

                            // Motion Settings Row
                            <div class="px-3 pb-3 border-b border-[#ffffff08] flex items-end gap-3 flex-wrap">
                                <div>
                                    <div class="flex items-center gap-2 mb-1">
                                        <div class="text-[8px] text-[#555555] uppercase">"Move Speed"</div>
                                        <div class="text-[7px] text-[#444444]">"(mm/s)"</div>
                                    </div>
                                    <input
                                        type="text"
                                        class="w-20 bg-[#111111] border border-[#ffffff10] rounded px-2 py-1 text-[10px] text-white font-mono"
                                        placeholder="100"
                                        prop:value=move || move_speed.get()
                                        on:input=move |ev| {
                                            set_move_speed.set(event_target_value(&ev));
                                            set_settings_modified.set(true);
                                        }
                                    />
                                </div>
                                <div>
                                    <div class="text-[8px] text-[#555555] uppercase mb-1">"Term Type"</div>
                                    <select
                                        class="w-20 bg-[#111111] border border-[#ffffff10] rounded px-2 py-1 text-[10px] text-white"
                                        prop:value=move || term_type.get()
                                        on:change=move |ev| {
                                            set_term_type.set(event_target_value(&ev));
                                            set_settings_modified.set(true);
                                        }
                                    >
                                        <option value="CNT">"CNT"</option>
                                        <option value="FINE">"FINE"</option>
                                    </select>
                                </div>
                                <div>
                                    <div class="flex items-center gap-2 mb-1">
                                        <div class="text-[8px] text-[#555555] uppercase">"Term Value"</div>
                                        <div class="text-[7px] text-[#444444]">"(0-100)"</div>
                                    </div>
                                    <input
                                        type="text"
                                        class="w-16 bg-[#111111] border border-[#ffffff10] rounded px-2 py-1 text-[10px] text-white font-mono"
                                        placeholder="100"
                                        prop:value=move || term_value.get()
                                        on:input=move |ev| {
                                            set_term_value.set(event_target_value(&ev));
                                            set_settings_modified.set(true);
                                        }
                                    />
                                </div>
                                <Show when=move || settings_modified.get()>
                                    <button
                                        class="bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] text-[9px] px-3 py-1 rounded hover:bg-[#22c55e30]"
                                        on:click=move |_| {
                                            // Validate required fields before saving
                                            let move_speed_val = move_speed.get();
                                            let term_type_val = term_type.get();
                                            let term_value_val = term_value.get();

                                            // Validate move_speed (required, must be positive number)
                                            let parsed_move_speed: Option<f64> = move_speed_val.parse().ok();
                                            if parsed_move_speed.is_none() || parsed_move_speed.unwrap() <= 0.0 {
                                                toast.error("Move Speed is required and must be a positive number");
                                                return;
                                            }

                                            // Validate term_type (required, must be CNT or FINE)
                                            if term_type_val != "CNT" && term_type_val != "FINE" {
                                                toast.error("Term Type must be CNT or FINE");
                                                return;
                                            }

                                            // Validate term_value (required, must be 0-100)
                                            let parsed_term_value: Option<u8> = term_value_val.parse().ok();
                                            if parsed_term_value.is_none() || parsed_term_value.unwrap() > 100 {
                                                toast.error("Term Value is required and must be 0-100");
                                                return;
                                            }

                                            update_settings.send(UpdateProgramSettings {
                                                program_id: prog_id,
                                                start_x: start_x.get().parse().ok(),
                                                start_y: start_y.get().parse().ok(),
                                                start_z: start_z.get().parse().ok(),
                                                start_w: start_w.get().parse().ok(),
                                                start_p: start_p.get().parse().ok(),
                                                start_r: start_r.get().parse().ok(),
                                                end_x: end_x.get().parse().ok(),
                                                end_y: end_y.get().parse().ok(),
                                                end_z: end_z.get().parse().ok(),
                                                end_w: end_w.get().parse().ok(),
                                                end_p: end_p.get().parse().ok(),
                                                end_r: end_r.get().parse().ok(),
                                                move_speed: parsed_move_speed,
                                                default_term_type: Some(term_type_val),
                                                default_term_value: parsed_term_value,
                                            });
                                            set_settings_modified.set(false);
                                        }
                                    >
                                        "Save Settings"
                                    </button>
                                </Show>
                            </div>

                            // Instructions table
                            <div class="flex-1 p-3 overflow-auto">
                                <h4 class="text-[9px] text-[#666666] uppercase mb-2">"Program Instructions"</h4>
                                <InstructionsTable instructions=instructions_for_table />
                            </div>
                        </div>
                    })
                } else {
                    Either::Right(view! {
                        <EmptyProgramState
                            set_show_open_modal=set_show_open_modal
                            set_show_new_program=set_show_new_program
                        />
                    })
                }
            }}
        </div>
    }
}

/// Position input row component (6 inputs: X, Y, Z, W, P, R)
#[component]
fn PositionInputRow(
    label: &'static str,
    hint: &'static str,
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
    set_modified: WriteSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="px-3 pb-2">
            <div class="flex items-center gap-2 mb-1">
                <div class="text-[8px] text-[#555555] uppercase">{label}</div>
                <div class="text-[7px] text-[#444444]">{hint}</div>
            </div>
            <div class="grid grid-cols-6 gap-2">
                <PositionInput label="X" value=x set_value=set_x set_modified=set_modified />
                <PositionInput label="Y" value=y set_value=set_y set_modified=set_modified />
                <PositionInput label="Z" value=z set_value=set_z set_modified=set_modified />
                <PositionInput label="W" value=w set_value=set_w set_modified=set_modified />
                <PositionInput label="P" value=p set_value=set_p set_modified=set_modified />
                <PositionInput label="R" value=r set_value=set_r set_modified=set_modified />
            </div>
        </div>
    }
}

/// Single position input
#[component]
fn PositionInput(
    label: &'static str,
    value: ReadSignal<String>,
    set_value: WriteSignal<String>,
    set_modified: WriteSignal<bool>,
) -> impl IntoView {
    view! {
        <div>
            <label class="text-[7px] text-[#444444]">{label}</label>
            <input
                type="text"
                class="w-full bg-[#111111] border border-[#ffffff10] rounded px-2 py-1 text-[10px] text-white font-mono"
                placeholder=label
                prop:value=move || value.get()
                on:input=move |ev| {
                    set_value.set(event_target_value(&ev));
                    set_modified.set(true);
                }
            />
        </div>
    }
}

/// Instructions table component
#[component]
fn InstructionsTable(instructions: Vec<fanuc_replica_types::Instruction>) -> impl IntoView {
    if instructions.is_empty() {
        Either::Left(view! {
            <div class="bg-[#111111] rounded border border-[#ffffff08] p-4 text-center text-[#555555] text-[10px]">
                "No instructions - upload a CSV to add instructions"
            </div>
        })
    } else {
        Either::Right(view! {
            <div class="bg-[#111111] rounded border border-[#ffffff08] overflow-auto max-h-[400px]">
                <table class="w-full text-[9px] font-mono">
                    <thead class="bg-[#1a1a1a] sticky top-0">
                        <tr class="text-[#666666] text-left">
                            <th class="px-2 py-1.5 font-medium">"#"</th>
                            <th class="px-2 py-1.5 font-medium">"X"</th>
                            <th class="px-2 py-1.5 font-medium">"Y"</th>
                            <th class="px-2 py-1.5 font-medium">"Z"</th>
                            <th class="px-2 py-1.5 font-medium">"W"</th>
                            <th class="px-2 py-1.5 font-medium">"P"</th>
                            <th class="px-2 py-1.5 font-medium">"R"</th>
                            <th class="px-2 py-1.5 font-medium">"Speed"</th>
                            <th class="px-2 py-1.5 font-medium">"Term"</th>
                            <th class="px-2 py-1.5 font-medium">"UFrame"</th>
                            <th class="px-2 py-1.5 font-medium">"UTool"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {instructions.into_iter().map(|instr| {
                            let w_str = instr.w.map(|v| format!("{:.2}", v)).unwrap_or_else(|| "-".to_string());
                            let p_str = instr.p.map(|v| format!("{:.2}", v)).unwrap_or_else(|| "-".to_string());
                            let r_str = instr.r.map(|v| format!("{:.2}", v)).unwrap_or_else(|| "-".to_string());
                            let speed_str = instr.speed.map(|v| format!("{:.0}", v)).unwrap_or_else(|| "-".to_string());
                            let term_str = instr.term_type.clone().unwrap_or_else(|| "-".to_string());
                            let uframe_str = instr.uframe.map(|v| v.to_string()).unwrap_or_else(|| "-".to_string());
                            let utool_str = instr.utool.map(|v| v.to_string()).unwrap_or_else(|| "-".to_string());
                            view! {
                                <tr class="border-t border-[#ffffff08] hover:bg-[#ffffff05]">
                                    <td class="px-2 py-1 text-[#00d9ff]">{instr.line_number}</td>
                                    <td class="px-2 py-1 text-white">{format!("{:.2}", instr.x)}</td>
                                    <td class="px-2 py-1 text-white">{format!("{:.2}", instr.y)}</td>
                                    <td class="px-2 py-1 text-white">{format!("{:.2}", instr.z)}</td>
                                    <td class="px-2 py-1 text-[#888888]">{w_str}</td>
                                    <td class="px-2 py-1 text-[#888888]">{p_str}</td>
                                    <td class="px-2 py-1 text-[#888888]">{r_str}</td>
                                    <td class="px-2 py-1 text-[#22c55e]">{speed_str}</td>
                                    <td class="px-2 py-1 text-[#888888]">{term_str}</td>
                                    <td class="px-2 py-1 text-[#888888]">{uframe_str}</td>
                                    <td class="px-2 py-1 text-[#888888]">{utool_str}</td>
                                </tr>
                            }
                        }).collect_view()}
                    </tbody>
                </table>
            </div>
        })
    }
}

/// Empty program state component
#[component]
fn EmptyProgramState(
    set_show_open_modal: WriteSignal<bool>,
    set_show_new_program: WriteSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="h-full flex items-center justify-center">
            <div class="text-center">
                <svg class="w-12 h-12 mx-auto mb-2 text-[#333333]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"/>
                </svg>
                <p class="text-[#555555] text-[10px] mb-3">"No program open"</p>
                <div class="flex gap-2 justify-center">
                    <button
                        class="bg-[#00d9ff20] border border-[#00d9ff40] text-[#00d9ff] text-[9px] px-3 py-1.5 rounded hover:bg-[#00d9ff30]"
                        on:click=move |_| set_show_open_modal.set(true)
                    >
                        "Open Program"
                    </button>
                    <button
                        class="bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] text-[9px] px-3 py-1.5 rounded hover:bg-[#22c55e30]"
                        on:click=move |_| set_show_new_program.set(true)
                    >
                        "New Program"
                    </button>
                </div>
            </div>
        </div>
    }
}
