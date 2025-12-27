//! Program modals - New, Open, Save As, and CSV Upload modals.

use leptos::prelude::*;
use leptos::either::Either;
use leptos::web_sys;
use pl3xus_client::{use_mutation, use_query_keyed};
use fanuc_replica_plugins::{CreateProgram, UploadCsv, GetProgram, ProgramDetail};

/// New Program Modal - Simple modal to create a program with name and description
#[component]
pub fn NewProgramModal(
    on_close: impl Fn() + 'static + Clone + Send,
    on_created: impl Fn(i64) + 'static + Clone + Send,
) -> impl IntoView {
    let (program_name, set_program_name) = signal("".to_string());
    let (description, set_description) = signal("".to_string());
    let (error_message, set_error_message) = signal::<Option<String>>(None);

    // CreateProgram mutation with handler
    let create_program = use_mutation::<CreateProgram>(move |result| {
        match result {
            Ok(r) if r.success => {
                if let Some(program_id) = r.program_id {
                    on_created(program_id);
                }
            }
            Ok(r) => set_error_message.set(r.error.clone()),
            Err(e) => set_error_message.set(Some(e.to_string())),
        }
    });

    let on_close_clone = on_close.clone();

    view! {
        <div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
            <div class="bg-card border border-border/10 rounded-lg w-[400px] flex flex-col">
                // Header
                <div class="flex items-center justify-between p-3 border-b border-border/8">
                    <h2 class="text-sm font-semibold text-foreground flex items-center">
                        <svg class="w-4 h-4 mr-2 text-primary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6"/>
                        </svg>
                        "New Program"
                    </h2>
                    <button
                        class="text-muted-foreground hover:text-foreground"
                        on:click={
                            let on_close = on_close.clone();
                            move |_| on_close()
                        }
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                        </svg>
                    </button>
                </div>

                // Content
                <div class="p-3 space-y-3">
                    <Show when=move || error_message.get().is_some()>
                        <div class="bg-destructive/15 border border-destructive/25 rounded p-2 flex items-start gap-2">
                            <svg class="w-4 h-4 text-destructive flex-shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                            </svg>
                            <span class="text-[10px] text-destructive">{move || error_message.get().unwrap_or_default()}</span>
                        </div>
                    </Show>
                    <div>
                        <label class="block text-[9px] text-muted-foreground mb-1">"Program Name *"</label>
                        <input
                            type="text"
                            placeholder="e.g., Spiral Cylinder"
                            class="w-full bg-background border border-border/8 rounded px-2 py-1.5 text-[10px] text-foreground focus:border-primary focus:outline-none"
                            prop:value=move || program_name.get()
                            on:input=move |ev| {
                                set_program_name.set(event_target_value(&ev));
                                set_error_message.set(None);
                            }
                        />
                    </div>
                    <div>
                        <label class="block text-[9px] text-muted-foreground mb-1">"Description"</label>
                        <textarea
                            placeholder="Optional description..."
                            rows="2"
                            class="w-full bg-background border border-border/8 rounded px-2 py-1.5 text-[10px] text-foreground focus:border-primary focus:outline-none resize-none"
                            prop:value=move || description.get()
                            on:input=move |ev| set_description.set(event_target_value(&ev))
                        ></textarea>
                    </div>
                    <p class="text-[8px] text-muted-foreground">
                        "After creating the program, you can upload a CSV file with motion data."
                    </p>
                </div>

                // Footer
                <div class="flex justify-end gap-2 p-3 border-t border-border/8">
                    <button
                        class="bg-popover border border-border/8 text-muted-foreground hover:text-foreground text-[10px] px-3 py-1.5 rounded"
                        on:click={
                            let on_close = on_close_clone.clone();
                            move |_| on_close()
                        }
                    >
                        "Cancel"
                    </button>
                    <button
                        class={move || format!(
                            "text-[10px] px-3 py-1.5 rounded {}",
                            if !program_name.get().is_empty() && !create_program.is_loading() {
                                "bg-[#22c55e20] border border-[#22c55e40] text-success hover:bg-success/20"
                            } else {
                                "bg-card border border-border/8 text-muted-foreground cursor-not-allowed"
                            }
                        )}
                        disabled=move || program_name.get().is_empty() || create_program.is_loading()
                        on:click=move |_| {
                            let name = program_name.get();
                            let desc = description.get();
                            create_program.send(CreateProgram {
                                name,
                                description: if desc.is_empty() { None } else { Some(desc) },
                            });
                        }
                    >
                        {move || if create_program.is_loading() { "Creating..." } else { "Create Program" }}
                    </button>
                </div>
            </div>
        </div>
    }
}

