//! Driver sync module - now handled by RobotPollingPlugin.
//!
//! This module previously contained response processing logic, but that has been
//! consolidated into the RobotPollingPlugin to avoid competing for responses
//! from the same channel.

/// Placeholder system - actual sync is now handled by RobotPollingPlugin.
/// This function is kept for backwards compatibility but does nothing.
pub fn sync_robot_state() {
    // Response processing is now handled by RobotPollingPlugin::process_poll_responses
    // to avoid two systems competing for the same response channel.
}
