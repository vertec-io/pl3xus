//! Programs module - Program management and editing.
//!
//! This module contains components for:
//! - Program browser and selection
//! - Program creation, editing, and deletion
//! - CSV upload for program data
//! - Program preview and details

mod modals;
mod browser;
mod details;
mod menus;

pub use modals::*;

use leptos::prelude::*;
use pl3xus_client::{use_query, use_query_keyed};
use fanuc_replica_plugins::{ListPrograms, GetProgram, ProgramDetail};
use crate::layout::LayoutContext;

/// Programs view (toolpath creation and editing).
#[component]
pub fn ProgramsView() -> impl IntoView {
    let layout_ctx = use_context::<LayoutContext>().expect("LayoutContext not found");

    let (show_new_program, set_show_new_program) = signal(false);
    let (show_csv_upload, set_show_csv_upload) = signal(false);
    let (show_open_modal, set_show_open_modal) = signal(false);
    let (show_save_as_modal, set_show_save_as_modal) = signal(false);

    // Use layout context's selected_program_id - persists across navigation
    let selected_program_id = Signal::derive(move || layout_ctx.selected_program_id.get());
    let set_selected_program_id = move |id: Option<i64>| layout_ctx.selected_program_id.set(id);

    // Menu dropdown states
    let (show_file_menu, set_show_file_menu) = signal(false);
    let (show_view_menu, set_show_view_menu) = signal(false);

    // Query hooks - auto-fetch on mount, auto-refetch on server invalidation
    let programs_query = use_query::<ListPrograms>(ListPrograms);

    // Keyed query for selected program - fetches when selected_program_id changes
    let program_query = use_query_keyed::<GetProgram, _>(move || {
        selected_program_id.get().map(|id| GetProgram { program_id: id })
    });

    // Current program state - use ProgramDetail for full editing capability
    let current_program: RwSignal<Option<ProgramDetail>> = RwSignal::new(None);

    // Watch for program query response to update current_program
    Effect::new(move |_| {
        leptos::logging::log!("[ProgramsView] Effect running, checking program_query.data()");
        if let Some(response) = program_query.data() {
            leptos::logging::log!("[ProgramsView] Got response, program: {:?}", response.program.as_ref().map(|p| &p.name));
            if let Some(prog) = response.program.clone() {
                leptos::logging::log!("[ProgramsView] Setting current_program to: {}", prog.name);
                current_program.set(Some(prog));
            }
        } else {
            leptos::logging::log!("[ProgramsView] No data from program_query");
        }
    });

    // Derive programs from query data
    let programs = Memo::new(move |_| {
        programs_query.data()
            .map(|r| r.programs.clone())
            .unwrap_or_default()
    });

    view! {
        <div class="h-full flex flex-col">
            // Menu bar
            <div class="h-7 border-b border-[#ffffff08] flex items-center px-2 shrink-0 bg-[#0d0d0d]">
                // File menu
                <menus::FileMenu
                    show_file_menu=show_file_menu
                    set_show_file_menu=set_show_file_menu
                    set_show_view_menu=set_show_view_menu
                    set_show_new_program=set_show_new_program
                    set_show_open_modal=set_show_open_modal
                    set_show_save_as_modal=set_show_save_as_modal
                    set_show_csv_upload=set_show_csv_upload
                    selected_program_id=selected_program_id
                    current_program=current_program
                />

                // View menu
                <menus::ViewMenu
                    show_view_menu=show_view_menu
                    set_show_view_menu=set_show_view_menu
                    set_show_file_menu=set_show_file_menu
                />

                // Spacer
                <div class="flex-1"></div>

                // Current program indicator
                {move || current_program.get().map(|prog| view! {
                    <span class="text-[9px] text-[#666666]">
                        "Current: "
                        <span class="text-[#00d9ff]">{prog.name}</span>
                    </span>
                })}
            </div>

            // Main content area
            <div class="flex-1 p-2 flex gap-2 min-h-0">
                // Left: Program browser (conditionally shown)
                <Show when=move || layout_ctx.show_program_browser.get()>
                    <browser::ProgramBrowser
                        programs=programs
                        selected_program_id=selected_program_id
                        on_select=set_selected_program_id
                    />
                </Show>

                // Right: Program details
                <details::ProgramDetails
                    current_program=current_program
                    selected_program_id=selected_program_id
                    on_select=set_selected_program_id
                    set_show_csv_upload=set_show_csv_upload
                    set_show_open_modal=set_show_open_modal
                    set_show_new_program=set_show_new_program
                />
            </div>

            // Modals
            // Note: With server-side invalidation, queries auto-refetch when server broadcasts
            // QueryInvalidation. No manual refetch needed - just set the selected ID.
            <Show when=move || show_new_program.get()>
                <NewProgramModal
                    on_close=move || set_show_new_program.set(false)
                    on_created=move |id| {
                        set_show_new_program.set(false);
                        // Setting selected_program_id triggers program_query to fetch the new program
                        set_selected_program_id(Some(id));
                        // Server will broadcast QueryInvalidation for ListPrograms
                    }
                />
            </Show>

            <Show when=move || show_open_modal.get()>
                <OpenProgramModal
                    programs=programs
                    on_close=move || set_show_open_modal.set(false)
                    on_selected=move |prog: ProgramDetail| {
                        set_show_open_modal.set(false);
                        set_selected_program_id(Some(prog.id));
                        current_program.set(Some(prog));
                    }
                />
            </Show>

            <Show when=move || show_save_as_modal.get()>
                <SaveAsProgramModal
                    on_close=move || set_show_save_as_modal.set(false)
                    on_saved=move |id| {
                        set_show_save_as_modal.set(false);
                        set_selected_program_id(Some(id));
                        // Server will broadcast QueryInvalidation for ListPrograms
                    }
                />
            </Show>

            <Show when=move || show_csv_upload.get() && selected_program_id.get().is_some()>
                {move || selected_program_id.get().map(|prog_id| {
                    view! {
                        <CSVUploadModal
                            program_id=prog_id
                            on_close=move || set_show_csv_upload.set(false)
                            on_uploaded=move || {
                                set_show_csv_upload.set(false);
                                // Server will broadcast QueryInvalidation for ListPrograms and GetProgram
                                // program_query will auto-refetch since selected_program_id hasn't changed
                                // but the server invalidated the query
                            }
                        />
                    }
                })}
            </Show>
        </div>
    }
}

