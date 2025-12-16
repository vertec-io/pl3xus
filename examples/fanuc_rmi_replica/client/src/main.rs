//! Fanuc RMI Replica Client - Full Implementation
//!
//! This client mirrors the Fanuc_RMI_API web application using pl3xus.
//!
//! # Key Features Demonstrated
//! - **Zero boilerplate sync**: `use_sync_component::<T>()` handles subscription lifecycle
//! - **Clean RPC**: `ctx.send(msg)` sends typed messages with automatic serialization
//! - **Blanket Pl3xusMessage**: No derive needed - just Serialize + Deserialize

use leptos::prelude::*;
use leptos_router::components::{ParentRoute, Redirect, Route, Router, Routes};
use leptos_router::path;
use leptos_router::hooks::use_location;

use pl3xus_client::{
    ClientTypeRegistry, SyncProvider, use_sync_component,
    use_sync_context, ControlRequest, EntityControl,
};
use fanuc_replica_types::*;

// ============================================================================
//                          MAIN ENTRY
// ============================================================================

fn main() {
    console_error_panic_hook::set_once();
    _ = console_log::init_with_level(log::Level::Debug);

    mount_to_body(|| view! { <App/> });
}

// ============================================================================
//                          CONTEXTS
// ============================================================================

/// Layout context - provides shared state across layout components.
#[derive(Clone, Copy)]
pub struct LayoutContext {
    /// Current dashboard tab (0 = Control, 1 = Info).
    pub dashboard_tab: RwSignal<usize>,
    /// Whether jog controls are popped out (floating).
    pub jog_popped: RwSignal<bool>,
    /// Jog controls floating position (x, y).
    pub jog_position: RwSignal<(i32, i32)>,
    /// Whether I/O status panel is popped out (floating).
    pub io_popped: RwSignal<bool>,
    /// Whether the program browser sidebar is visible.
    pub show_program_browser: RwSignal<bool>,
}

impl LayoutContext {
    pub fn new() -> Self {
        Self {
            dashboard_tab: RwSignal::new(0),
            jog_popped: RwSignal::new(false),
            jog_position: RwSignal::new((100, 100)),
            io_popped: RwSignal::new(false),
            show_program_browser: RwSignal::new(false),
        }
    }
}

/// Workspace context - provides shared state for workspace components.
#[derive(Clone, Copy)]
pub struct WorkspaceContext {
    /// Recently used commands for quick access.
    pub recent_commands: RwSignal<Vec<String>>,
    /// Currently selected command ID.
    pub selected_command_id: RwSignal<Option<i64>>,
    /// Command execution log.
    pub command_log: RwSignal<Vec<String>>,
}

impl WorkspaceContext {
    pub fn new() -> Self {
        Self {
            recent_commands: RwSignal::new(Vec::new()),
            selected_command_id: RwSignal::new(None),
            command_log: RwSignal::new(Vec::new()),
        }
    }
}

// ============================================================================
//                          APP ROOT
// ============================================================================

#[component]
fn App() -> impl IntoView {
    // ✅ Register all sync components - pl3xus handles subscription lifecycle
    let registry = ClientTypeRegistry::builder()
        .register::<RobotPosition>()
        .register::<JointAngles>()
        .register::<RobotStatus>()
        .register::<EntityControl>() // Control state from ExclusiveControlPlugin
        .register::<IoStatus>()
        .register::<ExecutionState>()
        .register::<ConnectionState>()
        .register::<ActiveConfigState>()
        .register::<JogSettingsState>()
        .build();

    let ws_url = "ws://127.0.0.1:8083/sync";

    view! {
        <SyncProvider url=ws_url.to_string() registry=registry auto_connect=true>
            <Router>
                <DesktopLayout/>
            </Router>
        </SyncProvider>
    }
}

// ============================================================================
//                          DESKTOP LAYOUT
// ============================================================================

