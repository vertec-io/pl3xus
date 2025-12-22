//! I/O status panel component with tabs for DIN/DOUT/AIN/AOUT/GIN/GOUT.

use leptos::prelude::*;
use leptos::web_sys;

use pl3xus_client::{use_entity_component, use_mutation, use_request};
use fanuc_replica_types::*;
use crate::components::use_toast;
use crate::layout::LayoutContext;
use crate::pages::dashboard::use_system_entity;

/// I/O status panel showing digital/analog/group inputs and outputs.
#[component]
pub fn IoStatusPanel(
    /// Whether to start collapsed. Defaults to true for docked panel, false for floating.
    #[prop(default = true)]
    start_collapsed: bool,
    /// Whether to show the pop-out button. Defaults to true.
    #[prop(default = true)]
    show_popout: bool,
) -> impl IntoView {
    let layout_ctx = use_context::<LayoutContext>().expect("LayoutContext required");
    let ctx = use_system_entity();

    // Subscribe to the active robot's I/O components
    let (io_status, _) = use_entity_component::<IoStatus, _>(move || ctx.robot_entity_id.get());
    let (io_config, _) = use_entity_component::<IoConfigState, _>(move || ctx.robot_entity_id.get());

    let get_io = move || io_status.get();
    let get_config = move || io_config.get();

    // Request hooks for reading I/O
    let (read_din_batch, _) = use_request::<ReadDinBatch>();
    let (read_ain, _) = use_request::<ReadAin>();
    let (read_gin, _) = use_request::<ReadGin>();

    // State
    let (collapsed, set_collapsed) = signal(start_collapsed);
    let (selected_tab, set_selected_tab) = signal::<&'static str>("din");

    // Default ports to display (1-8)
    const DEFAULT_PORTS: [u16; 8] = [1, 2, 3, 4, 5, 6, 7, 8];

    // Helper to get display name for an I/O port
    let get_display_name = move |io_type: &str, port: u16| -> String {
        get_config().get_display_name(io_type, port)
    };

    // Helper to check if a port is visible
    let is_port_visible = move |io_type: &str, port: u16| -> bool {
        get_config().is_port_visible(io_type, port)
    };

    // Refresh I/O data
    let refresh_io = {
        let read_din_batch = read_din_batch.clone();
        let read_ain = read_ain.clone();
        let read_gin = read_gin.clone();
        move || {
            // Read all digital inputs as a batch
            read_din_batch(ReadDinBatch {
                port_numbers: DEFAULT_PORTS.to_vec(),
            });
            // Read analog and group inputs
            for &port in &DEFAULT_PORTS {
                read_ain(ReadAin { port_number: port });
                read_gin(ReadGin { port_number: port });
            }
        }
    };

    // Helper to check if a bit is set in the I/O vector
    let get_bit = |io_vec: &[u16], index: usize| -> bool {
        if index == 0 { return false; }
        let word_index = (index - 1) / 16;
        let bit_index = (index - 1) % 16;
        io_vec.get(word_index).map(|word| (word >> bit_index) & 1 == 1).unwrap_or(false)
    };

    // Tab button class helper
    let tab_class = move |tab: &'static str| {
        format!(
            "flex-1 text-[8px] py-1 rounded transition-colors {}",
            if selected_tab.get() == tab {
                "bg-[#00d9ff20] text-[#00d9ff]"
            } else {
                "bg-[#ffffff05] text-[#666666] hover:text-[#888888]"
            }
        )
    };

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] relative">
            // Header buttons (refresh + pop-out)
            <div class="absolute top-1.5 right-1.5 flex gap-1 z-10">
                // Refresh button
                <button
                    class="p-0.5 hover:bg-[#ffffff10] rounded"
                    title="Refresh I/O"
                    on:click={
                        let refresh_io = refresh_io.clone();
                        move |_| refresh_io()
                    }
                >
                    <svg class="w-3 h-3 text-[#555555] hover:text-[#00d9ff]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/>
                    </svg>
                </button>
                // Pop-out button (only show when not already popped out)
                <Show when=move || show_popout>
                    <button
                        class="p-0.5 hover:bg-[#ffffff10] rounded"
                        title="Pop out I/O panel"
                        on:click=move |_| layout_ctx.io_popped.set(true)
                    >
                        <svg class="w-3 h-3 text-[#555555] hover:text-[#00d9ff]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"/>
                        </svg>
                    </button>
                </Show>
            </div>

            // Collapsible header
            <button
                class="w-full flex items-center justify-between p-2 pr-12 hover:bg-[#ffffff05] transition-colors"
                on:click=move |_| set_collapsed.update(|v| *v = !*v)
            >
                <h3 class="text-[10px] font-semibold text-[#00d9ff] uppercase tracking-wide flex items-center">
                    <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"/>
                    </svg>
                    "I/O"
                </h3>
                <svg
                    class=move || format!("w-3 h-3 text-[#666666] transition-transform {}", if collapsed.get() { "-rotate-90" } else { "" })
                    fill="none" stroke="currentColor" viewBox="0 0 24 24"
                >
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"/>
                </svg>
            </button>

            <Show when=move || !collapsed.get()>
                <div class="px-2 pb-2 space-y-2">
                    // Tab buttons
                    <div class="flex gap-1">
                        <button class=move || tab_class("din") on:click=move |_| set_selected_tab.set("din")>"DIN"</button>
                        <button class=move || tab_class("dout") on:click=move |_| set_selected_tab.set("dout")>"DOUT"</button>
                        <button class=move || tab_class("ain") on:click=move |_| set_selected_tab.set("ain")>"AIN"</button>
                        <button class=move || tab_class("aout") on:click=move |_| set_selected_tab.set("aout")>"AOUT"</button>
                        <button class=move || tab_class("gin") on:click=move |_| set_selected_tab.set("gin")>"GIN"</button>
                        <button class=move || tab_class("gout") on:click=move |_| set_selected_tab.set("gout")>"GOUT"</button>
                    </div>

                    // DIN - Digital Inputs (read only)
                    <Show when=move || selected_tab.get() == "din">
                        <div class="grid grid-cols-4 gap-1">
                            {DEFAULT_PORTS.iter().filter(|&&port| is_port_visible("DIN", port)).map(|&port| {
                                let name = get_display_name("DIN", port);
                                view! {
                                    <IOIndicator
                                        port=port
                                        name=name
                                        value=Signal::derive(move || get_bit(&get_io().digital_inputs, port as usize))
                                    />
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    </Show>

                    // DOUT - Digital Outputs (clickable)
                    <Show when=move || selected_tab.get() == "dout">
                        <div class="grid grid-cols-4 gap-1">
                            {DEFAULT_PORTS.iter().filter(|&&port| is_port_visible("DOUT", port)).map(|&port| {
                                let name = get_display_name("DOUT", port);
                                view! {
                                    <IOButton
                                        port=port
                                        name=name
                                        value=Signal::derive(move || get_bit(&get_io().digital_outputs, port as usize))
                                    />
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    </Show>

                    // AIN - Analog Inputs (read only)
                    <Show when=move || selected_tab.get() == "ain">
                        <div class="grid grid-cols-4 gap-1">
                            {DEFAULT_PORTS.iter().filter(|&&port| is_port_visible("AIN", port)).map(|&port| {
                                let name = get_display_name("AIN", port);
                                view! {
                                    <AnalogIndicator
                                        port=port
                                        name=name
                                        value=Signal::derive(move || get_io().analog_inputs.get(&port).copied().unwrap_or(0.0))
                                    />
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    </Show>

                    // AOUT - Analog Outputs (editable)
                    <Show when=move || selected_tab.get() == "aout">
                        <div class="grid grid-cols-4 gap-1">
                            {DEFAULT_PORTS.iter().filter(|&&port| is_port_visible("AOUT", port)).map(|&port| {
                                let name = get_display_name("AOUT", port);
                                view! {
                                    <AnalogOutput
                                        port=port
                                        name=name
                                        value=Signal::derive(move || get_io().analog_outputs.get(&port).copied().unwrap_or(0.0))
                                    />
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    </Show>

                    // GIN - Group Inputs (read only)
                    <Show when=move || selected_tab.get() == "gin">
                        <div class="grid grid-cols-4 gap-1">
                            {DEFAULT_PORTS.iter().filter(|&&port| is_port_visible("GIN", port)).map(|&port| {
                                let name = get_display_name("GIN", port);
                                view! {
                                    <GroupIndicator
                                        port=port
                                        name=name
                                        value=Signal::derive(move || get_io().group_inputs.get(&port).copied().unwrap_or(0))
                                    />
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    </Show>

                    // GOUT - Group Outputs (editable)
                    <Show when=move || selected_tab.get() == "gout">
                        <div class="grid grid-cols-4 gap-1">
                            {DEFAULT_PORTS.iter().filter(|&&port| is_port_visible("GOUT", port)).map(|&port| {
                                let name = get_display_name("GOUT", port);
                                view! {
                                    <GroupOutput
                                        port=port
                                        name=name
                                        value=Signal::derive(move || get_io().group_outputs.get(&port).copied().unwrap_or(0))
                                    />
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    </Show>
                </div>
            </Show>
        </div>
    }
}


/// Read-only digital I/O indicator (for DIN).
#[component]
fn IOIndicator(
    port: u16,
    name: String,
    value: Signal<bool>,
) -> impl IntoView {
    let display_name = name.clone();
    let title_name = name;
    view! {
        <div
            class=move || format!(
                "flex flex-col items-center justify-center p-1 rounded text-[8px] {}",
                if value.get() { "bg-[#00ff0020] text-[#00ff00]" } else { "bg-[#ffffff05] text-[#555555]" }
            )
            title=format!("DIN[{}]", port)
        >
            <span class="font-mono truncate max-w-full" title=title_name>{display_name}</span>
            <div class=move || format!(
                "w-2 h-2 rounded-full mt-0.5 {}",
                if value.get() { "bg-[#00ff00]" } else { "bg-[#333333]" }
            )/>
        </div>
    }
}

/// Clickable digital I/O button (for DOUT).
#[component]
fn IOButton(
    port: u16,
    name: String,
    value: Signal<bool>,
) -> impl IntoView {
    let toast = use_toast();
    let display_name = name.clone();
    let title_name = name;

    let write_dout = use_mutation::<WriteDout>(move |result| {
        match result {
            Ok(r) if r.success => {} // Silent success
            Ok(r) => toast.error(format!("DOUT write failed: {}", r.error.as_deref().unwrap_or(""))),
            Err(e) => toast.error(format!("DOUT error: {e}")),
        }
    });

    let toggle = move |_| {
        let current = value.get();
        write_dout.send(WriteDout {
            port_number: port,
            port_value: !current,
        });
    };

    view! {
        <button
            class=move || format!(
                "flex flex-col items-center justify-center p-1 rounded text-[8px] cursor-pointer transition-colors {}",
                if value.get() { "bg-[#ff880020] text-[#ff8800] hover:bg-[#ff880030]" } else { "bg-[#ffffff05] text-[#555555] hover:bg-[#ffffff10]" }
            )
            title=format!("DOUT[{}] - Click to toggle", port)
            on:click=toggle
        >
            <span class="font-mono truncate max-w-full" title=title_name>{display_name}</span>
            <div class=move || format!(
                "w-2 h-2 rounded-full mt-0.5 {}",
                if value.get() { "bg-[#ff8800]" } else { "bg-[#333333]" }
            )/>
        </button>
    }
}

/// Read-only analog input indicator (for AIN).
#[component]
fn AnalogIndicator(
    port: u16,
    name: String,
    value: Signal<f64>,
) -> impl IntoView {
    let display_name = name.clone();
    let title_name = name;
    view! {
        <div
            class="flex flex-col items-center justify-center p-1 rounded text-[8px] bg-[#ffffff05]"
            title=format!("AIN[{}]", port)
        >
            <span class="font-mono text-[#00d9ff] truncate max-w-full" title=title_name>{display_name}</span>
            <span class="font-mono text-[#888888] text-[7px]">
                {move || format!("{:.1}", value.get())}
            </span>
        </div>
    }
}

/// Editable analog output (for AOUT).
#[component]
fn AnalogOutput(
    port: u16,
    name: String,
    value: Signal<f64>,
) -> impl IntoView {
    let toast = use_toast();
    let (editing, set_editing) = signal(false);
    let (input_value, set_input_value) = signal(String::new());
    let display_name = name.clone();
    let title_name = name;

    let write_aout = use_mutation::<WriteAout>(move |result| {
        match result {
            Ok(r) if r.success => {} // Silent success
            Ok(r) => toast.error(format!("AOUT write failed: {}", r.error.as_deref().unwrap_or(""))),
            Err(e) => toast.error(format!("AOUT error: {e}")),
        }
    });

    // Inline submit for blur
    let do_blur_submit = move |_| {
        if let Ok(new_val) = input_value.get().parse::<f64>() {
            write_aout.send(WriteAout {
                port_number: port,
                port_value: new_val,
            });
        }
        set_editing.set(false);
    };

    // For keydown
    let do_key_submit = move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Enter" {
            if let Ok(new_val) = input_value.get().parse::<f64>() {
                write_aout.send(WriteAout {
                    port_number: port,
                    port_value: new_val,
                });
            }
            set_editing.set(false);
        }
        if ev.key() == "Escape" { set_editing.set(false); }
    };

    view! {
        <div
            class="flex flex-col items-center justify-center p-1 rounded text-[8px] bg-[#ff880010] cursor-pointer hover:bg-[#ff880020]"
            title=format!("AOUT[{}] - Click to edit", port)
            on:click=move |_| {
                if !editing.get() {
                    set_input_value.set(format!("{:.2}", value.get()));
                    set_editing.set(true);
                }
            }
        >
            <span class="font-mono text-[#ff8800] truncate max-w-full" title=title_name>{display_name}</span>
            {move || {
                if editing.get() {
                    view! {
                        <input
                            type="text"
                            class="w-10 text-[7px] bg-[#1a1a1a] border border-[#ff8800] rounded px-0.5 text-center text-white"
                            prop:value=input_value
                            on:input=move |ev| set_input_value.set(event_target_value(&ev))
                            on:blur=do_blur_submit.clone()
                            on:keydown=do_key_submit.clone()
                            on:click=move |ev| ev.stop_propagation()
                        />
                    }.into_any()
                } else {
                    view! {
                        <span class="font-mono text-[#888888] text-[7px]">
                            {move || format!("{:.1}", value.get())}
                        </span>
                    }.into_any()
                }
            }}
        </div>
    }
}

/// Read-only group input indicator (for GIN).
#[component]
fn GroupIndicator(
    port: u16,
    name: String,
    value: Signal<u32>,
) -> impl IntoView {
    let display_name = name.clone();
    let title_name = name;
    view! {
        <div
            class="flex flex-col items-center justify-center p-1 rounded text-[8px] bg-[#ffffff05]"
            title=format!("GIN[{}]", port)
        >
            <span class="font-mono text-[#00d9ff] truncate max-w-full" title=title_name>{display_name}</span>
            <span class="font-mono text-[#888888] text-[7px]">
                {move || value.get()}
            </span>
        </div>
    }
}

/// Editable group output (for GOUT).
#[component]
fn GroupOutput(
    port: u16,
    name: String,
    value: Signal<u32>,
) -> impl IntoView {
    let toast = use_toast();
    let (editing, set_editing) = signal(false);
    let (input_value, set_input_value) = signal(String::new());
    let display_name = name.clone();
    let title_name = name;

    let write_gout = use_mutation::<WriteGout>(move |result| {
        match result {
            Ok(r) if r.success => {} // Silent success
            Ok(r) => toast.error(format!("GOUT write failed: {}", r.error.as_deref().unwrap_or(""))),
            Err(e) => toast.error(format!("GOUT error: {e}")),
        }
    });

    // Inline submit for blur
    let do_blur_submit = move |_| {
        if let Ok(new_val) = input_value.get().parse::<u32>() {
            write_gout.send(WriteGout {
                port_number: port,
                port_value: new_val,
            });
        }
        set_editing.set(false);
    };

    // For keydown
    let do_key_submit = move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Enter" {
            if let Ok(new_val) = input_value.get().parse::<u32>() {
                write_gout.send(WriteGout {
                    port_number: port,
                    port_value: new_val,
                });
            }
            set_editing.set(false);
        }
        if ev.key() == "Escape" { set_editing.set(false); }
    };

    view! {
        <div
            class="flex flex-col items-center justify-center p-1 rounded text-[8px] bg-[#ff880010] cursor-pointer hover:bg-[#ff880020]"
            title=format!("GOUT[{}] - Click to edit", port)
            on:click=move |_| {
                if !editing.get() {
                    set_input_value.set(value.get().to_string());
                    set_editing.set(true);
                }
            }
        >
            <span class="font-mono text-[#ff8800] truncate max-w-full" title=title_name>{display_name}</span>
            {move || {
                if editing.get() {
                    view! {
                        <input
                            type="text"
                            class="w-10 text-[7px] bg-[#1a1a1a] border border-[#ff8800] rounded px-0.5 text-center text-white"
                            prop:value=input_value
                            on:input=move |ev| set_input_value.set(event_target_value(&ev))
                            on:blur=do_blur_submit.clone()
                            on:keydown=do_key_submit.clone()
                            on:click=move |ev| ev.stop_propagation()
                        />
                    }.into_any()
                } else {
                    view! {
                        <span class="font-mono text-[#888888] text-[7px]">
                            {move || value.get()}
                        </span>
                    }.into_any()
                }
            }}
        </div>
    }
}
