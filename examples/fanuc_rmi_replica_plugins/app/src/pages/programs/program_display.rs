//! Program display component with multi-sequence architecture.
//!
//! This component provides:
//! - Approach Sequences section (multiple approach sequences with inline editing)
//! - Main Program section (multiple main sequences)
//! - Retreat Sequences section (multiple retreat sequences)
//! - Stats display showing sequence and line counts
//! - CSV upload per sequence (in edit mode)
//! - Inline point editing with +/- row buttons

use crate::components::use_toast;
use fanuc_replica_plugins::{
    AddSequence, Instruction, InstructionSequence, ProgramDetail, RemoveSequence, SequenceType,
    UpdateSequenceInstructions, UploadCsv,
};
use leptos::prelude::*;
use leptos::web_sys;
use pl3xus_client::use_mutation;

/// Main program display component with multi-sequence support.
#[component]
pub fn ProgramDisplay(
    program: ProgramDetail,
    on_close: impl Fn() + 'static + Clone + Send + Sync,
) -> impl IntoView {
    let toast = use_toast();
    let prog_name = program.name.clone();
    let prog_desc = program.description.clone();
    let program_id = program.id;

    // State for CSV upload modal
    let (show_csv_upload, set_show_csv_upload) = signal::<Option<i64>>(None);

    // Calculate stats
    let approach_seq_count = program.approach_sequences.len();
    let main_seq_count = program.main_sequences.len();
    let retreat_seq_count = program.retreat_sequences.len();

    let approach_lines: usize = program
        .approach_sequences
        .iter()
        .map(|s| s.instructions.len())
        .sum();
    let main_lines: usize = program
        .main_sequences
        .iter()
        .map(|s| s.instructions.len())
        .sum();
    let retreat_lines: usize = program
        .retreat_sequences
        .iter()
        .map(|s| s.instructions.len())
        .sum();
    let total_lines = approach_lines + main_lines + retreat_lines;

    // Remove sequence mutation
    let remove_sequence = use_mutation::<RemoveSequence>(move |result| match result {
        Ok(r) if r.success => toast.success("Sequence deleted"),
        Ok(r) => toast.error(format!(
            "Delete failed: {}",
            r.error.as_deref().unwrap_or("")
        )),
        Err(e) => toast.error(format!("Error: {e}")),
    });

    // Add sequence mutation
    let add_sequence = use_mutation::<AddSequence>(move |result| match result {
        Ok(r) if r.success => toast.success("Sequence added"),
        Ok(r) => toast.error(format!(
            "Add failed: {}",
            r.error.as_deref().unwrap_or("")
        )),
        Err(e) => toast.error(format!("Error: {e}")),
    });

    let handle_delete = move |seq_id: i64| {
        remove_sequence.send(RemoveSequence { sequence_id: seq_id });
    };

    let handle_csv_upload = move |seq_id: i64| {
        set_show_csv_upload.set(Some(seq_id));
    };

    // Store default instructions in signals so they can be captured by Copy closures
    let default_approach_instr = StoredValue::new(
        program.main_sequences.first()
            .and_then(|s| s.instructions.first().cloned())
            .unwrap_or_else(|| Instruction {
                line_number: 1,
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: None,
                p: None,
                r: None,
                ext1: None,
                ext2: None,
                ext3: None,
                speed: Some(100.0),
                term_type: Some("FINE".to_string()),
                term_value: None,
            })
    );

    let default_retreat_instr = StoredValue::new(
        program.main_sequences.last()
            .and_then(|s| s.instructions.last().cloned())
            .unwrap_or_else(|| Instruction {
                line_number: 1,
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: None,
                p: None,
                r: None,
                ext1: None,
                ext2: None,
                ext3: None,
                speed: Some(100.0),
                term_type: Some("FINE".to_string()),
                term_value: None,
            })
    );

    let handle_add_approach = move || {
        add_sequence.send(AddSequence {
            program_id,
            sequence_type: SequenceType::Approach,
            name: None,
            instructions: vec![default_approach_instr.get_value()],
            csv_content: None,
        });
    };

    let handle_add_main = move || {
        add_sequence.send(AddSequence {
            program_id,
            sequence_type: SequenceType::Main,
            name: None,
            instructions: vec![],
            csv_content: None,
        });
    };

    let handle_add_retreat = move || {
        add_sequence.send(AddSequence {
            program_id,
            sequence_type: SequenceType::Retreat,
            name: None,
            instructions: vec![default_retreat_instr.get_value()],
            csv_content: None,
        });
    };

    let can_delete_main = program.main_sequences.len() > 1;

    view! {
        <div class="flex-1 bg-card rounded-lg border border-border/10 overflow-hidden flex flex-col">
            // Header
            <div class="p-3 border-b border-border/8">
                <div class="flex items-start justify-between">
                    <div>
                        <h2 class="text-sm font-semibold text-foreground">{prog_name}</h2>
                        <p class="text-muted-foreground text-[9px] mt-0.5">
                            {prog_desc.unwrap_or_else(|| "No description".to_string())}
                        </p>
                    </div>
                    <div class="flex gap-1">
                        <button
                            class="bg-destructive/15 border border-destructive/25 text-destructive text-[9px] px-2 py-1 rounded hover:bg-destructive/20"
                            on:click={
                                let on_close = on_close.clone();
                                move |_| on_close()
                            }
                        >
                            "Close"
                        </button>
                    </div>
                </div>
            </div>

            // Stats bar
            <div class="px-3 py-2 border-b border-border/8 flex gap-4 text-[9px] bg-card/50">
                <div class="flex items-center gap-1">
                    <span class="w-1.5 h-1.5 rounded-full bg-blue-500"></span>
                    <span class="text-muted-foreground">"Approach:"</span>
                    <span class="text-foreground">
                        {approach_seq_count}" seq, "{approach_lines}" lines"
                    </span>
                </div>
                <div class="flex items-center gap-1">
                    <span class="w-1.5 h-1.5 rounded-full bg-green-500"></span>
                    <span class="text-muted-foreground">"Main:"</span>
                    <span class="text-foreground">
                        {main_seq_count}" seq, "{main_lines}" lines"
                    </span>
                </div>
                <div class="flex items-center gap-1">
                    <span class="w-1.5 h-1.5 rounded-full bg-orange-500"></span>
                    <span class="text-muted-foreground">"Retreat:"</span>
                    <span class="text-foreground">
                        {retreat_seq_count}" seq, "{retreat_lines}" lines"
                    </span>
                </div>
                <div class="flex items-center gap-1 ml-auto">
                    <span class="text-muted-foreground">"Total:"</span>
                    <span class="text-primary font-medium">{total_lines}" lines"</span>
                </div>
            </div>

            // Scrollable content area with all sequences
            <div class="flex-1 overflow-y-auto p-3 space-y-4">
                // Approach Sequences Section
                <SequenceGroupView
                    title="Approach Sequences"
                    color="blue"
                    sequences=program.approach_sequences.clone()
                    program_id=program_id
                    sequence_type=SequenceType::Approach
                    default_point=program.main_sequences.first()
                        .and_then(|s| s.instructions.first().cloned())
                    can_delete_sequence=true
                    on_delete_sequence=handle_delete
                    on_csv_upload=handle_csv_upload
                    show_add_button=true
                    on_add_sequence=handle_add_approach
                />

                // Main Program Section
                <SequenceGroupView
                    title="Main Program"
                    color="green"
                    sequences=program.main_sequences.clone()
                    program_id=program_id
                    sequence_type=SequenceType::Main
                    default_point=None
                    can_delete_sequence=can_delete_main
                    on_delete_sequence=handle_delete
                    on_csv_upload=handle_csv_upload
                    show_add_button=true
                    on_add_sequence=handle_add_main
                />

                // Retreat Sequences Section
                <SequenceGroupView
                    title="Retreat Sequences"
                    color="orange"
                    sequences=program.retreat_sequences.clone()
                    program_id=program_id
                    sequence_type=SequenceType::Retreat
                    default_point=program.main_sequences.last()
                        .and_then(|s| s.instructions.last().cloned())
                    can_delete_sequence=true
                    on_delete_sequence=handle_delete
                    on_csv_upload=handle_csv_upload
                    show_add_button=true
                    on_add_sequence=handle_add_retreat
                />
            </div>

            // CSV Upload Modal (per-sequence)
            <Show when=move || show_csv_upload.get().is_some()>
                {move || {
                    show_csv_upload.get().map(|seq_id| {
                        view! {
                            <SequenceCSVUploadModal
                                program_id=program_id
                                sequence_id=seq_id
                                on_close=move || set_show_csv_upload.set(None)
                                on_uploaded=move || {
                                    set_show_csv_upload.set(None);
                                }
                            />
                        }
                    })
                }}
            </Show>
        </div>
    }
}

