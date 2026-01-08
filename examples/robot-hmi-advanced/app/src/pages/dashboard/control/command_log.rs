//! Command log panel showing console output.
//!
//! Displays all console messages in chronological order with sent/received indicators.
//! Has tabs for "Messages" (all messages) and "Errors" (error messages only).
//! Collapsible - when collapsed, header animates to bottom of container.
//! Tab buttons with badges are always visible; clicking them when collapsed expands to that tab.

use leptos::prelude::*;
use crate::pages::dashboard::context::{WorkspaceContext, MessageDirection, MessageType};

/// Command Log panel - console-style output with chronological ordering and tabs.
/// When collapsed, the header slides to the bottom of the container, leaving whitespace above.
/// Tab buttons are always visible with badges showing counts.
#[component]
pub fn CommandLogPanel() -> impl IntoView {
    let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let console_messages = ctx.console_messages;
    let error_log = ctx.error_log;

    // Collapsed state
    let (collapsed, set_collapsed) = signal(false);
    // Tab state: "messages" or "errors"
    let (active_tab, set_active_tab) = signal("messages");

    // Badge counts
    let message_count = move || console_messages.get().len();
    let error_count = move || error_log.get().len();

    // Container class changes based on collapsed state - justify-end pushes header to bottom
    let container_class = move || {
        if collapsed.get() {
            "flex flex-col justify-end h-full transition-all duration-300 ease-in-out"
        } else {
            "bg-background rounded border border-border/8 flex flex-col overflow-hidden h-full transition-all duration-300 ease-in-out"
        }
    };

    view! {
        <div class=container_class>
            // Header - entire header is clickable to toggle collapse
            <button
                class=move || format!(
                    "flex items-center justify-between p-2 shrink-0 hover:bg-card/50 transition-all duration-300 w-full text-left bg-background rounded border border-border/8 {}",
                    if collapsed.get() { "" } else { "border-b" }
                )
                on:click=move |_| set_collapsed.update(|c| *c = !*c)
            >
                // Left side: collapse indicator and title
                <div class="flex items-center gap-2">
                    // Collapse indicator
                    <svg
                        class=move || format!("w-3 h-3 text-muted-foreground transition-transform duration-300 {}", if collapsed.get() { "" } else { "rotate-90" })
                        fill="none" stroke="currentColor" viewBox="0 0 24 24"
                    >
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"/>
                    </svg>
                    <h3 class="text-[10px] font-semibold text-primary uppercase tracking-wide flex items-center">
                        <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"/>
                        </svg>
                        "Console"
                    </h3>
                </div>
                // Right side: Tabs always visible with badges - stop propagation to prevent toggle
                <div class="flex gap-1" on:click=move |ev| ev.stop_propagation()>
                    <button
                        class=move || if active_tab.get() == "messages" && !collapsed.get() {
                            "text-[9px] px-2 py-0.5 bg-[#00d9ff20] text-primary border border-primary rounded flex items-center gap-1"
                        } else {
                            "text-[9px] px-2 py-0.5 bg-[#ffffff05] text-muted-foreground border border-border/15 rounded hover:bg-border/10 flex items-center gap-1"
                        }
                        on:click=move |_| {
                            set_active_tab.set("messages");
                            set_collapsed.set(false); // Expand when clicking tab
                        }
                    >
                        "Msgs"
                        <span class="bg-primary/30 text-primary text-[8px] px-1 rounded-full font-mono">
                            {move || message_count()}
                        </span>
                    </button>
                    <button
                        class=move || {
                            let count = error_count();
                            let is_active = active_tab.get() == "errors" && !collapsed.get();
                            if is_active {
                                "text-[9px] px-2 py-0.5 bg-destructive/15 text-destructive border border-destructive rounded flex items-center gap-1".to_string()
                            } else if count > 0 {
                                "text-[9px] px-2 py-0.5 bg-[#ffffff05] text-destructive border border-border/15 rounded hover:bg-border/10 flex items-center gap-1".to_string()
                            } else {
                                "text-[9px] px-2 py-0.5 bg-[#ffffff05] text-muted-foreground border border-border/15 rounded hover:bg-border/10 flex items-center gap-1".to_string()
                            }
                        }
                        on:click=move |_| {
                            set_active_tab.set("errors");
                            set_collapsed.set(false); // Expand when clicking tab
                        }
                    >
                        "Errs"
                        <span class=move || format!("text-[8px] px-1 rounded-full font-mono {}",
                            if error_count() > 0 { "bg-destructive/30 text-destructive" } else { "bg-muted/30 text-muted-foreground" }
                        )>
                            {move || error_count()}
                        </span>
                    </button>
                    <button
                        class="text-[8px] text-muted-foreground hover:text-destructive px-1"
                        on:click=move |_| {
                            console_messages.set(Vec::new());
                            error_log.set(Vec::new());
                        }
                        title="Clear console"
                    >
                        "×"
                    </button>
                </div>
            </button>
            // Content - only shown when expanded
            <Show when=move || !collapsed.get()>
                <div class="flex-1 overflow-y-auto p-2 font-mono text-[9px]">
                    <Show when=move || active_tab.get() == "messages" fallback=move || {
                        let errors = error_log.get();
                        errors.into_iter().rev().map(|error| {
                            view! {
                                <div class="py-0.5 border-b border-[#ffffff05] flex items-start border-l-2 border-l-destructive pl-2">
                                    <span class="text-destructive">{error}</span>
                                </div>
                            }
                        }).collect_view()
                    }>
                        {move || {
                            let messages = console_messages.get();
                            messages.into_iter().map(|msg| {
                                let (dir_icon, dir_class) = match msg.direction {
                                    MessageDirection::Sent => ("→", "text-primary"),
                                    MessageDirection::Received => ("←", "text-success"),
                                    MessageDirection::System => ("•", "text-warning"),
                                };
                                let content_class = match msg.msg_type {
                                    MessageType::Command => "text-primary",
                                    MessageType::Response => "text-success",
                                    MessageType::Error => "text-destructive",
                                    MessageType::Status => "text-muted-foreground",
                                    MessageType::Config => "text-warning",
                                };
                                let seq_display = msg.sequence_id.map(|id| format!(" seq={}", id)).unwrap_or_default();
                                view! {
                                    <div class="py-0.5 border-b border-[#ffffff05] flex items-start">
                                        <span class="text-muted-foreground mr-1 shrink-0">{format!("[{}]", msg.timestamp)}</span>
                                        <span class={format!("{} mr-1 shrink-0", dir_class)}>{dir_icon}</span>
                                        <span class={content_class}>
                                            {msg.content}
                                            <span class="text-muted-foreground">{seq_display}</span>
                                        </span>
                                    </div>
                                }
                            }).collect_view()
                        }}
                    </Show>
                </div>
            </Show>
        </div>
    }
}

