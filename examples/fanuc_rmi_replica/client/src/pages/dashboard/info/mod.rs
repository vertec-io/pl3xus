//! Dashboard Info tab - Frame, tool, and configuration management.
//!
//! Contains components for viewing user frames and user tools,
//! active configuration display, and jog defaults editing.

mod active_config;
mod jog_defaults;
mod frame_panel;
mod tool_panel;
mod frame_display;
mod tool_display;
mod number_input;

pub use active_config::ActiveConfigurationPanel;
pub use jog_defaults::JogDefaultsPanel;
pub use frame_panel::FrameManagementPanel;
pub use tool_panel::ToolManagementPanel;
pub use frame_display::MultiFrameDisplay;
pub use tool_display::MultiToolDisplay;
pub use number_input::NumberInput;

use leptos::prelude::*;
use pl3xus_client::use_sync_component;
use fanuc_replica_types::ConnectionState;
use crate::pages::dashboard::context::WorkspaceContext;

/// Info tab showing frame, tool, and configuration data.
#[component]
pub fn InfoTab() -> impl IntoView {
    let _ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let connection_state = use_sync_component::<ConnectionState>();

    let robot_connected = Memo::new(move |_| {
        connection_state.get().values().next()
            .map(|s| s.robot_connected)
            .unwrap_or(false)
    });

    view! {
        <div class="h-full flex flex-col gap-2 overflow-y-auto">
            // Show "No Robot Connected" message when not connected
            <Show when=move || !robot_connected.get() fallback=move || {
                view! {
                    // Active Configuration Panel (full width at top)
                    <ActiveConfigurationPanel/>

                    // Jog Defaults Panel (full width)
                    <JogDefaultsPanel/>

                    // Two-column layout for frames and tools
                    <div class="grid grid-cols-2 gap-2">
                        // Left column - Frames
                        <div class="flex flex-col gap-2">
                            <FrameManagementPanel/>
                            <MultiFrameDisplay/>
                        </div>

                        // Right column - Tools
                        <div class="flex flex-col gap-2">
                            <ToolManagementPanel/>
                            <MultiToolDisplay/>
                        </div>
                    </div>
                }
            }>
                <div class="h-full flex items-center justify-center">
                    <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-8 max-w-md text-center">
                        <svg class="w-16 h-16 mx-auto mb-4 text-[#666666]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z"/>
                        </svg>
                        <h2 class="text-lg font-semibold text-white mb-2">"No Robot Connected"</h2>
                        <p class="text-sm text-[#888888] mb-4">
                            "Connect to a robot to view and configure frame/tool settings, jog defaults, and arm configuration."
                        </p>
                        <p class="text-xs text-[#666666]">
                            "Use the Settings panel to create a robot connection, then connect from the Dashboard."
                        </p>
                    </div>
                </div>
            </Show>
        </div>
    }
}

