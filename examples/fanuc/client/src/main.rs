use pl3xus_client::{
    ClientTypeRegistry, SyncProvider, use_sync_component,
};

#[cfg(target_arch = "wasm32")]
use pl3xus_client::devtools::DevTools;
use fanuc_real_types::{RobotPosition, RobotStatus, JointAngles, RobotInfo};
use leptos::prelude::*;

// SyncComponent is automatically implemented for all Serialize + Deserialize types.
// No manual implementation needed!

fn main() {
    console_error_panic_hook::set_once();
    _ = console_log::init_with_level(log::Level::Debug);

    leptos::mount::mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    // Create type registry with DevTools support
    let registry = ClientTypeRegistry::builder()
        .register::<RobotPosition>()
        .register::<RobotStatus>()
        .register::<JointAngles>()
        .register::<RobotInfo>()
        .with_devtools_support()  // Enable DevTools
        .build();

    let ws_url = "ws://127.0.0.1:8082/sync";
    let devtools_url = "ws://127.0.0.1:8082/sync?devtools=true";

    view! {
        <SyncProvider url=ws_url.to_string() registry=registry.clone() auto_connect=true>
            <div class="min-h-screen w-screen bg-slate-950 text-slate-50 flex flex-col">
                <header class="border-b border-slate-800 bg-slate-900/80 backdrop-blur px-6 py-4">
                    <h1 class="text-lg font-semibold tracking-tight">"FANUC Real Robot Control"</h1>
                    <p class="text-xs text-slate-400">"Real FANUC simulator control using pl3xus_sync"</p>
                </header>
                <div class="flex-1 flex overflow-hidden">
                    <main class="flex-1 p-6 overflow-auto">
                        <div class="max-w-4xl mx-auto space-y-6">
                            <RobotStatusDisplay />
                            <PositionDisplay />
                            <JointAnglesDisplay />
                        </div>
                    </main>
                    <aside class="w-96 border-l border-slate-800 overflow-hidden">
                        {
                            #[cfg(target_arch = "wasm32")]
                            {
                                view! { <DevTools ws_url=devtools_url registry=registry /> }
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                view! { <div>"DevTools only available on WASM"</div> }
                            }
                        }
                    </aside>
                </div>
            </div>
        </SyncProvider>
    }
}

#[component]
fn RobotStatusDisplay() -> impl IntoView {
    let robot_statuses = use_sync_component::<RobotStatus>();

    // Get the first robot status (assuming single robot)
    let robot_status = move || {
        robot_statuses.get()
            .values()
            .next()
            .cloned()
    };

    view! {
        <div class="bg-slate-900 rounded-lg border border-slate-800 p-4">
            <h2 class="text-sm font-semibold mb-3">"Robot Status"</h2>
            <div class="grid grid-cols-2 gap-3 text-xs">
                <div class="flex items-center gap-2">
                    <div class="w-2 h-2 rounded-full"
                        class:bg-emerald-500=move || robot_status().map(|s| s.servo_ready).unwrap_or(false)
                        class:bg-slate-600=move || !robot_status().map(|s| s.servo_ready).unwrap_or(false)
                    ></div>
                    <span class="text-slate-400">"Servo Ready"</span>
                </div>
                <div class="flex items-center gap-2">
                    <div class="w-2 h-2 rounded-full"
                        class:bg-emerald-500=move || robot_status().map(|s| s.tp_enabled).unwrap_or(false)
                        class:bg-slate-600=move || !robot_status().map(|s| s.tp_enabled).unwrap_or(false)
                    ></div>
                    <span class="text-slate-400">"TP Enabled"</span>
                </div>
                <div class="flex items-center gap-2">
                    <div class="w-2 h-2 rounded-full"
                        class:bg-amber-500=move || robot_status().map(|s| s.in_motion).unwrap_or(false)
                        class:bg-slate-600=move || !robot_status().map(|s| s.in_motion).unwrap_or(false)
                    ></div>
                    <span class="text-slate-400">"In Motion"</span>
                </div>
            </div>
        </div>
    }
}

