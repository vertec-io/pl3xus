//! Fixed version of program display without component macro limitations
use leptos::prelude::*;
use pl3xus_client::use_mutation;
use robot_hmi_plugins::{
    Instruction, InstructionSequence, ProgramDetail, RemoveSequence, SequenceType, UploadCsv,
};
use crate::components::use_toast;
use leptos::web_sys;

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

    let (show_csv_upload, set_show_csv_upload) = signal::<Option<i64>>(None);

    let approach_seq_count = program.approach_sequences.len();
    let main_seq_count = 1;
    let retreat_seq_count = program.retreat_sequences.len();

    let approach_lines: usize = program
        .approach_sequences
        .iter()
        .map(|s| s.instructions.len())
        .sum();
    let main_lines = program.main_sequence.instructions.len();
    let retreat_lines: usize = program
        .retreat_sequences
        .iter()
        .map(|s| s.instructions.len())
        .sum();
    let total_lines = approach_lines + main_lines + retreat_lines;

    let remove_sequence = use_mutation::<RemoveSequence>(move |result| match result {
        Ok(r) if r.success => toast.success("Sequence deleted"),
        Ok(r) => toast.error(format!(
            "Delete failed: {}",
            r.error.as_deref().unwrap_or("")
        )),
        Err(e) => toast.error(format!("Error: {e}")),
    });

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
                    <span class="text-foreground">{approach_seq_count}" seq, "{approach_lines}" lines"</span>
                </div>
                <div class="flex items-center gap-1">
                    <span class="w-1.5 h-1.5 rounded-full bg-green-500"></span>
                    <span class="text-muted-foreground">"Main:"</span>
                    <span class="text-foreground">{main_seq_count}" seq, "{main_lines}" lines"</span>
                </div>
                <div class="flex items-center gap-1">
                    <span class="w-1.5 h-1.5 rounded-full bg-orange-500"></span>
                    <span class="text-muted-foreground">"Retreat:"</span>
                    <span class="text-foreground">{retreat_seq_count}" seq, "{retreat_lines}" lines"</span>
                </div>
                <div class="flex items-center gap-1 ml-auto">
                    <span class="text-muted-foreground">"Total:"</span>
                    <span class="text-primary font-medium">{total_lines}" lines"</span>
                </div>
            </div>

            // Content
            <div class="flex-1 overflow-y-auto p-3 space-y-4">
                <div class="text-center py-8 text-muted-foreground">
                    <p>"Program sequences will be displayed here"</p>
                    <p class="text-[9px] mt-2">"Approach: "{approach_seq_count}" | Main: "{main_seq_count}" | Retreat: "{retreat_seq_count}</p>
                </div>
            </div>
        </div>
    }
}
