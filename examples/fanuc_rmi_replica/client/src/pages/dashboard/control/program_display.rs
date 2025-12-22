//! Program visual display - G-code style line-by-line view.
//!
//! This component reads directly from the synced ExecutionState component,
//! following the server-authoritative architecture. The server is the single
//! source of truth for all program execution state.
//!
//! ## Server-Driven UI State Pattern
//!
//! This component demonstrates the idiomatic pl3xus pattern for server-driven UI:
//! - The server provides `state` enum and `can_*` action flags in ExecutionState
//! - The client simply reflects these values - **no client-side state machine logic**
//! - Button visibility is driven by server-provided `can_*` flags
//! - Actions are only allowed when the client has control
//!
//! ## Response Handling
//!
//! All program commands use targeted requests which return responses.
//! The client shows toast notifications based on server responses.
//! The UI never lies to the user - success is only shown when the server confirms.
//!
//! NOTE: Program completion notifications are handled by ProgramNotificationHandler
//! in the layout module, which receives server-broadcast ProgramNotification messages.
//! This ensures all connected clients see the same notification simultaneously.

use leptos::prelude::*;
use pl3xus_client::{use_mutation_targeted, use_components, use_sync_context, EntityControl};
use fanuc_replica_types::*;
use super::LoadProgramModal;
use crate::components::use_toast;
use crate::pages::dashboard::use_system_entity;

