//! Robot plugin - all robot-related functionality.
//!
//! This plugin handles:
//! - Robot state, position, joints, status
//! - Robot connections and configurations
//! - Program management and execution
//! - I/O status and configuration
//! - Jog commands and motion
//! - Database operations for robot data

pub mod types;

#[cfg(feature = "server")]
mod plugin;
#[cfg(feature = "server")]
pub mod connection;
#[cfg(feature = "server")]
mod sync;
#[cfg(feature = "server")]
mod handlers;
#[cfg(feature = "server")]
mod polling;
#[cfg(feature = "server")]
pub mod program;
#[cfg(feature = "server")]
mod jogging;

// Re-export all types for convenience
pub use types::*;

// Server-only exports
#[cfg(feature = "server")]
pub use plugin::RobotPlugin;