/// Display a group of sequences with all functionality
#[component]
fn SequenceGroupView(
    title: &'static str,
    color: &'static str,
    sequences: Vec<InstructionSequence>,
    program_id: i64,
    sequence_type: SequenceType,
    default_point: Option<Instruction>,
    can_delete_sequence: bool,
    on_delete_sequence: impl Fn(i64) + 'static + Copy + Send + Sync,
    on_csv_upload: impl Fn(i64) + 'static + Copy + Send + Sync,
    #[prop(default = false)] show_add_button: bool,
    on_add_sequence: impl Fn() + 'static + Copy + Send + Sync,
) -> impl IntoView {
    let (header_bg, border_color, dot_color) = match color {
        "blue" => ("bg-blue-500/5", "border-blue-500/20", "bg-blue-500"),
        "orange" => ("bg-orange-500/5", "border-orange-500/20", "bg-orange-500"),
        _ => ("bg-green-500/5", "border-green-500/20", "bg-green-500"),
    };

    let is_empty = sequences.is_empty();
    let (all_collapsed, set_all_collapsed) = signal(false);

    view! {
        <div class={format!("rounded-lg border {} overflow-hidden", border_color)}>
            // Section header
            <div class={format!("{} px-3 py-2 flex items-center justify-between", header_bg)}>
                <div class="flex items-center gap-2">
                    <span class={format!("w-2 h-2 rounded-full {}", dot_color)}></span>
                    <span class="text-[11px] font-semibold text-foreground">{title}</span>
                    <span class="text-[9px] text-muted-foreground">"("{sequences.len()}" sequences)"</span>
                </div>

                // Action buttons
                <div class="flex items-center gap-1">
                    <Show when=move || !is_empty>
                        <button
                            class="bg-card border border-border/10 text-muted-foreground text-[8px] px-2 py-0.5 rounded hover:text-foreground"
                            title={move || if all_collapsed.get() { "Expand All" } else { "Collapse All" }}
                            on:click=move |_| set_all_collapsed.update(|v| *v = !*v)
                        >
                            {move || if all_collapsed.get() { "▼ Expand All" } else { "▲ Collapse All" }}
                        </button>
                    </Show>
                    <Show when=move || show_add_button>
                        <button
                            class="bg-[#22c55e20] border border-[#22c55e40] text-success text-[8px] px-2 py-0.5 rounded hover:bg-success/20"
                            on:click=move |_| on_add_sequence()
                        >
                            "+ Add Sequence"
                        </button>
                    </Show>
                </div>
            </div>

            // Sequences list
            <div class="p-2">
                {if is_empty {
                    view! {
                        <div class="text-center py-4 text-[9px] text-muted-foreground italic">
                            "No sequences"
                        </div>
                    }
                    .into_any()
                } else {
                    view! {
                        <div class="space-y-2">
                            {sequences
                                .into_iter()
                                .map(|seq| {
                                    let seq_id = seq.id;
                                    let seq_name = seq.name.clone().unwrap_or_else(|| {
                                        match sequence_type {
                                            SequenceType::Approach => {
                                                format!("Approach {}", seq.order_index + 1)
                                            }
                                            SequenceType::Main => {
                                                format!("Main {}", seq.order_index + 1)
                                            }
                                            SequenceType::Retreat => {
                                                format!("Retreat {}", seq.order_index + 1)
                                            }
                                        }
                                    });

                                    view! {
                                        <SequenceItemView
                                            sequence=seq
                                            seq_name=seq_name
                                            seq_id=seq_id
                                            color=color
                                            can_delete=can_delete_sequence
                                            program_id=program_id
                                            default_point=default_point.clone()
                                            on_delete=move || on_delete_sequence(seq_id)
                                            on_csv_upload=move || on_csv_upload(seq_id)
                                            external_collapse=all_collapsed
                                        />
                                    }
                                })
                                .collect_view()}
                        </div>
                    }
                    .into_any()
                }}
            </div>
        </div>
    }
}

