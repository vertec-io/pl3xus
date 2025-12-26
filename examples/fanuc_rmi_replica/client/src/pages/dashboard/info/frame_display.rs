//! Multi Frame Display - Accordion showing detailed frame data.

use leptos::prelude::*;
use pl3xus_client::use_entity_component;
use fanuc_replica_types::{ConnectionState, FrameToolDataState};
use crate::pages::dashboard::context::WorkspaceContext;
use crate::pages::dashboard::use_system_entity;

/// Multi Frame Display - Accordion showing detailed frame data (X,Y,Z,W,P,R) for frames 0-9
#[component]
pub fn MultiFrameDisplay() -> impl IntoView {
    let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let system_ctx = use_system_entity();

    // Subscribe to entity-specific components
    // All these components live on the robot entity
    let (connection_state, robot_exists) = use_entity_component::<ConnectionState, _>(move || system_ctx.robot_entity_id.get());
    let (frame_tool_data, _) = use_entity_component::<FrameToolDataState, _>(move || system_ctx.robot_entity_id.get());
    let expanded_frames = ctx.expanded_frames;

    let robot_connected = Memo::new(move |_| robot_exists.get() && connection_state.get().robot_connected);

    // Get frame data from synced component
    let frame_data = move |frame_num: i32| -> (f64, f64, f64, f64, f64, f64) {
        let data = frame_tool_data.get().get_frame(frame_num);
        (data.x, data.y, data.z, data.w, data.p, data.r)
    };

    view! {
        <Show when=move || robot_connected.get()>
            <div class="bg-background rounded border border-border/8 p-2 flex-1 overflow-y-auto">
                <h3 class="text-[10px] font-semibold text-primary mb-1.5 uppercase tracking-wide flex items-center group">
                    <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 17V7m0 10a2 2 0 01-2 2H5a2 2 0 01-2-2V7a2 2 0 012-2h2a2 2 0 012 2m0 10a2 2 0 002 2h2a2 2 0 002-2M9 7a2 2 0 012-2h2a2 2 0 012 2m0 10V7m0 10a2 2 0 002 2h2a2 2 0 002-2V7a2 2 0 00-2-2h-2a2 2 0 00-2 2"/>
                    </svg>
                    "Frame Data"
                </h3>

                <div class="space-y-0.5">
                    {(0..10).map(|i| {
                        let is_expanded = move || expanded_frames.get().contains(&i);
                        let (x, y, z, w, p, r) = frame_data(i);

                        view! {
                            <div class="border border-border/8 rounded overflow-hidden">
                                // Header (clickable)
                                <button
                                    class="w-full flex items-center justify-between px-2 py-1 bg-card hover:bg-muted text-left"
                                    on:click=move |_| {
                                        let mut current = expanded_frames.get();
                                        if current.contains(&i) {
                                            current.remove(&i);
                                        } else {
                                            current.insert(i);
                                        }
                                        expanded_frames.set(current);
                                    }
                                >
                                    <span class="text-[9px] text-muted-foreground">
                                        {format!("UFrame {}", i)}
                                    </span>
                                    <span class="text-[8px] text-muted-foreground">
                                        {move || if is_expanded() { "▼" } else { "▶" }}
                                    </span>
                                </button>

                                // Expanded content
                                <Show when=is_expanded>
                                    <div class="bg-background px-2 py-1.5">
                                        <div class="grid grid-cols-3 gap-x-3 gap-y-0.5 text-[8px]">
                                            <div class="flex justify-between">
                                                <span class="text-muted-foreground">"X:"</span>
                                                <span class="text-white font-mono">{format!("{:.3}", x)}</span>
                                            </div>
                                            <div class="flex justify-between">
                                                <span class="text-muted-foreground">"Y:"</span>
                                                <span class="text-white font-mono">{format!("{:.3}", y)}</span>
                                            </div>
                                            <div class="flex justify-between">
                                                <span class="text-muted-foreground">"Z:"</span>
                                                <span class="text-white font-mono">{format!("{:.3}", z)}</span>
                                            </div>
                                            <div class="flex justify-between">
                                                <span class="text-muted-foreground">"W:"</span>
                                                <span class="text-white font-mono">{format!("{:.3}", w)}</span>
                                            </div>
                                            <div class="flex justify-between">
                                                <span class="text-muted-foreground">"P:"</span>
                                                <span class="text-white font-mono">{format!("{:.3}", p)}</span>
                                            </div>
                                            <div class="flex justify-between">
                                                <span class="text-muted-foreground">"R:"</span>
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