#[component]
fn DesktopLayout() -> impl IntoView {
    // Create and provide layout context
    let layout_ctx = LayoutContext::new();
    provide_context(layout_ctx);

    // Create and provide workspace context
    let workspace_ctx = WorkspaceContext::new();
    provide_context(workspace_ctx);

    // Get current location to determine if we're on dashboard
    let location = use_location();
    let is_dashboard = move || {
        let path = location.pathname.get();
        path == "/" || path.starts_with("/dashboard")
    };

    view! {
        <div class="h-screen w-screen flex flex-col bg-[#0a0a0a] overflow-hidden">
            // Header
            <TopBar/>

            // Main content area (navbar + workspace + right panel)
            <div class="flex-1 flex overflow-hidden">
                // Left navbar
                <LeftNavbar/>

                // Main workspace with routes
                <MainWorkspace/>

                // Right panel (only visible in Dashboard routes)
                <Show when=is_dashboard>
                    <RightPanel/>
                </Show>
            </div>
            
            // Floating controls (rendered outside normal flow)
            <FloatingJogControls/>
            <FloatingIOStatus/>
        </div>
    }
}

// ============================================================================
//                          TOP BAR
// ============================================================================

#[component]
fn TopBar() -> impl IntoView {

    view! {
        <div class="h-10 bg-[#0a0a0a] border-b border-[#ffffff10] flex items-center justify-between px-3 select-none shrink-0">
            // Logo Area
            <div class="flex items-center gap-2">
                <img src="assets/logo.png" class="h-6 object-contain" alt="Fanuc RMI" />
                <div class="flex flex-col">
                    <span class="text-[#00d9ff] text-[10px] font-bold leading-tight tracking-wider">"FANUC"</span>
                    <span class="text-white text-[8px] font-medium leading-tight tracking-widest opacity-80">"RMI REPLICA"</span>
                </div>
            </div>

            // Center: Connection Status
            <div class="flex items-center gap-4">
                <ConnectionDropdown/>
            </div>

             // Right: Control Button
            <div class="flex items-center gap-2">
                <ControlButton/>
            </div>
        </div>
    }
}

/// Connection dropdown - shows robot connection status and allows connecting
#[component]
fn ConnectionDropdown() -> impl IntoView {
    let ctx = use_sync_context();
    let connection_state = use_sync_component::<ConnectionState>();

    // Check if robot is connected (from synced ConnectionState component)
    let is_robot_connected = move || {
        connection_state.get().values().next()
            .map(|s| s.robot_connected)
            .unwrap_or(false)
    };

    let robot_addr = move || {
        connection_state.get().values().next()
            .map(|s| s.robot_addr.clone())
            .unwrap_or_else(|| "Not connected".to_string())
    };

    // Connect to the simulator when clicked
    let connect_click = {
        let ctx = ctx.clone();
        move |_| {
            log::info!("Sending ConnectToRobot command");
            ctx.send(ConnectToRobot {
                connection_id: None,
                addr: "127.0.0.1".to_string(),
                port: 16001,
                name: Some("CRX-10iA Simulator".to_string()),
            });
        }
    };

    view! {
        <button
            class="flex items-center gap-1.5 px-2 py-0.5 bg-[#ffffff05] rounded border border-[#ffffff05] hover:bg-[#ffffff10] cursor-pointer"
            on:click=connect_click
        >
            <div class=move || {
                if is_robot_connected() {
                    "w-1.5 h-1.5 rounded-full bg-[#22c55e] animate-pulse"
                } else {
                    "w-1.5 h-1.5 rounded-full bg-[#ef4444]"
                }
            }></div>
            <span class="text-[9px] text-[#cccccc] font-medium">
                {move || if is_robot_connected() {
                    robot_addr()
                } else {
                    "Click to Connect".to_string()
                }}
            </span>
        </button>
    }
}

