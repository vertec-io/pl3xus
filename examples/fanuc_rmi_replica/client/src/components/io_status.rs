//! I/O status panel component.

use leptos::prelude::*;

use pl3xus_client::{use_sync_component, use_request};
use fanuc_replica_types::*;

/// I/O status panel showing digital/analog inputs and outputs.
#[component]
pub fn IoStatusPanel() -> impl IntoView {
    let io_status = use_sync_component::<IoStatus>();
    let get_io = move || io_status.get().values().next().cloned().unwrap_or_default();

    // Request hook for writing digital outputs
    let (write_dout, _write_dout_state) = use_request::<WriteDout>();
    let write_dout = StoredValue::new(write_dout);

    // Helper to check if a bit is set in the I/O vector
    // Each u16 represents 16 I/O bits, so index i corresponds to word i/16, bit i%16
    let get_bit = |io_vec: &[u16], index: usize| -> bool {
        if index == 0 { return false; }
        let word_index = (index - 1) / 16;
        let bit_index = (index - 1) % 16;
        io_vec.get(word_index).map(|word| (word >> bit_index) & 1 == 1).unwrap_or(false)
    };

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-2">
            <h2 class="text-[10px] font-semibold text-[#00d9ff] mb-1.5 uppercase tracking-wide">"I/O Status"</h2>

            // Digital Inputs (DI 1-8) - Read only
            <div class="mb-2">
                <div class="text-[8px] text-[#666666] mb-1">"Digital Inputs"</div>
                <div class="flex gap-0.5">
                    {move || {
                        let io = get_io();
                        (1..=8).map(|i| {
                            let value = get_bit(&io.digital_inputs, i);
                            view! {
                                <div
                                    class=if value {
                                        "w-4 h-4 rounded text-[8px] flex items-center justify-center bg-[#22c55e] text-white"
                                    } else {
                                        "w-4 h-4 rounded text-[8px] flex items-center justify-center bg-[#333333] text-[#666666]"
                                    }
                                    title=format!("DI{} = {}", i, if value { "ON" } else { "OFF" })
                                >
                                    {i}
                                </div>
                            }
                        }).collect::<Vec<_>>()
                    }}
                </div>
            </div>

            // Digital Outputs (DO 1-8) - Clickable to toggle
            <div>
                <div class="text-[8px] text-[#666666] mb-1">"Digital Outputs"</div>
                <div class="flex gap-0.5">
                    {move || {
                        let io = get_io();
                        (1..=8).map(|i| {
                            let value = get_bit(&io.digital_outputs, i);
                            view! {
                                <button
                                    class=if value {
                                        "w-4 h-4 rounded text-[8px] flex items-center justify-center bg-[#00d9ff] text-black cursor-pointer hover:opacity-80"
                                    } else {
                                        "w-4 h-4 rounded text-[8px] flex items-center justify-center bg-[#333333] text-[#666666] cursor-pointer hover:bg-[#444444]"
                                    }
                                    title=format!("DO{} = {} (click to toggle)", i, if value { "ON" } else { "OFF" })
                                    on:click=move |_| {
                                        // Toggle the output
                                        write_dout.get_value()(WriteDout {
                                            port_number: i as u16,
                                            port_value: !value,
                                        });
                                    }
                                >
                                    {i}
                                </button>
                            }
                        }).collect::<Vec<_>>()
                    }}
                </div>
            </div>
        </div>
    }
}
