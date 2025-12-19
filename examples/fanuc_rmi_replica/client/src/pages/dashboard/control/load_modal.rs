//! Load Program Modal - Select and load a program for execution.

use leptos::prelude::*;
use pl3xus_client::use_request;
use fanuc_replica_types::*;
use crate::pages::dashboard::context::{WorkspaceContext, ProgramLine};

/// Load Program Modal - Select and load a program for execution.
#[component]
pub fn LoadProgramModal<F>(
    on_close: F,
) -> impl IntoView
where
    F: Fn() + Clone + 'static,
{
    let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let (list_programs, programs_state) = use_request::<ListPrograms>();
    let (selected_id, set_selected_id) = signal::<Option<i64>>(None);
    let (loading, set_loading) = signal(false);
    let (has_loaded, set_has_loaded) = signal(false);

    // Fetch programs on mount - with guard to prevent infinite loop
    Effect::new(move |_| {
        if !has_loaded.get_untracked() {
            set_has_loaded.set(true);
            list_programs(ListPrograms);
        }
    });

    // Get programs from state
    let programs = Memo::new(move |_| {
        let state = programs_state.get();
        state.data.map(|r| r.programs).unwrap_or_default()
    });

    let is_loading = Memo::new(move |_| {
        programs_state.get().is_loading()
    });

    let on_close_header = on_close.clone();
    let on_close_cancel = on_close.clone();
    let on_close_load = on_close.clone();

    // Load selected program
    let load_program = move |_| {
        if let Some(id) = selected_id.get() {
            set_loading.set(true);
            // Find the program
            if let Some(program) = programs.get().iter().find(|p| p.id == id).cloned() {
                // Convert program lines to ProgramLine format
                let lines: Vec<ProgramLine> = program.lines.iter().enumerate().map(|(i, line)| {
                    ProgramLine {
                        line_number: i + 1,
                        x: line.x,
                        y: line.y,
                        z: line.z,
                        w: line.w,
                        p: line.p,
                        r: line.r,
                        speed: line.speed,
                        term_type: line.term_type.clone(),
                        uframe: line.uframe,
                        utool: line.utool,
                    }
                }).collect();

                ctx.program_lines.set(lines);
                ctx.loaded_program_id.set(Some(id));
                ctx.loaded_program_name.set(Some(program.name));
                ctx.executing_line.set(-1);
                ctx.program_running.set(false);
                ctx.program_paused.set(false);
                on_close_load();
            }
            set_loading.set(false);
        }
    };

    view! {
        <div class="fixed inset-0 bg-black/70 flex items-center justify-center z-50">
            <div class="bg-[#0d0d0d] border border-[#ffffff15] rounded-lg w-[500px] max-h-[80vh] flex flex-col">
                // Header
                <div class="flex items-center justify-between p-3 border-b border-[#ffffff08]">
                    <h2 class="text-sm font-semibold text-white flex items-center gap-2">
                        <svg class="w-4 h-4 text-[#00d9ff]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 19a2 2 0 01-2-2V7a2 2 0 012-2h4l2 2h4a2 2 0 012 2v1M5 19h14a2 2 0 002-2v-5a2 2 0 00-2-2H9a2 2 0 00-2 2v5a2 2 0 01-2 2z"/>
                        </svg>
                        "Load Program"
                    </h2>
                    <button
                        class="text-[#666666] hover:text-white"
                        on:click=move |_| on_close_header()
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                        </svg>
                    </button>
                </div>

                // Content
                <div class="flex-1 overflow-y-auto p-3">
                    <Show when=move || is_loading.get()>
                        <div class="text-center py-8 text-[#666666] text-sm">
                            "Loading programs..."
                        </div>
                    </Show>
                    <Show when=move || !is_loading.get() && programs.get().is_empty()>
                        <div class="text-center py-8 text-[#666666] text-sm">
                            "No programs available. Create a program in the Programs page."
                        </div>
                    </Show>
                    <Show when=move || !is_loading.get() && !programs.get().is_empty()>
                        <div class="space-y-1">
                            {move || programs.get().into_iter().map(|program| {
                                let id = program.id;
                                let name = program.name.clone();
                                let description = program.description.clone();
                                let line_count = program.lines.len();
                                let has_description = description.is_some();
                                let description_text = description.unwrap_or_default();
                                let is_selected = move || selected_id.get() == Some(id);
                                view! {
                                    <button
                                        class=move || format!(
                                            "w-full text-left p-2 rounded border transition-colors {}",
                                            if is_selected() {
                                                "bg-[#00d9ff20] border-[#00d9ff] text-white"
                                            } else {
                                                "bg-[#111111] border-[#ffffff08] text-[#cccccc] hover:bg-[#1a1a1a] hover:border-[#ffffff15]"
                                            }
                                        )
                                        on:click=move |_| set_selected_id.set(Some(id))
                                    >
                                        <div class="flex items-center justify-between">
                                            <span class="text-[11px] font-medium">{name.clone()}</span>
                                            <span class="text-[9px] text-[#666666]">
                                                {format!("{} lines", line_count)}
                                            </span>
                                        </div>
                                        <Show when=move || has_description>
                                            <div class="text-[9px] text-[#888888] mt-0.5">
                                                {description_text.clone()}
                                            </div>
                                        </Show>
                                    </button>
                                }
                            }).collect_view()}
                        </div>
                    </Show>
                </div>

                // Footer
                <div class="flex justify-end gap-2 p-3 border-t border-[#ffffff08]">
                    <button
                        class="bg-[#1a1a1a] border border-[#ffffff15] text-[#cccccc] text-[10px] px-4 py-1.5 rounded hover:bg-[#222222]"
                        on:click=move |_| on_close_cancel()
                    >
                        "Cancel"
                    </button>
                    <button
                        class=move || format!(
                            "text-[10px] px-4 py-1.5 rounded transition-colors {}",
                            if selected_id.get().is_some() && !loading.get() {
                                "bg-[#00d9ff] text-black hover:bg-[#00c4e6]"
                            } else {
                                "bg-[#333333] text-[#666666] cursor-not-allowed"
                            }
                        )
                        disabled=move || selected_id.get().is_none() || loading.get()
                        on:click=load_program
                    >
                        {move || if loading.get() { "Loading..." } else { "Load Program" }}
                    </button>
                </div>
            </div>
        </div>
    }
}

