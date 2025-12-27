//! Command log panel showing console output.
//!
//! Displays all console messages in chronological order with sent/received indicators.
//! Has tabs for "Messages" (all messages) and "Errors" (error messages only).

use leptos::prelude::*;
use crate::pages::dashboard::context::{WorkspaceContext, MessageDirection, MessageType};

/// Command Log panel - console-style output with chronological ordering and tabs
#[component]
pub fn CommandLogPanel() -> impl IntoView {
    let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let console_messages = ctx.console_messages;
    let error_log = ctx.error_log;

    // Tab state: "messages" or "errors"
    let (active_tab, set_active_tab) = signal("messages");

    view! {
        <div class="bg-surface-1 backdrop-blur-theme rounded-theme border border-border shadow-theme flex flex-col overflow-hidden transition-all duration-300">
            <div class="flex items-center justify-between p-2 border-b border-border/8 shrink-0">
                <div class="flex items-center gap-2">
                    <h3 class="text-[10px] font-semibold text-primary uppercase tracking-wide flex items-center">
                        <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"/>
                        </svg>
                        "Console"
                    </h3>
                    // Tabs
                    <div class="flex gap-1">
                        <button
                            class=move || if active_tab.get() == "messages" {
                                "text-[9px] px-2 py-0.5 bg-primary/20 text-primary border border-primary rounded"
                            } else {
                                "text-[9px] px-2 py-0.5 bg-border/4 text-muted-foreground border border-border/8 rounded hover:bg-border/6"
                            }
                            on:click=move |_| set_active_tab.set("messages")
                        >
                            "Messages"
                        </button>
                        <button
                            class=move || {
                                let error_count = error_log.get().len();
                                let base_class = if active_tab.get() == "errors" {
                                    "text-[9px] px-2 py-0.5 bg-destructive/20 text-destructive border border-destructive rounded"
                                } else {
                                    "text-[9px] px-2 py-0.5 bg-border/4 text-muted-foreground border border-border/8 rounded hover:bg-border/6"
                                };
                                if error_count > 0 {
                                    format!("{} relative", base_class)
                                } else {
                                    base_class.to_string()
                                }
                            }
                            on:click=move |_| set_active_tab.set("errors")
                        >
                            "Errors"
                            {move || {
                                let count = error_log.get().len();
                                if count > 0 {
                                    Some(view! {
                                        <span class="ml-1 bg-destructive text-destructive-foreground text-[8px] px-1 rounded-full font-bold">
                                            {count}
                                        </span>
                                    })
                                } else {
                                    None
                                }
                            }}
                        </button>
                    </div>
                </div>
                <button
                    class="text-[8px] text-muted-foreground hover:text-destructive"
                    on:click=move |_| {
                        console_messages.set(Vec::new());
                        error_log.set(Vec::new());
                    }
                    title="Clear console"
                >
                    "Clear"
                </button>
            </div>
            <div class="flex-1 overflow-y-auto p-2 font-mono text-[9px]">
                <Show when=move || active_tab.get() == "messages" fallback=move || {
                    // Show errors only
                    let errors = error_log.get();
                    errors.into_iter().rev().map(|error| {
                        view! {
                            <div class="py-0.5 border-b border-border/4 flex items-start border-l-2 border-l-destructive pl-2">
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
                                <div class="py-0.5 border-b border-border/4 flex items-start">
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
        </div>
    }
}

