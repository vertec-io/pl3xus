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
//!
//! # Usage
//!
//! ```rust,ignore
//! use robot_hmi_duet::types::*;
//! use robot_hmi_duet::systems::*;
//! ```

use cfg_if::cfg_if;

// ============================================================================
// TYPES - Always available (ECS derives gated by "ecs" feature)
// ============================================================================
pub mod types;

// ============================================================================
// SERVER-ONLY - Plugin, systems
// ============================================================================
cfg_if! {
    if #[cfg(feature = "server")] {
        mod plugin;
        pub mod systems;

        pub use plugin::DuetPlugin;
    }
}

