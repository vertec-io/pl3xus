//! Menu components for the Programs view.

use leptos::prelude::*;
use fanuc_replica_plugins::ProgramDetail;
use crate::layout::LayoutContext;

/// File menu dropdown
#[component]
pub fn FileMenu(
    show_file_menu: ReadSignal<bool>,
    set_show_file_menu: WriteSignal<bool>,
    set_show_view_menu: WriteSignal<bool>,
    set_show_new_program: WriteSignal<bool>,
    set_show_open_modal: WriteSignal<bool>,
    set_show_save_as_modal: WriteSignal<bool>,
    set_show_csv_upload: WriteSignal<bool>,
    #[prop(into)] selected_program_id: Signal<Option<i64>>,
    current_program: RwSignal<Option<ProgramDetail>>,
) -> impl IntoView {
    view! {
        <div class="relative">
            <button
                class={move || format!(
                    "px-2 py-1 text-[10px] rounded transition-colors {}",
                    if show_file_menu.get() { "bg-border/10 text-foreground" } else { "text-muted-foreground hover:text-foreground hover:bg-border/8" }
                )}
                on:click=move |_| {
                    set_show_file_menu.update(|v| *v = !*v);
                    set_show_view_menu.set(false);
                }
            >
                "File"
            </button>
            {move || if show_file_menu.get() {
                view! {
                    <div class="absolute left-0 top-full mt-0.5 w-40 bg-popover border border-border/15 rounded shadow-lg z-50">
                        <button
                            class="w-full text-left px-3 py-1.5 text-[10px] text-muted-foreground hover:bg-border/10 hover:text-foreground flex items-center gap-2"
                            on:click=move |_| {
                                set_show_new_program.set(true);
                                set_show_file_menu.set(false);
                            }
                        >
                            <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6"/>
                            </svg>
                            "New Program"
                        </button>
                        <button
                            class="w-full text-left px-3 py-1.5 text-[10px] text-muted-foreground hover:bg-border/10 hover:text-foreground flex items-center gap-2"
                            on:click=move |_| {
                                set_show_open_modal.set(true);
                                set_show_file_menu.set(false);
                            }
                        >
                            <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/>
                            </svg>
                            "Open..."
                        </button>
                        <div class="border-t border-border/10 my-1"></div>
                        <button
                            class={move || format!(
                                "w-full text-left px-3 py-1.5 text-[10px] flex items-center gap-2 {}",
                                if current_program.get().is_some() {
                                    "text-muted-foreground hover:bg-border/10 hover:text-foreground"
                                } else {
                                    "text-muted-foreground cursor-not-allowed"
                                }
                            )}
                            on:click=move |_| {
                                if current_program.get().is_some() {
                                    set_show_save_as_modal.set(true);
                                    set_show_file_menu.set(false);
                                }
                            }
                        >
                            <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4"/>
                            </svg>
                            "Save As..."
                        </button>
                        <div class="border-t border-border/10 my-1"></div>
                        <button
                            class={move || format!(
                                "w-full text-left px-3 py-1.5 text-[10px] flex items-center gap-2 {}",
                                if selected_program_id.get().is_some() {
                                    "text-muted-foreground hover:bg-border/10 hover:text-foreground"
                                } else {
                                    "text-muted-foreground cursor-not-allowed"
                                }
                            )}
                            on:click=move |_| {
                                if selected_program_id.get().is_some() {
                                    set_show_csv_upload.set(true);
                                    set_show_file_menu.set(false);
                                }
                            }
                        >
                            <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"/>
                            </svg>
                            "Upload CSV..."
                        </button>
                        <div class="border-t border-border/10 my-1"></div>
                        <button
                            class={move || format!(
                                "w-full text-left px-3 py-1.5 text-[10px] flex items-center gap-2 {}",
                                if selected_program_id.get().is_some() || current_program.get().is_some() {
                                    "text-muted-foreground hover:bg-border/10 hover:text-foreground"
                                } else {
                                    "text-muted-foreground cursor-not-allowed"
                                }
                            )}
                            disabled=move || selected_program_id.get().is_none() && current_program.get().is_none()
                            on:click=move |_| {
                                current_program.set(None);
                                set_show_file_menu.set(false);
                            }
                        >
                            <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                            </svg>
                            "Close"
                        </button>
                    </div>
                }.into_any()
            } else {
                view! { <div></div> }.into_any()
            }}
        </div>
    }
}

/// View menu dropdown
#[component]
pub fn ViewMenu(
    show_view_menu: ReadSignal<bool>,
    set_show_view_menu: WriteSignal<bool>,
    set_show_file_menu: WriteSignal<bool>,
) -> impl IntoView {
    let layout_ctx = use_context::<LayoutContext>().expect("LayoutContext not found");

    view! {
        <div class="relative">
            <button
                class={move || format!(
                    "px-2 py-1 text-[10px] rounded transition-colors {}",
                    if show_view_menu.get() { "bg-border/10 text-foreground" } else { "text-muted-foreground hover:text-foreground hover:bg-border/8" }
                )}
                on:click=move |_| {
                    set_show_view_menu.update(|v| *v = !*v);
                    set_show_file_menu.set(false);
                }
            >
                "View"
            </button>
            {move || if show_view_menu.get() {
                view! {
                    <div class="absolute left-0 top-full mt-0.5 w-48 bg-popover border border-border/15 rounded shadow-lg z-50">
                        <button
                            class="w-full text-left px-3 py-1.5 text-[10px] text-muted-foreground hover:bg-border/10 hover:text-foreground flex items-center justify-between"
                            on:click=move |_| {
                                layout_ctx.show_program_browser.update(|v| *v = !*v);
                                set_show_view_menu.set(false);
                            }
                        >
                            <span class="flex items-center gap-2">
                                <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h7"/>
                                </svg>
                                "Program Browser"
                            </span>
                            {move || if layout_ctx.show_program_browser.get() {
                                view! { <span class="text-primary">"✓"</span> }.into_any()
                            } else {
                                view! { <span></span> }.into_any()
                            }}
                        </button>
                    </div>
                }.into_any()
            } else {
                view! { <div></div> }.into_any()
            }}
        </div>
    }
}
