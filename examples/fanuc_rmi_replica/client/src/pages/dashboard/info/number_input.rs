//! Number input component with validation.

use leptos::prelude::*;

/// Number input component with validation
#[component]
pub fn NumberInput(
    #[prop(into)] value: Signal<String>,
    on_input: impl Fn(String) + 'static + Send + Sync,
    #[prop(optional)] placeholder: &'static str,
    #[prop(default = 0.0)] min: f64,
    #[prop(default = f64::MAX)] max: f64,
    #[prop(optional, into)] disabled: Signal<bool>,
) -> impl IntoView {
    let is_valid = move || {
        if let Ok(v) = value.get().parse::<f64>() {
            if v < min || v > max {
                return false;
            }
            true
        } else {
            value.get().is_empty()
        }
    };

    view! {
        <input
            type="text"
            inputmode="decimal"
            class=move || format!(
                "w-full bg-background rounded px-2 py-1 text-[10px] text-white {} {}",
                if is_valid() {
                    "border border-border/8"
                } else {
                    "border-2 border-destructive"
                },
                if disabled.get() { "opacity-50 cursor-not-allowed" } else { "" }
            )
            placeholder=placeholder
            prop:value=value
            disabled=move || disabled.get()
            on:input=move |ev| {
                on_input(event_target_value(&ev));
            }
        />
    }
}

