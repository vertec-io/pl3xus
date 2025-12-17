//! Application root component.

use std::sync::Arc;

use leptos::prelude::*;
use leptos_router::components::Router;

use pl3xus_client::{ClientTypeRegistry, SyncProvider, EntityControl};
use fanuc_replica_types::*;

use crate::layout::{DesktopLayout, FloatingJogControls, FloatingIOStatus};

/// Build the client type registry with all synced components.
fn build_registry() -> Arc<ClientTypeRegistry> {
    ClientTypeRegistry::builder()
        .register::<RobotPosition>()
        .register::<JointAngles>()
        .register::<RobotStatus>()
        .register::<EntityControl>()
        .register::<IoStatus>()
        .register::<ExecutionState>()
        .register::<ConnectionState>()
        .register::<ActiveConfigState>()
        .register::<JogSettingsState>()
        .build()
}

/// Root application component.
#[component]
pub fn App() -> impl IntoView {
    let registry = build_registry();
    let ws_url = "ws://127.0.0.1:8083/sync";

    view! {
        <SyncProvider url=ws_url.to_string() registry=registry auto_connect=true>
            <Router>
                <DesktopLayout/>
            </Router>
            // Floating controls (rendered outside normal flow)
            <FloatingJogControls/>
            <FloatingIOStatus/>
        </SyncProvider>
    }
}

