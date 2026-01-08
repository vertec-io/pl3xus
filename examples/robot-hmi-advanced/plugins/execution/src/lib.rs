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
//! - FANUC: `robot_hmi_plugins::robot::motion`
//! - Duet: `robot_hmi_duet`
//!
//! # Example Entity Hierarchy
//!
//! ```text
//! PrinterSystem [ExecutionCoordinator, ToolpathBuffer, BufferState]
//! ├── FanucRobot [MotionDevice impl, ExecutionTarget, PrimaryMotion]
//! └── DuetExtruder [AuxiliaryDevice impl, ExecutionTarget]
//! ```

use cfg_if::cfg_if;

// ============================================================================
// TYPES - Always available (ECS derives gated by "ecs" feature)
// ============================================================================
pub mod traits;
pub mod types;

// ============================================================================
// SERVER-ONLY - Plugin, handlers, systems
// ============================================================================
cfg_if! {
    if #[cfg(feature = "server")] {
        pub mod handlers;
        mod plugin;
        pub mod systems;

        pub use plugin::ExecutionPlugin;
        pub use plugin::SubsystemValidation;
    }
}

