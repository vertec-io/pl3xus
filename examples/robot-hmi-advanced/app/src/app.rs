//! Application root component.

use std::sync::Arc;

use leptos::prelude::*;
use leptos_router::components::Router;

use pl3xus_client::{ClientTypeRegistry, SyncProvider, EntityControl, ControlResponse, ServerNotification};
#[cfg(all(feature = "devtools", target_arch = "wasm32"))]
use pl3xus_client::{DevTools, DevToolsMode, use_sync_context};
use robot_hmi_plugins::core::types::{ActiveSystem, ConsoleLogEntry};
use robot_hmi_plugins::fanuc::types::{
    ActiveRobot, RobotPosition, JointAngles, RobotStatus, IoStatus, IoConfigState,
    ConnectionState, ActiveConfigState, JogSettingsState, FrameToolDataState,
};
use robot_hmi_plugins::execution::types::{BufferDisplayData, ExecutionState};
use robot_hmi_plugins::programs::types::{ProgramNotification, ProgramActions};

use crate::components::ToastProvider;
use crate::layout::{DesktopLayout, FloatingJogControls, FloatingIOStatus, ControlResponseHandler, ConnectionStateHandler, ProgramNotificationHandler, ConsoleLogHandler, ServerNotificationHandler};
use crate::theme::provide_theme_context;

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
        .register::<BufferDisplayData>()
        .register::<ConnectionState>()
        .register::<ActiveConfigState>()
        .register::<JogSettingsState>()
        .register::<FrameToolDataState>()
        .register::<ControlResponse>()
        .register::<ProgramNotification>()
        .register::<ProgramActions>()
        .register::<ConsoleLogEntry>()
        .register::<ServerNotification>();

    #[cfg(all(feature = "devtools", target_arch = "wasm32"))]
    let builder = builder.with_devtools_support();

    builder.build()
}

/// Root application component.
#[component]
pub fn App() -> impl IntoView {
    // Provide theme context at the root level
    provide_theme_context();

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
/// When devtools feature is disabled or not on wasm32, this renders nothing
#[component]
fn DevToolsWrapper(registry: Arc<ClientTypeRegistry>) -> impl IntoView {
    #[cfg(all(feature = "devtools", target_arch = "wasm32"))]
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

    #[cfg(not(all(feature = "devtools", target_arch = "wasm32")))]
    {
        let _ = registry; // Suppress unused warning
        view! { <></> }.into_any()
    }
}

