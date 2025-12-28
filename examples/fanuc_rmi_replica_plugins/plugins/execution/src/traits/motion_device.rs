//! MotionDevice trait for robot drivers.

use crate::components::{ExecutionPoint, MotionCommand};
use fanuc_replica_robotics::RobotPose;

use super::DeviceError;

/// Trait for devices that execute motion commands.
///
/// This is the primary abstraction for robot drivers. The motion device
/// controls the timing of execution - the orchestrator won't send a new
/// point until `ready_for_next()` returns true.
///
/// # Implementation Notes
///
/// - `send_motion()` should convert the universal RobotPose (quaternion) to
///   the device-specific format (e.g., WPR for FANUC)
/// - `ready_for_next()` should check the device's buffer status
/// - `motion_complete()` is used for tracking completed points
///
/// # Example
///
/// ```rust,ignore
/// impl MotionDevice for FanucDriver {
///     fn device_type(&self) -> &str { "fanuc_rmi" }
///
///     fn send_motion(
///         &mut self,
///         target: &RobotPose,
///         motion: &MotionCommand,
///         point: &ExecutionPoint,
///     ) -> Result<(), DeviceError> {
///         // Convert quaternion to WPR
///         let wpr = quaternion_to_wpr(target.orientation);
///         let position = FrcPosition::cartesian(
///             target.translation.x, target.translation.y, target.translation.z,
///             wpr.w, wpr.p, wpr.r,
///         );
///         // Send via RMI...
///         Ok(())
///     }
///
///     fn ready_for_next(&self) -> bool {
///         self.buffer_slots_available() > 0
///     }
/// }
/// ```
pub trait MotionDevice {
    /// Unique identifier for this device type.
    fn device_type(&self) -> &str;

    /// Send a motion command to the device.
    ///
    /// The implementation is responsible for:
    /// - Converting RobotPose (quaternion) to device-specific format
    /// - Applying motion parameters (speed, blending, etc.)
    /// - Handling device-specific protocol
    fn send_motion(
        &mut self,
        target: &RobotPose,
        motion: &MotionCommand,
        point: &ExecutionPoint,
    ) -> Result<(), DeviceError>;

    /// Check if the device is ready to receive the next motion command.
    ///
    /// This controls the flow of the orchestrator. Return true when:
    /// - Device buffer has space
    /// - Device is not in error state
    /// - Device is connected and operational
    fn ready_for_next(&self) -> bool;

    /// Check if a previously sent motion is confirmed complete.
    ///
    /// Called by the orchestrator to track completed_count.
    /// Returns the number of newly completed motions since last check.
    fn motions_completed(&self) -> u32 {
        0 // Default: no tracking
    }

    /// Check if the device is connected.
    fn is_connected(&self) -> bool;

    /// Get the current position of the device (if known).
    fn current_pose(&self) -> Option<RobotPose> {
        None // Default: unknown
    }
}

