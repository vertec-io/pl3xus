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
//! This crate bridges the robot-agnostic types from `robot_hmi_robotics`
//! to FANUC-specific types from `fanuc_rmi`. All quaternion↔WPR conversion
//! happens at this layer.
//!
//! # Features
//!
//! - `ecs` - Bevy ECS components and systems
//! - `server` - Server-only functionality (driver, tokio, database)
//! - `stores` - Client-side reactive stores
//!
//! # Usage
//!
//! ```rust,ignore
//! use robot_hmi_fanuc::types::*;
//! use robot_hmi_fanuc::database;
//! use robot_hmi_fanuc::conversion;
//! ```

use cfg_if::cfg_if;

// ============================================================================
// ALWAYS AVAILABLE - No feature gating at lib.rs level
// ============================================================================

// Types - always available (internal feature gating for ECS derives, stores)
pub mod types;

// Conversion utilities - always available
pub mod conversion;

// Re-export vendor type for convenience
pub use fanuc_rmi::Position;

// ============================================================================
// SERVER-ONLY - Plugin, systems, handlers, database, driver
// No internal feature gating needed in these files!
// ============================================================================
cfg_if! {
    if #[cfg(feature = "server")] {
        mod connection;
        mod plugin;
        mod sync;
        pub mod database;
        pub mod handlers;
        pub mod systems;

        pub use plugin::FanucPlugin;
    }
}

