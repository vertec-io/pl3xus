//! Multi Tool Display - Accordion showing detailed tool data.
//!
//! Features:
//! - Auto-expands the active UTool when it changes (unless user has manually interacted)
//! - Expand All / Collapse All buttons in header
//! - User-expanded tools stay open

use leptos::prelude::*;
use pl3xus_client::use_entity_component;
use fanuc_replica_plugins::{ConnectionState, FrameToolDataState};
use crate::pages::dashboard::context::WorkspaceContext;
use crate::pages::dashboard::use_system_entity;

/// Multi Tool Display - Accordion showing detailed tool geometry for tools 1-10
#[component]
pub fn MultiToolDisplay() -> impl IntoView {
    let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let system_ctx = use_system_entity();

    // Subscribe to entity-specific components
    // All these components live on the robot entity
    let (connection_state, robot_exists) = use_entity_component::<ConnectionState, _>(move || system_ctx.robot_entity_id.get());
    let (frame_tool_data, _) = use_entity_component::<FrameToolDataState, _>(move || system_ctx.robot_entity_id.get());
    let expanded_tools = ctx.expanded_tools;

    let robot_connected = Memo::new(move |_| robot_exists.get() && connection_state.get().robot_connected);

    // Local state for expand all mode and user interaction tracking
    let (expand_all, set_expand_all) = signal(false);
    let (user_interacted, set_user_interacted) = signal(false);

    // Get active tool from synced component
    let active_tool = Memo::new(move |_| frame_tool_data.get().active_tool);

    // Reset user_interacted flag when robot connection changes
    Effect::new(move || {
        let _connected = robot_connected.get();
        set_user_interacted.set(false);
        set_expand_all.set(false);
    });

    // Auto-expand active tool when it changes (unless user has manually interacted or expand_all is active)
    Effect::new(move || {
        let active = active_tool.get();
        // Only auto-expand if:
        // 1. Not in "expand all" mode
        // 2. User hasn't manually interacted
        // 3. Active tool is valid (1-10)
        if !expand_all.get() && !user_interacted.get() && active >= 1 && active <= 10 {
            expanded_tools.update(|set| {
                set.clear();
                set.insert(active);
            });
        }
    });

    // Get tool data from synced component
    let tool_data = move |tool_num: i32| -> (f64, f64, f64, f64, f64, f64) {
        let data = frame_tool_data.get().get_tool(tool_num);
        (data.x, data.y, data.z, data.w, data.p, data.r)
    };

    view! {
        <Show when=move || robot_connected.get()>
            <div class="bg-background rounded border border-border/8 flex flex-col flex-1 overflow-y-auto">
                // Header with Expand All / Collapse All buttons
                <div class="flex items-center justify-between p-2 shrink-0">
                    <h3 class="text-[10px] font-semibold text-primary uppercase tracking-wide flex items-center group">
                        <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/>
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>
                        </svg>
                        "Tool Data"
                    </h3>
                    <div class="flex gap-1">
                        <button
                            class="text-[8px] text-muted-foreground hover:text-primary px-1"
                            on:click=move |_| {
                                set_expand_all.set(true);
                                set_user_interacted.set(false);
                                // Expand all tools (1-10)
                                expanded_tools.update(|set| {
                                    set.clear();
                                    for i in 1..=10 {
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
                                // Collapse all tools
                                expanded_tools.update(|set| {
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
                    {(1..=10).map(|i| {
                        let is_expanded = move || {
                            expand_all.get() || expanded_tools.get().contains(&i)
                        };
                        let is_active = move || active_tool.get() == i;

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

                                        // Toggle this tool in the set
                                        expanded_tools.update(|set| {
                                            if set.contains(&i) {
                                                set.remove(&i);
                                            } else {
                                                set.insert(i);
                                            }
                                        });
                                    }
                                >
                                    <span class={move || if is_active() { "font-medium" } else { "" }}>
                                        {format!("UTool {}", i)}
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
                                            let (x, y, z, w, p, r) = tool_data(i);
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
