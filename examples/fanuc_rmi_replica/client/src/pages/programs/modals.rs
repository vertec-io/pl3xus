//! Program modals - New, Open, Save As, and CSV Upload modals.

use leptos::prelude::*;
use leptos::either::Either;
use leptos::web_sys;
use pl3xus_client::use_request;
use fanuc_replica_types::{ListPrograms, CreateProgram, UploadCsv};

/// New Program Modal - Simple modal to create a program with name and description
#[component]
pub fn NewProgramModal(
    on_close: impl Fn() + 'static + Clone + Send,
    on_created: impl Fn(i64) + 'static + Clone + Send,
) -> impl IntoView {
    let (program_name, set_program_name) = signal("".to_string());
    let (description, set_description) = signal("".to_string());
    let (error_message, set_error_message) = signal::<Option<String>>(None);
    let (has_processed, set_has_processed) = signal(false);

    let (create_program, create_state) = use_request::<CreateProgram>();

    // Watch for create response - with guard to prevent re-processing
    let on_created_clone = on_created.clone();
    Effect::new(move |_| {
        // Only process once
        if has_processed.get_untracked() {
            return;
        }
        if let Some(response) = create_state.get().data {
            set_has_processed.set(true);
            if response.success {
                if let Some(program_id) = response.program_id {
                    on_created_clone(program_id);
                }
            } else {
                set_error_message.set(response.error);
                // Allow retrying after an error
                set_has_processed.set(false);
            }
        }
    });

    let is_creating = Memo::new(move |_| create_state.get().is_loading);

    let on_close_clone = on_close.clone();

    view! {
        <div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
            <div class="bg-[#111111] border border-[#ffffff10] rounded-lg w-[400px] flex flex-col">
                // Header
                <div class="flex items-center justify-between p-3 border-b border-[#ffffff08]">
                    <h2 class="text-sm font-semibold text-white flex items-center">
                        <svg class="w-4 h-4 mr-2 text-[#00d9ff]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6"/>
                        </svg>
                        "New Program"
                    </h2>
                    <button
                        class="text-[#666666] hover:text-white"
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
                        <div class="bg-[#ff444420] border border-[#ff444440] rounded p-2 flex items-start gap-2">
                            <svg class="w-4 h-4 text-[#ff4444] flex-shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                            </svg>
                            <span class="text-[10px] text-[#ff4444]">{move || error_message.get().unwrap_or_default()}</span>
                        </div>
                    </Show>
                    <div>
                        <label class="block text-[9px] text-[#888888] mb-1">"Program Name *"</label>
                        <input
                            type="text"
                            placeholder="e.g., Spiral Cylinder"
                            class="w-full bg-[#0a0a0a] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                            prop:value=move || program_name.get()
                            on:input=move |ev| {
                                set_program_name.set(event_target_value(&ev));
                                set_error_message.set(None);
                            }
                        />
                    </div>
                    <div>
                        <label class="block text-[9px] text-[#888888] mb-1">"Description"</label>
                        <textarea
                            placeholder="Optional description..."
                            rows="2"
                            class="w-full bg-[#0a0a0a] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none resize-none"
                            prop:value=move || description.get()
                            on:input=move |ev| set_description.set(event_target_value(&ev))
                        ></textarea>
                    </div>
                    <p class="text-[8px] text-[#555555]">
                        "After creating the program, you can upload a CSV file with motion data."
                    </p>
                </div>

                // Footer
                <div class="flex justify-end gap-2 p-3 border-t border-[#ffffff08]">
                    <button
                        class="bg-[#1a1a1a] border border-[#ffffff08] text-[#888888] hover:text-white text-[10px] px-3 py-1.5 rounded"
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
                            if !program_name.get().is_empty() && !is_creating.get() {
                                "bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] hover:bg-[#22c55e30]"
                            } else {
                                "bg-[#111111] border border-[#ffffff08] text-[#555555] cursor-not-allowed"
                            }
                        )}
                        disabled=move || program_name.get().is_empty() || is_creating.get()
                        on:click=move |_| {
                            let name = program_name.get();
                            let desc = description.get();
                            create_program(CreateProgram {
                                name,
                                description: if desc.is_empty() { None } else { Some(desc) },
                            });
                        }
                    >
                        {move || if is_creating.get() { "Creating..." } else { "Create Program" }}
                    </button>
                </div>
            </div>
        </div>
    }
}