/// Display and edit a single sequence
#[component]
fn SequenceItemView(
    sequence: InstructionSequence,
    seq_name: String,
    seq_id: i64,
    color: &'static str,
    can_delete: bool,
    program_id: i64,
    default_point: Option<Instruction>,
    on_delete: impl Fn() + 'static + Clone + Send + Sync,
    on_csv_upload: impl Fn() + 'static + Clone + Send + Sync,
    #[prop(optional)] external_collapse: Option<ReadSignal<bool>>,
) -> impl IntoView {
    let toast = use_toast();
    let (is_editing, set_is_editing) = signal(false);
    let (is_collapsed, set_is_collapsed) = signal(false);
    let (instructions, set_instructions) = signal(sequence.instructions.clone());
    let (show_delete_confirm, set_show_delete_confirm) = signal(false);

    // Sync with external collapse signal if provided
    if let Some(ext_collapse) = external_collapse {
        Effect::new(move || {
            set_is_collapsed.set(ext_collapse.get());
        });
    }

    let original_instructions = sequence.instructions.clone();
    let on_delete_stored = store_value(on_delete);
    let on_csv_upload_stored = store_value(on_csv_upload);
    let default_point_stored = store_value(default_point);

    // Mutation to update sequence instructions
    let update_instructions = use_mutation::<UpdateSequenceInstructions>(move |result| {
        match result {
            Ok(r) if r.success => {
                toast.success("Sequence updated");
                set_is_editing.set(false);
            }
            Ok(r) => toast.error(format!(
                "Update failed: {}",
                r.error.as_deref().unwrap_or("")
            )),
            Err(e) => toast.error(format!("Error: {e}")),
        }
    });

    let (header_bg, header_border, dot_color) = match color {
        "blue" => ("bg-blue-500/10", "border-blue-500/30", "bg-blue-500"),
        "orange" => ("bg-orange-500/10", "border-orange-500/30", "bg-orange-500"),
        _ => ("bg-green-500/10", "border-green-500/30", "bg-green-500"),
    };

    view! {
        <div class="rounded border border-border/10 mb-2 overflow-hidden">
            // Header row
            <div
                class={format!(
                    "{} {} border-b cursor-pointer flex items-center justify-between px-2 py-1.5",
                    header_bg,
                    header_border
                )}
                on:click=move |_| set_is_collapsed.update(|v| *v = !*v)
            >
                <div class="flex items-center gap-2">
                    <svg
                        class={move || format!(
                            "w-3 h-3 text-muted-foreground transition-transform {}",
                            if is_collapsed.get() { "" } else { "rotate-90" }
                        )}
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                    >
                        <path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            stroke-width="2"
                            d="M9 5l7 7-7 7"
                        />
                    </svg>
                    <span class={format!("w-2 h-2 rounded-full {}", dot_color)}></span>
                    <span class="text-[10px] font-medium text-foreground">{seq_name}</span>
                    <span class="text-[8px] text-muted-foreground">
                        "("{move || instructions.get().len()}" pts)"
                    </span>
                </div>

                // Action buttons
                <div class="flex items-center gap-1" on:click=|e| e.stop_propagation()>
                    <Show when=move || is_editing.get()>
                        <button
                            class="bg-[#22c55e20] border border-[#22c55e40] text-success text-[8px] px-2 py-0.5 rounded hover:bg-success/20"
                            disabled=move || update_instructions.is_loading()
                            on:click=move |_| {
                                update_instructions.send(UpdateSequenceInstructions {
                                    sequence_id: seq_id,
                                    instructions: instructions.get(),
                                });
                            }
                        >
                            {move || if update_instructions.is_loading() { "Saving..." } else { "Apply" }}
                        </button>
                        <button
                            class="bg-[#00d9ff20] border border-[#00d9ff40] text-primary text-[8px] px-2 py-0.5 rounded hover:bg-primary/20"
                            on:click=move |_| on_csv_upload_stored.with_value(|cb| cb())
                        >
                            "⬆ CSV"
                        </button>
                        <button
                            class="bg-card border border-border/10 text-muted-foreground text-[8px] px-2 py-0.5 rounded hover:text-foreground"
                            on:click={
                                let orig = original_instructions.clone();
                                move |_| {
                                    set_is_editing.set(false);
                                    set_instructions.set(orig.clone());
                                }
                            }
                        >
                            "Cancel"
                        </button>
                    </Show>
                    <Show when=move || !is_editing.get()>
                        <button
                            class="bg-card border border-border/10 text-muted-foreground text-[8px] px-2 py-0.5 rounded hover:text-foreground"
                            on:click=move |_| set_is_editing.set(true)
                        >
                            "Edit"
                        </button>
                        <Show when=move || can_delete>
                            <button
                                class="bg-destructive/15 border border-destructive/25 text-destructive text-[8px] px-2 py-0.5 rounded hover:bg-destructive/20"
                                on:click=move |_| set_show_delete_confirm.set(true)
                            >
                                "Delete"
                            </button>
                        </Show>
                    </Show>
                </div>
            </div>

            // Table body
            <Show when=move || !is_collapsed.get()>
                <div class="max-h-[200px] overflow-y-auto">
                    {move || {
                        let instrs = instructions.get();
                        if instrs.is_empty() {
                            view! {
                                <div class="p-2 text-center text-[9px] text-muted-foreground italic">
                                    "No instructions"
                                </div>
                            }
                            .into_any()
                        } else {
                            view! {
                                <table class="w-full text-[9px]">
                                    <thead class="bg-card/50 sticky top-0">
                                        <tr class="text-muted-foreground">
                                            <th class="px-1 py-1 text-left w-8">"#"</th>
                                            <th class="px-1 py-1 text-right">"X"</th>
                                            <th class="px-1 py-1 text-right">"Y"</th>
                                            <th class="px-1 py-1 text-right">"Z"</th>
                                            <th class="px-1 py-1 text-right">"W"</th>
                                            <th class="px-1 py-1 text-right">"P"</th>
                                            <th class="px-1 py-1 text-right">"R"</th>
                                            <th class="px-1 py-1 text-right">"Speed"</th>
                                            <Show when=move || is_editing.get()>
                                                <th class="px-1 py-1 w-12"></th>
                                            </Show>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {move || {
                                            let instrs = instructions.get();
                                            let can_delete_row = instrs.len() > 1;
                                            instrs
                                                .into_iter()
                                                .enumerate()
                                                .map(|(idx, instr)| {
                                                    let row_idx = idx;
                                                    let default_for_row = default_point_stored.get_value();

                                                    view! {
                                                        <tr class="border-b border-border/5 hover:bg-card/30">
                                                            <td class="px-1 py-0.5 text-muted-foreground">{idx + 1}</td>
                                                            <EditableCell
                                                                value=instr.x
                                                                is_editing=is_editing
                                                                on_change=move |val| {
                                                                    let mut new_instrs = instructions.get();
                                                                    new_instrs[row_idx].x = val;
                                                                    set_instructions.set(new_instrs);
                                                                }
                                                            />
                                                            <EditableCell
                                                                value=instr.y
                                                                is_editing=is_editing
                                                                on_change=move |val| {
                                                                    let mut new_instrs = instructions.get();
                                                                    new_instrs[row_idx].y = val;
                                                                    set_instructions.set(new_instrs);
                                                                }
                                                            />
                                                            <EditableCell
                                                                value=instr.z
                                                                is_editing=is_editing
                                                                on_change=move |val| {
                                                                    let mut new_instrs = instructions.get();
                                                                    new_instrs[row_idx].z = val;
                                                                    set_instructions.set(new_instrs);
                                                                }
                                                            />
                                                            <EditableCell
                                                                value=instr.w.unwrap_or(0.0)
                                                                is_editing=is_editing
                                                                is_optional=true
                                                                on_change=move |val| {
                                                                    let mut new_instrs = instructions.get();
                                                                    new_instrs[row_idx].w = Some(val);
                                                                    set_instructions.set(new_instrs);
                                                                }
                                                            />
                                                            <EditableCell
                                                                value=instr.p.unwrap_or(0.0)
                                                                is_editing=is_editing
                                                                is_optional=true
                                                                on_change=move |val| {
                                                                    let mut new_instrs = instructions.get();
                                                                    new_instrs[row_idx].p = Some(val);
                                                                    set_instructions.set(new_instrs);
                                                                }
                                                            />
                                                            <EditableCell
                                                                value=instr.r.unwrap_or(0.0)
                                                                is_editing=is_editing
                                                                is_optional=true
                                                                on_change=move |val| {
                                                                    let mut new_instrs = instructions.get();
                                                                    new_instrs[row_idx].r = Some(val);
                                                                    set_instructions.set(new_instrs);
                                                                }
                                                            />
                                                            <EditableCell
                                                                value=instr.speed.unwrap_or(0.0)
                                                                is_editing=is_editing
                                                                is_optional=true
                                                                on_change=move |val| {
                                                                    let mut new_instrs = instructions.get();
                                                                    new_instrs[row_idx].speed = Some(val);
                                                                    set_instructions.set(new_instrs);
                                                                }
                                                            />
                                                            <Show when=move || is_editing.get()>
                                                                <td class="px-1 py-0.5">
                                                                    <div class="flex gap-0.5 justify-end">
                                                                        <button
                                                                            class="w-4 h-4 rounded bg-[#22c55e20] text-success hover:bg-success/30 flex items-center justify-center text-[8px]"
                                                                            title="Add row below"
                                                                            on:click={
                                                                                let default_for_btn = default_for_row.clone();
                                                                                move |_| {
                                                                                let mut new_instrs = instructions.get();
                                                                                let new_instr = default_for_btn.clone().unwrap_or_else(|| Instruction {
                                                                                    line_number: (row_idx + 2) as i32,
                                                                                    x: 0.0, y: 0.0, z: 0.0,
                                                                                    w: None, p: None, r: None,
                                                                                    ext1: None, ext2: None, ext3: None,
                                                                                    speed: None,
                                                                                    term_type: Some("FINE".to_string()),
                                                                                    term_value: None,
                                                                                });
                                                                                new_instrs.insert(row_idx + 1, new_instr);
                                                                                set_instructions.set(new_instrs);
                                                                            }
                                                                            }
                                                                        >
                                                                            "+"
                                                                        </button>
                                                                        <Show when=move || can_delete_row>
                                                                            <button
                                                                                class="w-4 h-4 rounded bg-destructive/15 text-destructive hover:bg-destructive/25 flex items-center justify-center text-[8px]"
                                                                                title="Delete row"
                                                                                on:click=move |_| {
                                                                                    let mut new_instrs = instructions.get();
                                                                                    new_instrs.remove(row_idx);
                                                                                    set_instructions.set(new_instrs);
                                                                                }
                                                                            >
                                                                                "-"
                                                                            </button>
                                                                        </Show>
                                                                    </div>
                                                                </td>
                                                            </Show>
                                                        </tr>
                                                    }
                                                })
                                                .collect_view()
                                        }}
                                    </tbody>
                                </table>
                            }
                            .into_any()
                        }
                    }}
                </div>
            </Show>

            // Delete confirmation modal
            <Show when=move || show_delete_confirm.get()>
                <div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
                    <div class="bg-card border border-border/10 rounded-lg p-4 max-w-sm">
                        <h3 class="text-sm font-semibold text-foreground mb-2">"Delete Sequence?"</h3>
                        <p class="text-[10px] text-muted-foreground mb-4">
                            "Are you sure you want to delete this sequence? This action cannot be undone."
                        </p>
                        <div class="flex justify-end gap-2">
                            <button
                                class="bg-card border border-border/10 text-muted-foreground text-[10px] px-3 py-1.5 rounded hover:text-foreground"
                                on:click=move |_| set_show_delete_confirm.set(false)
                            >
                                "Cancel"
                            </button>
                            <button
                                class="bg-destructive/15 border border-destructive/25 text-destructive text-[10px] px-3 py-1.5 rounded hover:bg-destructive/20"
                                on:click=move |_| {
                                    on_delete_stored.with_value(|cb| cb());
                                    set_show_delete_confirm.set(false);
                                }
                            >
                                "Delete"
                            </button>
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    }
}

