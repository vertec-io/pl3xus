//! Robot plugin - aggregates all robot-related sub-plugins.
//!
//! This is the main entry point for all robot functionality.
//! It adds all the individual sub-plugins that handle specific concerns.

use bevy::prelude::*;
use pl3xus_sync::{ComponentSyncConfig, AppPl3xusSyncExt};
use pl3xus_websockets::WebSocketProvider;

use super::connection::RobotConnectionPlugin;
use super::sync::RobotSyncPlugin;
use super::handlers::RequestHandlerPlugin;
use super::polling::RobotPollingPlugin;
use super::program::ProgramPlugin;
use super::jogging;
use super::types::*;

/// Robot plugin - handles all robot-related functionality.
///
/// Includes:
/// - Connection state machine
/// - Robot state synchronization (jogging, motion)
/// - Request/response handlers
/// - Position/status polling
/// - Program execution with orchestrator pattern
pub struct RobotPlugin;

impl Plugin for RobotPlugin {
    fn build(&self, app: &mut App) {
        // =====================================================================
        // SYNCED COMPONENTS
        // =====================================================================
        // Register all robot-related components for synchronization.
        // These are read-only from the client's perspective - the server
        // updates them based on robot state.

        // Marker components (read-only, no meaningful mutations)
        app.sync_component::<ActiveRobot>(Some(ComponentSyncConfig::read_only()));

        // Robot status/state components (read-only, updated by server from robot)
        app.sync_component::<RobotPosition>(Some(ComponentSyncConfig::read_only_with_message(
            "RobotPosition is read-only. Robot position is controlled by the robot controller."
        )));
        app.sync_component::<JointAngles>(Some(ComponentSyncConfig::read_only_with_message(
            "JointAngles is read-only. Joint positions are controlled by the robot controller."
        )));
        app.sync_component::<RobotStatus>(Some(ComponentSyncConfig::read_only_with_message(
            "RobotStatus is read-only. Use SetSpeedOverride command to change speed."
        )));
        app.sync_component::<IoStatus>(Some(ComponentSyncConfig::read_only_with_message(
            "IoStatus is read-only. Use SetDigitalOutput command to control outputs."
        )));
        app.sync_component::<ExecutionState>(Some(ComponentSyncConfig::read_only_with_message(
            "ExecutionState is read-only. Use program execution commands (Start, Stop, Pause, etc)."
        )));
        app.sync_component::<ConnectionState>(Some(ComponentSyncConfig::read_only_with_message(
            "ConnectionState is read-only. Use ConnectToRobot/DisconnectFromRobot commands."
        )));
        app.sync_component::<FrameToolDataState>(Some(ComponentSyncConfig::read_only_with_message(
            "FrameToolDataState is read-only. Use SetActiveFrameTool, WriteFrameData, WriteToolData commands."
        )));
        app.sync_component::<IoConfigState>(Some(ComponentSyncConfig::read_only_with_message(
            "IoConfigState is read-only. Use UpdateIoConfig command to modify I/O display settings."
        )));

        // User-configurable components (clients can mutate with proper authorization)
        app.sync_component::<ActiveConfigState>(None);  // User can change active configuration

        // JogSettingsState uses an authorized mutation handler for validation and logging.
        // Only clients with control of the robot entity can mutate these settings.
        app.sync_component_builder::<JogSettingsState>()
            .with_handler::<WebSocketProvider, _, _>(jogging::handle_jog_settings_mutation)
            .targeted()
            .with_default_entity_policy()
            .build();

        // =====================================================================
        // SUB-PLUGINS
        // =====================================================================
        app.add_plugins((
            RobotConnectionPlugin,  // Connection state machine
            RobotSyncPlugin,        // Driver polling and jogging
            RequestHandlerPlugin,   // Database request handlers
            RobotPollingPlugin,     // Periodic position/status polling
            ProgramPlugin,          // Orchestrator-based program execution
        ));

        info!("🤖 RobotPlugin initialized");
    }
}
