//! Desktop layout components.

mod top_bar;
mod left_navbar;
mod right_panel;
mod floating;

pub use top_bar::{TopBar, ControlResponseHandler, ConnectionStateHandler, ProgramNotificationHandler, ConsoleLogHandler, ServerNotificationHandler};
pub use left_navbar::LeftNavbar;
pub use right_panel::RightPanel;
pub use floating::{FloatingJogControls, FloatingIOStatus};

use leptos::prelude::*;
use leptos_router::hooks::use_location;
use pl3xus_client::use_components;
use fanuc_replica_types::{ActiveSystem, ActiveRobot};

use crate::pages::MainWorkspace;
use crate::pages::dashboard::context::WorkspaceContext;
use crate::pages::dashboard::SystemEntityContext;
use crate::components::ThemeModal;

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
    /// Currently selected/open program ID - persists across navigation.
    pub selected_program_id: RwSignal<Option<i64>>,
    /// Whether the theme selection modal is visible.
    pub show_themes: RwSignal<bool>,
}

impl LayoutContext {
    pub fn new() -> Self {
        Self {
            dashboard_tab: RwSignal::new(0),
            jog_popped: RwSignal::new(false),
            jog_position: RwSignal::new((100, 100)),
            io_popped: RwSignal::new(false),
            show_program_browser: RwSignal::new(false),
            selected_program_id: RwSignal::new(None),
            show_themes: RwSignal::new(false),
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

    // Subscribe to ActiveSystem and ActiveRobot to get entity IDs.
    // These markers are synced from the server and only exist on their respective entities.
    // This provides child components with entity IDs for:
    // - Entity-specific component subscriptions
    // - Targeted messages to the correct entity (System vs Robot)
    //
    // IMPORTANT: We use Memo instead of Signal::derive because:
    // - Signal::derive re-notifies subscribers whenever the source signal changes
    // - Memo only notifies when the computed value actually changes
    // - Without Memo, updates would trigger all downstream Effects,
    //   even if the entity_id stayed the same, causing infinite reactivity loops
    let active_systems = use_components::<ActiveSystem>();
    let active_robots = use_components::<ActiveRobot>();
    let system_entity_id = Memo::new(move |_| active_systems.get().keys().next().copied());
    let robot_entity_id = Memo::new(move |_| active_robots.get().keys().next().copied());
    provide_context(SystemEntityContext::new(
        system_entity_id.into(),
        robot_entity_id.into(),
    ));

    // Get current location to determine if we're on dashboard
    let location = use_location();
    let is_dashboard = move || {
        let path = location.pathname.get();
        path == "/" || path.starts_with("/dashboard")
    };

    view! {
        <div class="h-screen w-screen flex flex-col bg-background overflow-hidden">
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

            <ThemeModal show=layout_ctx.show_themes />
        </div>

        // NOTE: ExecutionStateHandler has been REMOVED as part of the architecture refactor.
        // UI components now read directly from the synced ExecutionState component using
        // use_components::<ExecutionState>(). This ensures all clients see the same state.
        // See ARCHITECTURE_SPECIFICATION.md for details.
    }
}

