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
use pl3xus_client::{use_sync_context, use_request, ConnectionReadyState};
use fanuc_replica_types::{ListPrograms, GetProgram, ProgramDetail};
use crate::layout::LayoutContext;

/// Programs view (toolpath creation and editing).
#[component]
pub fn ProgramsView() -> impl IntoView {
    let ctx = use_sync_context();
    let layout_ctx = use_context::<LayoutContext>().expect("LayoutContext not found");

    let (show_new_program, set_show_new_program) = signal(false);
    let (show_csv_upload, set_show_csv_upload) = signal(false);
    let (show_open_modal, set_show_open_modal) = signal(false);
    let (show_save_as_modal, set_show_save_as_modal) = signal(false);
    let (selected_program_id, set_selected_program_id) = signal::<Option<i64>>(None);

    // Menu dropdown states
    let (show_file_menu, set_show_file_menu) = signal(false);
    let (show_view_menu, set_show_view_menu) = signal(false);

    // Request hooks
    let (list_programs, programs_state) = use_request::<ListPrograms>();
    let (get_program, get_program_state) = use_request::<GetProgram>();

    // Current program state - use ProgramDetail for full editing capability
    let current_program: RwSignal<Option<ProgramDetail>> = RwSignal::new(None);

    // Watch for get_program response to update current_program
    Effect::new(move |_| {
        if let Some(response) = get_program_state.get().data {
            if let Some(prog) = response.program {
                current_program.set(Some(prog));
            }
        }
    });

    // Guard to prevent infinite loop
    let (has_loaded, set_has_loaded) = signal(false);

    // Load programs when WebSocket is open - wait for connection before sending request
    {
        let list_programs = list_programs.clone();
        let ready_state = ctx.ready_state;
        Effect::new(move |_| {
            // Only send request when WebSocket is open AND we haven't loaded yet
            if ready_state.get() == ConnectionReadyState::Open && !has_loaded.get_untracked() {
                set_has_loaded.set(true);
                list_programs(ListPrograms);
            }
        });
    }

    // Derive programs from request state
    let programs = Memo::new(move |_| {
        programs_state.get().data
            .map(|r| r.programs)
            .unwrap_or_default()
    });

    // Clone list_programs and get_program for use in closures
    let list_programs_1 = list_programs.clone();
    let list_programs_2 = list_programs.clone();
    let list_programs_3 = list_programs;
    let get_program_1 = get_program.clone();
    let get_program_2 = get_program.clone();
    let get_program_3 = get_program;

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
                        set_selected_program_id=set_selected_program_id
                    />
                </Show>

                // Right: Program details
                <details::ProgramDetails
                    current_program=current_program
                    selected_program_id=selected_program_id
                    set_selected_program_id=set_selected_program_id
                    set_show_csv_upload=set_show_csv_upload
                    set_show_open_modal=set_show_open_modal
                    set_show_new_program=set_show_new_program
                />
            </div>

            // Modals
            <Show when=move || show_new_program.get()>
                <NewProgramModal
                    on_close=move || set_show_new_program.set(false)
                    on_created={
                        let list_programs = list_programs_1.clone();
                        let get_program = get_program_1.clone();
                        move |id| {
                            set_show_new_program.set(false);
                            set_selected_program_id.set(Some(id));
                            // Fetch the newly created program to update current_program
                            get_program(GetProgram { program_id: id });
                            list_programs(ListPrograms);
                        }
                    }
                />
            </Show>

            <Show when=move || show_open_modal.get()>
                <OpenProgramModal
                    programs=programs
                    on_close=move || set_show_open_modal.set(false)
                    on_selected=move |prog: ProgramDetail| {
                        set_show_open_modal.set(false);
                        set_selected_program_id.set(Some(prog.id));
                        current_program.set(Some(prog));
                    }
                />
            </Show>

            <Show when=move || show_save_as_modal.get()>
                <SaveAsProgramModal
                    on_close=move || set_show_save_as_modal.set(false)
                    on_saved={
                        let list_programs = list_programs_2.clone();
                        move |id| {
                            set_show_save_as_modal.set(false);
                            set_selected_program_id.set(Some(id));
                            list_programs(ListPrograms);
                        }
                    }
                />
            </Show>

            <Show when=move || show_csv_upload.get() && selected_program_id.get().is_some()>
                {
                    let list_programs = list_programs_3.clone();
                    let get_program = get_program_3.clone();
                    move || selected_program_id.get().map(|prog_id| {
                        let list_programs = list_programs.clone();
                        let get_program = get_program.clone();
                        view! {
                            <CSVUploadModal
                                program_id=prog_id
                                on_close=move || set_show_csv_upload.set(false)
                                on_uploaded=move || {
                                    set_show_csv_upload.set(false);
                                    list_programs(ListPrograms);
                                    // Re-fetch the current program to get updated instructions
                                    get_program(GetProgram { program_id: prog_id });
                                }
                            />
                        }
                    })
                }
            </Show>
        </div>
    }
}