/// Control button using pl3xus ExclusiveControlPlugin
///
/// The server uses EntityControl from pl3xus_common (with ecs feature).
/// We send ControlRequest messages and observe the synced control state.
#[component]
fn ControlButton() -> impl IntoView {
    let ctx = use_sync_context();
    // Use EntityControl from pl3xus_common to track control state
    let control_state = use_sync_component::<EntityControl>();
    // Use ConnectionState to get the robot entity ID
    let connection_state = use_sync_component::<ConnectionState>();

    // Get the robot entity bits from any synced component
    let robot_entity_bits = move || -> Option<u64> {
        // Get the first key (entity ID) from connection_state
        connection_state.get().keys().next().copied()
    };

    // Check if current client has control
    // TODO: Compare with our client ID once we expose it via use_sync_context
    let has_control = move || {
        control_state.get().values().next()
            .map(|s| s.client_id.id != 0) // Non-zero means someone has control
            .unwrap_or(false)
    };

    // Toggle control using pl3xus ControlRequest messages
    let toggle_control = {
        let ctx = ctx.clone();
        move |_| {
            // Get actual robot entity bits from synced components
            let Some(entity_bits) = robot_entity_bits() else {
                log::warn!("No robot entity found to control");
                return;
            };

            log::info!("Robot entity bits: {}", entity_bits);

            if has_control() {
                // Send release request via pl3xus control protocol
                log::info!("Sending ControlRequest::Release({})", entity_bits);
                ctx.send(ControlRequest::Release(entity_bits));
            } else {
                // Send take request via pl3xus control protocol
                log::info!("Sending ControlRequest::Take({})", entity_bits);
                ctx.send(ControlRequest::Take(entity_bits));
            }
        }
    };

    view! {
        <button
            class=move || if has_control() {
                "bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] text-[8px] px-2 py-0.5 rounded hover:bg-[#22c55e30] flex items-center gap-1"
            } else {
                "bg-[#f59e0b20] border border-[#f59e0b40] text-[#f59e0b] text-[8px] px-2 py-0.5 rounded hover:bg-[#f59e0b30] flex items-center gap-1"
            }
            on:click=toggle_control
        >
            {move || if has_control() { "IN CONTROL" } else { "REQUEST CONTROL" }}
        </button>
    }
}

// ============================================================================
//                          LEFT NAVBAR
// ============================================================================

#[component]
fn LeftNavbar() -> impl IntoView {
    view! {
        <nav class="w-12 bg-[#111111] border-r border-[#ffffff10] flex flex-col items-center py-2 shrink-0">
            <NavLink
                icon="📊"
                label="DASH"
                href="/dashboard"
                match_prefix="/dashboard"
                is_root=true
            />
            <NavLink
                icon="📁"
                label="PROG"
                href="/programs"
                match_prefix="/programs"
                is_root=false
            />
            <NavLink
                icon="⚙️"
                label="SET"
                href="/settings"
                match_prefix="/settings"
                is_root=false
            />
        </nav>
    }
}

#[component]
fn NavLink(
    icon: &'static str,
    label: &'static str,
    href: &'static str,
    match_prefix: &'static str,
    is_root: bool,
) -> impl IntoView {
    let location = use_location();
    let is_active = move || {
        let path = location.pathname.get();
        if is_root {
            path == "/" || path.starts_with(match_prefix)
        } else {
            path.starts_with(match_prefix)
        }
    };

    view! {
        <a
            href=href
            class=move || if is_active() {
                "w-10 h-10 rounded bg-[#00d9ff15] border border-[#00d9ff40] flex flex-col items-center justify-center mb-1 transition-all no-underline"
            } else {
                "w-10 h-10 rounded hover:bg-[#ffffff08] border border-transparent flex flex-col items-center justify-center mb-1 transition-all no-underline"
            }
        >
            <span class="text-sm leading-none">{icon}</span>
            <span class=move || if is_active() {
                "text-[8px] text-[#00d9ff] mt-0.5 font-medium leading-none"
            } else {
                "text-[8px] text-[#666666] mt-0.5 leading-none"
            }>{label}</span>
        </a>
    }
}

// ============================================================================
//                          MAIN WORKSPACE
// ============================================================================

#[component]
fn MainWorkspace() -> impl IntoView {
    view! {
        <main class="flex-1 flex flex-col overflow-hidden bg-[#080808]">
            <Routes fallback=|| view! { <DashboardView/> }>
                // Root redirects to dashboard
                <Route path=path!("/") view=|| view! { <Redirect path="/dashboard/control" /> } />

                // Dashboard with nested tabs
                <ParentRoute path=path!("/dashboard") view=DashboardView>
                    <Route path=path!("/") view=|| view! { <Redirect path="/dashboard/control" /> } />
                    <Route path=path!("/control") view=ControlTab />
                    <Route path=path!("/info") view=InfoTab />
                </ParentRoute>

                // Programs view
                <Route path=path!("/programs") view=ProgramsView />

                // Settings view
                <Route path=path!("/settings") view=SettingsView />
            </Routes>
        </main>
    }
}

