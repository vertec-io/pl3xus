//! Application root component.

use std::sync::Arc;

use leptos::prelude::*;
use leptos_router::components::Router;

use pl3xus_client::{ClientTypeRegistry, SyncProvider, EntityControl, ControlResponse, ServerNotification};
#[cfg(feature = "devtools")]
use pl3xus_client::{DevTools, DevToolsMode, use_sync_context};
use fanuc_replica_types::*;

use crate::components::ToastProvider;
use crate::layout::{DesktopLayout, FloatingJogControls, FloatingIOStatus, ControlResponseHandler, ConnectionStateHandler, ProgramNotificationHandler, ConsoleLogHandler, ServerNotificationHandler};

/// Build the client type registry with all synced components.
fn build_registry() -> Arc<ClientTypeRegistry> {
    let builder = ClientTypeRegistry::builder()
        .register::<ActiveSystem>()
        .register::<ActiveRobot>()
        .register::<RobotPosition>()
        .register::<JointAngles>()
        .register::<RobotStatus>()
        .register::<EntityControl>()
        .register::<IoStatus>()
        .register::<IoConfigState>()
        .register::<ExecutionState>()
        .register::<ConnectionState>()
        .register::<ActiveConfigState>()
        .register::<JogSettingsState>()
        .register::<FrameToolDataState>()
        .register::<ControlResponse>()
        .register::<ProgramNotification>()
        .register::<ConsoleLogEntry>()
        .register::<ServerNotification>();

    #[cfg(feature = "devtools")]
    let builder = builder.with_devtools_support();

    builder.build()
}

/// Root application component.
#[component]
pub fn App() -> impl IntoView {
    let registry = build_registry();
    let ws_url = "ws://127.0.0.1:8083/sync";

    view! {
        <ToastProvider>
            <SyncProvider url=ws_url.to_string() registry=registry.clone() auto_connect=true>
                <Router>
                    <DesktopLayout/>
                </Router>
                // Floating controls (rendered outside normal flow)
                <FloatingJogControls/>
                <FloatingIOStatus/>
                // Headless components to handle server responses
                <ControlResponseHandler/>
                <ConnectionStateHandler/>
                <ProgramNotificationHandler/>
                <ConsoleLogHandler/>
                <ServerNotificationHandler/>
                // DevTools (when feature is enabled)
                <DevToolsWrapper registry=registry.clone() />
            </SyncProvider>
        </ToastProvider>
    }
}

/// DevTools wrapper component that has access to SyncContext
/// When devtools feature is disabled, this renders nothing
#[component]
fn DevToolsWrapper(registry: Arc<ClientTypeRegistry>) -> impl IntoView {
    #[cfg(feature = "devtools")]
    {
        let ctx = use_sync_context();
        view! {
            <DevTools
                ws_url="ws://127.0.0.1:8083/sync"
                registry=registry
                mode=DevToolsMode::Widget
                app_context=ctx
            />
        }.into_any()
    }

    #[cfg(not(feature = "devtools"))]
    {
        let _ = registry; // Suppress unused warning
        view! { <></> }.into_any()
    }
}

