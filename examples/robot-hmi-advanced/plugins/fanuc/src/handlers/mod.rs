//! Request/Response handlers plugin.
//!
//! Handles client requests for:
//! - Robot connections and configurations
//! - Frame and tool operations
//! - I/O operations
//! - Settings

use bevy::prelude::*;
use pl3xus_websockets::WebSocketProvider;
use pl3xus_sync::authorization::AppBatchRequestRegistrationExt;

use crate::types::*;

mod connection;
mod frame_tool;
mod io;
mod settings;

pub use connection::*;
pub use frame_tool::*;
pub use io::*;
pub use settings::*;

// Type alias for WebSocket network provider - reduces verbosity
type WS = WebSocketProvider;

pub struct RequestHandlerPlugin;

impl Plugin for RequestHandlerPlugin {
    fn build(&self, app: &mut App) {
        // =====================================================================
        // Request Registration (using pl3xus_sync builder API)
        // =====================================================================

        // Robot Connections & Configurations
        app.requests::<(
            ListRobotConnections,
            GetRobotConfigurations,
            CreateRobotConnection,
            UpdateRobotConnection,
            DeleteRobotConnection,
            CreateConfiguration,
            UpdateConfiguration,
            DeleteConfiguration,
            SetDefaultConfiguration,
            LoadConfiguration,
            SaveCurrentConfiguration,
        ), WS>().register();

        // Frame/Tool and I/O Operations are registered as targeted requests in sync.rs
        // Only non-targeted I/O config operations remain here
        app.requests::<(
            GetIoConfig,
            UpdateIoConfig,
        ), WS>().register();

        // Settings (non-targeted, global operations)
        app.requests::<(
            GetSettings,
            UpdateSettings,
            UpdateJogSettings,
        ), WS>().register();

        // Add handler systems - Robot Connections & Configurations
        app.add_systems(Update, (
            handle_list_robot_connections,
            handle_get_robot_configurations,
            handle_create_robot_connection,
            handle_update_robot_connection,
            handle_delete_robot_connection,
            handle_create_configuration,
            handle_update_configuration,
            handle_delete_configuration,
            handle_set_default_configuration,
            handle_load_configuration,
            handle_save_current_configuration,
        ));

        // Add handler systems - I/O Config (non-targeted)
        app.add_systems(Update, (
            handle_get_io_config,
            handle_update_io_config,
        ));

        // Add handler systems - Settings (non-targeted)
        app.add_systems(Update, (
            handle_get_settings,
            handle_update_settings,
            handle_update_jog_settings,
        ));
    }
}

