//! Traits for device abstraction.

mod auxiliary_device;
mod motion_device;

pub use auxiliary_device::{AuxiliaryCommand, AuxiliaryDevice};
pub use motion_device::MotionDevice;

use thiserror::Error;

/// Error type for device operations.
#[derive(Debug, Error)]
pub enum DeviceError {
    /// Device is not connected
    #[error("Device not connected")]
    NotConnected,

    /// Device is busy and cannot accept commands
    #[error("Device busy")]
    Busy,

    /// Communication error
    #[error("Communication error: {0}")]
    Communication(String),

    /// Invalid command for this device
    #[error("Invalid command: {0}")]
    InvalidCommand(String),

    /// Device-specific error
    #[error("Device error: {0}")]
    Device(String),

    /// Command send failed (channel closed, etc.)
    #[error("Send failed: {0}")]
    SendFailed(String),
}

