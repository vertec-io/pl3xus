//! ECS systems for the execution plugin.
//!
//! These systems run on the server and handle:
//! - Buffer state management
//! - Orchestration of motion and auxiliary commands
//! - Device status tracking
//! - Lifecycle management (disconnect cleanup)
//!
//! Device-specific handlers (FANUC, Duet, etc.) are in their respective plugin crates.

mod lifecycle;
mod orchestrator;

pub use lifecycle::{reset_on_disconnect_system, DeviceConnected};
pub use orchestrator::{
    orchestrator_system, update_buffer_state_system,
    AuxiliaryCommandEvent, DeviceStatus, DeviceType, MotionCommandEvent,
};

