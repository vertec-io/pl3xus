//! Status panel component showing robot state.

use leptos::prelude::*;

use pl3xus_client::use_entity_component;
use fanuc_replica_plugins::*;
use crate::pages::dashboard::use_system_entity;
use crate::pages::dashboard::context::WorkspaceContext;

/// Status panel showing robot status indicators.
#[component]
pub fn StatusPanel() -> impl IntoView {
    let ctx = use_system_entity();
    let workspace_ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let error_log = workspace_ctx.error_log;

    // Subscribe to the active robot's components
    // All these components live on the robot entity
    let (status, _) = use_entity_component::<RobotStatus, _>(move || ctx.robot_entity_id.get());
    let (connection, robot_exists) = use_entity_component::<ConnectionState, _>(move || ctx.robot_entity_id.get());
    let (frame_tool, _) = use_entity_component::<FrameToolDataState, _>(move || ctx.robot_entity_id.get());

    let is_connected = move || robot_exists.get() && connection.get().robot_connected;

    let get_status = move || status.get();
    let get_frame_tool = move || frame_tool.get();

    // Check if there's an error
    let has_robot_error = move || get_status().error_message.is_some();
    let error_count = move || error_log.get().len();

    view! {
        <div class="bg-background rounded border border-border/8 p-2">
            <h2 class="text-[10px] font-semibold text-primary mb-1.5 uppercase tracking-wide">"Status"</h2>
            <div class="grid grid-cols-3 gap-1">
                <StatusIndicator label="Servo" value=move || get_status().servo_ready />
                <StatusIndicator label="TP" value=move || get_status().tp_enabled />
                <StatusIndicator label="Motion" value=move || get_status().in_motion />
            </div>

            // Error display row - shows robot error or error count
            <div class="mt-1.5 flex items-center gap-1.5">
                // Robot hardware error
                <div class=move || format!("flex-1 flex items-center gap-1 px-1.5 py-1 rounded text-[9px] {}",
                    if has_robot_error() { "bg-destructive/15 border border-destructive/25" } else { "bg-card" }
                )>
                    <svg class=move || format!("w-3 h-3 shrink-0 {}", if has_robot_error() { "text-destructive" } else { "text-muted-foreground" })
                        fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                    </svg>
                    <span class=move || if has_robot_error() { "text-destructive font-medium truncate" } else { "text-muted-foreground truncate" }>
                        {move || get_status().error_message.clone().unwrap_or_else(|| "No errors".to_string())}
                    </span>
                </div>
                // Console error count badge
                {move || {
                    let count = error_count();
                    (count > 0).then(|| view! {
                        <div class="flex items-center gap-1 px-1.5 py-1 rounded bg-destructive/15 border border-destructive/25">
                            <svg class="w-3 h-3 text-destructive shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                            </svg>
                            <span class="text-destructive text-[9px] font-semibold">{count}</span>
                        </div>
                    })
                }}
            </div>

            // Active Frame/Tool display - only show when robot is connected
            <Show when=move || is_connected()>
                <div class="mt-2 pt-2 border-t border-border/8">
                    <div class="flex items-center justify-between">
                        <span class="text-muted-foreground text-[8px]">"UFrame:"</span>
                        <span class="text-[10px] font-medium text-primary">{move || get_frame_tool().active_frame}</span>
                        <span class="text-muted-foreground text-[8px] ml-2">"UTool:"</span>
                        <span class="text-[10px] font-medium text-primary">{move || get_frame_tool().active_tool}</span>
                    </div>
                </div>
            </Show>
        </div>
    }
}

#[component]
fn StatusIndicator<F>(label: &'static str, value: F) -> impl IntoView
where F: Fn() -> bool + Copy + Send + Sync + 'static {
    view! {
        <div class="bg-card rounded px-1 py-1 text-center">
            <div class="text-muted-foreground text-[8px] mb-0.5">{label}</div>
            <div class=move || if value() {
                 "text-[10px] font-semibold text-primary"
            } else {
                 "text-[10px] font-semibold text-muted-foreground"
            }>
                {move || if value() { "ON" } else { "OFF" }}
            </div>
        </div>
    }
}
