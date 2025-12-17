//! Desktop layout components.

mod top_bar;
mod left_navbar;
mod right_panel;
mod floating;

pub use top_bar::TopBar;
pub use left_navbar::LeftNavbar;
pub use right_panel::RightPanel;
pub use floating::{FloatingJogControls, FloatingIOStatus};

use leptos::prelude::*;
use leptos_router::hooks::use_location;

use crate::pages::MainWorkspace;

/// Desktop layout context - provides shared state across layout components.
#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct LayoutContext {
    /// Current dashboard tab (0 = Control, 1 = Info).
    pub dashboard_tab: RwSignal<usize>,
    /// Whether jog controls are popped out (floating).
    pub jog_popped: RwSignal<bool>,
    /// Jog controls floating position (x, y).
    pub jog_position: RwSignal<(i32, i32)>,
    /// Whether I/O status panel is popped out (floating).
    pub io_popped: RwSignal<bool>,
    /// Whether the program browser sidebar is visible.
    pub show_program_browser: RwSignal<bool>,
}

impl LayoutContext {
    pub fn new() -> Self {
        Self {
            dashboard_tab: RwSignal::new(0),
            jog_popped: RwSignal::new(false),
            jog_position: RwSignal::new((100, 100)),
            io_popped: RwSignal::new(false),
            show_program_browser: RwSignal::new(false),
        }
    }
}

/// Workspace context - provides shared state for workspace components.
#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct WorkspaceContext {
    /// Recently used commands for quick access.
    pub recent_commands: RwSignal<Vec<String>>,
    /// Currently selected command ID.
    pub selected_command_id: RwSignal<Option<i64>>,
    /// Command execution log.
    pub command_log: RwSignal<Vec<String>>,
}

impl WorkspaceContext {
    pub fn new() -> Self {
        Self {
            recent_commands: RwSignal::new(Vec::new()),
            selected_command_id: RwSignal::new(None),
            command_log: RwSignal::new(Vec::new()),
        }
    }
}

/// Root desktop layout component.
#[component]
pub fn DesktopLayout() -> impl IntoView {
    // Create and provide layout context
    let layout_ctx = LayoutContext::new();
    provide_context(layout_ctx);

    // Create and provide workspace context
    let workspace_ctx = WorkspaceContext::new();
    provide_context(workspace_ctx);

    // Get current location to determine if we're on dashboard
    let location = use_location();
    let is_dashboard = move || {
        let path = location.pathname.get();
        path == "/" || path.starts_with("/dashboard")
    };

    view! {
        <div class="h-screen w-screen flex flex-col bg-[#0a0a0a] overflow-hidden">
            // Header
            <TopBar/>

            // Main content area (navbar + workspace + right panel)
            <div class="flex-1 flex overflow-hidden">
                // Left navbar
                <LeftNavbar/>

                // Main workspace with routes
                <MainWorkspace/>

                // Right panel (only visible in Dashboard routes)
                <Show when=is_dashboard>
                    <RightPanel/>
                </Show>
            </div>
        </div>
    }
}

