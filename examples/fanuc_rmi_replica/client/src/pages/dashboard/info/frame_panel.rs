//! Frame Management Panel - Frame selector with Apply button.
//!
//! Reads active frame directly from the synced FrameToolDataState component.
//! Server is the single source of truth for active frame/tool.

use leptos::prelude::*;
use pl3xus_client::{use_sync_component, use_request};
use fanuc_replica_types::{ConnectionState, FrameToolDataState, SetActiveFrameTool};

/// Frame Management Panel - Frame selector with Apply button
///
/// Reads active frame from synced FrameToolDataState. The pending_frame
/// is UI-local state for the selection before "Apply" is clicked.
#[component]
pub fn FrameManagementPanel() -> impl IntoView {
    let connection_state = use_sync_component::<ConnectionState>();
    let frame_tool_state = use_sync_component::<FrameToolDataState>();

    // Derive active frame/tool from synced server state
    let active_frame = Memo::new(move |_| {
        frame_tool_state.get().values().next()
            .map(|s| s.active_frame as usize)
            .unwrap_or(0)
    });
    let active_tool = Memo::new(move |_| {
        frame_tool_state.get().values().next()
            .map(|s| s.active_tool as usize)
            .unwrap_or(1)
    });

    // Request hook for setting active frame/tool
    let (set_frame_tool, _set_frame_tool_state) = use_request::<SetActiveFrameTool>();
    // Store the function so it can be used in multiple closures
    let set_frame_tool = StoredValue::new(set_frame_tool);

    let robot_connected = Memo::new(move |_| {
        connection_state.get().values().next()
            .map(|s| s.robot_connected)
            .unwrap_or(false)
    });

    // Local UI state for pending frame selection (before Apply is clicked)
    let (pending_frame, set_pending_frame) = signal::<Option<usize>>(None);

    // View mode: "buttons" or "dropdown" - UI-local state
    let (view_mode, set_view_mode) = signal("buttons");

    // Get effective frame (pending or current from server)
    let effective_frame = move || {
        pending_frame.get().unwrap_or_else(|| active_frame.get())
    };

    // Check if there are pending changes
    let has_pending = move || pending_frame.get().is_some();

    view! {
        <Show when=move || robot_connected.get()>
            <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-2">
                <div class="flex items-center justify-between mb-1.5">
                    <h3 class="text-[10px] font-semibold text-[#00d9ff] uppercase tracking-wide flex items-center group">
                        <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 5a1 1 0 011-1h14a1 1 0 011 1v2a1 1 0 01-1 1H5a1 1 0 01-1-1V5zM4 13a1 1 0 011-1h6a1 1 0 011 1v6a1 1 0 01-1 1H5a1 1 0 01-1-1v-6zM16 13a1 1 0 011-1h2a1 1 0 011 1v6a1 1 0 01-1 1h-2a1 1 0 01-1-1v-6z"/>
                        </svg>
                        "User Frames"
                    </h3>
                    // View toggle button
                    <button
                        class="text-[8px] text-[#666666] hover:text-[#00d9ff] px-1.5 py-0.5 border border-[#ffffff08] rounded"
                        on:click=move |_| {
                            if view_mode.get() == "buttons" {
                                set_view_mode.set("dropdown");
                            } else {
                                set_view_mode.set("buttons");
                            }
                        }
                        title="Toggle view mode"
                    >
                        {move || if view_mode.get() == "buttons" { "▼" } else { "▦" }}
                    </button>
                </div>

                // Button grid view
                <Show when=move || view_mode.get() == "buttons" fallback=move || {
                    // Dropdown view
                    view! {
                        <div class="flex items-center gap-2">
                            <select
                                class="flex-1 bg-[#111111] border border-[#ffffff15] rounded px-2 py-1 text-[10px] text-white"
                                on:change=move |ev| {
                                    let value = event_target_value(&ev);
                                    if let Ok(v) = value.parse::<usize>() {
                                        set_pending_frame.set(Some(v));
                                    }
                                }
                            >
                                {(0..10).map(|i| {
                                    let is_selected = move || effective_frame() == i;
                                    view! {
                                        <option value={i.to_string()} selected=is_selected>
                                            {format!("UFrame {}", i)}
                                        </option>
                                    }
                                }).collect_view()}
                            </select>
                            // Apply button
                            <Show when=has_pending>
                                <button
                                    class="px-2 py-1 text-[9px] bg-[#00d9ff20] text-[#00d9ff] border border-[#00d9ff] rounded hover:bg-[#00d9ff30]"
                                    on:click=move |_| {
                                        if let Some(frame) = pending_frame.get() {
                                            let tool = active_tool.get();
                                            // Send request to server - server updates FrameToolDataState
                                            // which syncs back to all clients
                                            set_frame_tool.get_value()(SetActiveFrameTool {
                                                uframe: frame as i32,
                                                utool: tool as i32,
                                            });
                                            set_pending_frame.set(None);
                                        }
                                    }
                                    title="Apply frame change to robot"
                                >
                                    "Apply"
                                </button>
                            </Show>
                        </div>
                    }
                }>
                    <div class="space-y-1">
                        <div class="grid grid-cols-5 gap-0.5">
                            {(0..10).map(|i| {
                                let is_selected = move || effective_frame() == i;
                                let is_active = move || active_frame.get() == i;
                                view! {
                                    <button
                                        class={move || {
                                            let selected = is_selected();
                                            let active = is_active();

                                            if selected && active {
                                                "bg-[#00d9ff20] border border-[#00d9ff] text-[#00d9ff] text-[9px] py-1 rounded font-medium"
                                            } else if selected {
                                                "bg-[#ffaa0020] border border-[#ffaa00] text-[#ffaa00] text-[9px] py-1 rounded font-medium"
                                            } else {
                                                "bg-[#111111] border border-[#ffffff08] text-[#555555] text-[9px] py-1 rounded hover:border-[#ffffff20] hover:text-[#888888]"
                                            }
                                        }}
                                        on:click=move |_| {
                                            set_pending_frame.set(Some(i));
                                        }
                                        title=format!("UFrame {}", i)
                                    >
                                        {i}
                                    </button>
                                }
                            }).collect_view()}
                        </div>
                        // Apply button (only show if pending changes)
                        <Show when=has_pending>
                            <button
                                class="w-full px-2 py-1 text-[9px] bg-[#00d9ff20] text-[#00d9ff] border border-[#00d9ff] rounded hover:bg-[#00d9ff30]"
                                on:click=move |_| {
                                    if let Some(frame) = pending_frame.get() {
                                        let tool = active_tool.get();
                                        // Send request to server - server updates FrameToolDataState
                                        // which syncs back to all clients
                                        set_frame_tool.get_value()(SetActiveFrameTool {
                                            uframe: frame as i32,
                                            utool: tool as i32,
                                        });
                                        set_pending_frame.set(None);
                                    }
                                }
                                title="Apply frame change to robot"
                            >
                                "Apply"
                            </button>
                        </Show>
                    </div>
                </Show>
            </div>
        </Show>
    }
}

