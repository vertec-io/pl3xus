//! Program browser sidebar component.

use leptos::prelude::*;
use leptos::either::Either;
use fanuc_replica_types::ProgramWithLines;
use crate::layout::LayoutContext;

/// Program browser sidebar
#[component]
pub fn ProgramBrowser(
    programs: Memo<Vec<ProgramWithLines>>,
    selected_program_id: ReadSignal<Option<i64>>,
    set_selected_program_id: WriteSignal<Option<i64>>,
) -> impl IntoView {
    let layout_ctx = use_context::<LayoutContext>().expect("LayoutContext not found");

    view! {
        <div class="w-64 bg-[#0a0a0a] rounded border border-[#ffffff08] flex flex-col overflow-hidden shrink-0">
            <div class="flex items-center justify-between p-2 border-b border-[#ffffff08]">
                <h3 class="text-[10px] font-semibold text-[#00d9ff] uppercase tracking-wide flex items-center">
                    <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/>
                    </svg>
                    "Programs"
                </h3>
                <button
                    class="text-[#666666] hover:text-white"
                    on:click=move |_| layout_ctx.show_program_browser.set(false)
                    title="Close browser"
                >
                    <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                    </svg>
                </button>
            </div>
            <div class="flex-1 overflow-y-auto p-1.5 space-y-1">
                {move || {
                    let progs = programs.get();
                    if progs.is_empty() {
                        Either::Left(view! {
                            <div class="text-[#555555] text-[9px] text-center py-4">
                                "No programs saved"
                            </div>
                        })
                    } else {
                        Either::Right(progs.into_iter().map(|prog| {
                            let is_selected = move || selected_program_id.get() == Some(prog.id);
                            let prog_id = prog.id;
                            let prog_name = prog.name.clone();
                            let lines_str = format!("{} lines", prog.lines.len());
                            view! {
                                <button
                                    class={move || format!(
                                        "w-full text-left p-2 rounded border text-[9px] transition-colors {}",
                                        if is_selected() {
                                            "bg-[#00d9ff10] border-[#00d9ff40] text-white"
                                        } else {
                                            "bg-[#111111] border-[#ffffff08] text-[#888888] hover:border-[#ffffff20]"
                                        }
                                    )}
                                    on:click=move |_| set_selected_program_id.set(Some(prog_id))
                                >
                                    <div class="font-medium text-[10px] mb-0.5">{prog_name}</div>
                                    <div class="text-[#555555]">{lines_str}</div>
                                </button>
                            }
                        }).collect_view())
                    }
                }}
            </div>
        </div>
    }
}

