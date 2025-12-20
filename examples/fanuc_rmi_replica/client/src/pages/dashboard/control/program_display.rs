//! Program visual display - G-code style line-by-line view.
//!
//! This component reads directly from the synced ExecutionState component,
//! following the server-authoritative architecture. The server is the single
//! source of truth for all program execution state.
//!
//! NOTE: Program completion notifications are handled by ProgramNotificationHandler
//! in the layout module, which receives server-broadcast ProgramNotification messages.
//! This ensures all connected clients see the same notification simultaneously.

use leptos::prelude::*;
use pl3xus_client::{use_request, use_sync_component};
use fanuc_replica_types::*;
use super::LoadProgramModal;

/// Program Visual Display - G-code style line-by-line view
///
/// Reads directly from the synced ExecutionState component. All program state
/// (loaded program, lines, running, paused, current line) comes from the server.
#[component]
pub fn ProgramVisualDisplay() -> impl IntoView {
    // Read directly from synced ExecutionState - server is source of truth
    let execution_state = use_sync_component::<ExecutionState>();
    let (show_load_modal, set_show_load_modal) = signal(false);

    // Derive program state from synced ExecutionState
    let loaded_name = Memo::new(move |_| {
        execution_state.get().values().next()
            .and_then(|s| s.loaded_program_name.clone())
    });

    let loaded_id = Memo::new(move |_| {
        execution_state.get().values().next()
            .and_then(|s| s.loaded_program_id)
    });

    let is_running = Memo::new(move |_| {
        execution_state.get().values().next()
            .map(|s| s.running)
            .unwrap_or(false)
    });

    let is_paused = Memo::new(move |_| {
        execution_state.get().values().next()
            .map(|s| s.paused)
            .unwrap_or(false)
    });

    let executing = Memo::new(move |_| {
        execution_state.get().values().next()
            .map(|s| s.current_line as i32)
            .unwrap_or(-1)
    });

    let lines = Memo::new(move |_| {
        execution_state.get().values().next()
            .map(|s| s.program_lines.clone())
            .unwrap_or_default()
    });

    let total_lines = Memo::new(move |_| {
        execution_state.get().values().next()
            .map(|s| s.total_lines)
            .unwrap_or(0)
    });

    // Request hooks for program control - store in StoredValue for use in multiple closures
    let (start_program_fn, _) = use_request::<StartProgram>();
    let (pause_program_fn, _) = use_request::<PauseProgram>();
    let (resume_program_fn, _) = use_request::<ResumeProgram>();
    let (stop_program_fn, _) = use_request::<StopProgram>();
    let (unload_program_fn, _) = use_request::<UnloadProgram>();

    // Store in StoredValue so they can be accessed from multiple closures
    let start_program = StoredValue::new(start_program_fn);
    let pause_program = StoredValue::new(pause_program_fn);
    let resume_program = StoredValue::new(resume_program_fn);
    let stop_program = StoredValue::new(stop_program_fn);
    let unload_program = StoredValue::new(unload_program_fn);

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] flex flex-col overflow-hidden">
            <div class="flex items-center justify-between p-2 border-b border-[#ffffff08] shrink-0">
                <h3 class="text-[10px] font-semibold text-[#00d9ff] uppercase tracking-wide flex items-center">
                    <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/>
                    </svg>
                    {move || loaded_name.get().unwrap_or_else(|| "Program".to_string())}
                </h3>
                <div class="flex items-center gap-1">
                    <span class="text-[8px] text-[#666666] mr-1">
                        {move || format!("{} lines", lines.get().len())}
                    </span>
                    // Load button - show when no program is loaded
                    <Show when=move || loaded_id.get().is_none()>
                        <button
                            class="bg-[#00d9ff20] border border-[#00d9ff40] text-[#00d9ff] text-[8px] px-2 py-0.5 rounded hover:bg-[#00d9ff30]"
                            on:click=move |_| set_show_load_modal.set(true)
                        >
                            "📂 Load"
                        </button>
                    </Show>
                    // Control buttons - only show when program is loaded
                    <Show when=move || loaded_id.get().is_some()>
                        // Run button - show when not running
                        <Show when=move || !is_running.get()>
                            <button
                                class="bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] text-[8px] px-2 py-0.5 rounded hover:bg-[#22c55e30]"
                                on:click=move |_| {
                                    // StartProgram no longer needs program_id - it uses the already loaded program
                                    start_program.with_value(|f| f(StartProgram));
                                }
                            >
                                "▶ Run"
                            </button>
                        </Show>
                        // Pause/Resume button - show when running
                        <Show when=move || is_running.get()>
                            {move || {
                                if is_paused.get() {
                                    view! {
                                        <button
                                            class="bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] text-[8px] px-2 py-0.5 rounded hover:bg-[#22c55e30]"
                                            on:click=move |_| {
                                                resume_program.with_value(|f| f(ResumeProgram));
                                            }
                                        >
                                            "▶ Resume"
                                        </button>
                                    }.into_any()
                                } else {
                                    view! {
                                        <button
                                            class="bg-[#f59e0b20] border border-[#f59e0b40] text-[#f59e0b] text-[8px] px-2 py-0.5 rounded hover:bg-[#f59e0b30]"
                                            on:click=move |_| {
                                                pause_program.with_value(|f| f(PauseProgram));
                                            }
                                        >
                                            "⏸ Pause"
                                        </button>
                                    }.into_any()
                                }
                            }}
                        </Show>
                        // Stop button - show when running
                        <Show when=move || is_running.get()>
                            <button
                                class="bg-[#ff444420] border border-[#ff444440] text-[#ff4444] text-[8px] px-2 py-0.5 rounded hover:bg-[#ff444430]"
                                on:click=move |_| {
                                    stop_program.with_value(|f| f(StopProgram));
                                }
                            >
                                "■ Stop"
                            </button>
                        </Show>
                    </Show>
                    // Unload button - only show when not running
                    <Show when=move || loaded_id.get().is_some() && !is_running.get()>
                        <button
                            class="bg-[#ff444420] border border-[#ff444440] text-[#ff4444] text-[8px] px-2 py-0.5 rounded hover:bg-[#ff444430] flex items-center gap-1"
                            on:click=move |_| {
                                unload_program.with_value(|f| f(UnloadProgram));
                            }
                            title="Unload program"
                        >
                            <svg class="w-2.5 h-2.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1"/>
                            </svg>
                            "Unload"
                        </button>
                    </Show>
                </div>
            </div>
            // Progress bar - show when program is running
            <Show when=move || is_running.get() && (total_lines.get() > 0)>
                <ProgramProgressBar
                    current_line=Signal::derive(move || executing.get().max(0) as usize)
                    total_lines=Signal::derive(move || total_lines.get())
                    is_paused=Signal::derive(move || is_paused.get())
                />
            </Show>
            <ProgramTable
                lines=Signal::derive(move || lines.get())
                executing=Signal::derive(move || executing.get())
            />
        </div>

        // Load Program Modal
        <Show when=move || show_load_modal.get()>
            <LoadProgramModal on_close=move || set_show_load_modal.set(false)/>
        </Show>
    }
}