/// Open Program Modal - Select a program to open
#[component]
pub fn OpenProgramModal(
    programs: Memo<Vec<fanuc_replica_plugins::ProgramWithLines>>,
    on_close: impl Fn() + 'static + Clone + Send,
    on_selected: impl Fn(ProgramDetail) + 'static + Clone + Send,
) -> impl IntoView {
    let (selected_id, set_selected_id) = signal::<Option<i64>>(None);
    // Trigger signal - when set to Some(id), the query will fetch that program
    let (fetch_program_id, set_fetch_program_id) = signal::<Option<i64>>(None);

    // Query for program - only fetches when fetch_program_id is Some
    let program_query = use_query_keyed::<GetProgram, _>(move || {
        fetch_program_id.get().map(|id| GetProgram { program_id: id })
    });

    let on_close_clone = on_close.clone();

    // Watch for query response
    let on_selected_clone = on_selected.clone();
    let on_close_for_effect = on_close.clone();
    Effect::new(move |_| {
        // Only process when we have data and we're not loading
        if program_query.is_loading() {
            return;
        }
        if let Some(response) = program_query.data() {
            if let Some(program_detail) = response.program.clone() {
                // Pass the full ProgramDetail directly
                on_selected_clone(program_detail);
            } else {
                // Program not found, close modal
                on_close_for_effect();
            }
            // Reset trigger to prevent re-processing
            set_fetch_program_id.set(None);
        }
    });

    view! {
        <div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
            <div class="bg-card border border-border/10 rounded-lg w-[400px] max-h-[500px] flex flex-col">
                // Header
                <div class="flex items-center justify-between p-3 border-b border-border/8">
                    <h2 class="text-sm font-semibold text-foreground flex items-center">
                        <svg class="w-4 h-4 mr-2 text-primary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/>
                        </svg>
                        "Open Program"
                    </h2>
                    <button
                        class="text-muted-foreground hover:text-foreground"
                        on:click={
                            let on_close = on_close.clone();
                            move |_| on_close()
                        }
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                        </svg>
                    </button>
                </div>

                // Content - Program list
                <div class="flex-1 overflow-y-auto p-3 space-y-1.5">
                    {move || {
                        let progs = programs.get();
                        if progs.is_empty() {
                            Either::Left(view! {
                                <div class="text-center py-8 text-muted-foreground text-[10px]">
                                    "No programs available"
                                </div>
                            })
                        } else {
                            Either::Right(progs.into_iter().map(|prog| {
                                let prog_id = prog.id;
                                let prog_name = prog.name.clone();
                                let lines_str = format!("{} lines", prog.lines.len());
                                let is_selected = move || selected_id.get() == Some(prog_id);
                                view! {
                                    <button
                                        class={move || format!(
                                            "w-full text-left p-2 rounded border text-[9px] transition-colors {}",
                                            if is_selected() {
                                                "bg-[#00d9ff10] border-[#00d9ff40] text-foreground"
                                            } else {
                                                "bg-background border-border/8 text-muted-foreground hover:border-border/20"
                                            }
                                        )}
                                        on:click=move |_| set_selected_id.set(Some(prog_id))
                                    >
                                        <div class="font-medium text-[10px] mb-0.5">{prog_name}</div>
                                        <div class="text-muted-foreground">{lines_str}</div>
                                    </button>
                                }
                            }).collect_view())
                        }
                    }}
                </div>

                // Footer
                <div class="flex justify-end gap-2 p-3 border-t border-border/8">
                    <button
                        class="bg-popover border border-border/8 text-muted-foreground hover:text-foreground text-[10px] px-3 py-1.5 rounded"
                        on:click={
                            let on_close = on_close_clone.clone();
                            move |_| on_close()
                        }
                    >
                        "Cancel"
                    </button>
                    <button
                        class={move || format!(
                            "text-[10px] px-3 py-1.5 rounded {}",
                            if program_query.is_loading() {
                                "bg-card border border-border/8 text-muted-foreground cursor-wait"
                            } else if selected_id.get().is_some() {
                                "bg-[#00d9ff20] border border-[#00d9ff40] text-primary hover:bg-primary/20"
                            } else {
                                "bg-card border border-border/8 text-muted-foreground cursor-not-allowed"
                            }
                        )}
                        disabled=move || selected_id.get().is_none() || program_query.is_loading()
                        on:click=move |_| {
                            if let Some(id) = selected_id.get() {
                                // Setting fetch_program_id triggers the query
                                set_fetch_program_id.set(Some(id));
                            }
                        }
                    >
                        {move || if program_query.is_loading() { "Loading..." } else { "Open" }}
                    </button>
                </div>
            </div>
        </div>
    }
}

