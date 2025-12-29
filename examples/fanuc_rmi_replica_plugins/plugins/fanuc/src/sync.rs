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
use crate::types::*;

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
            SetActiveFrameTool,
        ), WebSocketProvider>()
            .targeted()
            .with_default_entity_policy()
            .with_error_response();

        // Frame/Tool and I/O write operations - require entity control
        app.requests::<(
            WriteFrameData,
            WriteToolData,
            WriteDout,
            WriteAout,
            WriteGout,
        ), WebSocketProvider>()
            .targeted()
            .with_default_entity_policy()
            .with_error_response();

        // =====================================================================
        // TARGETED QUERIES (no authorization, just targeting for multi-robot)
        // =====================================================================
        // Read operations that need to target a specific robot but don't require
        // authorization (anyone can read the state of any robot).

        // Frame/Tool and I/O read operations - no authorization needed
        app.requests::<(
            GetActiveFrameTool,
            GetFrameData,
            GetToolData,
            ReadDin,
            ReadDinBatch,
            ReadAin,
            ReadGin,
            GetConnectionStatus,
        ), WebSocketProvider>()
            .targeted()
            .register();

        // =====================================================================
        // COMMAND HANDLERS
        // =====================================================================
        // Targeted messages use AuthorizedTargetedMessage<T>
        // Targeted requests use AuthorizedRequest<T>
        // Targeted queries use Request<TargetedRequest<T>>

        // Jogging and robot control handlers
        app.add_systems(Update, (
            jogging::handle_authorized_jog_commands,
            jogging::handle_initialize_robot,
            jogging::handle_abort_motion,
            jogging::handle_reset_robot,
            jogging::handle_set_speed_override,
            jogging::handle_send_packet,
            super::handlers::handle_set_active_frame_tool,
        ));

        // Frame/Tool and I/O write handlers (require authorization)
        app.add_systems(Update, (
            super::handlers::handle_write_frame_data,
            super::handlers::handle_write_tool_data,
            super::handlers::handle_write_dout,
            super::handlers::handle_write_aout,
            super::handlers::handle_write_gout,
        ));

        // Frame/Tool and I/O read handlers (no authorization)
        app.add_systems(Update, (
            super::handlers::handle_get_active_frame_tool,
            super::handlers::handle_get_frame_data,
            super::handlers::handle_get_tool_data,
            super::handlers::handle_read_din,
            super::handlers::handle_read_din_batch,
            super::handlers::handle_read_ain,
            super::handlers::handle_read_gin,
            super::handlers::handle_get_connection_status,
        ));
    }
}
