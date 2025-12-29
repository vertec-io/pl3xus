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
//! - `stores` - Client-side reactive stores

use cfg_if::cfg_if;

// Always available
mod conversion;
pub use conversion::{
    FanucConversion, isometry_f32_to_position, isometry_to_position, position_to_isometry,
};
pub use fanuc_rmi::Position;

cfg_if! {
    if #[cfg(feature = "server")] {
        mod motion;
        mod connection;
        mod handlers;
        mod jogging;
        mod polling;
        mod sync;
        mod validation;
        pub mod database;

        pub use motion::{
            fanuc_motion_handler_system, fanuc_motion_response_system, fanuc_sent_instruction_system,
            robot_pose_to_fanuc_position, FanucInFlightInstructions, FanucMotionDevice,
        };
        pub use database::FanucDatabaseInit;
    }
}

cfg_if! {
    if #[cfg(feature = "ecs")] {
        mod plugin;

        pub use plugin::FanucPlugin;
    }
}

cfg_if! {
    if #[cfg(any(feature = "ecs", feature = "stores"))] {
        pub mod types;

        pub use types::*;
    }
}

