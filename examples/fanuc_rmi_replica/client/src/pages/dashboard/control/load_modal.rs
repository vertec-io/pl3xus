//! Load Program Modal - Select and load a program for execution.
//!
//! This modal sends a LoadProgram targeted mutation to the server. The server
//! updates the ExecutionState component which is automatically synced to all clients.
//! No client-side state updates are needed - the UI reads directly from the
//! synced ExecutionState.

use leptos::prelude::*;
use pl3xus_client::{use_query, use_mutation_targeted};
use fanuc_replica_types::*;
use crate::components::{use_toast, ToastType};
use crate::pages::dashboard::use_system_entity;

/// Load Program Modal - Select and load a program for execution.
///
/// Sends LoadProgram targeted mutation to server. Server updates ExecutionState which
/// is automatically synced to all clients. No client-side state updates needed.
#[component]
pub fn LoadProgramModal<F>(
    on_close: F,
) -> impl IntoView
where
    F: Fn() + Clone + 'static,
{
    let toast = use_toast();
    let system_ctx = use_system_entity();

    // Query for programs - auto-fetches on mount, auto-refetches on server invalidation
    let programs_query = use_query::<ListPrograms>(ListPrograms);

    let (selected_id, set_selected_id) = signal::<Option<i64>>(None);
    let on_close_load = on_close.clone();

    // LoadProgram targeted mutation - shows toasts and closes modal on success
    let load = use_mutation_targeted::<LoadProgram>(move |result| {
        match result {
            Ok(r) if r.success => {
                if let Some(program) = &r.program {
                    toast.show(&format!("Loaded '{}'", program.name), ToastType::Success);
                    on_close_load();
                }
            }
            Ok(r) => toast.show(r.error.as_deref().unwrap_or("Load failed"), ToastType::Error),
            Err(e) => toast.show(e, ToastType::Error),
        }
    });

    // Get the system entity ID for targeting
    let system_entity_id = system_ctx.system_entity_id;

    // Get programs from query data
    let programs = Memo::new(move |_| {
        programs_query.data().map(|r| r.programs.clone()).unwrap_or_default()
    });

    let is_loading = move || programs_query.is_loading();

    let on_close_header = on_close.clone();
    let on_close_cancel = on_close.clone();

    // Load selected program - sends targeted mutation to server
    // Note: TargetedMutationHandle is Copy, so it can be used directly in closures
    let load_click = move |_| {
        if let (Some(program_id), Some(entity_id)) = (selected_id.get(), system_entity_id.get()) {
            load.send(entity_id, LoadProgram { program_id });
        }
    };

    view! {
        <div class="fixed inset-0 bg-black/70 flex items-center justify-center z-50">
            <div class="bg-background border border-border/8 rounded-lg w-[500px] max-h-[80vh] flex flex-col">
                // Header
                <div class="flex items-center justify-between p-3 border-b border-border/8">
                    <h2 class="text-sm font-semibold text-foreground flex items-center gap-2">
                        <svg class="w-4 h-4 text-primary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 19a2 2 0 01-2-2V7a2 2 0 012-2h4l2 2h4a2 2 0 012 2v1M5 19h14a2 2 0 002-2v-5a2 2 0 00-2-2H9a2 2 0 00-2 2v5a2 2 0 01-2 2z"/>
                        </svg>
                        "Load Program"
                    </h2>
                    <button
                        class="text-muted-foreground hover:text-foreground"
                        on:click=move |_| on_close_header()
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                        </svg>
                    </button>
                </div>

                // Content
                <div class="flex-1 overflow-y-auto p-3">
                    <Show when=move || is_loading()>
                        <div class="text-center py-8 text-muted-foreground text-sm">
                            "Loading programs..."
                        </div>
                    </Show>
                    <Show when=move || !is_loading() && programs.get().is_empty()>
                        <div class="text-center py-8 text-muted-foreground text-sm">
                            "No programs available. Create a program in the Programs page."
                        </div>
                    </Show>
                    <Show when=move || !is_loading() && !programs.get().is_empty()>
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
                                                "bg-primary/20 border-primary text-foreground"
                                            } else {
                                                "bg-card border-border/8 text-foreground hover:bg-popover hover:border-border/8"
                                            }
                                        )
                                        on:click=move |_| set_selected_id.set(Some(id))
                                    >
                                        <div class="flex items-center justify-between">
                                            <span class="text-[11px] font-medium">{name.clone()}</span>
                                            <span class="text-[9px] text-muted-foreground">
                                                {format!("{} lines", line_count)}
                                            </span>
                                        </div>
                                        <Show when=move || has_description>
                                            <div class="text-[9px] text-muted-foreground mt-0.5">
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
                <div class="flex justify-end gap-2 p-3 border-t border-border/8">
                    <button
                        class="bg-popover border border-border/8 text-foreground text-[10px] px-4 py-1.5 rounded hover:bg-secondary"
                        on:click=move |_| on_close_cancel()
                    >
                        "Cancel"
                    </button>
                    <button
                        class=move || format!(
                            "text-[10px] px-4 py-1.5 rounded transition-colors {}",
                            if selected_id.get().is_some() && !load.is_loading() {
                                "bg-primary text-primary-foreground hover:brightness-110"
                            } else {
                                "bg-muted text-muted-foreground cursor-not-allowed"
                            }
                        )
                        disabled=move || selected_id.get().is_none() || load.is_loading()
                        on:click=load_click.clone()
                    >
                        {move || if load.is_loading() { "Loading..." } else { "Load Program" }}
                    </button>
                </div>
            </div>
        </div>
    }
}

