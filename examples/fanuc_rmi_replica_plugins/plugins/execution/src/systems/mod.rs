//! ECS systems for the execution plugin.
//!
//! These systems run on the server and handle:
//! - Buffer state management
//! - Orchestration of motion and auxiliary commands
//! - Device status tracking
//!
//! Device-specific handlers (FANUC, Duet, etc.) are in their respective plugin crates.

mod orchestrator;

pub use orchestrator::{
    orchestrator_system, update_buffer_state_system,
    AuxiliaryCommandEvent, DeviceStatus, DeviceType, MotionCommandEvent,
};

