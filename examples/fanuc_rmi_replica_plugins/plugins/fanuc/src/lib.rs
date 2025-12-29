//! FANUC-specific Driver and Conversion Utilities
//!
//! This crate provides FANUC-specific implementations including:
//! - Conversion between robot-agnostic types and FANUC Position format
//! - Motion command handler for execution orchestration
//! - FANUC RMI driver integration
//! - Robot connection management
//! - Program management
//! - Jogging functionality
//!
//! # Architecture
//!
//! This crate bridges the robot-agnostic types from `fanuc_replica_robotics`
//! to FANUC-specific types from `fanuc_rmi`. All quaternion↔WPR conversion
//! happens at this layer.
//!
//! # Features
//!
//! - `ecs` - Bevy ECS components and systems
//! - `server` - Server-only functionality (driver, tokio, database)

mod conversion;
#[cfg(feature = "server")]
mod motion;
#[cfg(feature = "ecs")]
mod plugin;

// Robot management modules (from robot/)
#[cfg(feature = "server")]
mod connection;
#[cfg(feature = "server")]
mod handlers;
#[cfg(feature = "server")]
mod jogging;
#[cfg(feature = "server")]
mod polling;
#[cfg(feature = "server")]
mod program;
#[cfg(feature = "server")]
mod sync;
// Types module is needed for both ecs (server) and stores (client) features
#[cfg(any(feature = "ecs", feature = "stores"))]
pub mod types;
#[cfg(feature = "server")]
pub mod database;

// Re-export conversion utilities
pub use conversion::{
    FanucConversion, isometry_f32_to_position, isometry_to_position, position_to_isometry,
};

// Re-export motion handler and related systems
#[cfg(feature = "server")]
pub use motion::{
    fanuc_motion_handler_system, fanuc_motion_response_system, fanuc_sent_instruction_system,
    robot_pose_to_fanuc_position, FanucInFlightInstructions, FanucMotionDevice,
};

// Re-export plugin
#[cfg(feature = "ecs")]
pub use plugin::FanucPlugin;

// Re-export fanuc_rmi types for convenience
pub use fanuc_rmi::Position;

// Re-export robot types (available for both ecs and stores features)
#[cfg(any(feature = "ecs", feature = "stores"))]
pub use types::*;

// Re-export database
#[cfg(feature = "server")]
pub use database::FanucDatabaseInit;

