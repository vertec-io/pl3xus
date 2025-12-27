//! Right panel with status display, position, jog controls, and I/O status.

use leptos::prelude::*;
use pl3xus_client::use_entity_component;
use fanuc_replica_plugins::ConnectionState;

use crate::components::{StatusPanel, PositionDisplay, JogControls, IoStatusPanel, ErrorLog};
use crate::layout::LayoutContext;
use crate::pages::dashboard::use_system_entity;

/// Right panel component (visible on dashboard).
#[component]
pub fn RightPanel() -> impl IntoView {
    let layout_ctx = use_context::<LayoutContext>().expect("LayoutContext required");
    let system_ctx = use_system_entity();

    // Subscribe to the robot's connection state (ConnectionState lives on robot entity)
    let (connection_state, robot_exists) = use_entity_component::<ConnectionState, _>(move || system_ctx.robot_entity_id.get());

    let robot_connected = Memo::new(move |_| robot_exists.get() && connection_state.get().robot_connected);

    view! {
        <aside class="w-56 bg-background border-l border-border/8 flex flex-col overflow-hidden shrink-0">
            <div class="flex-1 overflow-y-auto p-1.5 space-y-1.5">
                // Robot status (compact)
                <StatusPanel/>

                // Position display
                <PositionDisplay/>

                // Errors panel
                <ErrorLog/>

                // I/O Status (only show when robot connected and not popped)
                <Show when=move || robot_connected.get() && !layout_ctx.io_popped.get()>
                    <IOStatusPanelWrapper/>
                </Show>

                // Jog controls (only show when robot connected and not popped)
                <Show when=move || robot_connected.get() && !layout_ctx.jog_popped.get()>
                    <JogControlsPanelWrapper/>
                </Show>
            </div>
        </aside>
    }
}

/// I/O Status panel wrapper with pop-out button.
#[component]
fn IOStatusPanelWrapper() -> impl IntoView {
    let layout_ctx = use_context::<LayoutContext>().expect("LayoutContext required");

    view! {
        <div class="relative">
            // Pop-out button
            <button
                class="absolute top-1.5 right-1.5 p-0.5 hover:bg-border/10 rounded z-10"
                title="Pop out I/O panel"
                on:click=move |_| layout_ctx.io_popped.set(true)
            >
                <svg class="w-3 h-3 text-muted-foreground hover:text-primary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"/>
                </svg>
            </button>
            <IoStatusPanel/>
        </div>
    }
}

/// Jog controls panel wrapper with pop-out button.
#[component]
fn JogControlsPanelWrapper() -> impl IntoView {
    let layout_ctx = use_context::<LayoutContext>().expect("LayoutContext required");

    view! {
        <div class="relative">
            // Pop-out button
            <button
                class="absolute top-1.5 right-1.5 p-0.5 hover:bg-border/10 rounded z-10"
                title="Pop out jog controls"
                on:click=move |_| layout_ctx.jog_popped.set(true)
            >
                <svg class="w-3 h-3 text-muted-foreground hover:text-primary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"/>
                </svg>
            </button>
            <JogControls/>
        </div>
    }
}
