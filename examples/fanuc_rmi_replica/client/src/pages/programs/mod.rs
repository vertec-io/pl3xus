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
use pl3xus_client::{use_sync_context, use_request};
use fanuc_replica_types::{ListPrograms, ProgramWithLines};
use crate::layout::LayoutContext;

/// Programs view (toolpath creation and editing).
#[component]
pub fn ProgramsView() -> impl IntoView {
    let _ctx = use_sync_context();
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

    // Current program state
    let current_program: RwSignal<Option<ProgramWithLines>> = RwSignal::new(None);

    // Load programs on mount
    {
        let list_programs = list_programs.clone();
        Effect::new(move |_| {
            list_programs(ListPrograms);
        });
    }

    // Derive programs from request state
    let programs = Memo::new(move |_| {
        programs_state.get().data
            .map(|r| r.programs)
            .unwrap_or_default()
    });

    // Clone list_programs for use in closures
    let list_programs_1 = list_programs.clone();
    let list_programs_2 = list_programs.clone();
    let list_programs_3 = list_programs;

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
                        move |id| {
                            set_show_new_program.set(false);
                            set_selected_program_id.set(Some(id));
                            list_programs(ListPrograms);
                        }
                    }
                />
            </Show>

            <Show when=move || show_open_modal.get()>
                <OpenProgramModal
                    on_close=move || set_show_open_modal.set(false)
                    on_selected=move |id| {
                        set_show_open_modal.set(false);
                        set_selected_program_id.set(Some(id));
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
                    move || selected_program_id.get().map(|prog_id| {
                        let list_programs = list_programs.clone();
                        view! {
                            <CSVUploadModal
                                program_id=prog_id
                                on_close=move || set_show_csv_upload.set(false)
                                on_uploaded=move || {
                                    set_show_csv_upload.set(false);
                                    list_programs(ListPrograms);
                                }
                            />
                        }
                    })
                }
            </Show>
        </div>
    }
}

