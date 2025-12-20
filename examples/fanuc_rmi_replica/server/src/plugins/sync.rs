//! Robot state synchronization plugin.
//!
//! Polls the FANUC driver and updates synced components.

use bevy::prelude::*;
use pl3xus::AppNetworkMessage;
use pl3xus_websockets::WebSocketProvider;
use fanuc_replica_types::*;

use crate::jogging;

pub struct RobotSyncPlugin;

impl Plugin for RobotSyncPlugin {
    fn build(&self, app: &mut App) {
        // Register network messages for jogging
        app.register_network_message::<JogCommand, WebSocketProvider>();
        app.register_network_message::<JogRobot, WebSocketProvider>();
        app.register_network_message::<InitializeRobot, WebSocketProvider>();
        app.register_network_message::<ResetRobot, WebSocketProvider>();
        app.register_network_message::<AbortMotion, WebSocketProvider>();
        app.register_network_message::<SetSpeedOverride, WebSocketProvider>();
        app.register_network_message::<LinearMotionCommand, WebSocketProvider>();
        app.register_network_message::<JointMotionCommand, WebSocketProvider>();
        app.register_network_message::<ExecuteProgram, WebSocketProvider>();
        app.register_network_message::<StopExecution, WebSocketProvider>();
        app.register_network_message::<LoadProgram, WebSocketProvider>();
        app.register_network_message::<StartProgram, WebSocketProvider>();
        app.register_network_message::<PauseProgram, WebSocketProvider>();
        app.register_network_message::<ResumeProgram, WebSocketProvider>();
        app.register_network_message::<StopProgram, WebSocketProvider>();

        // Register fanuc_rmi::dto::SendPacket for direct motion commands
        // This allows the client to send motion instructions directly using DTO types
        app.register_network_message::<fanuc_rmi::dto::SendPacket, WebSocketProvider>();

        // Add sync systems
        app.add_systems(Update, (
            crate::driver_sync::sync_robot_state,
            jogging::handle_jog_commands,
            jogging::handle_jog_robot_commands,
            jogging::handle_initialize_robot,
            jogging::handle_abort_motion,
            jogging::handle_reset_robot,
            jogging::handle_set_speed_override,
            jogging::handle_send_packet,
        ));
    }
}
