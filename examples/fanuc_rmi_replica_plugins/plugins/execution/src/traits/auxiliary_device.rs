//! AuxiliaryDevice trait for peripheral devices.

use serde::{Deserialize, Serialize};

use super::DeviceError;

/// Commands that can be sent to auxiliary devices.
///
/// This enum covers common auxiliary operations. Device implementations
/// should handle the commands relevant to them and return an error for
/// unsupported commands.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuxiliaryCommand {
    /// No operation - skip this device for this point
    None,

    /// Extruder move command (for Duet-style extruders)
    Extruder {
        /// Distance to move piston in mm (relative)
        distance: f32,
        /// Speed in mm/s
        speed: f32,
    },

    /// Digital output control
    DigitalOutput {
        /// Output channel number
        channel: u8,
        /// Desired state
        state: bool,
    },

    /// Analog output control
    AnalogOutput {
        /// Output channel number
        channel: u8,
        /// Normalized value (0.0 - 1.0)
        value: f32,
    },

    /// Generic G-code pass-through
    Gcode(String),

    /// Valve control
    Valve {
        /// Valve identifier
        id: String,
        /// Open (true) or close (false)
        open: bool,
    },

    /// Wait for time
    Dwell {
        /// Time to wait in seconds
        seconds: f32,
    },

    /// Custom command with arbitrary data
    Custom {
        /// Command type identifier
        command_type: String,
        /// JSON-serialized parameters
        parameters: String,
    },
}

impl Default for AuxiliaryCommand {
    fn default() -> Self {
        AuxiliaryCommand::None
    }
}

/// Trait for devices that execute auxiliary commands.
///
/// Auxiliary devices are peripherals that operate in coordination with
/// the motion device but don't control motion timing.
///
/// # Implementation Notes
///
/// - `send_command()` should handle the command asynchronously if needed
/// - `device_type()` is used to match commands from ExecutionPoint.aux_commands
/// - Unlike MotionDevice, the orchestrator doesn't wait for auxiliary devices
///
/// # Example
///
/// ```rust,ignore
/// impl AuxiliaryDevice for DuetExtruder {
///     fn device_type(&self) -> &str { "duet_extruder" }
///
///     fn send_command(&mut self, cmd: &AuxiliaryCommand) -> Result<(), DeviceError> {
///         match cmd {
///             AuxiliaryCommand::Extruder { distance, speed } => {
///                 let gcode = format!("G1 Y{:.4} F{:.1}", distance, speed * 60.0);
///                 self.channel.send(gcode)?;
///                 Ok(())
///             }
///             AuxiliaryCommand::None => Ok(()),
///             _ => Err(DeviceError::InvalidCommand("Unsupported command".into())),
///         }
///     }
///
///     fn is_ready(&self) -> bool { true } // Duet buffers commands
/// }
/// ```
pub trait AuxiliaryDevice {
    /// Unique identifier for this device type.
    ///
    /// This is used to match commands from `ExecutionPoint.aux_commands`.
    fn device_type(&self) -> &str;

    /// Send a command to the device.
    ///
    /// The implementation should:
    /// - Handle the specific command types it supports
    /// - Return an error for unsupported commands
    /// - Handle async communication if needed
    fn send_command(&mut self, cmd: &AuxiliaryCommand) -> Result<(), DeviceError>;

    /// Check if the device is ready to receive commands.
    ///
    /// Most auxiliary devices buffer commands and return true here.
    /// Return false if the device is in an error state or disconnected.
    fn is_ready(&self) -> bool;

    /// Check if the device is connected.
    fn is_connected(&self) -> bool;
}