/// CSV Upload Modal for a specific sequence
#[component]
fn SequenceCSVUploadModal(
    program_id: i64,
    sequence_id: i64,
    on_close: impl Fn() + 'static + Clone + Send + Sync,
    on_uploaded: impl Fn() + 'static + Clone + Send + Sync,
) -> impl IntoView {
    let (file_name, set_file_name) = signal::<Option<String>>(None);
    let (csv_content, set_csv_content) = signal::<Option<String>>(None);
    let (error_message, set_error_message) = signal::<Option<String>>(None);

    let upload_csv = use_mutation::<UploadCsv>({
        let on_uploaded_clone = on_uploaded.clone();
        move |result| match result {
            Ok(r) if r.success => on_uploaded_clone(),
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
                        <svg
                            class="w-4 h-4 mr-2 text-primary"
                            fill="none"
                            stroke="currentColor"
                            viewBox="0 0 24 24"
                        >
                            <path
                                stroke-linecap="round"
                                stroke-linejoin="round"
                                stroke-width="2"
                                d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"
                            />
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
                        <svg
                            class="w-4 h-4"
                            fill="none"
                            stroke="currentColor"
                            viewBox="0 0 24 24"
                        >
                            <path
                                stroke-linecap="round"
                                stroke-linejoin="round"
                                stroke-width="2"
                                d="M6 18L18 6M6 6l12 12"
                            />
                        </svg>
                    </button>
                </div>

                // Content
                <div class="p-3 space-y-3">
                    <Show when=move || error_message.get().is_some()>
                        <div class="bg-destructive/15 border border-destructive/25 rounded p-2 flex items-start gap-2">
                            <svg
                                class="w-4 h-4 text-destructive flex-shrink-0 mt-0.5"
                                fill="none"
                                stroke="currentColor"
                                viewBox="0 0 24 24"
                            >
                                <path
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                    stroke-width="2"
                                    d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                                />
                            </svg>
                            <span class="text-[10px] text-destructive">
                                {move || error_message.get().unwrap_or_default()}
                            </span>
                        </div>
                    </Show>

                    <div class="border-2 border-dashed border-border/10 rounded-lg p-6 text-center relative">
                        <svg
                            class="w-8 h-8 mx-auto mb-2 text-muted-foreground"
                            fill="none"
                            stroke="currentColor"
                            viewBox="0 0 24 24"
                        >
                            <path
                                stroke-linecap="round"
                                stroke-linejoin="round"
                                stroke-width="2"
                                d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"
                            />
                        </svg>
                        {move || {
                            if let Some(name) = file_name.get() {
                                view! { <p class="text-[10px] text-primary">{name}</p> }.into_any()
                            } else {
                                view! {
                                    <p class="text-[10px] text-muted-foreground">
                                        "Drop CSV file here or click to browse"
                                    </p>
                                }
                                .into_any()
                            }
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
                                        let reader = web_sys::FileReader::new().unwrap();
                                        let reader_clone = reader.clone();
                                        let onload = wasm_bindgen::closure::Closure::wrap(Box::new(
                                            move |_: web_sys::Event| {
                                                if let Ok(result) = reader_clone.result() {
                                                    if let Some(text) = result.as_string() {
                                                        set_csv_content.set(Some(text));
                                                    }
                                                }
                                            },
                                        ) as Box<dyn FnMut(_)>);
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
                            let on_close = on_close.clone();
                            move |_| on_close()
                        }
                    >
                        "Cancel"
                    </button>
                    <button
                        class={move || {
                            format!(
                                "text-[10px] px-3 py-1.5 rounded {}",
                                if csv_content.get().is_some() && !upload_csv.is_loading() {
                                    "bg-[#22c55e20] border border-[#22c55e40] text-success hover:bg-success/20"
                                } else {
                                    "bg-card border border-border/8 text-muted-foreground cursor-not-allowed"
                                }
                            )
                        }}
                        disabled=move || csv_content.get().is_none() || upload_csv.is_loading()
                        on:click=move |_| {
                            if let Some(content) = csv_content.get() {
                                upload_csv.send(UploadCsv {
                                    program_id,
                                    csv_content: content,
                                    sequence_id: Some(sequence_id),
                                    sequence_type: None,
                                });
                            }
                        }
                    >
                        {move || {
                            if upload_csv.is_loading() {
                                "Uploading..."
                            } else {
                                "Upload"
                            }
                        }}
                    </button>
                </div>
            </div>
        </div>
    }
}

/// Editable cell component that handles text input with validation
#[component]
fn EditableCell(
    value: f64,
    is_editing: ReadSignal<bool>,
    #[prop(default = false)] is_optional: bool,
    on_change: impl Fn(f64) + 'static + Clone + Send + Sync,
) -> impl IntoView {
    use leptos::html::Input;

    // Use RwSignal for local state to allow updates without re-creating the input
    let (local_value, set_local_value) = signal(format!("{:.2}", value));
    let (is_valid, set_is_valid) = signal(true);
    let input_ref = NodeRef::<Input>::new();

    let text_class = if is_optional {
        "text-muted-foreground"
    } else {
        "text-foreground"
    };

    // Store the callback in StoredValue so it can be called without moving
    let on_change_stored = StoredValue::new(on_change);

    view! {
        <td class=format!("px-1 py-0.5 text-right font-mono {}", text_class)>
            <Show
                when=move || is_editing.get()
                fallback=move || {
                    let display = if is_optional && value == 0.0 {
                        "-".to_string()
                    } else {
                        format!("{:.2}", value)
                    };
                    view! { <span>{display}</span> }
                }
            >
                <input
                    node_ref=input_ref
                    type="text"
                    tabindex="0"
                    class=move || format!(
                        "w-full bg-card border rounded px-1 text-[10px] text-right font-mono {}",
                        if is_valid.get() { "border-border/20" } else { "border-destructive" }
                    )
                    prop:value=move || local_value.get()
                    on:input=move |ev| {
                        let val_str = event_target_value(&ev);
                        set_local_value.set(val_str.clone());

                        // Validate and update parent
                        if let Ok(val) = val_str.parse::<f64>() {
                            set_is_valid.set(true);
                            on_change_stored.with_value(|cb| cb(val));
                        } else {
                            set_is_valid.set(false);
                        }
                    }
                    on:blur=move |_| {
                        // On blur, if invalid, reset to last valid value
                        if !is_valid.get() {
                            set_local_value.set(format!("{:.2}", value));
                            set_is_valid.set(true);
                        }
                    }
                />
            </Show>
        </td>
    }
}
