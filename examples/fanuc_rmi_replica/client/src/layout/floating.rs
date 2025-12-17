//! Floating panel components for jog controls and I/O status.

use leptos::prelude::*;

use crate::components::{JogControls, IoStatusPanel};
use crate::layout::LayoutContext;

/// Floating jog controls component (draggable).
#[component]
pub fn FloatingJogControls() -> impl IntoView {
    let layout_ctx = use_context::<LayoutContext>().expect("LayoutContext required");

    view! {
        <Show when=move || layout_ctx.jog_popped.get()>
            <div
                class="fixed bg-[#0d0d0d] border border-[#ffffff20] rounded-lg shadow-2xl z-50"
                style=move || format!(
                    "left: {}px; top: {}px;",
                    layout_ctx.jog_position.get().0,
                    layout_ctx.jog_position.get().1
                )
            >
                <div class="px-2 py-1 border-b border-[#ffffff10] flex items-center justify-between cursor-move">
                    <span class="text-[10px] text-[#888888] font-medium">"JOG CONTROLS"</span>
                    <button
                        class="text-[#666666] hover:text-white text-sm"
                        on:click=move |_| layout_ctx.jog_popped.set(false)
                    >
                        "✕"
                    </button>
                </div>
                <JogControls/>
            </div>
        </Show>
    }
}

/// Floating I/O status panel component.
#[component]
pub fn FloatingIOStatus() -> impl IntoView {
    let layout_ctx = use_context::<LayoutContext>().expect("LayoutContext required");

    view! {
        <Show when=move || layout_ctx.io_popped.get()>
            <div class="fixed right-4 bottom-4 bg-[#0d0d0d] border border-[#ffffff20] rounded-lg shadow-2xl z-50 w-72">
                <div class="px-2 py-1 border-b border-[#ffffff10] flex items-center justify-between">
                    <span class="text-[10px] text-[#888888] font-medium">"I/O STATUS"</span>
                    <button
                        class="text-[#666666] hover:text-white text-sm"
                        on:click=move |_| layout_ctx.io_popped.set(false)
                    >
                        "✕"
                    </button>
                </div>
                <IoStatusPanel/>
            </div>
        </Show>
    }
}
