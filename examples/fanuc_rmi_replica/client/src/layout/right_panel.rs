//! Right panel with status display, position, jog controls, and I/O status.

use leptos::prelude::*;
use pl3xus_client::use_sync_component;
use fanuc_replica_types::ConnectionState;

use crate::components::{StatusPanel, PositionDisplay, JogControls, IoStatusPanel, ErrorLog};
use crate::layout::LayoutContext;

/// Right panel component (visible on dashboard).
#[component]
pub fn RightPanel() -> impl IntoView {
    let layout_ctx = use_context::<LayoutContext>().expect("LayoutContext required");
    let connection_state = use_sync_component::<ConnectionState>();

    let robot_connected = Memo::new(move |_| {
        connection_state.get().values().next()
            .map(|s| s.robot_connected)
            .unwrap_or(false)
    });

    view! {
        <aside class="w-56 bg-[#0d0d0d] border-l border-[#ffffff08] flex flex-col overflow-hidden shrink-0">
            <div class="flex-1 overflow-y-auto p-1.5 space-y-1.5">
                // Robot status (compact)
                <StatusPanel/>

                // Position display
                <PositionDisplay/>

                // Errors panel
                <ErrorLog/>

                // I/O Status (only show when robot connected and not popped)
                <Show when=move || robot_connected.get() && !layout_ctx.io_popped.get()>
                    <IoStatusPanel/>
                </Show>

                // Jog controls (only show when robot connected and not popped)
                <Show when=move || robot_connected.get() && !layout_ctx.jog_popped.get()>
                    <JogControls/>
                </Show>
            </div>
        </aside>
    }
}