/// Program table component showing the program lines
///
/// Uses ProgramLineInfo from the shared types crate. Line numbers are derived
/// from the index in the vector (1-based).
#[component]
fn ProgramTable(
    lines: Signal<Vec<ProgramLineInfo>>,
    executing: Signal<i32>,
) -> impl IntoView {
    view! {
        <div class="flex-1 overflow-y-auto">
            <Show
                when=move || !lines.get().is_empty()
                fallback=|| view! {
                    <div class="text-[#555555] text-[9px] text-center py-4 px-2">
                        "No program loaded. Click 'Load' to select a program."
                    </div>
                }
            >
                <table class="w-full text-[9px]">
                    <thead class="sticky top-0 bg-[#0d0d0d]">
                        <tr class="text-[#666666] border-b border-[#ffffff08]">
                            <th class="text-left px-1.5 py-1 w-8">"#"</th>
                            <th class="text-right px-1.5 py-1">"X"</th>
                            <th class="text-right px-1.5 py-1">"Y"</th>
                            <th class="text-right px-1.5 py-1">"Z"</th>
                            <th class="text-right px-1.5 py-1">"W"</th>
                            <th class="text-right px-1.5 py-1">"P"</th>
                            <th class="text-right px-1.5 py-1">"R"</th>
                            <th class="text-right px-1.5 py-1">"Spd"</th>
                            <th class="text-center px-1.5 py-1">"Term"</th>
                        </tr>
                    </thead>
                    <tbody>
                        <For
                            each=move || lines.get().into_iter().enumerate()
                            key=|(i, _)| *i
                            children=move |(i, line)| {
                                // Line numbers are 1-based
                                let line_num = i + 1;
                                let term = line.term_type.clone();
                                view! {
                                    <tr class=move || format!(
                                        "border-b border-[#ffffff05] {}",
                                        if executing.get() == line_num as i32 { "bg-[#00d9ff20] text-[#00d9ff]" } else { "text-[#cccccc]" }
                                    )>
                                        <td class="px-1.5 py-0.5 text-[#555555] font-mono">{line_num}</td>
                                        <td class="px-1.5 py-0.5 text-right font-mono tabular-nums">{format!("{:.2}", line.x)}</td>
                                        <td class="px-1.5 py-0.5 text-right font-mono tabular-nums">{format!("{:.2}", line.y)}</td>
                                        <td class="px-1.5 py-0.5 text-right font-mono tabular-nums">{format!("{:.2}", line.z)}</td>
                                        <td class="px-1.5 py-0.5 text-right font-mono tabular-nums text-[#888888]">{format!("{:.1}", line.w)}</td>
                                        <td class="px-1.5 py-0.5 text-right font-mono tabular-nums text-[#888888]">{format!("{:.1}", line.p)}</td>
                                        <td class="px-1.5 py-0.5 text-right font-mono tabular-nums text-[#888888]">{format!("{:.1}", line.r)}</td>
                                        <td class="px-1.5 py-0.5 text-right font-mono tabular-nums">{format!("{:.0}", line.speed)}</td>
                                        <td class="px-1.5 py-0.5 text-center">{term}</td>
                                    </tr>
                                }
                            }
                        />
                    </tbody>
                </table>
            </Show>
        </div>
    }
}

