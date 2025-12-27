//! Tabbed panel combining I/O and Joint Jog controls.
//!
//! Provides a compact tabbed interface to switch between I/O and Joint Jog panels,
//! saving vertical space in the control layout.

use leptos::prelude::*;
use super::JointJogPanel;
use crate::components::IoStatusPanel;

/// Tabbed panel for I/O and Joint Jog controls.
/// Allows switching between the two panels to save vertical space.
/// I/O is the default/first tab.
#[component]
pub fn JogIoTabs() -> impl IntoView {
    // Tab state: "io" or "jog" - I/O is first/default
    let (active_tab, set_active_tab) = signal("io");

    let tab_class = move |tab: &'static str| {
        if active_tab.get() == tab {
            "text-[9px] px-3 py-1 bg-[#00d9ff20] text-primary border border-primary rounded-t font-medium"
        } else {
            "text-[9px] px-3 py-1 bg-[#ffffff05] text-muted-foreground border border-border/15 rounded-t hover:bg-border/10"
        }
    };

    view! {
        <div class="bg-background rounded border border-border/8 flex flex-col overflow-hidden">
            // Tab header - I/O first
            <div class="flex items-center gap-1 px-2 pt-2 border-b border-border/8">
                <button
                    class=move || tab_class("io")
                    on:click=move |_| set_active_tab.set("io")
                >
                    <span class="flex items-center gap-1">
                        <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z"/>
                        </svg>
                        "I/O"
                    </span>
                </button>
                <button
                    class=move || tab_class("jog")
                    on:click=move |_| set_active_tab.set("jog")
                >
                    <span class="flex items-center gap-1">
                        <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/>
                        </svg>
                        "Joint Jog"
                    </span>
                </button>
            </div>

            // Tab content
            <div class="p-2">
                <Show when=move || active_tab.get() == "io">
                    <IoStatusPanelInline/>
                </Show>
                <Show when=move || active_tab.get() == "jog">
                    <JointJogPanelInline/>
                </Show>
            </div>
        </div>
    }
}

/// Inline version of JointJogPanel without the outer container (for embedding in tabs).
#[component]
fn JointJogPanelInline() -> impl IntoView {
    // Re-use the existing JointJogPanel but we need to strip its outer container
    // For now, just use the full panel - the styling will be slightly nested but functional
    view! {
        <div class="-m-2">
            <JointJogPanel/>
        </div>
    }
}

/// Inline version of IoStatusPanel without the outer container (for embedding in tabs).
#[component]
fn IoStatusPanelInline() -> impl IntoView {
    // Use IoStatusPanel with start_collapsed=false and no popout button
    view! {
        <div class="-m-2">
            <IoStatusPanel start_collapsed=false show_popout=false/>
        </div>
    }
}