/// Program Visual Display - G-code style line-by-line view
///
/// Demonstrates the **server-driven UI state pattern**:
/// - The server's `ExecutionState` contains both current state and available actions
/// - Button visibility is driven by server-provided `can_*` flags
/// - Actions require control - clients without control see disabled buttons
/// - Zero client-side state machine logic
#[component]
pub fn ProgramVisualDisplay() -> impl IntoView {
    let ctx = use_sync_context();
    let toast = use_toast();
    let system_ctx = use_system_entity();

    // === Server-Driven State ===
    //
    // We subscribe to the full component signals and extract the first entity.
    // Using use_components (returns ReadSignal) for reliable reactivity.
    let all_exec = use_components::<ExecutionState>();
    let all_control = use_components::<EntityControl>();

    let (show_load_modal, set_show_load_modal) = signal(false);

    // Get the System entity ID for control checks
    // Note: Program commands use the request/response pattern, not targeted messages.
    // Authorization is handled in the server request handlers.
    let system_entity_bits = move || -> Option<u64> {
        system_ctx.system_entity_id.get()
    };

    // Check if THIS client has control of the System entity
    let has_control = move || {
        let my_id = ctx.my_connection_id.get();
        system_entity_bits()
            .and_then(|sys_entity| all_control.get().get(&sys_entity).cloned())
            .map(|c| Some(c.client_id) == my_id)
            .unwrap_or(false)
    };

    // Helper to get the first ExecutionState
    let get_exec = move || {
        all_exec.get().values().next().cloned().unwrap_or_default()
    };

    // === Derived State ===
    let loaded_name = move || get_exec().loaded_program_name;
    let exec_state = move || get_exec().state;
    let is_paused = move || exec_state() == ProgramExecutionState::Paused;
    let is_active = move || matches!(exec_state(), ProgramExecutionState::Running | ProgramExecutionState::Paused);
    let executing = move || get_exec().current_line as i32;
    let lines = move || get_exec().program_lines;
    let total_lines = move || get_exec().total_lines;

    // === Available Actions (server-driven + control check) ===
    // The server tells us what actions are valid, but we also require control.
    let can_load = move || {
        let exec = get_exec();
        let result = has_control() && exec.can_load;
        leptos::logging::log!("[ProgramDisplay] can_load={} (has_control={}, exec.can_load={})", result, has_control(), exec.can_load);
        result
    };
    let can_start = move || {
        let exec = get_exec();
        let result = has_control() && exec.can_start;
        leptos::logging::log!("[ProgramDisplay] can_start={} (has_control={}, exec.can_start={})", result, has_control(), exec.can_start);
        result
    };
    let can_pause = move || has_control() && get_exec().can_pause;
    let can_resume = move || has_control() && get_exec().can_resume;
    let can_stop = move || has_control() && get_exec().can_stop;
    let can_unload = move || {
        let exec = get_exec();
        let result = has_control() && exec.can_unload;
        leptos::logging::log!("[ProgramDisplay] can_unload={} (has_control={}, exec.can_unload={})", result, has_control(), exec.can_unload);
        result
    };

    // =========================================================================
    // Targeted Mutation Hooks (TanStack Query-inspired API)
    //
    // use_mutation_targeted handles response deduplication internally
    // - The handler is called exactly once per response
    // - Returns a MutationHandle with .send() and state accessors
    // - Success cases: no toast (ExecutionState sync updates the UI)
    // - Error cases: show error toast with reason
    // =========================================================================

    let start = use_mutation_targeted::<StartProgram>(move |result| {
        match result {
            Ok(r) if r.success => { /* Success: no toast, UI updates via ExecutionState sync */ }
            Ok(r) => toast.error(format!("Start denied: {}", r.error.as_deref().unwrap_or("No control"))),
            Err(e) => toast.error(format!("Start failed: {e}")),
        }
    });

    let pause = use_mutation_targeted::<PauseProgram>(move |result| {
        match result {
            Ok(r) if r.success => { /* Success: no toast */ }
            Ok(r) => toast.error(format!("Pause denied: {}", r.error.as_deref().unwrap_or("No control"))),
            Err(e) => toast.error(format!("Pause failed: {e}")),
        }
    });

    let resume = use_mutation_targeted::<ResumeProgram>(move |result| {
        match result {
            Ok(r) if r.success => { /* Success: no toast */ }
            Ok(r) => toast.error(format!("Resume denied: {}", r.error.as_deref().unwrap_or("No control"))),
            Err(e) => toast.error(format!("Resume failed: {e}")),
        }
    });

    let stop = use_mutation_targeted::<StopProgram>(move |result| {
        match result {
            Ok(r) if r.success => { /* Success: no toast */ }
            Ok(r) => toast.error(format!("Stop denied: {}", r.error.as_deref().unwrap_or("No control"))),
            Err(e) => toast.error(format!("Stop failed: {e}")),
        }
    });

    let unload = use_mutation_targeted::<UnloadProgram>(move |result| {
        match result {
            Ok(r) if r.success => { /* Success: no toast */ }
            Ok(r) => toast.error(format!("Unload denied: {}", r.error.as_deref().unwrap_or("No control"))),
            Err(e) => toast.error(format!("Unload failed: {e}")),
        }
    });

    // Get the system entity ID for targeting
    let system_entity_id = system_ctx.system_entity_id;

    // Note: TargetedMutationHandle is Copy, so it can be used directly in closures

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] flex flex-col overflow-hidden">
            <div class="flex items-center justify-between p-2 border-b border-[#ffffff08] shrink-0">
                <h3 class="text-[10px] font-semibold text-[#00d9ff] uppercase tracking-wide flex items-center">
                    <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/>
                    </svg>
                    {move || loaded_name().unwrap_or_else(|| "Program".to_string())}
                </h3>
                // === Server-Driven Action Buttons ===
                // Button visibility is determined entirely by server's can_* flags.
                <div class="flex items-center gap-1">
                    <span class="text-[8px] text-[#666666] mr-1">
                        {move || format!("{} lines", lines().len())}
                    </span>
                    // Load button - server tells us when loading is available
                    <Show when=can_load>
                        <button
                            class="bg-[#00d9ff20] border border-[#00d9ff40] text-[#00d9ff] text-[8px] px-2 py-0.5 rounded hover:bg-[#00d9ff30]"
                            on:click=move |_| set_show_load_modal.set(true)
                        >
                            "📂 Load"
                        </button>
                    </Show>
                    // Run button - server tells us when starting is available
                    <Show when=move || can_start()>
                        <button
                            class="bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] text-[8px] px-2 py-0.5 rounded hover:bg-[#22c55e30]"
                            on:click=move |_| {
                                if let Some(entity_id) = system_entity_id.get() {
                                    start.send(entity_id, StartProgram);
                                }
                            }
                        >
                            "▶ Run"
                        </button>
                    </Show>
                    // Pause button - server tells us when pausing is available
                    <Show when=move || can_pause()>
                        <button
                            class="bg-[#f59e0b20] border border-[#f59e0b40] text-[#f59e0b] text-[8px] px-2 py-0.5 rounded hover:bg-[#f59e0b30]"
                            on:click=move |_| {
                                if let Some(entity_id) = system_entity_id.get() {
                                    pause.send(entity_id, PauseProgram);
                                }
                            }
                        >
                            "⏸ Pause"
                        </button>
                    </Show>
                    // Resume button - server tells us when resuming is available
                    <Show when=move || can_resume()>
                        <button
                            class="bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] text-[8px] px-2 py-0.5 rounded hover:bg-[#22c55e30]"
                            on:click=move |_| {
                                if let Some(entity_id) = system_entity_id.get() {
                                    resume.send(entity_id, ResumeProgram);
                                }
                            }
                        >
                            "▶ Resume"
                        </button>
                    </Show>
                    // Stop button - server tells us when stopping is available
                    <Show when=move || can_stop()>
                        <button
                            class="bg-[#ff444420] border border-[#ff444440] text-[#ff4444] text-[8px] px-2 py-0.5 rounded hover:bg-[#ff444430]"
                            on:click=move |_| {
                                if let Some(entity_id) = system_entity_id.get() {
                                    stop.send(entity_id, StopProgram);
                                }
                            }
                        >
                            "■ Stop"
                        </button>
                    </Show>
                    // Unload button - server tells us when unloading is available
                    <Show when=move || can_unload()>
                        <button
                            class="bg-[#ff444420] border border-[#ff444440] text-[#ff4444] text-[8px] px-2 py-0.5 rounded hover:bg-[#ff444430] flex items-center gap-1"
                            on:click=move |_| {
                                if let Some(entity_id) = system_entity_id.get() {
                                    unload.send(entity_id, UnloadProgram);
                                }
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
            // Progress bar - show when program is active (running or paused)
            <Show when=move || is_active() && (total_lines() > 0)>
                <ProgramProgressBar
                    current_line=Signal::derive(move || executing().max(0) as usize)
                    total_lines=Signal::derive(move || total_lines())
                    is_paused=Signal::derive(move || is_paused())
                />
            </Show>
            <ProgramTable
                lines=Signal::derive(move || lines())
                executing=Signal::derive(move || executing())
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

