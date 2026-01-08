//! FANUC-specific types for the replica system.
//!
//! This module contains FANUC-specific types including:
//! - Synced components (wrapped `fanuc_rmi::dto` types)
//! - Network messages and DTOs
//! - I/O status types
//!
//! Import types from their canonical sources:
//! - `fanuc_rmi::dto` for raw FANUC DTO types
//! - `robot_hmi_execution` for execution state types
//! - `robot_hmi_programs` for program types
//! - `robot_hmi_robotics` for robotics types
//! - `pl3xus_common` for common traits

mod device;
mod config;
mod requests;

// Re-export all types for backwards compatibility
pub use device::*;
pub use config::*;
pub use requests::*;

