//! Application pages (routes).

pub mod dashboard;
mod programs;
mod settings;

pub use dashboard::DashboardView;
pub use programs::ProgramsView;
pub use settings::SettingsView;

use leptos::prelude::*;
use leptos_router::components::{ParentRoute, Route, Routes, Redirect};
use leptos_router::path;

/// Main workspace with route definitions.
#[component]
pub fn MainWorkspace() -> impl IntoView {
    view! {
        <main class="flex-1 overflow-hidden flex flex-col">
            <Routes fallback=|| "Not Found">
                <ParentRoute path=path!("/dashboard") view=DashboardView>
                    <Route path=path!("control") view=dashboard::ControlTab />
                    <Route path=path!("info") view=dashboard::InfoTab />
                    <Route path=path!("") view=|| view! { <Redirect path="/dashboard/control" /> } />
                </ParentRoute>
                <Route path=path!("/programs") view=ProgramsView />
                <Route path=path!("/settings") view=SettingsView />
                <Route path=path!("/") view=|| view! { <Redirect path="/dashboard/control" /> } />
            </Routes>
        </main>
    }
}