#[component]
fn PositionDisplay() -> impl IntoView {
    let robot_positions = use_sync_component::<RobotPosition>();

    // Get the first robot position (assuming single robot)
    let robot_position = move || {
        robot_positions.get()
            .values()
            .next()
            .cloned()
    };

    view! {
        <div class="bg-slate-900 rounded-lg border border-slate-800 p-4">
            <h2 class="text-sm font-semibold mb-3">"Robot Position (Cartesian)"</h2>
            <div class="grid grid-cols-3 gap-3 text-xs">
                <div>
                    <div class="text-slate-400 mb-1">"X (mm)"</div>
                    <div class="font-mono text-emerald-400">
                        {move || format!("{:.2}", robot_position().map(|p| p.x).unwrap_or(0.0))}
                    </div>
                </div>
                <div>
                    <div class="text-slate-400 mb-1">"Y (mm)"</div>
                    <div class="font-mono text-emerald-400">
                        {move || format!("{:.2}", robot_position().map(|p| p.y).unwrap_or(0.0))}
                    </div>
                </div>
                <div>
                    <div class="text-slate-400 mb-1">"Z (mm)"</div>
                    <div class="font-mono text-emerald-400">
                        {move || format!("{:.2}", robot_position().map(|p| p.z).unwrap_or(0.0))}
                    </div>
                </div>
                <div>
                    <div class="text-slate-400 mb-1">"W (deg)"</div>
                    <div class="font-mono text-emerald-400">
                        {move || format!("{:.2}", robot_position().map(|p| p.w).unwrap_or(0.0))}
                    </div>
                </div>
                <div>
                    <div class="text-slate-400 mb-1">"P (deg)"</div>
                    <div class="font-mono text-emerald-400">
                        {move || format!("{:.2}", robot_position().map(|p| p.p).unwrap_or(0.0))}
                    </div>
                </div>
                <div>
                    <div class="text-slate-400 mb-1">"R (deg)"</div>
                    <div class="font-mono text-emerald-400">
                        {move || format!("{:.2}", robot_position().map(|p| p.r).unwrap_or(0.0))}
                    </div>
                </div>
            </div>
        </div>
    }
}

#[component]
fn JointAnglesDisplay() -> impl IntoView {
    let joint_angles = use_sync_component::<JointAngles>();

    // Get the first joint angles (assuming single robot)
    let joints = move || {
        joint_angles.get()
            .values()
            .next()
            .cloned()
    };

    view! {
        <div class="bg-slate-900 rounded-lg border border-slate-800 p-4">
            <h2 class="text-sm font-semibold mb-3">"Joint Angles"</h2>
            <div class="grid grid-cols-3 gap-3 text-xs">
                <div>
                    <div class="text-slate-400 mb-1">"J1 (deg)"</div>
                    <div class="font-mono text-blue-400">
                        {move || format!("{:.2}", joints().map(|j| j.j1).unwrap_or(0.0))}
                    </div>
                </div>
                <div>
                    <div class="text-slate-400 mb-1">"J2 (deg)"</div>
                    <div class="font-mono text-blue-400">
                        {move || format!("{:.2}", joints().map(|j| j.j2).unwrap_or(0.0))}
                    </div>
                </div>
                <div>
                    <div class="text-slate-400 mb-1">"J3 (deg)"</div>
                    <div class="font-mono text-blue-400">
                        {move || format!("{:.2}", joints().map(|j| j.j3).unwrap_or(0.0))}
                    </div>
                </div>
                <div>
                    <div class="text-slate-400 mb-1">"J4 (deg)"</div>
                    <div class="font-mono text-blue-400">
                        {move || format!("{:.2}", joints().map(|j| j.j4).unwrap_or(0.0))}
                    </div>
                </div>
                <div>
                    <div class="text-slate-400 mb-1">"J5 (deg)"</div>
                    <div class="font-mono text-blue-400">
                        {move || format!("{:.2}", joints().map(|j| j.j5).unwrap_or(0.0))}
                    </div>
                </div>
                <div>
                    <div class="text-slate-400 mb-1">"J6 (deg)"</div>
                    <div class="font-mono text-blue-400">
                        {move || format!("{:.2}", joints().map(|j| j.j6).unwrap_or(0.0))}
                    </div>
                </div>
            </div>
        </div>
    }
}

