//! Status panel component showing robot state.

use leptos::prelude::*;

use pl3xus_client::use_entity_component;
use fanuc_replica_types::*;
use crate::pages::dashboard::use_system_entity;

/// Status panel showing robot status indicators.
#[component]
pub fn StatusPanel() -> impl IntoView {
    let ctx = use_system_entity();

    // Subscribe to the active robot's components
    // All these components live on the robot entity
    let (status, _) = use_entity_component::<RobotStatus, _>(move || ctx.robot_entity_id.get());
    let (connection, robot_exists) = use_entity_component::<ConnectionState, _>(move || ctx.robot_entity_id.get());
    let (frame_tool, _) = use_entity_component::<FrameToolDataState, _>(move || ctx.robot_entity_id.get());

    let is_connected = move || robot_exists.get() && connection.get().robot_connected;

    let get_status = move || status.get();
    let get_frame_tool = move || frame_tool.get();

    view! {
        <div class="bg-background rounded border border-border/8 p-2">
            <h2 class="text-[10px] font-semibold text-primary mb-1.5 uppercase tracking-wide">"Status"</h2>
            <div class="grid grid-cols-4 gap-1">
                <StatusIndicator label="Servo" value=move || get_status().servo_ready />
                <StatusIndicator label="TP" value=move || get_status().tp_enabled />
                <StatusIndicator label="Motion" value=move || get_status().in_motion />
                <StatusIndicatorText label="Error" value=move || get_status().error_message.clone().unwrap_or_else(|| "None".to_string()) />
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

#[component]
fn StatusIndicatorText<F>(label: &'static str, value: F) -> impl IntoView 
where F: Fn() -> String + Send + Sync + 'static {
     view! {
        <div class="bg-card rounded px-1 py-1 text-center">
            <div class="text-muted-foreground text-[8px] mb-0.5">{label}</div>
            <div class="text-[10px] font-semibold text-foreground truncate">{value}</div>
        </div>
    }
}
