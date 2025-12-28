//! Duet Extruder Plugin
//!
//! This crate provides support for Duet-based extruders that communicate
//! via HTTP using the RepRapFirmware API.
//!
//! # Architecture
//!
//! The Duet controller accepts G-code commands via HTTP:
//! - `/rr_gcode?gcode=<url-encoded-gcode>` - Execute G-code
//! - `/rr_model?key=<key>&flags=<flags>` - Query object model
//!
//! For extrusion, we use:
//! - `G1 Y{position} F{feedrate}` - Move Y axis (piston) to position
//! - `M220 S{percent}` - Set speed override

mod device;
#[cfg(feature = "server")]
mod handler;
#[cfg(feature = "ecs")]
mod plugin;

// Re-export device types
pub use device::{
    DuetCommandEvent, DuetConnectionState, DuetExtruder, DuetExtruderBundle,
    DuetExtruderConfig, DuetHttpClient, DuetPositionState,
    format_extrusion_gcode, piston_travel_to_volume, volume_to_piston_travel,
};

// Re-export handler systems
#[cfg(feature = "server")]
pub use handler::{duet_command_handler_system, duet_http_sender_system};

// Re-export plugin
#[cfg(feature = "ecs")]
pub use plugin::DuetPlugin;

