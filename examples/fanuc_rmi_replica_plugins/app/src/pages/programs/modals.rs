//! Program modals - New, Open, Save As, and CSV Upload modals.

use leptos::prelude::*;
use leptos::either::Either;
use leptos::web_sys;
use pl3xus_client::{use_mutation, use_query_keyed};
use fanuc_replica_plugins::{
    CreateProgram, UploadCsv, GetProgram, ProgramDetail, SequenceType,
    AddSequence, RemoveSequence, Instruction,
};

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
    programs: Memo<Vec<fanuc_replica_plugins::ProgramInfo>>,
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
                                let lines_str = format!("{} lines", prog.instruction_count);
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

/// CSV Upload Modal with sequence type selection
#[component]
pub fn CSVUploadModal(
    program_id: i64,
    on_close: impl Fn() + 'static + Clone + Send,
    on_uploaded: impl Fn() + 'static + Clone + Send,
) -> impl IntoView {
    let (file_name, set_file_name) = signal::<Option<String>>(None);
    let (csv_content, set_csv_content) = signal::<Option<String>>(None);
    let (error_message, set_error_message) = signal::<Option<String>>(None);
    let (sequence_type, set_sequence_type) = signal::<String>("main".to_string());
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

                    // Sequence type selector
                    <div>
                        <label class="block text-[9px] text-muted-foreground mb-1">"Sequence Type"</label>
                        <select
                            class="w-full bg-background border border-border/8 rounded px-2 py-1.5 text-[10px] text-foreground focus:border-primary focus:outline-none"
                            prop:value=move || sequence_type.get()
                            on:change=move |ev| set_sequence_type.set(event_target_value(&ev))
                        >
                            <option value="main">"Main (replaces existing)"</option>
                            <option value="approach">"Approach (adds new sequence)"</option>
                            <option value="retreat">"Retreat (adds new sequence)"</option>
                        </select>
                    </div>

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
                                let seq_type = match sequence_type.get().as_str() {
                                    "approach" => Some(SequenceType::Approach),
                                    "retreat" => Some(SequenceType::Retreat),
                                    _ => Some(SequenceType::Main),
                                };
                                upload_csv.send(UploadCsv {
                                    program_id,
                                    csv_content: content,
                                    sequence_id: None,
                                    sequence_type: seq_type,
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



/// Add Sequence Modal - Add approach or retreat sequence with manual position entry or CSV upload
#[component]
pub fn AddSequenceModal(
    program_id: i64,
    sequence_type: SequenceType,
    on_close: impl Fn() + 'static + Clone + Send,
    on_added: impl Fn() + 'static + Clone + Send,
) -> impl IntoView {
    let (name, set_name) = signal(String::new());
    let (x, set_x) = signal(String::new());
    let (y, set_y) = signal(String::new());
    let (z, set_z) = signal(String::new());
    let (w, set_w) = signal(String::new());
    let (p, set_p) = signal(String::new());
    let (r, set_r) = signal(String::new());
    let (speed, set_speed) = signal("100".to_string());
    let (file_name, set_file_name) = signal::<Option<String>>(None);
    let (csv_content, set_csv_content) = signal::<Option<String>>(None);
    let (error_message, set_error_message) = signal::<Option<String>>(None);
    let on_close_clone = on_close.clone();

    let add_sequence = use_mutation::<AddSequence>(move |result| {
        match result {
            Ok(r) if r.success => on_added(),
            Ok(r) => set_error_message.set(r.error.clone()),
            Err(e) => set_error_message.set(Some(e.to_string())),
        }
    });

    let type_label = match sequence_type {
        SequenceType::Approach => "Approach",
        SequenceType::Retreat => "Retreat",
        SequenceType::Main => "Main",
    };

    view! {
        <div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
            <div class="bg-card border border-border/10 rounded-lg w-[450px] flex flex-col">
                // Header
                <div class="flex items-center justify-between p-3 border-b border-border/8">
                    <h2 class="text-sm font-semibold text-foreground flex items-center">
                        <svg class="w-4 h-4 mr-2 text-primary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6"/>
                        </svg>
                        "Add "{type_label}" Sequence"
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
                        <label class="block text-[9px] text-muted-foreground mb-1">"Sequence Name (optional)"</label>
                        <input
                            type="text"
                            placeholder={format!("{} 1", type_label)}
                            class="w-full bg-background border border-border/8 rounded px-2 py-1.5 text-[10px] text-foreground focus:border-primary focus:outline-none"
                            prop:value=move || name.get()
                            on:input=move |ev| set_name.set(event_target_value(&ev))
                        />
                    </div>

                    <div>
                        <label class="block text-[9px] text-muted-foreground mb-1">"Position (single point)"</label>
                        <div class="grid grid-cols-6 gap-2">
                            <div>
                                <label class="text-[7px] text-muted-foreground">"X"</label>
                                <input type="text" class="w-full bg-card border border-border/10 rounded px-2 py-1 text-[10px] text-foreground font-mono" placeholder="0.0"
                                    prop:value=move || x.get() on:input=move |ev| set_x.set(event_target_value(&ev)) />
                            </div>
                            <div>
                                <label class="text-[7px] text-muted-foreground">"Y"</label>
                                <input type="text" class="w-full bg-card border border-border/10 rounded px-2 py-1 text-[10px] text-foreground font-mono" placeholder="0.0"
                                    prop:value=move || y.get() on:input=move |ev| set_y.set(event_target_value(&ev)) />
                            </div>
                            <div>
                                <label class="text-[7px] text-muted-foreground">"Z"</label>
                                <input type="text" class="w-full bg-card border border-border/10 rounded px-2 py-1 text-[10px] text-foreground font-mono" placeholder="0.0"
                                    prop:value=move || z.get() on:input=move |ev| set_z.set(event_target_value(&ev)) />
                            </div>
                            <div>
                                <label class="text-[7px] text-muted-foreground">"W"</label>
                                <input type="text" class="w-full bg-card border border-border/10 rounded px-2 py-1 text-[10px] text-foreground font-mono" placeholder="0.0"
                                    prop:value=move || w.get() on:input=move |ev| set_w.set(event_target_value(&ev)) />
                            </div>
                            <div>
                                <label class="text-[7px] text-muted-foreground">"P"</label>
                                <input type="text" class="w-full bg-card border border-border/10 rounded px-2 py-1 text-[10px] text-foreground font-mono" placeholder="0.0"
                                    prop:value=move || p.get() on:input=move |ev| set_p.set(event_target_value(&ev)) />
                            </div>
                            <div>
                                <label class="text-[7px] text-muted-foreground">"R"</label>
                                <input type="text" class="w-full bg-card border border-border/10 rounded px-2 py-1 text-[10px] text-foreground font-mono" placeholder="0.0"
                                    prop:value=move || r.get() on:input=move |ev| set_r.set(event_target_value(&ev)) />
                            </div>
                        </div>
                    </div>

                    <div>
                        <label class="block text-[9px] text-muted-foreground mb-1">"Speed (mm/s)"</label>
                        <input
                            type="text"
                            class="w-20 bg-background border border-border/8 rounded px-2 py-1.5 text-[10px] text-foreground font-mono focus:border-primary focus:outline-none"
                            prop:value=move || speed.get()
                            on:input=move |ev| set_speed.set(event_target_value(&ev))
                        />
                    </div>

                    <div class="border-t border-border/8 pt-3">
                        <label class="block text-[9px] text-muted-foreground mb-2">"Or upload CSV for multi-point sequence"</label>
                        <div class="border-2 border-dashed border-border/10 rounded-lg p-4 text-center relative">
                            <svg class="w-6 h-6 mx-auto mb-1 text-muted-foreground" fill="none" stroke="currentColor" viewBox="0 0 24 24">
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
                        <p class="text-[8px] text-muted-foreground mt-1">
                            "CSV should have columns: X, Y, Z, W (optional), P (optional), R (optional), Speed (optional)"
                        </p>
                    </div>
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
                        class={move || {
                            let has_csv = csv_content.get().is_some();
                            let has_manual = !x.get().is_empty() && !y.get().is_empty() && !z.get().is_empty();
                            let can_submit = (has_csv || has_manual) && !add_sequence.is_loading();
                            format!(
                                "text-[10px] px-3 py-1.5 rounded {}",
                                if can_submit {
                                    "bg-[#22c55e20] border border-[#22c55e40] text-success hover:bg-success/20"
                                } else {
                                    "bg-card border border-border/8 text-muted-foreground cursor-not-allowed"
                                }
                            )
                        }}
                        disabled=move || {
                            let has_csv = csv_content.get().is_some();
                            let has_manual = !x.get().is_empty() && !y.get().is_empty() && !z.get().is_empty();
                            !(has_csv || has_manual) || add_sequence.is_loading()
                        }
                        on:click=move |_| {
                            // If CSV is provided, use it; otherwise use manual entry
                            if let Some(csv) = csv_content.get() {
                                add_sequence.send(AddSequence {
                                    program_id,
                                    sequence_type,
                                    name: if name.get().is_empty() { None } else { Some(name.get()) },
                                    instructions: vec![],
                                    csv_content: Some(csv),
                                });
                            } else {
                                let x_val: f64 = x.get().parse().unwrap_or(0.0);
                                let y_val: f64 = y.get().parse().unwrap_or(0.0);
                                let z_val: f64 = z.get().parse().unwrap_or(0.0);
                                let w_val: Option<f64> = w.get().parse().ok();
                                let p_val: Option<f64> = p.get().parse().ok();
                                let r_val: Option<f64> = r.get().parse().ok();
                                let speed_val: Option<f64> = speed.get().parse().ok();

                                let instruction = Instruction {
                                    line_number: 1,
                                    x: x_val,
                                    y: y_val,
                                    z: z_val,
                                    w: w_val,
                                    p: p_val,
                                    r: r_val,
                                    ext1: None,
                                    ext2: None,
                                    ext3: None,
                                    speed: speed_val,
                                    term_type: Some("FINE".to_string()),
                                    term_value: None,
                                };

                                add_sequence.send(AddSequence {
                                    program_id,
                                    sequence_type,
                                    name: if name.get().is_empty() { None } else { Some(name.get()) },
                                    instructions: vec![instruction],
                                    csv_content: None,
                                });
                            }
                        }
                    >
                        {move || if add_sequence.is_loading() { "Adding..." } else { "Add Sequence" }}
                    </button>
                </div>
            </div>
        </div>
    }
}


/// Manage Sequences Modal - View and delete approach/retreat sequences
#[component]
pub fn ManageSequencesModal(
    program: ProgramDetail,
    on_close: impl Fn() + 'static + Clone + Send,
    on_changed: impl Fn() + 'static + Clone + Send,
) -> impl IntoView {
    let (error_message, set_error_message) = signal::<Option<String>>(None);
    let (deleting_id, set_deleting_id) = signal::<Option<i64>>(None);
    let on_close_clone = on_close.clone();
    let on_changed_clone = on_changed.clone();

    let remove_sequence = use_mutation::<RemoveSequence>(move |result| {
        set_deleting_id.set(None);
        match result {
            Ok(r) if r.success => on_changed_clone(),
            Ok(r) => set_error_message.set(r.error.clone()),
            Err(e) => set_error_message.set(Some(e.to_string())),
        }
    });

    let approach_seqs = program.approach_sequences.clone();
    let retreat_seqs = program.retreat_sequences.clone();
    let main_seqs = program.main_sequences.clone();

    view! {
        <div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
            <div class="bg-card border border-border/10 rounded-lg w-[500px] max-h-[600px] flex flex-col">
                // Header
                <div class="flex items-center justify-between p-3 border-b border-border/8">
                    <h2 class="text-sm font-semibold text-foreground flex items-center">
                        <svg class="w-4 h-4 mr-2 text-primary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 10h16M4 14h16M4 18h16"/>
                        </svg>
                        "Manage Sequences"
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
                <div class="flex-1 overflow-y-auto p-3 space-y-4">
                    <Show when=move || error_message.get().is_some()>
                        <div class="bg-destructive/15 border border-destructive/25 rounded p-2 flex items-start gap-2">
                            <svg class="w-4 h-4 text-destructive flex-shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                            </svg>
                            <span class="text-[10px] text-destructive">{move || error_message.get().unwrap_or_default()}</span>
                        </div>
                    </Show>

                    // Approach Sequences
                    <div>
                        <h3 class="text-[10px] font-semibold text-muted-foreground uppercase mb-2 flex items-center">
                            <span class="w-2 h-2 rounded-full bg-blue-500 mr-2"></span>
                            "Approach Sequences ("{approach_seqs.len()}")"
                        </h3>
                        {if approach_seqs.is_empty() {
                            Either::Left(view! {
                                <div class="text-[9px] text-muted-foreground italic pl-4">"No approach sequences"</div>
                            })
                        } else {
                            Either::Right(approach_seqs.into_iter().map(|seq| {
                                let seq_id = seq.id;
                                let seq_name = seq.name.clone().unwrap_or_else(|| format!("Approach {}", seq.order_index + 1));
                                let instr_count = seq.instructions.len();
                                view! {
                                    <SequenceRow
                                        seq_id=seq_id
                                        name=seq_name
                                        instr_count=instr_count
                                        color="blue"
                                        deleting_id=deleting_id
                                        on_delete=move || {
                                            set_deleting_id.set(Some(seq_id));
                                            remove_sequence.send(RemoveSequence { sequence_id: seq_id });
                                        }
                                    />
                                }
                            }).collect_view())
                        }}
                    </div>

                    // Main Sequences (read-only)
                    <div>
                        <h3 class="text-[10px] font-semibold text-muted-foreground uppercase mb-2 flex items-center">
                            <span class="w-2 h-2 rounded-full bg-green-500 mr-2"></span>
                            "Main Sequences ("{main_seqs.len()}")"
                        </h3>
                        <div class="space-y-1">
                            {main_seqs.iter().enumerate().map(|(idx, seq)| {
                                let seq_name = seq.name.clone().unwrap_or_else(|| format!("Main {}", idx + 1));
                                let inst_count = seq.instructions.len();
                                view! {
                                    <div class="bg-card rounded border border-border/8 p-2 flex items-center justify-between">
                                        <div>
                                            <div class="text-[10px] text-foreground font-medium">{seq_name}</div>
                                            <div class="text-[8px] text-muted-foreground">{inst_count}" instructions"</div>
                                        </div>
                                    </div>
                                }
                            }).collect_view()}
                        </div>
                    </div>

                    // Retreat Sequences
                    <div>
                        <h3 class="text-[10px] font-semibold text-muted-foreground uppercase mb-2 flex items-center">
                            <span class="w-2 h-2 rounded-full bg-orange-500 mr-2"></span>
                            "Retreat Sequences ("{retreat_seqs.len()}")"
                        </h3>
                        {if retreat_seqs.is_empty() {
                            Either::Left(view! {
                                <div class="text-[9px] text-muted-foreground italic pl-4">"No retreat sequences"</div>
                            })
                        } else {
                            Either::Right(retreat_seqs.into_iter().map(|seq| {
                                let seq_id = seq.id;
                                let seq_name = seq.name.clone().unwrap_or_else(|| format!("Retreat {}", seq.order_index + 1));
                                let instr_count = seq.instructions.len();
                                view! {
                                    <SequenceRow
                                        seq_id=seq_id
                                        name=seq_name
                                        instr_count=instr_count
                                        color="orange"
                                        deleting_id=deleting_id
                                        on_delete=move || {
                                            set_deleting_id.set(Some(seq_id));
                                            remove_sequence.send(RemoveSequence { sequence_id: seq_id });
                                        }
                                    />
                                }
                            }).collect_view())
                        }}
                    </div>
                </div>

                // Footer
                <div class="flex justify-end gap-2 p-3 border-t border-border/8">
                    <button
                        class="bg-[#00d9ff20] border border-[#00d9ff40] text-primary text-[10px] px-3 py-1.5 rounded hover:bg-primary/20"
                        on:click={
                            let on_close = on_close_clone.clone();
                            move |_| on_close()
                        }
                    >
                        "Done"
                    </button>
                </div>
            </div>
        </div>
    }
}

/// Sequence row component for ManageSequencesModal
#[component]
fn SequenceRow(
    seq_id: i64,
    name: String,
    instr_count: usize,
    color: &'static str,
    deleting_id: ReadSignal<Option<i64>>,
    on_delete: impl Fn() + 'static + Clone + Send,
) -> impl IntoView {
    let is_deleting = move || deleting_id.get() == Some(seq_id);
    let border_color = match color {
        "blue" => "border-blue-500/30",
        "orange" => "border-orange-500/30",
        _ => "border-border/8",
    };

    view! {
        <div class={format!("bg-card rounded border {} p-2 flex items-center justify-between mb-1", border_color)}>
            <div>
                <div class="text-[10px] text-foreground font-medium">{name}</div>
                <div class="text-[8px] text-muted-foreground">{instr_count}" instructions"</div>
            </div>
            <button
                class="bg-destructive/15 border border-destructive/25 text-destructive text-[8px] px-2 py-1 rounded hover:bg-destructive/20 disabled:opacity-50"
                disabled=is_deleting
                on:click=move |_| on_delete()
            >
                {move || if is_deleting() { "Deleting..." } else { "Delete" }}
            </button>
        </div>
    }
}