//! Program details panel component.

use leptos::prelude::*;
use leptos::either::Either;
use fanuc_replica_types::ProgramWithLines;

/// Program details panel
#[component]
pub fn ProgramDetails(
    current_program: RwSignal<Option<ProgramWithLines>>,
    #[allow(unused_variables)]
    selected_program_id: ReadSignal<Option<i64>>,
    #[allow(unused_variables)]
    set_selected_program_id: WriteSignal<Option<i64>>,
    set_show_csv_upload: WriteSignal<bool>,
    set_show_open_modal: WriteSignal<bool>,
    set_show_new_program: WriteSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="flex-1 bg-[#0a0a0a] rounded border border-[#ffffff08] flex flex-col overflow-hidden">
            {move || {
                if let Some(prog) = current_program.get() {
                    let _prog_id = prog.id;
                    let prog_name = prog.name.clone();
                    let prog_desc = prog.description.clone().unwrap_or_default();
                    let line_count = prog.lines.len();

                    // Clone instructions for the table display
                    let instructions_for_table = prog.lines.clone();

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
                                            on:click=move |_| {
                                                set_selected_program_id.set(None);
                                                current_program.set(None);
                                            }
                                        >
                                            "Delete"
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

                            // Position inputs and instructions table will be added in next edit
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

/// Instructions table component
#[component]
fn InstructionsTable(instructions: Vec<fanuc_replica_types::ProgramLineInfo>) -> impl IntoView {
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
                        {instructions.into_iter().enumerate().map(|(idx, instr)| {
                            let uframe_str = instr.uframe.map(|v| v.to_string()).unwrap_or_else(|| "-".to_string());
                            let utool_str = instr.utool.map(|v| v.to_string()).unwrap_or_else(|| "-".to_string());
                            view! {
                                <tr class="border-t border-[#ffffff08] hover:bg-[#ffffff05]">
                                    <td class="px-2 py-1 text-[#00d9ff]">{idx + 1}</td>
                                    <td class="px-2 py-1 text-white">{format!("{:.2}", instr.x)}</td>
                                    <td class="px-2 py-1 text-white">{format!("{:.2}", instr.y)}</td>
                                    <td class="px-2 py-1 text-white">{format!("{:.2}", instr.z)}</td>
                                    <td class="px-2 py-1 text-[#888888]">{format!("{:.2}", instr.w)}</td>
                                    <td class="px-2 py-1 text-[#888888]">{format!("{:.2}", instr.p)}</td>
                                    <td class="px-2 py-1 text-[#888888]">{format!("{:.2}", instr.r)}</td>
                                    <td class="px-2 py-1 text-[#22c55e]">{format!("{:.0}", instr.speed)}</td>
                                    <td class="px-2 py-1 text-[#888888]">{instr.term_type.clone()}</td>
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