/// Program progress bar component
#[component]
fn ProgramProgressBar(
    current_line: Signal<usize>,
    total_lines: Signal<usize>,
    is_paused: Signal<bool>,
) -> impl IntoView {
    let progress_percent = move || {
        let total = total_lines.get();
        if total == 0 { 0.0 } else { (current_line.get() as f64 / total as f64) * 100.0 }
    };

    view! {
        <div class="px-2 py-1 border-b border-[#ffffff08] bg-[#0d0d0d]">
            <div class="flex items-center gap-2">
                <span class=move || format!("text-[8px] font-medium uppercase {}",
                    if is_paused.get() { "text-[#f59e0b]" } else { "text-[#22c55e]" }
                )>
                    {move || if is_paused.get() { "paused" } else { "running" }}
                </span>
                <div class="flex-1 h-1.5 bg-[#1a1a1a] rounded-full overflow-hidden">
                    <div
                        class="h-full bg-gradient-to-r from-[#00d9ff] to-[#22c55e] transition-all duration-300"
                        style=move || format!("width: {}%", progress_percent())
                    />
                </div>
                <span class="text-[8px] text-[#666666] font-mono tabular-nums min-w-[60px] text-right">
                    {move || format!("{} / {} ({:.0}%)", current_line.get(), total_lines.get(), progress_percent())}
                </span>
            </div>
        </div>
    }
}

