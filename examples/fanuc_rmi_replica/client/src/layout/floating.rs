//! Floating panel components for jog controls and I/O status.

use leptos::prelude::*;
use leptos::html::Div;
use leptos_use::{use_draggable_with_options, UseDraggableOptions, UseDraggableReturn, core::Position};

use crate::components::{JogControls, IoStatusPanel};
use crate::layout::LayoutContext;

/// Floating jog controls component (draggable).
#[component]
pub fn FloatingJogControls() -> impl IntoView {
    let layout_ctx = use_context::<LayoutContext>().expect("LayoutContext required");

    // Create node refs for container and header (drag handle)
    let container_el = NodeRef::<Div>::new();
    let header_el = NodeRef::<Div>::new();

    // Use leptos-use draggable hook - container is the target, header is the handle
    let UseDraggableReturn { style, .. } = use_draggable_with_options(
        container_el,
        UseDraggableOptions::default()
            .initial_value(Position { x: 100.0, y: 100.0 })
            .prevent_default(true)
            .handle(Some(header_el)),
    );

    view! {
        <Show when=move || layout_ctx.jog_popped.get()>
            <div
                node_ref=container_el
                class="fixed bg-card rounded border border-primary/40 shadow-2xl z-50 select-none flex flex-col"
                style=move || format!(
                    "touch-action: none; min-width: 200px; min-height: 200px; {}",
                    style.get()
                )
            >
                // Header with close button - THIS is the drag handle
                <div
                    node_ref=header_el
                    class="flex items-center justify-between p-2 cursor-move border-b border-border/6 shrink-0"
                >
                    <h3 class="text-[10px] font-semibold text-primary uppercase tracking-wide flex items-center gap-1">
                        <svg class="w-3 h-3 text-muted-foreground" fill="currentColor" viewBox="0 0 24 24">
                            <path d="M8 6h2v2H8V6zm6 0h2v2h-2V6zM8 11h2v2H8v-2zm6 0h2v2h-2v-2zm-6 5h2v2H8v-2zm6 0h2v2h-2v-2z"/>
                        </svg>
                        "Jog Controls"
                    </h3>
                    <button
                        class="p-0.5 hover:bg-border/6 rounded cursor-pointer"
                        title="Dock jog controls"
                        on:click=move |_| layout_ctx.jog_popped.set(false)
                    >
                        <svg class="w-3 h-3 text-muted-foreground hover:text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                        </svg>
                    </button>
                </div>

                // Content area - NOT draggable, so inputs work normally
                <div class="p-2 flex-1 overflow-auto">
                    <JogControls/>
                </div>
            </div>
        </Show>
    }
}

/// Floating I/O status panel component (draggable).
#[component]
pub fn FloatingIOStatus() -> impl IntoView {
    let layout_ctx = use_context::<LayoutContext>().expect("LayoutContext required");

    // Create node refs for container and header (drag handle)
    let container_el = NodeRef::<Div>::new();
    let header_el = NodeRef::<Div>::new();

    // Use leptos-use draggable hook - container is the target, header is the handle
    let UseDraggableReturn { style, .. } = use_draggable_with_options(
        container_el,
        UseDraggableOptions::default()
            .initial_value(Position { x: 200.0, y: 200.0 })
            .prevent_default(true)
            .handle(Some(header_el)),
    );

    view! {
        <Show when=move || layout_ctx.io_popped.get()>
            <div
                node_ref=container_el
                class="fixed bg-card rounded border border-primary/40 shadow-2xl z-50 w-72 select-none flex flex-col"
                style=move || format!(
                    "touch-action: none; {}",
                    style.get()
                )
            >
                // Header with close button - THIS is the drag handle
                <div
                    node_ref=header_el
                    class="flex items-center justify-between p-2 cursor-move border-b border-border/6 shrink-0"
                >
                    <h3 class="text-[10px] font-semibold text-primary uppercase tracking-wide flex items-center gap-1">
                        <svg class="w-3 h-3 text-muted-foreground" fill="currentColor" viewBox="0 0 24 24">
                            <path d="M8 6h2v2H8V6zm6 0h2v2h-2V6zM8 11h2v2H8v-2zm6 0h2v2h-2v-2zm-6 5h2v2H8v-2zm6 0h2v2h-2v-2z"/>
                        </svg>
                        "I/O Status"
                    </h3>
                    <button
                        class="p-0.5 hover:bg-border/6 rounded cursor-pointer"
                        title="Dock I/O panel"
                        on:click=move |_| layout_ctx.io_popped.set(false)
                    >
                        <svg class="w-3 h-3 text-muted-foreground hover:text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                        </svg>
                    </button>
                </div>

                // Content area - NOT draggable
                <div class="p-2 flex-1 overflow-auto">
                    <IoStatusPanel start_collapsed=false show_popout=false/>
                </div>
            </div>
        </Show>
    }
}