/// Open Program Modal - Select a program to open
#[component]
pub fn OpenProgramModal(
    on_close: impl Fn() + 'static + Clone + Send,
    on_selected: impl Fn(i64) + 'static + Clone + Send,
) -> impl IntoView {
    let (list_programs, programs_state) = use_request::<ListPrograms>();
    let (selected_id, set_selected_id) = signal::<Option<i64>>(None);
    let (has_loaded, set_has_loaded) = signal(false);

    // Load programs on mount - with guard to prevent infinite loop
    Effect::new(move |_| {
        if !has_loaded.get_untracked() {
            set_has_loaded.set(true);
            list_programs(ListPrograms);
        }
    });

    let programs = Memo::new(move |_| {
        programs_state.get().data
            .map(|r| r.programs)
            .unwrap_or_default()
    });

    let on_close_clone = on_close.clone();

    view! {
        <div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
            <div class="bg-[#111111] border border-[#ffffff10] rounded-lg w-[400px] max-h-[500px] flex flex-col">
                // Header
                <div class="flex items-center justify-between p-3 border-b border-[#ffffff08]">
                    <h2 class="text-sm font-semibold text-white flex items-center">
                        <svg class="w-4 h-4 mr-2 text-[#00d9ff]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/>
                        </svg>
                        "Open Program"
                    </h2>
                    <button
                        class="text-[#666666] hover:text-white"
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
                                <div class="text-center py-8 text-[#555555] text-[10px]">
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
                                                "bg-[#00d9ff10] border-[#00d9ff40] text-white"
                                            } else {
                                                "bg-[#0a0a0a] border-[#ffffff08] text-[#888888] hover:border-[#ffffff20]"
                                            }
                                        )}
                                        on:click=move |_| set_selected_id.set(Some(prog_id))
                                    >
                                        <div class="font-medium text-[10px] mb-0.5">{prog_name}</div>
                                        <div class="text-[#555555]">{lines_str}</div>
                                    </button>
                                }
                            }).collect_view())
                        }
                    }}
                </div>

                // Footer
                <div class="flex justify-end gap-2 p-3 border-t border-[#ffffff08]">
                    <button
                        class="bg-[#1a1a1a] border border-[#ffffff08] text-[#888888] hover:text-white text-[10px] px-3 py-1.5 rounded"
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
                            if selected_id.get().is_some() {
                                "bg-[#00d9ff20] border border-[#00d9ff40] text-[#00d9ff] hover:bg-[#00d9ff30]"
                            } else {
                                "bg-[#111111] border border-[#ffffff08] text-[#555555] cursor-not-allowed"
                            }
                        )}
                        disabled=move || selected_id.get().is_none()
                        on:click=move |_| {
                            if let Some(id) = selected_id.get() {
                                on_selected(id);
                            }
                        }
                    >
                        "Open"
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
            <div class="bg-[#111111] border border-[#ffffff10] rounded-lg w-[400px] flex flex-col">
                // Header
                <div class="flex items-center justify-between p-3 border-b border-[#ffffff08]">
                    <h2 class="text-sm font-semibold text-white">"Save As"</h2>
                    <button
                        class="text-[#666666] hover:text-white"
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
                    <label class="block text-[9px] text-[#888888] mb-1">"New Program Name"</label>
                    <input
                        type="text"
                        placeholder="Enter new name..."
                        class="w-full bg-[#0a0a0a] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                        prop:value=move || program_name.get()
                        on:input=move |ev| set_program_name.set(event_target_value(&ev))
                    />
                </div>

                // Footer
                <div class="flex justify-end gap-2 p-3 border-t border-[#ffffff08]">
                    <button
                        class="bg-[#1a1a1a] border border-[#ffffff08] text-[#888888] hover:text-white text-[10px] px-3 py-1.5 rounded"
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
                                "bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] hover:bg-[#22c55e30]"
                            } else {
                                "bg-[#111111] border border-[#ffffff08] text-[#555555] cursor-not-allowed"
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
    let (has_processed, set_has_processed) = signal(false);
    let on_close_clone = on_close.clone();

    let (upload_csv, upload_state) = use_request::<UploadCsv>();

    // Watch for upload response - with guard to prevent re-processing
    let on_uploaded_clone = on_uploaded.clone();
    Effect::new(move |_| {
        if has_processed.get_untracked() {
            return;
        }
        if let Some(response) = upload_state.get().data {
            set_has_processed.set(true);
            if response.success {
                on_uploaded_clone();
            } else {
                set_error_message.set(response.error);
                // Allow retrying after an error
                set_has_processed.set(false);
            }
        }
    });

    let is_uploading = Memo::new(move |_| upload_state.get().is_loading);

    view! {
        <div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
            <div class="bg-[#111111] border border-[#ffffff10] rounded-lg w-[400px] flex flex-col">
                // Header
                <div class="flex items-center justify-between p-3 border-b border-[#ffffff08]">
                    <h2 class="text-sm font-semibold text-white flex items-center">
                        <svg class="w-4 h-4 mr-2 text-[#00d9ff]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"/>
                        </svg>
                        "Upload CSV"
                    </h2>
                    <button
                        class="text-[#666666] hover:text-white"
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
                        <div class="bg-[#ff444420] border border-[#ff444440] rounded p-2 flex items-start gap-2">
                            <svg class="w-4 h-4 text-[#ff4444] flex-shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                            </svg>
                            <span class="text-[10px] text-[#ff4444]">{move || error_message.get().unwrap_or_default()}</span>
                        </div>
                    </Show>
                    <div class="border-2 border-dashed border-[#ffffff10] rounded-lg p-6 text-center relative">
                        <svg class="w-8 h-8 mx-auto mb-2 text-[#555555]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"/>
                        </svg>
                        {move || if let Some(name) = file_name.get() {
                            view! { <p class="text-[10px] text-[#00d9ff]">{name}</p> }.into_any()
                        } else {
                            view! { <p class="text-[10px] text-[#555555]">"Drop CSV file here or click to browse"</p> }.into_any()
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
                    <p class="text-[8px] text-[#555555]">
                        "CSV should have columns: X, Y, Z, W (optional), P (optional), R (optional), Speed (optional)"
                    </p>
                </div>

                // Footer
                <div class="flex justify-end gap-2 p-3 border-t border-[#ffffff08]">
                    <button
                        class="bg-[#1a1a1a] border border-[#ffffff08] text-[#888888] hover:text-white text-[10px] px-3 py-1.5 rounded"
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
                            if csv_content.get().is_some() && !is_uploading.get() {
                                "bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] hover:bg-[#22c55e30]"
                            } else {
                                "bg-[#111111] border border-[#ffffff08] text-[#555555] cursor-not-allowed"
                            }
                        )}
                        disabled=move || csv_content.get().is_none() || is_uploading.get()
                        on:click=move |_| {
                            if let Some(content) = csv_content.get() {
                                upload_csv(UploadCsv {
                                    program_id,
                                    csv_content: content,
                                    start_position: None,
                                });
                            }
                        }
                    >
                        {move || if is_uploading.get() { "Uploading..." } else { "Upload" }}
                    </button>
                </div>
            </div>
        </div>
    }
}
