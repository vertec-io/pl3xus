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

use cfg_if::cfg_if;

// Always available
mod device;
pub use device::{
    DuetCommandEvent, DuetConnectionState, DuetExtruder, DuetExtruderBundle,
    DuetExtruderConfig, DuetHttpClient, DuetPositionState,
    format_extrusion_gcode, piston_travel_to_volume, volume_to_piston_travel,
};

cfg_if! {
    if #[cfg(feature = "server")] {
        mod handler;

        pub use handler::{duet_command_handler_system, duet_http_sender_system};
    }
}

cfg_if! {
    if #[cfg(feature = "ecs")] {
        mod plugin;

        pub use plugin::DuetPlugin;
    }
}

