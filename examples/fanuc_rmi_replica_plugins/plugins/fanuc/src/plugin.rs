//! Bevy plugin registration for FANUC-specific functionality.
//!
//! This is the main entry point for all FANUC robot functionality.
//! It adds all the individual sub-plugins that handle specific concerns.

use bevy::prelude::*;
use pl3xus_sync::{ComponentSyncConfig, AppPl3xusSyncExt};
use pl3xus_websockets::WebSocketProvider;

#[cfg(feature = "server")]
use crate::motion::{
    fanuc_motion_handler_system, fanuc_sent_instruction_system, fanuc_motion_response_system,
    sync_buffer_state_to_execution_state, sync_device_status_to_buffer_state,
    FanucInFlightInstructions,
};
#[cfg(feature = "server")]
use crate::connection::RobotConnectionPlugin;
#[cfg(feature = "server")]
use crate::sync::RobotSyncPlugin;
#[cfg(feature = "server")]
use crate::handlers::RequestHandlerPlugin;
#[cfg(feature = "server")]
use crate::polling::RobotPollingPlugin;
#[cfg(feature = "server")]
use crate::jogging;

use crate::types::*;

/// Plugin for FANUC-specific functionality.
///
/// This plugin registers:
/// - All synced components for robot state
/// - Connection state machine
/// - Robot state synchronization (jogging, motion)
/// - Request/response handlers
/// - Position/status polling
/// - Program execution with orchestrator pattern
/// - FANUC motion command handler (converts MotionCommandEvent to driver calls)
///
/// # Usage
///
/// ```rust,ignore
/// app.add_plugins(FanucPlugin);
/// ```
///
/// # Dependencies
///
/// This plugin expects ExecutionPlugin to be registered first,
/// as it consumes MotionCommandEvent from the orchestrator.
pub struct FanucPlugin;

impl Plugin for FanucPlugin {
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

        // ActiveConfigSyncState tracks sync status between UI and robot (read-only for clients)
        app.sync_component::<ActiveConfigSyncState>(Some(ComponentSyncConfig::read_only_with_message(
            "ActiveConfigSyncState is read-only. Use retry commands to attempt resync."
        )));

        // JogSettingsState uses an authorized mutation handler for validation and logging.
        // Only clients with control of the robot entity can mutate these settings.
        #[cfg(feature = "server")]
        app.sync_component_builder::<JogSettingsState>()
            .with_handler::<WebSocketProvider, _, _>(jogging::handle_jog_settings_mutation)
            .targeted()
            .with_default_entity_policy()
            .build();

        #[cfg(not(feature = "server"))]
        app.sync_component::<JogSettingsState>(None);

        #[cfg(feature = "server")]
        {
            // =====================================================================
            // RESOURCES
            // =====================================================================
            // In-flight instruction tracking for motion completion feedback
            // Note: This resource is available for the new orchestrator pattern.
            // The legacy ProgramPlugin uses ExecutionBuffer instead.
            app.init_resource::<FanucInFlightInstructions>();

            // =====================================================================
            // SUB-PLUGINS
            // =====================================================================
            app.add_plugins((
                RobotConnectionPlugin,  // Connection state machine
                RobotSyncPlugin,        // Driver polling and jogging
                RequestHandlerPlugin,   // Database request handlers
                RobotPollingPlugin,     // Periodic position/status polling
                // Note: ProgramPlugin removed - execution is now handled by ExecutionPlugin
            ));

            // =====================================================================
            // MOTION SYSTEMS (New Execution Architecture)
            // =====================================================================
            // These systems handle the full motion lifecycle:
            // 1. orchestrator_system (ExecutionPlugin) emits MotionCommandEvent
            // 2. fanuc_motion_handler_system converts to driver calls
            // 3. fanuc_sent_instruction_system maps request_id -> sequence_id
            // 4. fanuc_motion_response_system updates DeviceStatus on completion
            // 5. sync_device_status_to_buffer_state updates BufferState
            // 6. sync_buffer_state_to_execution_state syncs to client
            app.add_systems(Update, (
                fanuc_motion_handler_system,
                fanuc_sent_instruction_system,
                fanuc_motion_response_system,
                sync_device_status_to_buffer_state,
                sync_buffer_state_to_execution_state,
            ).chain());

            info!("🤖 FanucPlugin initialized");
        }
    }
}

