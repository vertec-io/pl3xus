//! Position and joint angles display components.

use leptos::prelude::*;

use pl3xus_client::use_entity_component;
use fanuc_replica_types::*;
use crate::pages::dashboard::use_system_entity;

/// Position display showing XYZ and WPR values.
#[component]
pub fn PositionDisplay() -> impl IntoView {
    let ctx = use_system_entity();

    // Subscribe to the active robot's position and joint angles
    let (pos, _pos_exists) = use_entity_component::<RobotPosition, _>(move || ctx.robot_entity_id.get());
    let (joints, _joints_exists) = use_entity_component::<JointAngles, _>(move || ctx.robot_entity_id.get());

    let get_pos = move || pos.get();
    let get_joints = move || joints.get();

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-2">
            <h2 class="text-[10px] font-semibold text-[#00d9ff] mb-1.5 uppercase tracking-wide">"Position"</h2>
            
            // Cartesian Position
            <div class="grid grid-cols-3 gap-1 mb-2">
                <PositionItem label="X" value=move || get_pos().x as f32 />
                <PositionItem label="Y" value=move || get_pos().y as f32 />
                <PositionItem label="Z" value=move || get_pos().z as f32 />
                <PositionItem label="W" value=move || get_pos().w as f32 />
                <PositionItem label="P" value=move || get_pos().p as f32 />
                <PositionItem label="R" value=move || get_pos().r as f32 />
            </div>
            
            // Joint Angles
            <h2 class="text-[10px] font-semibold text-[#00d9ff] mb-1.5 uppercase tracking-wide">"Joint Angles"</h2>
            <div class="grid grid-cols-3 gap-1">
                <PositionItem label="J1" value=move || get_joints().j1 />
                <PositionItem label="J2" value=move || get_joints().j2 />
                <PositionItem label="J3" value=move || get_joints().j3 />
                <PositionItem label="J4" value=move || get_joints().j4 />
                <PositionItem label="J5" value=move || get_joints().j5 />
                <PositionItem label="J6" value=move || get_joints().j6 />
            </div>
        </div>
    }
}

#[component]
fn PositionItem<F>(label: &'static str, value: F) -> impl IntoView 
where F: Fn() -> f32 + Copy + Send + Sync + 'static {
    view! {
        <div class="flex justify-between items-center bg-[#111111] rounded px-1.5 py-1">
             <span class="text-[#888888] text-[10px] font-medium">{label}</span>
             <span class="text-[11px] font-mono text-[#aaaaaa] tabular-nums">
                {move || format!("{:.2}", value())}
             </span>
        </div>
    }
}
