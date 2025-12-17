//! Multi Tool Display - Accordion showing detailed tool data.

use leptos::prelude::*;
use pl3xus_client::use_sync_component;
use fanuc_replica_types::ConnectionState;
use crate::pages::dashboard::context::WorkspaceContext;

/// Multi Tool Display - Accordion showing detailed tool geometry for tools 1-10
#[component]
pub fn MultiToolDisplay() -> impl IntoView {
    let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let connection_state = use_sync_component::<ConnectionState>();
    let expanded_tools = ctx.expanded_tools;

    let robot_connected = Memo::new(move |_| {
        connection_state.get().values().next()
            .map(|s| s.robot_connected)
            .unwrap_or(false)
    });

    // TODO: Get tool data from server via request or sync component
    // For now, use placeholder data
    let tool_data = move |_tool_num: i32| -> (f64, f64, f64, f64, f64, f64) {
        (0.0, 0.0, 0.0, 0.0, 0.0, 0.0)
    };

    view! {
        <Show when=move || robot_connected.get()>
            <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-2 flex-1 overflow-y-auto">
                <h3 class="text-[10px] font-semibold text-[#00d9ff] mb-1.5 uppercase tracking-wide flex items-center group">
                    <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/>
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>
                    </svg>
                    "Tool Data"
                </h3>

                <div class="space-y-0.5">
                    {(1..=10).map(|i| {
                        let is_expanded = move || expanded_tools.get().contains(&i);
                        let (x, y, z, w, p, r) = tool_data(i);

                        view! {
                            <div class="border border-[#ffffff08] rounded overflow-hidden">
                                // Header (clickable)
                                <button
                                    class="w-full flex items-center justify-between px-2 py-1 bg-[#111111] hover:bg-[#151515] text-left"
                                    on:click=move |_| {
                                        let mut current = expanded_tools.get();
                                        if current.contains(&i) {
                                            current.remove(&i);
                                        } else {
                                            current.insert(i);
                                        }
                                        expanded_tools.set(current);
                                    }
                                >
                                    <span class="text-[9px] text-[#888888]">
                                        {format!("UTool {}", i)}
                                    </span>
                                    <span class="text-[8px] text-[#555555]">
                                        {move || if is_expanded() { "▼" } else { "▶" }}
                                    </span>
                                </button>

                                // Expanded content
                                <Show when=is_expanded>
                                    <div class="bg-[#0a0a0a] px-2 py-1.5">
                                        <div class="grid grid-cols-3 gap-x-3 gap-y-0.5 text-[8px]">
                                            <div class="flex justify-between">
                                                <span class="text-[#555555]">"X:"</span>
                                                <span class="text-white font-mono">{format!("{:.3}", x)}</span>
                                            </div>
                                            <div class="flex justify-between">
                                                <span class="text-[#555555]">"Y:"</span>
                                                <span class="text-white font-mono">{format!("{:.3}", y)}</span>
                                            </div>
                                            <div class="flex justify-between">
                                                <span class="text-[#555555]">"Z:"</span>
                                                <span class="text-white font-mono">{format!("{:.3}", z)}</span>
                                            </div>
                                            <div class="flex justify-between">
                                                <span class="text-[#555555]">"W:"</span>
                                                <span class="text-white font-mono">{format!("{:.3}", w)}</span>
                                            </div>
                                            <div class="flex justify-between">
                                                <span class="text-[#555555]">"P:"</span>
                                                <span class="text-white font-mono">{format!("{:.3}", p)}</span>
                                            </div>
                                            <div class="flex justify-between">
                                                <span class="text-[#555555]">"R:"</span>
                                                <span class="text-white font-mono">{format!("{:.3}", r)}</span>
                                            </div>
                                        </div>
                                    </div>
                                </Show>
                            </div>
                        }
                    }).collect_view()}
                </div>
            </div>
        </Show>
    }
}

