//! Duet Extruder Device Implementation
//!
//! This module implements the AuxiliaryDevice trait for Duet-based extruders
//! that communicate via HTTP using the RepRapFirmware API.
//!
//! The Duet controller accepts G-code commands via HTTP:
//! - `/rr_gcode?gcode=<url-encoded-gcode>` - Execute G-code
//! - `/rr_model?key=<key>&flags=<flags>` - Query object model
//!
//! For extrusion, we use:
//! - `G1 Y{position} F{feedrate}` - Move Y axis (piston) to position
//! - `M220 S{percent}` - Set speed override

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Configuration for a Duet-based extruder device.
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct DuetExtruderConfig {
    /// IP address or hostname of the Duet controller
    pub host: String,
    /// HTTP port (default: 80)
    pub port: u16,
    /// Axis to use for extrusion (typically Y)
    pub axis: char,
    /// Maximum feedrate in mm/min
    pub max_feedrate: f32,
    /// Piston diameter in mm (for volume calculations)
    pub piston_diameter: f32,
}

impl Default for DuetExtruderConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            axis: 'Y',
            max_feedrate: 6000.0,
            piston_diameter: 50.0,
        }
    }
}

/// Connection state for a Duet extruder.
#[derive(Component, Debug, Clone, Default)]
pub struct DuetConnectionState {
    /// Whether we're connected to the Duet
    pub connected: bool,
    /// Last error message, if any
    pub last_error: Option<String>,
    /// Number of commands sent
    pub commands_sent: u64,
}

/// Current position state for the Duet extruder.
#[derive(Component, Debug, Clone, Default)]
pub struct DuetPositionState {
    /// Current Y-axis position (piston position in mm)
    pub position: f32,
    /// Current feedrate (mm/min)
    pub feedrate: f32,
}

/// Marker component for Duet extruder entities.
#[derive(Component, Debug, Clone, Default)]
pub struct DuetExtruder;

/// HTTP client resource for Duet communication.
/// This is a placeholder - actual implementation would use reqwest or similar.
#[derive(Resource, Default)]
pub struct DuetHttpClient {
    // In a real implementation, this would hold a reqwest::Client
    // For now, we'll use a simple marker
}

/// Event for sending commands to Duet extruders.
#[derive(bevy::prelude::Message, Debug, Clone)]
pub struct DuetCommandEvent {
    /// Target extruder entity
    pub extruder: Entity,
    /// Target Y position (piston position in mm)
    pub target_position: f32,
    /// Feedrate in mm/min
    pub feedrate: f32,
    /// Point index for tracking
    pub point_index: u32,
}

/// Bundle for spawning a Duet extruder entity.
#[derive(Bundle, Default)]
pub struct DuetExtruderBundle {
    pub marker: DuetExtruder,
    pub config: DuetExtruderConfig,
    pub connection: DuetConnectionState,
    pub position: DuetPositionState,
}

impl DuetExtruderBundle {
    pub fn new(config: DuetExtruderConfig) -> Self {
        Self {
            marker: DuetExtruder,
            config,
            connection: DuetConnectionState::default(),
            position: DuetPositionState::default(),
        }
    }
}

/// Convert extrusion volume to piston travel distance.
///
/// Given a target extrusion volume in mm³ and piston diameter,
/// calculates the required piston travel in mm.
pub fn volume_to_piston_travel(volume_mm3: f32, piston_diameter_mm: f32) -> f32 {
    let piston_area = std::f32::consts::PI * (piston_diameter_mm / 2.0).powi(2);
    volume_mm3 / piston_area
}

/// Convert piston travel to extrusion volume.
pub fn piston_travel_to_volume(travel_mm: f32, piston_diameter_mm: f32) -> f32 {
    let piston_area = std::f32::consts::PI * (piston_diameter_mm / 2.0).powi(2);
    travel_mm * piston_area
}

/// Format a G-code command for Duet extrusion.
pub fn format_extrusion_gcode(axis: char, position: f32, feedrate: f32) -> String {
    format!("G1 {}{:.4} F{:.0}", axis, position, feedrate)
}