/// Save As Program Modal
#[component]
pub fn SaveAsProgramModal(
    on_close: impl Fn() + 'static + Clone + Send,
    on_saved: impl Fn(i64) + 'static + Clone + Send,
) -> impl IntoView {
    let (program_name, set_program_name) = signal("".to_string());
    let on_close_clone = on_close.clone();

    view! {
        <div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
            <div class="bg-card border border-border/10 rounded-lg w-[400px] flex flex-col">
                // Header
                <div class="flex items-center justify-between p-3 border-b border-border/8">
                    <h2 class="text-sm font-semibold text-foreground">"Save As"</h2>
                    <button
                        class="text-muted-foreground hover:text-foreground"
                        on:click={
                            let on_close = on_close.clone();
                            move |_| on_close()
                        }
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                        </svg>
                    </button>
                </div>

                // Content
                <div class="p-3">
                    <label class="block text-[9px] text-muted-foreground mb-1">"New Program Name"</label>
                    <input
                        type="text"
                        placeholder="Enter new name..."
                        class="w-full bg-background border border-border/8 rounded px-2 py-1.5 text-[10px] text-foreground focus:border-primary focus:outline-none"
                        prop:value=move || program_name.get()
                        on:input=move |ev| set_program_name.set(event_target_value(&ev))
                    />
                </div>

                // Footer
                <div class="flex justify-end gap-2 p-3 border-t border-border/8">
                    <button
                        class="bg-popover border border-border/8 text-muted-foreground hover:text-foreground text-[10px] px-3 py-1.5 rounded"
                        on:click={
                            let on_close = on_close_clone.clone();
                            move |_| on_close()
                        }
                    >
                        "Cancel"
                    </button>
                    <button
                        class={move || format!(
                            "text-[10px] px-3 py-1.5 rounded {}",
                            if !program_name.get().is_empty() {
                                "bg-[#22c55e20] border border-[#22c55e40] text-success hover:bg-success/20"
                            } else {
                                "bg-card border border-border/8 text-muted-foreground cursor-not-allowed"
                            }
                        )}
                        disabled=move || program_name.get().is_empty()
                        on:click=move |_| {
                            // TODO: Implement save as
                            on_saved(1);
                        }
                    >
                        "Save"
                    </button>
                </div>
            </div>
        </div>
    }
}

