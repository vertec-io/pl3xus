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

use cfg_if::cfg_if;

// Always available modules
pub mod components;
pub mod traits;
pub mod types;

// Always available exports
pub use components::{
    BufferDisplayData, BufferLineDisplay, BufferState, ExecutionCoordinator, ExecutionPoint,
    ExecutionState, ExecutionTarget, MotionCommand, MotionType, PointMetadata, PrimaryMotion,
    SourceType, SubsystemEntry, SubsystemReadiness, Subsystems, SystemState, ToolpathBuffer,
    UiActions, SUBSYSTEM_DUET, SUBSYSTEM_EXECUTION, SUBSYSTEM_FANUC, SUBSYSTEM_PROGRAMS,
    VALIDATION_TIMEOUT,
};
pub use traits::{AuxiliaryCommand, AuxiliaryDevice, DeviceError, MotionDevice};
pub use types::{Pause, PauseResponse, Resume, ResumeResponse, Start, StartResponse, Stop, StopResponse};

cfg_if! {
    if #[cfg(feature = "server")] {
        pub mod handlers;
        pub mod systems;

        pub use handlers::{handle_pause, handle_resume, handle_start, handle_stop};
        pub use systems::{
            AuxiliaryCommandEvent, DeviceConnected, DeviceStatus, DeviceType, MotionCommandEvent,
        };
    }
}

cfg_if! {
    if #[cfg(feature = "ecs")] {
        pub mod plugin;

        pub use plugin::{ExecutionPlugin, SubsystemValidation};
    }
}

