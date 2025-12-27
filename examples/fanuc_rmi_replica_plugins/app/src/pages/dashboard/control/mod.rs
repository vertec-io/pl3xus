//! Dashboard Control tab - Robot control and program execution.
//!
//! Contains components for quick commands (consolidated with recent commands and speed),
//! command composition, console logging, program execution visualization, and joint jogging.

mod quick_commands;
mod command_input;
mod command_log;
mod program_display;
mod load_modal;
mod composer;
mod joint_jog;
mod jog_io_tabs;

pub use quick_commands::QuickCommandsPanel;
// CommandInputSection is now consolidated into QuickCommandsPanel
#[allow(unused_imports)]
pub use command_input::CommandInputSection;
pub use command_log::CommandLogPanel;
pub use program_display::ProgramVisualDisplay;
pub use load_modal::LoadProgramModal;
pub use composer::CommandComposerModal;
pub use joint_jog::JointJogPanel;
pub use jog_io_tabs::JogIoTabs;

use leptos::prelude::*;
use pl3xus_client::use_entity_component;
use fanuc_replica_plugins::*;
use crate::pages::dashboard::context::{WorkspaceContext, use_system_entity};

/// Control tab content (command composer).
///
/// NOTE: Program completion notifications are handled by ProgramNotificationHandler
/// in the layout module, which receives server-broadcast ProgramNotification messages.
#[component]
pub fn ControlTab() -> impl IntoView {
    let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let show_composer = ctx.show_composer;
    let system_ctx = use_system_entity();

    // Subscribe to the robot's connection state (ConnectionState lives on robot entity)
    let (connection_state, robot_exists) = use_entity_component::<ConnectionState, _>(move || system_ctx.robot_entity_id.get());
    let robot_connected = Memo::new(move |_| robot_exists.get() && connection_state.get().robot_connected);

    view! {
        <div class="h-full flex flex-col gap-2">
            // Consolidated Commands section (quick actions + recent commands + speed override)
            <QuickCommandsPanel/>

            // Tabbed Jog/IO Panel (only show when connected to robot)
            <Show when=move || robot_connected.get()>
                <JogIoTabs/>
            </Show>

            // Two-column layout for Command Log and Program Display (both collapsible)
            <div class="flex-1 grid grid-cols-2 gap-2 min-h-0">
                <CommandLogPanel/>
                <ProgramVisualDisplay/>
            </div>

            // Command Composer Modal
            <Show when=move || show_composer.get()>
                <CommandComposerModal/>
            </Show>
        </div>
    }
}