/// CSV Upload Modal
#[component]
pub fn CSVUploadModal(
    program_id: i64,
    on_close: impl Fn() + 'static + Clone + Send,
    on_uploaded: impl Fn() + 'static + Clone + Send,
) -> impl IntoView {
    let (file_name, set_file_name) = signal::<Option<String>>(None);
    let (csv_content, set_csv_content) = signal::<Option<String>>(None);
    let (error_message, set_error_message) = signal::<Option<String>>(None);
    let on_close_clone = on_close.clone();

    // UploadCsv mutation with handler
    let upload_csv = use_mutation::<UploadCsv>(move |result| {
        match result {
            Ok(r) if r.success => on_uploaded(),
            Ok(r) => set_error_message.set(r.error.clone()),
            Err(e) => set_error_message.set(Some(e.to_string())),
        }
    });

    view! {
        <div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
            <div class="bg-card border border-border/10 rounded-lg w-[400px] flex flex-col">
                // Header
                <div class="flex items-center justify-between p-3 border-b border-border/8">
                    <h2 class="text-sm font-semibold text-foreground flex items-center">
                        <svg class="w-4 h-4 mr-2 text-primary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"/>
                        </svg>
                        "Upload CSV"
                    </h2>
                    <button
                        class="text-muted-foreground hover:text-foreground"
                        on:click={
                            let on_close = on_close.clone();
                            move |_| on_close()
                        }
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                        </svg>
                    </button>
                </div>

                // Content
                <div class="p-3 space-y-3">
                    <Show when=move || error_message.get().is_some()>
                        <div class="bg-destructive/15 border border-destructive/25 rounded p-2 flex items-start gap-2">
                            <svg class="w-4 h-4 text-destructive flex-shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                            </svg>
                            <span class="text-[10px] text-destructive">{move || error_message.get().unwrap_or_default()}</span>
                        </div>
                    </Show>
                    <div class="border-2 border-dashed border-border/10 rounded-lg p-6 text-center relative">
                        <svg class="w-8 h-8 mx-auto mb-2 text-muted-foreground" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"/>
                        </svg>
                        {move || if let Some(name) = file_name.get() {
                            view! { <p class="text-[10px] text-primary">{name}</p> }.into_any()
                        } else {
                            view! { <p class="text-[10px] text-muted-foreground">"Drop CSV file here or click to browse"</p> }.into_any()
                        }}
                        <input
                            type="file"
                            accept=".csv"
                            class="absolute inset-0 opacity-0 cursor-pointer"
                            on:change=move |ev| {
                                use wasm_bindgen::JsCast;
                                let target = ev.target().unwrap();
                                let input: web_sys::HtmlInputElement = target.unchecked_into();
                                if let Some(files) = input.files() {
                                    if let Some(file) = files.get(0) {
                                        let name = file.name();
                                        set_file_name.set(Some(name));
                                        // Read file content
                                        let reader = web_sys::FileReader::new().unwrap();
                                        let reader_clone = reader.clone();
                                        let onload = wasm_bindgen::closure::Closure::wrap(Box::new(move |_: web_sys::Event| {
                                            if let Ok(result) = reader_clone.result() {
                                                if let Some(text) = result.as_string() {
                                                    set_csv_content.set(Some(text));
                                                }
                                            }
                                        }) as Box<dyn FnMut(_)>);
                                        reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                                        onload.forget();
                                        let _ = reader.read_as_text(&file);
                                    }
                                }
                            }
                        />
                    </div>
                    <p class="text-[8px] text-muted-foreground">
                        "CSV should have columns: X, Y, Z, W (optional), P (optional), R (optional), Speed (optional)"
                    </p>
                </div>

                // Footer
                <div class="flex justify-end gap-2 p-3 border-t border-border/8">
                    <button
                        class="bg-popover border border-border/8 text-muted-foreground hover:text-foreground text-[10px] px-3 py-1.5 rounded"
                        on:click={
                            let on_close = on_close_clone.clone();
                            move |_| on_close()
                        }
                    >
                        "Cancel"
                    </button>
                    <button
                        class={move || format!(
                            "text-[10px] px-3 py-1.5 rounded {}",
                            if csv_content.get().is_some() && !upload_csv.is_loading() {
                                "bg-[#22c55e20] border border-[#22c55e40] text-success hover:bg-success/20"
                            } else {
                                "bg-card border border-border/8 text-muted-foreground cursor-not-allowed"
                            }
                        )}
                        disabled=move || csv_content.get().is_none() || upload_csv.is_loading()
                        on:click=move |_| {
                            if let Some(content) = csv_content.get() {
                                upload_csv.send(UploadCsv {
                                    program_id,
                                    csv_content: content,
                                    start_position: None,
                                });
                            }
                        }
                    >
                        {move || if upload_csv.is_loading() { "Uploading..." } else { "Upload" }}
                    </button>
                </div>
            </div>
        </div>
    }
}
