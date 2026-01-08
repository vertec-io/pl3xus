//! ECS systems for the execution plugin.
//!
//! These systems run on the server and handle:
//! - Buffer state management
//! - Orchestration of motion and auxiliary commands
//! - Device status tracking
//! - Lifecycle management (disconnect cleanup)
//! - State synchronization (BufferState ↔ ExecutionState)
//! - Validation coordination (subsystem readiness checks)
//!
//! Device-specific handlers (FANUC, Duet, etc.) are in their respective plugin crates.

mod lifecycle;
mod orchestrator;
mod sync;
mod validation;

pub use lifecycle::{reset_on_disconnect_system, DeviceConnected};
pub use orchestrator::{
    orchestrator_system, update_buffer_state_system, AuxiliaryCommandEvent, DeviceStatus,
    DeviceType, MotionCommandEvent,
};
pub use sync::{sync_buffer_state_to_execution_state, sync_device_status_to_buffer_state};
pub use validation::{coordinate_validation, ValidationStartTime};

