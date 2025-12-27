//! Toast notification component for user feedback.

use leptos::prelude::*;
use std::collections::VecDeque;

/// Toast notification type.
#[derive(Clone, Debug, PartialEq)]
pub enum ToastType {
    Success,
    Error,
    Warning,
    Info,
}

/// A single toast notification.
#[derive(Clone, Debug)]
pub struct Toast {
    pub id: u64,
    pub message: String,
    pub toast_type: ToastType,
}

/// Toast context for managing notifications.
#[derive(Clone, Copy)]
pub struct ToastContext {
    pub toasts: RwSignal<VecDeque<Toast>>,
    // Use StoredValue instead of RwSignal to avoid reactive issues when called from Effects
    next_id: StoredValue<u64>,
}

impl ToastContext {
    pub fn new() -> Self {
        Self {
            toasts: RwSignal::new(VecDeque::new()),
            next_id: StoredValue::new(0),
        }
    }

    /// Show a toast notification.
    pub fn show(&self, message: impl Into<String>, toast_type: ToastType) {
        // Use StoredValue to avoid reactive loops when called from Effects
        let id = self.next_id.get_value();
        self.next_id.set_value(id + 1);

        // Determine auto-dismiss duration based on type
        // Success: 5 seconds, Error: 8 seconds, others: 5 seconds
        let dismiss_secs = match &toast_type {
            ToastType::Error => 8,
            _ => 5,
        };

        let toast = Toast {
            id,
            message: message.into(),
            toast_type,
        };

        // Use try_update_untracked to avoid reactive graph issues when called from Effects
        // (per research/LESSONS_LEARNED.md)
        self.toasts.try_update_untracked(|toasts| {
            toasts.push_back(toast);
            // Keep max 5 toasts
            while toasts.len() > 5 {
                toasts.pop_front();
            }
        });
        self.toasts.notify();

        // Auto-dismiss after configured duration
        let toasts = self.toasts;
        set_timeout(
            move || {
                // This runs outside any Effect, so regular update is fine
                toasts.update(|t| {
                    t.retain(|toast| toast.id != id);
                });
            },
            std::time::Duration::from_secs(dismiss_secs),
        );
    }

    pub fn success(&self, message: impl Into<String>) {
        self.show(message, ToastType::Success);
    }

    pub fn error(&self, message: impl Into<String>) {
        self.show(message, ToastType::Error);
    }

    pub fn warning(&self, message: impl Into<String>) {
        self.show(message, ToastType::Warning);
    }

    pub fn info(&self, message: impl Into<String>) {
        self.show(message, ToastType::Info);
    }

    /// Dismiss a specific toast.
    pub fn dismiss(&self, id: u64) {
        self.toasts.update(|toasts| {
            toasts.retain(|t| t.id != id);
        });
    }
}

/// Hook to get the toast context.
pub fn use_toast() -> ToastContext {
    use_context::<ToastContext>().expect("ToastContext not found. Wrap your app in ToastProvider.")
}

/// Toast provider component.
#[component]
pub fn ToastProvider(children: Children) -> impl IntoView {
    let ctx = ToastContext::new();
    provide_context(ctx.clone());

    view! {
        {children()}
        <ToastContainer ctx=ctx />
    }
}

/// Toast container that renders all active toasts.
/// Positioned at bottom-left to avoid overlap with jog controls (bottom-right).
#[component]
fn ToastContainer(ctx: ToastContext) -> impl IntoView {
    view! {
        <div class="fixed bottom-4 left-4 z-50 flex flex-col gap-2 pointer-events-none">
            {move || {
                ctx.toasts.get().into_iter().map(|toast| {
                    let ctx = ctx.clone();
                    let id = toast.id;
                    view! { <ToastItem toast=toast on_dismiss=move || ctx.dismiss(id) /> }
                }).collect::<Vec<_>>()
            }}
        </div>
    }
}

/// Individual toast item.
#[component]
fn ToastItem(toast: Toast, on_dismiss: impl Fn() + 'static) -> impl IntoView {
    let (bg_class, icon) = match toast.toast_type {
        ToastType::Success => ("bg-success/20 border-success/40 text-success", "✓"),
        ToastType::Error => ("bg-destructive/20 border-destructive/40 text-destructive", "✕"),
        ToastType::Warning => ("bg-warning/20 border-warning/40 text-warning", "⚠"),
        ToastType::Info => ("bg-primary/20 border-primary/40 text-primary", "ℹ"),
    };

    view! {
        <div class=format!("pointer-events-auto flex items-center gap-2 px-3 py-2 rounded border {} min-w-[200px] max-w-[400px] shadow-lg animate-slide-in", bg_class)>
            <span class="text-sm">{icon}</span>
            <span class="text-[11px] flex-1">{toast.message}</span>
            <button
                class="text-muted-foreground hover:text-foreground text-xs"
                on:click=move |_| on_dismiss()
            >
                "×"
            </button>
        </div>
    }
}