// ============================================================================
//                          DASHBOARD VIEW
// ============================================================================

#[component]
fn DashboardView() -> impl IntoView {
    let location = use_location();
    
    let active_tab = move || {
        let path = location.pathname.get();
        if path.ends_with("/info") { 1 } else { 0 }
    };

    view! {
        <div class="flex-1 flex flex-col overflow-hidden">
            // Tab header
            <div class="flex border-b border-[#ffffff10] bg-[#0a0a0a]">
                <a 
                    href="/dashboard/control"
                    class=move || if active_tab() == 0 {
                        "px-4 py-2 text-[10px] font-medium text-[#00d9ff] border-b-2 border-[#00d9ff]"
                    } else {
                        "px-4 py-2 text-[10px] font-medium text-[#666666] hover:text-white"
                    }
                >"CONTROL"</a>
                <a 
                    href="/dashboard/info"
                    class=move || if active_tab() == 1 {
                        "px-4 py-2 text-[10px] font-medium text-[#00d9ff] border-b-2 border-[#00d9ff]"
                    } else {
                        "px-4 py-2 text-[10px] font-medium text-[#666666] hover:text-white"
                    }
                >"INFO"</a>
            </div>
            
            // Tab content (outlet)
            <div class="flex-1 overflow-auto p-3">
                <leptos_router::components::Outlet/>
            </div>
        </div>
    }
}

#[component]
fn ControlTab() -> impl IntoView {
    view! {
        <div class="space-y-3">
            <QuickCommands/>
            <CommandInput/>
        </div>
    }
}

#[component]
fn InfoTab() -> impl IntoView {
    view! {
        <div class="space-y-3">
            <StatusPanel/>
            <PositionPanel/>
            <JointAnglesPanel/>
        </div>
    }
}

// ============================================================================
//                          RIGHT PANEL
// ============================================================================

#[component]
fn RightPanel() -> impl IntoView {
    view! {
        <aside class="w-64 bg-[#0a0a0a] border-l border-[#ffffff10] flex flex-col overflow-hidden shrink-0">
            <div class="p-2 space-y-2 overflow-auto">
                <StatusPanel/>
                <PositionPanel/>
                <JogControls/>
            </div>
        </aside>
    }
}

// ============================================================================
//                          PROGRAMS VIEW
// ============================================================================

#[component]
fn ProgramsView() -> impl IntoView {
    view! {
        <div class="flex-1 flex flex-col p-4">
            <h1 class="text-[#00d9ff] text-lg font-bold mb-4">"Programs"</h1>
            <div class="bg-[#111111] rounded border border-[#ffffff08] p-4">
                <p class="text-[#666666] text-sm">"Program management coming in Phase 4..."</p>
            </div>
        </div>
    }
}

// ============================================================================
//                          SETTINGS VIEW
// ============================================================================

#[component]
fn SettingsView() -> impl IntoView {
    view! {
        <div class="flex-1 flex flex-col p-4">
            <h1 class="text-[#00d9ff] text-lg font-bold mb-4">"Settings"</h1>
            <div class="bg-[#111111] rounded border border-[#ffffff08] p-4">
                <p class="text-[#666666] text-sm">"Robot connections and settings coming in Phase 5..."</p>
            </div>
        </div>
    }
}

// ============================================================================
//                          SHARED COMPONENTS
// ============================================================================

