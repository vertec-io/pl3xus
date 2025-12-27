//! Multi Frame Display - Accordion showing detailed frame data.
//!
//! Features:
//! - Auto-expands the active UFrame when it changes (unless user has manually interacted)
//! - Expand All / Collapse All buttons in header
//! - User-expanded frames stay open

use leptos::prelude::*;
use pl3xus_client::use_entity_component;
use fanuc_replica_plugins::{ConnectionState, FrameToolDataState};
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

    // Local state for expand all mode and user interaction tracking
    let (expand_all, set_expand_all) = signal(false);
    let (user_interacted, set_user_interacted) = signal(false);

    // Get active frame from synced component
    let active_frame = Memo::new(move |_| frame_tool_data.get().active_frame);

    // Reset user_interacted flag when robot connection changes
    Effect::new(move || {
        let _connected = robot_connected.get();
        set_user_interacted.set(false);
        set_expand_all.set(false);
    });

    // Auto-expand active frame when it changes (unless user has manually interacted or expand_all is active)
    Effect::new(move || {
        let active = active_frame.get();
        // Only auto-expand if:
        // 1. Not in "expand all" mode
        // 2. User hasn't manually interacted
        // 3. Active frame is valid (0-9)
        if !expand_all.get() && !user_interacted.get() && active >= 0 && active <= 9 {
            expanded_frames.update(|set| {
                set.clear();
                set.insert(active);
            });
        }
    });

    // Get frame data from synced component
    let frame_data = move |frame_num: i32| -> (f64, f64, f64, f64, f64, f64) {
        let data = frame_tool_data.get().get_frame(frame_num);
        (data.x, data.y, data.z, data.w, data.p, data.r)
    };

    view! {
        <Show when=move || robot_connected.get()>
            <div class="bg-background rounded border border-border/8 flex flex-col flex-1 overflow-y-auto">
                // Header with Expand All / Collapse All buttons
                <div class="flex items-center justify-between p-2 shrink-0">
                    <h3 class="text-[10px] font-semibold text-primary uppercase tracking-wide flex items-center group">
                        <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 17V7m0 10a2 2 0 01-2 2H5a2 2 0 01-2-2V7a2 2 0 012-2h2a2 2 0 012 2m0 10a2 2 0 002 2h2a2 2 0 002-2M9 7a2 2 0 012-2h2a2 2 0 012 2m0 10V7m0 10a2 2 0 002 2h2a2 2 0 002-2V7a2 2 0 00-2-2h-2a2 2 0 00-2 2"/>
                        </svg>
                        "Frame Data"
                    </h3>
                    <div class="flex gap-1">
                        <button
                            class="text-[8px] text-muted-foreground hover:text-primary px-1"
                            on:click=move |_| {
                                set_expand_all.set(true);
                                set_user_interacted.set(false);
                                // Expand all frames (0-9)
                                expanded_frames.update(|set| {
                                    set.clear();
                                    for i in 0..=9 {
                                        set.insert(i);
                                    }
                                });
                            }
                            title="Expand All"
                        >
                            "▼ All"
                        </button>
                        <button
                            class="text-[8px] text-muted-foreground hover:text-primary px-1"
                            on:click=move |_| {
                                set_expand_all.set(false);
                                set_user_interacted.set(true);
                                // Collapse all frames
                                expanded_frames.update(|set| {
                                    set.clear();
                                });
                            }
                            title="Collapse All"
                        >
                            "▲ All"
                        </button>
                    </div>
                </div>

                <div class="px-2 pb-2 space-y-0.5">
                    {(0..10).map(|i| {
                        let is_expanded = move || {
                            expand_all.get() || expanded_frames.get().contains(&i)
                        };
                        let is_active = move || active_frame.get() == i;

                        view! {
                            <div class="border border-border/8 rounded overflow-hidden">
                                // Header (clickable)
                                <button
                                    class={move || format!(
                                        "w-full flex items-center justify-between px-2 py-1 text-[9px] transition-colors {}",
                                        if is_active() {
                                            "bg-primary/10 text-primary font-medium"
                                        } else if is_expanded() {
                                            "bg-card text-foreground"
                                        } else {
                                            "bg-card text-muted-foreground hover:bg-secondary"
                                        }
                                    )}
                                    on:click=move |_| {
                                        // Always exit expand_all mode when clicking individual accordion
                                        set_expand_all.set(false);
                                        // Mark that user has manually interacted
                                        set_user_interacted.set(true);

                                        // Toggle this frame in the set
                                        expanded_frames.update(|set| {
                                            if set.contains(&i) {
                                                set.remove(&i);
                                            } else {
                                                set.insert(i);
                                            }
                                        });
                                    }
                                >
                                    <span class={move || if is_active() { "font-medium" } else { "" }}>
                                        {format!("UFrame {}", i)}
                                        {move || if is_active() { " (active)" } else { "" }}
                                    </span>
                                    <svg
                                        class={move || format!("w-2.5 h-2.5 transition-transform {}", if is_expanded() { "" } else { "-rotate-90" })}
                                        fill="none" stroke="currentColor" viewBox="0 0 24 24"
                                    >
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"/>
                                    </svg>
                                </button>

                                // Expanded content
                                <Show when=is_expanded>
                                    <div class="bg-background px-2 py-1.5 border-t border-border/8">
                                        {move || {
                                            let (x, y, z, w, p, r) = frame_data(i);
                                            view! {
                                                <div class="grid grid-cols-3 gap-x-3 gap-y-0.5 text-[9px]">
                                                    <div class="flex justify-between">
                                                        <span class="text-muted-foreground">"X"</span>
                                                        <span class="text-foreground font-mono tabular-nums">{format!("{:.3}", x)}</span>
                                                    </div>
                                                    <div class="flex justify-between">
                                                        <span class="text-muted-foreground">"Y"</span>
                                                        <span class="text-foreground font-mono tabular-nums">{format!("{:.3}", y)}</span>
                                                    </div>
                                                    <div class="flex justify-between">
                                                        <span class="text-muted-foreground">"Z"</span>
                                                        <span class="text-foreground font-mono tabular-nums">{format!("{:.3}", z)}</span>
                                                    </div>
                                                    <div class="flex justify-between">
                                                        <span class="text-muted-foreground">"W"</span>
                                                        <span class="text-muted-foreground font-mono tabular-nums">{format!("{:.2}°", w)}</span>
                                                    </div>
                                                    <div class="flex justify-between">
                                                        <span class="text-muted-foreground">"P"</span>
                                                        <span class="text-muted-foreground font-mono tabular-nums">{format!("{:.2}°", p)}</span>
                                                    </div>
                                                    <div class="flex justify-between">
                                                        <span class="text-muted-foreground">"R"</span>
                                                        <span class="text-muted-foreground font-mono tabular-nums">{format!("{:.2}°", r)}</span>
                                                    </div>
                                                </div>
                                            }
                                        }}
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

