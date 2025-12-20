//! Status panel component showing robot state.

use leptos::prelude::*;

use pl3xus_client::use_sync_component;
use fanuc_replica_types::*;

/// Status panel showing robot status indicators.
#[component]
pub fn StatusPanel() -> impl IntoView {
    let status = use_sync_component::<RobotStatus>();
    let connection = use_sync_component::<ConnectionState>();
    let frame_tool = use_sync_component::<FrameToolDataState>();

    let get_status = move || status.get().values().next().cloned().unwrap_or_default();
    let get_connection = move || connection.get().values().next().cloned().unwrap_or_default();
    let get_frame_tool = move || frame_tool.get().values().next().cloned().unwrap_or_default();

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-2">
            <h2 class="text-[10px] font-semibold text-[#00d9ff] mb-1.5 uppercase tracking-wide">"Status"</h2>
            <div class="grid grid-cols-4 gap-1">
                <StatusIndicator label="Servo" value=move || get_status().servo_ready />
                <StatusIndicator label="TP" value=move || get_status().tp_enabled />
                <StatusIndicator label="Motion" value=move || get_status().in_motion />
                <StatusIndicatorText label="Error" value=move || get_status().error_message.clone().unwrap_or_else(|| "None".to_string()) />
            </div>

            // Active Frame/Tool display - only show when robot is connected
            <Show when=move || get_connection().robot_connected>
                <div class="mt-2 pt-2 border-t border-[#ffffff08]">
                    <div class="flex items-center justify-between">
                        <span class="text-[#666666] text-[8px]">"UFrame:"</span>
                        <span class="text-[10px] font-medium text-[#00d9ff]">{move || get_frame_tool().active_frame}</span>
                        <span class="text-[#666666] text-[8px] ml-2">"UTool:"</span>
                        <span class="text-[10px] font-medium text-[#00d9ff]">{move || get_frame_tool().active_tool}</span>
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
        <div class="bg-[#111111] rounded px-1 py-1 text-center">
            <div class="text-[#666666] text-[8px] mb-0.5">{label}</div>
            <div class=move || if value() {
                 "text-[10px] font-semibold text-[#00d9ff]"
            } else {
                 "text-[10px] font-semibold text-[#555555]"
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
        <div class="bg-[#111111] rounded px-1 py-1 text-center">
            <div class="text-[#666666] text-[8px] mb-0.5">{label}</div>
            <div class="text-[10px] font-semibold text-white truncate">{value}</div>
        </div>
    }
}
