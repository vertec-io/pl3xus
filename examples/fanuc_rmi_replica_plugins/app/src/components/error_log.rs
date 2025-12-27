//! Error log component - shows error count indicator.

use leptos::prelude::*;
use crate::pages::dashboard::context::{WorkspaceContext, MessageType};

/// Simple error indicator - shows error count, errors are displayed in console
#[component]
pub fn ErrorLog() -> impl IntoView {
    let ctx = use_context::<WorkspaceContext>();

    let error_count = move || {
        ctx.as_ref()
            .map(|c| c.console_messages.get().iter().filter(|m| m.msg_type == MessageType::Error).count())
            .unwrap_or(0)
    };

    view! {
        <div class="bg-background rounded border border-border/8 p-2">
            <h2 class="text-[10px] font-semibold text-destructive flex items-center uppercase tracking-wide">
                <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                </svg>
                "Errors"
                {move || {
                    let count = error_count();
                    if count > 0 {
                        Some(view! {
                            <span class="ml-1.5 bg-destructive text-primary-foreground text-[9px] px-1 py-0.5 rounded-full font-bold">
                                {count}
                            </span>
                        })
                    } else {
                        None
                    }
                }}
            </h2>
            <div class="text-[8px] text-muted-foreground mt-0.5">
                "View errors in Console"
            </div>
        </div>
    }
}

