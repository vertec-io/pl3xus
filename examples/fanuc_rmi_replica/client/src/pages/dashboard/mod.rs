//! Dashboard page with Control and Info tabs.

pub mod context;
mod control;
mod info;

pub use context::{SystemEntityContext, use_system_entity};
pub use control::ControlTab;
pub use info::InfoTab;

use leptos::prelude::*;
use leptos_router::hooks::use_location;

/// Dashboard view with tabbed navigation.
/// NOTE: WorkspaceContext is provided by DesktopLayout, not here.
#[component]
pub fn DashboardView() -> impl IntoView {
    let location = use_location();

    let is_control = move || {
        let path = location.pathname.get();
        path.ends_with("/control") || path == "/dashboard" || path == "/"
    };

    view! {
        <div class="flex-1 flex flex-col overflow-hidden">
            // Tab bar with navigation links
            <div class="flex border-b border-border/8 shrink-0">
                <a
                    href="/dashboard/control"
                    class=move || if is_control() {
                        "px-4 py-2 text-[10px] font-medium text-primary border-b-2 border-primary transition-colors"
                    } else {
                        "px-4 py-2 text-[10px] font-medium text-muted-foreground hover:text-muted-foreground transition-colors"
                    }
                >"Control"</a>
                <a
                    href="/dashboard/info"
                    class=move || if !is_control() {
                        "px-4 py-2 text-[10px] font-medium text-primary border-b-2 border-primary transition-colors"
                    } else {
                        "px-4 py-2 text-[10px] font-medium text-muted-foreground hover:text-muted-foreground transition-colors"
                    }
                >"Configuration"</a>
            </div>

            // Tab content (outlet)
            <div class="flex-1 overflow-auto p-3">
                <leptos_router::components::Outlet/>
            </div>
        </div>
    }
}