#[component]
fn StatusPanel() -> impl IntoView {
    let status = use_sync_component::<RobotStatus>();
    let get_status = move || status.get().values().next().cloned().unwrap_or_default();

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-2">
            <h2 class="text-[10px] font-semibold text-[#00d9ff] mb-1.5 uppercase tracking-wide">"Status"</h2>
            <div class="grid grid-cols-4 gap-1">
                <StatusIndicator label="Servo" value=move || get_status().servo_ready />
                <StatusIndicator label="TP" value=move || get_status().tp_enabled />
                <StatusIndicator label="Motion" value=move || get_status().in_motion />
                <StatusIndicatorText label="Error" value=move || get_status().error_message.clone().unwrap_or_else(|| "None".to_string()) />
            </div>
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

#[component]
fn PositionPanel() -> impl IntoView {
    let pos = use_sync_component::<RobotPosition>();
    let get_pos = move || pos.get().values().next().cloned().unwrap_or_default();

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-2">
            <h2 class="text-[10px] font-semibold text-[#00d9ff] mb-1.5 uppercase tracking-wide">"Position"</h2>
            <div class="grid grid-cols-3 gap-1">
                <PositionItem label="X" value=move || get_pos().x as f32 />
                <PositionItem label="Y" value=move || get_pos().y as f32 />
                <PositionItem label="Z" value=move || get_pos().z as f32 />
                <PositionItem label="W" value=move || get_pos().w as f32 />
                <PositionItem label="P" value=move || get_pos().p as f32 />
                <PositionItem label="R" value=move || get_pos().r as f32 />
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

#[component]
fn JointAnglesPanel() -> impl IntoView {
    let joints = use_sync_component::<JointAngles>();
    let get_joints = move || joints.get().values().next().cloned().unwrap_or_default();

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-2">
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
fn JogControls() -> impl IntoView {
    let ctx = use_sync_context();
    let (speed, set_speed) = signal(50.0f32);
    let (step, set_step) = signal(10.0f32);

    // ✅ Clean RPC with ctx.send() - no boilerplate!
    let jog = move |axis: JogAxis, direction: JogDirection| {
        ctx.send(JogCommand {
            axis,
            direction,
            distance: step.get(),
            speed: speed.get(),
        });
    };

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-2">
            <h2 class="text-[10px] font-semibold text-[#00d9ff] mb-2 uppercase tracking-wide">"Jog Control"</h2>
            
            // Settings
            <div class="grid grid-cols-2 gap-2 mb-2">
                <div class="bg-[#111111] rounded p-1.5">
                    <div class="text-[8px] text-[#666666] mb-1">"Speed"</div>
                    <input type="range" min="1" max="100" class="w-full accent-[#00d9ff] h-1"
                        prop:value=speed
                        on:input=move |ev| {
                            if let Ok(v) = event_target_value(&ev).parse::<f32>() { set_speed.set(v); }
                        }
                    />
                    <div class="text-[9px] text-[#00d9ff] text-right font-mono">{move || speed.get()}</div>
                </div>
                <div class="bg-[#111111] rounded p-1.5">
                    <div class="text-[8px] text-[#666666] mb-1">"Step"</div>
                    <input type="range" min="1" max="50" class="w-full accent-[#00d9ff] h-1"
                        prop:value=step
                        on:input=move |ev| {
                            if let Ok(v) = event_target_value(&ev).parse::<f32>() { set_step.set(v); }
                        }
                    />
                    <div class="text-[9px] text-[#00d9ff] text-right font-mono">{move || step.get()}</div>
                </div>
            </div>

            // Directional Buttons
            <div class="grid grid-cols-3 gap-1">
                <div></div>
                <JogButton label="Y+" jog=jog.clone() axis=JogAxis::Y direction=JogDirection::Positive />
                <div></div>
                <JogButton label="X-" jog=jog.clone() axis=JogAxis::X direction=JogDirection::Negative />
                <JogButton label="Z+" jog=jog.clone() axis=JogAxis::Z direction=JogDirection::Positive />
                <JogButton label="X+" jog=jog.clone() axis=JogAxis::X direction=JogDirection::Positive />
                <div></div>
                <JogButton label="Y-" jog=jog.clone() axis=JogAxis::Y direction=JogDirection::Negative />
                <JogButton label="Z-" jog=jog.clone() axis=JogAxis::Z direction=JogDirection::Negative />
            </div>
        </div>
    }
}

#[component]
fn JogButton<F>(label: &'static str, jog: F, axis: JogAxis, direction: JogDirection) -> impl IntoView 
where F: Fn(JogAxis, JogDirection) + Clone + 'static {
    let do_jog = {
        let jog = jog.clone();
        move |_| jog(axis, direction)
    };

    view! {
        <button 
            class="bg-[#111111] hover:bg-[#00d9ff] border border-[#ffffff08] hover:border-[#00d9ff] text-white hover:text-black font-semibold py-1.5 rounded transition-colors text-center text-[10px]"
            on:click=do_jog
        >
            {label}
        </button>
    }
}

#[component]
fn QuickCommands() -> impl IntoView {
    let ctx = use_sync_context();
    let status = use_sync_component::<RobotStatus>();

    // Get current speed override from robot status
    let speed_override = move || {
        status.get().values().next()
            .map(|s| s.speed_override)
            .unwrap_or(100)
    };

    // ✅ Clean RPC - one-liner per command
    let init_click = {
        let ctx = ctx.clone();
        move |_| ctx.send(InitializeRobot { group_mask: Some(1) })
    };
    let reset_click = {
        let ctx = ctx.clone();
        move |_| ctx.send(ResetRobot)
    };
    let abort_click = {
        let ctx = ctx.clone();
        move |_| ctx.send(AbortMotion)
    };
    let set_speed = {
        let ctx = ctx.clone();
        move |speed: u8| ctx.send(SetSpeedOverride { speed })
    };

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-2">
            <h2 class="text-[10px] font-semibold text-[#00d9ff] mb-2 uppercase tracking-wide">"Quick Commands"</h2>
            <div class="flex gap-2 mb-2">
                // Initialize - Green button
                <button
                    class="flex-1 bg-[#22c55e20] hover:bg-[#22c55e30] border border-[#22c55e40] text-[#22c55e] text-[9px] px-2 py-1.5 rounded transition-colors flex items-center justify-center gap-1"
                    on:click=init_click
                >
                    <span>"▶"</span>
                    "INIT"
                </button>
                // Reset - Orange button
                <button
                    class="flex-1 bg-[#f59e0b20] hover:bg-[#f59e0b30] border border-[#f59e0b40] text-[#f59e0b] text-[9px] px-2 py-1.5 rounded transition-colors flex items-center justify-center gap-1"
                    on:click=reset_click
                >
                    <span>"↻"</span>
                    "RESET"
                </button>
                // Abort - Red button
                <button
                    class="flex-1 bg-[#ef444420] hover:bg-[#ef444430] border border-[#ef444440] text-[#ef4444] text-[9px] px-2 py-1.5 rounded transition-colors flex items-center justify-center gap-1"
                    on:click=abort_click
                >
                    <span>"■"</span>
                    "ABORT"
                </button>
            </div>
            // Speed Override Slider
            <div class="flex items-center gap-2">
                <span class="text-[8px] text-[#666666] w-12">"Speed:"</span>
                <input
                    type="range"
                    min="0"
                    max="100"
                    value=speed_override
                    on:change=move |ev| {
                        if let Ok(val) = event_target_value(&ev).parse::<u8>() {
                            set_speed(val);
                        }
                    }
                    class="flex-1 h-1 bg-[#333333] rounded-lg appearance-none cursor-pointer accent-[#00d9ff]"
                />
                <span class="text-[10px] text-[#00d9ff] w-8 text-right">{move || format!("{}%", speed_override())}</span>
            </div>
        </div>
    }
}

#[component]
fn CommandInput() -> impl IntoView {
    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-2">
            <h2 class="text-[10px] font-semibold text-[#00d9ff] mb-2 uppercase tracking-wide">"Command Input"</h2>
            <div class="flex gap-2">
                <input 
                    type="text" 
                    placeholder="Enter command..."
                    class="flex-1 bg-[#111111] border border-[#ffffff08] rounded px-2 py-1 text-white text-[11px] focus:outline-none focus:border-[#00d9ff]"
                />
                <button class="bg-[#00d9ff20] border border-[#00d9ff40] text-[#00d9ff] px-3 py-1 rounded text-[10px] hover:bg-[#00d9ff30]">
                    "SEND"
                </button>
            </div>
        </div>
    }
}

