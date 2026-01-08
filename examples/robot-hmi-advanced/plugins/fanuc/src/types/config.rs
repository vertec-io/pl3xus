//! Configuration types and DTOs.
//!
//! This module contains configuration-related types including:
//! - Active configuration state
//! - Jog settings
//! - I/O configuration
//! - Database DTOs for saved configurations

use serde::{Deserialize, Serialize};

#[cfg(feature = "ecs")]
use bevy::prelude::*;

// ============================================================================
//                          SYNCED CONFIG COMPONENTS
// ============================================================================

/// A single change entry in the active config changelog.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfigChangeEntry {
    pub field_name: String,
    pub old_value: String,
    pub new_value: String,
}

/// Active configuration state (Synced 1-way: Server -> Client)
#[cfg_attr(feature = "ecs", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ActiveConfigState {
    /// ID of the configuration this was loaded from (None if new/unsaved)
    pub loaded_from_id: Option<i64>,
    /// Name of the configuration this was loaded from
    pub loaded_from_name: Option<String>,
    /// Number of changes made since loading (0 = no changes)
    pub changes_count: u32,
    /// Detailed log of each change made since loading
    pub change_log: Vec<ConfigChangeEntry>,
    // Current active values:
    pub u_frame_number: i32,
    pub u_tool_number: i32,
    pub front: i32,
    pub up: i32,
    pub left: i32,
    pub flip: i32,
    pub turn4: i32,
    pub turn5: i32,
    pub turn6: i32,
}

impl Default for ActiveConfigState {
    fn default() -> Self {
        Self {
            loaded_from_id: None,
            loaded_from_name: None,
            changes_count: 0,
            change_log: Vec::new(),
            u_frame_number: 0,
            u_tool_number: 1,
            front: 1,
            up: 0,
            left: 0,
            flip: 0,
            turn4: 0,
            turn5: 0,
            turn6: 0,
        }
    }
}

/// Status of active config synchronization with robot.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub enum ConfigSyncStatus {
    #[default]
    Synced,
    Mismatch,
    Retrying,
    Failed,
}

/// Tracks synchronization state between ActiveConfigState and the actual robot.
#[cfg_attr(feature = "ecs", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct ActiveConfigSyncState {
    pub status: ConfigSyncStatus,
    pub retry_count: u32,
    pub max_retries: u32,
    pub sync_in_progress: bool,
    pub error_message: Option<String>,
}

impl ActiveConfigSyncState {
    pub fn new() -> Self {
        Self {
            status: ConfigSyncStatus::Synced,
            retry_count: 0,
            max_retries: 3,
            sync_in_progress: false,
            error_message: None,
        }
    }
}

/// Active jog settings state (Synced 1-way: Server -> Client)
#[cfg_attr(feature = "ecs", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct JogSettingsState {
    pub cartesian_jog_speed: f64,
    pub cartesian_jog_step: f64,
    pub joint_jog_speed: f64,
    pub joint_jog_step: f64,
    pub rotation_jog_speed: f64,
    pub rotation_jog_step: f64,
}

impl Default for JogSettingsState {
    fn default() -> Self {
        Self {
            cartesian_jog_speed: 10.0,
            cartesian_jog_step: 1.0,
            joint_jog_speed: 10.0,
            joint_jog_step: 1.0,
            rotation_jog_speed: 5.0,
            rotation_jog_step: 1.0,
        }
    }
}

/// I/O display configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IoDisplayConfig {
    pub io_type: String,
    pub io_index: i32,
    pub display_name: Option<String>,
    pub is_visible: bool,
    pub display_order: Option<i32>,
}

/// I/O display configuration state - synced component.
#[cfg_attr(feature = "ecs", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct IoConfigState {
    pub configs: std::collections::HashMap<(String, i32), IoDisplayConfig>,
}

impl IoConfigState {
    /// Get display name for an I/O port, returns port number as string if not configured.
    pub fn get_display_name(&self, io_type: &str, port: u16) -> String {
        if let Some(cfg) = self.configs.get(&(io_type.to_string(), port as i32)) {
            if let Some(ref name) = cfg.display_name {
                return name.clone();
            }
        }
        port.to_string()
    }

    /// Check if a port is visible (defaults to true if not configured).
    pub fn is_port_visible(&self, io_type: &str, port: u16) -> bool {
        if let Some(cfg) = self.configs.get(&(io_type.to_string(), port as i32)) {
            return cfg.is_visible;
        }
        true
    }
}

// ============================================================================
//                          DATA TRANSFER OBJECTS (DTOs)
// ============================================================================

/// Saved robot connection from database.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RobotConnection {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub ip_address: String,
    pub port: u32,
    // Motion defaults
    pub default_speed: f64,
    pub default_speed_type: String,
    pub default_term_type: String,
    pub default_w: f64,
    pub default_p: f64,
    pub default_r: f64,
    // Jog defaults
    pub default_cartesian_jog_speed: f64,
    pub default_cartesian_jog_step: f64,
    pub default_joint_jog_speed: f64,
    pub default_joint_jog_step: f64,
    pub default_rotation_jog_speed: f64,
    pub default_rotation_jog_step: f64,
}

/// Named configuration for a robot (frame, tool, arm config).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RobotConfiguration {
    pub id: i64,
    pub robot_connection_id: i64,
    pub name: String,
    pub is_default: bool,
    pub u_frame_number: i32,
    pub u_tool_number: i32,
    pub front: i32,
    pub up: i32,
    pub left: i32,
    pub flip: i32,
    pub turn4: i32,
    pub turn5: i32,
    pub turn6: i32,
}

/// New robot configuration (for creating without ID).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewRobotConfiguration {
    pub name: String,
    pub is_default: bool,
    pub u_frame_number: i32,
    pub u_tool_number: i32,
    pub front: i32,
    pub up: i32,
    pub left: i32,
    pub flip: i32,
    pub turn4: i32,
    pub turn5: i32,
    pub turn6: i32,
}

/// Optional start position for program execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartPosition {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Robot default settings.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RobotSettings {
    pub default_w: f64,
    pub default_p: f64,
    pub default_r: f64,
    pub default_speed: f64,
    pub default_term_type: String,
    pub default_uframe: i32,
    pub default_utool: i32,
}

/// A single change entry in the changelog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeLogEntry {
    pub field_name: String,
    pub old_value: String,
    pub new_value: String,
}

/// Active jog settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveJogSettings {
    pub cartesian_jog_speed: f64,
    pub cartesian_jog_step: f64,
    pub joint_jog_speed: f64,
    pub joint_jog_step: f64,
}

/// Active configuration state (DTO version).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveConfiguration {
    pub loaded_from_id: Option<i64>,
    pub loaded_from_name: Option<String>,
    pub changes_count: u32,
    pub change_log: Vec<ChangeLogEntry>,
    pub u_frame_number: i32,
    pub u_tool_number: i32,
    pub front: i32,
    pub up: i32,
    pub left: i32,
    pub flip: i32,
    pub turn4: i32,
    pub turn5: i32,
    pub turn6: i32,
}
