//! Robot state synchronization plugin.
//!
//! Polls the FANUC driver and updates synced components.

use bevy::prelude::*;
use pl3xus_sync::{
    AppMessageRegistrationExt,
    AppBatchMessageRegistrationExt,
    AppBatchRequestRegistrationExt,
};
use pl3xus_websockets::WebSocketProvider;
use fanuc_replica_types::*;

use crate::jogging;

pub struct RobotSyncPlugin;

impl Plugin for RobotSyncPlugin {
    fn build(&self, app: &mut App) {
        // =====================================================================
        // TARGETED MESSAGES (fire-and-forget, require entity control)
        // =====================================================================
        // High-frequency streaming commands that don't need responses.
        // The DefaultEntityAccessPolicy (from ExclusiveControlPlugin) is used.

        // Jog commands are high-frequency and don't need responses
        app.messages::<(
            JogCommand,
            JogRobot,
            LinearMotionCommand,
            JointMotionCommand,
        ), WebSocketProvider>()
            .targeted()
            .with_default_entity_policy()
            .register();

        // Register fanuc_rmi::dto::SendPacket for direct motion commands (targeted)
        app.message::<fanuc_rmi::dto::SendPacket, WebSocketProvider>()
            .targeted()
            .with_default_entity_policy()
            .register();

        // =====================================================================
        // TARGETED REQUESTS (require entity control, need responses)
        // =====================================================================
        // Robot control commands that need confirmation responses.
        // Using with_error_response() to send proper error responses
        // when authorization fails instead of silently dropping requests.

        // Robot control commands - need response for UI feedback
        app.requests::<(
            InitializeRobot,
            ResetRobot,
            AbortMotion,
            SetSpeedOverride,
        ), WebSocketProvider>()
            .targeted()
            .with_default_entity_policy()
            .with_error_response();

        // Program execution commands - need response for UI feedback
        // Note: These target the system entity (ActiveSystem), not the robot
        app.requests::<(
            StartProgram,
            PauseProgram,
            ResumeProgram,
            StopProgram,
            UnloadProgram,
        ), WebSocketProvider>()
            .targeted()
            .with_default_entity_policy()
            .with_error_response();

        // Legacy targeted messages for program execution (to be migrated)
        app.messages::<(
            ExecuteProgram,
            StopExecution,
            LoadProgram,
        ), WebSocketProvider>()
            .targeted()
            .with_default_entity_policy()
            .register();

        // =====================================================================
        // COMMAND HANDLERS
        // =====================================================================
        // Targeted messages use AuthorizedTargetedMessage<T>
        // Targeted requests use AuthorizedRequest<T>

        app.add_systems(Update, (
            jogging::handle_authorized_jog_commands,
            jogging::handle_jog_robot_commands,
            jogging::handle_initialize_robot,
            jogging::handle_abort_motion,
            jogging::handle_reset_robot,
            jogging::handle_set_speed_override,
            jogging::handle_send_packet,
        ));
    }
}