// ============================================================================
//                          FLOATING CONTROLS
// ============================================================================

#[component]
fn FloatingJogControls() -> impl IntoView {
    let layout_ctx = use_context::<LayoutContext>();
    
    let is_visible = move || {
        layout_ctx.map(|ctx| ctx.jog_popped.get()).unwrap_or(false)
    };
    
    let close = move |_| {
        if let Some(ctx) = layout_ctx {
            ctx.jog_popped.set(false);
        }
    };

    view! {
        <Show when=is_visible>
            <div class="fixed top-20 right-4 w-72 bg-[#0a0a0a] border border-[#00d9ff40] rounded-lg shadow-2xl z-50">
                <div class="flex items-center justify-between px-3 py-2 border-b border-[#ffffff10] cursor-move">
                    <span class="text-[10px] font-semibold text-[#00d9ff] uppercase">"Jog Controls"</span>
                    <button
                        class="text-[#666666] hover:text-white text-xs"
                        on:click=close
                    >"✕"</button>
                </div>
                <div class="p-2">
                    <JogControls/>
                </div>
            </div>
        </Show>
    }
}

#[component]
fn FloatingIOStatus() -> impl IntoView {
    let layout_ctx = use_context::<LayoutContext>();
    
    let is_visible = move || {
        layout_ctx.map(|ctx| ctx.io_popped.get()).unwrap_or(false)
    };
    
    let close = move |_| {
        if let Some(ctx) = layout_ctx {
            ctx.io_popped.set(false);
        }
    };

    view! {
        <Show when=is_visible>
            <div class="fixed top-20 left-20 w-64 bg-[#0a0a0a] border border-[#00d9ff40] rounded-lg shadow-2xl z-50">
                <div class="flex items-center justify-between px-3 py-2 border-b border-[#ffffff10] cursor-move">
                    <span class="text-[10px] font-semibold text-[#00d9ff] uppercase">"I/O Status"</span>
                    <button
                        class="text-[#666666] hover:text-white text-xs"
                        on:click=close
                    >"✕"</button>
                </div>
                <div class="p-2">
                    <IOStatusPanel/>
                </div>
            </div>
        </Show>
    }
}

