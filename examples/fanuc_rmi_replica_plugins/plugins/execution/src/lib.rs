//! Execution Plugin for Coordinated Multi-Device Toolpath Execution
//!
//! This crate provides the core execution infrastructure for coordinating
//! multiple devices (robots, extruders, peripherals) during toolpath execution.
//!
//! # Architecture
//!
//! The execution system is built around ECS entities with clear roles:
//!
//! - **ExecutionCoordinator**: Entity that owns a ToolpathBuffer and coordinates devices
//! - **ExecutionTarget**: Marker for entities that receive commands (via parent relationship)
//! - **PrimaryMotion**: Marker for the main motion device (controls timing)
//!
//! # Key Components
//!
//! - `ToolpathBuffer`: VecDeque of ExecutionPoints, the work queue
//! - `BufferState`: State machine (Idle, Buffering, Ready, Executing, etc.)
//! - `ExecutionPoint`: A single point with motion command + auxiliary commands
//!
//! # Key Traits
//!
//! - `MotionDevice`: Implemented by robot drivers (FANUC, ABB, etc.)
//! - `AuxiliaryDevice`: Implemented by peripherals (extruders, grippers, etc.)
//!
//! # Device-Specific Handlers
//!
//! Device-specific handlers are in their respective plugin crates:
//! - FANUC: `fanuc_replica_plugins::robot::motion`
//! - Duet: `fanuc_replica_duet`
//!
//! # Example Entity Hierarchy
//!
//! ```text
//! PrinterSystem [ExecutionCoordinator, ToolpathBuffer, BufferState]
//! ├── FanucRobot [MotionDevice impl, ExecutionTarget, PrimaryMotion]
//! └── DuetExtruder [AuxiliaryDevice impl, ExecutionTarget]
//! ```

pub mod components;
pub mod devices;
pub mod traits;

#[cfg(feature = "server")]
pub mod systems;

// Re-export main types
pub use components::{
    BufferState, ExecutionCoordinator, ExecutionPoint, ExecutionTarget,
    MotionCommand, MotionType, PointMetadata, PrimaryMotion, ToolpathBuffer,
};

pub use traits::{AuxiliaryCommand, AuxiliaryDevice, DeviceError, MotionDevice};

#[cfg(feature = "server")]
pub use systems::{
    AuxiliaryCommandEvent, DeviceStatus, DeviceType, MotionCommandEvent,
};

#[cfg(feature = "ecs")]
pub mod plugin;

#[cfg(feature = "ecs")]
pub use plugin::ExecutionPlugin;

