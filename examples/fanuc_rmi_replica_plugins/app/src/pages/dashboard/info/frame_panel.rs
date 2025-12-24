//! Frame Management Panel - Frame selector with Apply button.
//!
//! Reads active frame directly from the synced FrameToolDataState component.
//! Server is the single source of truth for active frame/tool.

use leptos::prelude::*;
use pl3xus_client::{use_sync_context, use_entity_component, use_mutation_targeted, EntityControl};
use fanuc_replica_plugins::{ConnectionState, FrameToolDataState, SetActiveFrameTool};
use crate::components::use_toast;
use crate::pages::dashboard::use_system_entity;

/// Frame Management Panel - Frame selector with Apply button
///
/// Reads active frame from synced FrameToolDataState. The pending_frame
/// is UI-local state for the selection before "Apply" is clicked.
/// Keeps pending state until server confirms to avoid flash back to server value.
#[component]
pub fn FrameManagementPanel() -> impl IntoView {
    let ctx = use_sync_context();
    let toast = use_toast();
    let system_ctx = use_system_entity();

    // Subscribe to entity-specific components
    // All these components live on the robot entity
    let (connection_state, robot_exists) = use_entity_component::<ConnectionState, _>(move || system_ctx.robot_entity_id.get());
    let (frame_tool_state, _) = use_entity_component::<FrameToolDataState, _>(move || system_ctx.robot_entity_id.get());

    // Subscribe to entity control on the System entity (control is at hierarchy level)
    let (control_state, _) = use_entity_component::<EntityControl, _>(move || system_ctx.system_entity_id.get());

    // Check if THIS client has control
    let has_control = Memo::new(move |_| {
        let my_id = ctx.my_connection_id.get();
        let state = control_state.get();
        Some(state.client_id) == my_id
    });

    // Derive active frame/tool from synced server state
    let active_frame = Memo::new(move |_| frame_tool_state.get().active_frame as usize);
    let active_tool = Memo::new(move |_| frame_tool_state.get().active_tool as usize);

    // Local UI state for pending frame selection (before Apply is clicked)
    let (pending_frame, set_pending_frame) = signal::<Option<usize>>(None);

    // Targeted mutation for setting active frame/tool with error handling
    // Using targeted mutation since frame/tool changes require entity control
    let set_frame_tool = use_mutation_targeted::<SetActiveFrameTool>(move |result| {
        match result {
            Ok(r) if r.success => {
                // Success - pending_frame will be cleared when synced component updates
            }
            Ok(r) => {
                // Failure - clear pending and revert to server value
                set_pending_frame.set(None);
                toast.error(format!("Frame change failed: {}", r.error.as_deref().unwrap_or("No control")));
            }
            Err(e) => {
                set_pending_frame.set(None);
                toast.error(format!("Frame error: {e}"));
            }
        }
    });

    let robot_connected = Memo::new(move |_| robot_exists.get() && connection_state.get().robot_connected);

    // View mode: "buttons" or "dropdown" - UI-local state
    let (view_mode, set_view_mode) = signal("buttons");

    // Get effective frame (pending or current from server)
    let effective_frame = move || {
        pending_frame.get().unwrap_or_else(|| active_frame.get())
    };

    // Check if there are pending changes (only if we have control)
    let has_pending = move || pending_frame.get().is_some() && has_control.get();

    // Effect to clear pending value when server catches up
    Effect::new(move |_| {
        let server = active_frame.get();
        let pending = pending_frame.get_untracked();
        if let Some(pending_val) = pending {
            if server == pending_val {
                set_pending_frame.set(None);
            }
        }
    });

    // Get the Robot entity bits (for targeted mutation)
    let robot_entity_bits = move || system_ctx.robot_entity_id.get();

    // Disabled state for buttons/select when not in control
    let is_disabled = move || !has_control.get();

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
                    <div class="flex items-center gap-1">
                        <Show when=move || !has_control.get()>
                            <span class="text-[8px] text-[#ff4444] bg-[#ff444420] px-1.5 py-0.5 rounded">"No Control"</span>
                        </Show>
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
                </div>

                // Button grid view
                <Show when=move || view_mode.get() == "buttons" fallback=move || {
                    // Dropdown view
                    view! {
                        <div class="flex items-center gap-2">
                            <select
                                class=move || if is_disabled() {
                                    "flex-1 bg-[#111111] border border-[#ffffff15] rounded px-2 py-1 text-[10px] text-[#555555] opacity-50 cursor-not-allowed"
                                } else {
                                    "flex-1 bg-[#111111] border border-[#ffffff15] rounded px-2 py-1 text-[10px] text-white"
                                }
                                disabled=is_disabled
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
                                        if let (Some(frame), Some(entity_bits)) = (pending_frame.get(), robot_entity_bits()) {
                                            let tool = active_tool.get();
                                            // Send targeted request to server - server updates FrameToolDataState
                                            // which syncs back to all clients. Don't clear pending - let Effect do it.
                                            set_frame_tool.send(entity_bits, SetActiveFrameTool {
                                                uframe: frame as i32,
                                                utool: tool as i32,
                                            });
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
                                            let disabled = is_disabled();

                                            if disabled {
                                                "bg-[#111111] border border-[#ffffff08] text-[#444444] text-[9px] py-1 rounded opacity-50 cursor-not-allowed"
                                            } else if selected && active {
                                                "bg-[#00d9ff20] border border-[#00d9ff] text-[#00d9ff] text-[9px] py-1 rounded font-medium"
                                            } else if selected {
                                                "bg-[#ffaa0020] border border-[#ffaa00] text-[#ffaa00] text-[9px] py-1 rounded font-medium"
                                            } else {
                                                "bg-[#111111] border border-[#ffffff08] text-[#555555] text-[9px] py-1 rounded hover:border-[#ffffff20] hover:text-[#888888]"
                                            }
                                        }}
                                        disabled=is_disabled
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
                        // Apply button (only show if pending changes and have control)
                        <Show when=has_pending>
                            <button
                                class="w-full px-2 py-1 text-[9px] bg-[#00d9ff20] text-[#00d9ff] border border-[#00d9ff] rounded hover:bg-[#00d9ff30]"
                                on:click=move |_| {
                                    if let (Some(frame), Some(entity_bits)) = (pending_frame.get(), robot_entity_bits()) {
                                        let tool = active_tool.get();
                                        // Send targeted request to server - server updates FrameToolDataState
                                        // which syncs back to all clients. Don't clear pending - let Effect do it.
                                        set_frame_tool.send(entity_bits, SetActiveFrameTool {
                                            uframe: frame as i32,
                                            utool: tool as i32,
                                        });
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