#[component]
fn IOStatusPanel() -> impl IntoView {
    let io = use_sync_component::<IoStatus>();
    let get_io = move || io.get().values().next().cloned().unwrap_or_default();

    view! {
        <div class="space-y-2">
            <div class="bg-[#111111] rounded p-2">
                <div class="text-[9px] text-[#666666] mb-1 uppercase">"Digital Inputs"</div>
                <div class="grid grid-cols-8 gap-0.5">
                    {move || {
                        let io_data = get_io();
                        (0..8).map(|i| {
                            let active = io_data.digital_inputs.get(i).copied().unwrap_or(0) > 0;
                            view! {
                                <div class=move || if active {
                                    "w-4 h-4 rounded bg-[#22c55e] flex items-center justify-center text-[8px] text-black font-bold"
                                } else {
                                    "w-4 h-4 rounded bg-[#333333] flex items-center justify-center text-[8px] text-[#666666]"
                                }>
                                    {i + 1}
                                </div>
                            }
                        }).collect::<Vec<_>>()
                    }}
                </div>
            </div>
            <div class="bg-[#111111] rounded p-2">
                <div class="text-[9px] text-[#666666] mb-1 uppercase">"Digital Outputs"</div>
                <div class="grid grid-cols-8 gap-0.5">
                    {move || {
                        let io_data = get_io();
                        (0..8).map(|i| {
                            let active = io_data.digital_outputs.get(i).copied().unwrap_or(0) > 0;
                            view! {
                                <div class=move || if active {
                                    "w-4 h-4 rounded bg-[#f59e0b] flex items-center justify-center text-[8px] text-black font-bold"
                                } else {
                                    "w-4 h-4 rounded bg-[#333333] flex items-center justify-center text-[8px] text-[#666666]"
                                }>
                                    {i + 1}
                                </div>
                            }
                        }).collect::<Vec<_>>()
                    }}
                </div>
            </div>
        </div>
    }
}
