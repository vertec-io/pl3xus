//! FANUC Motion Command Handler System
//!
//! This system processes MotionCommandEvents for FANUC robots,
//! converting quaternion-based poses to WPR format and sending
//! instructions via the RMI driver.
//!
//! # Architecture
//!
//! The handler listens for MotionCommandEvent and:
//! 1. Looks up the RmiDriver for the target entity
//! 2. Converts the quaternion pose to FANUC WPR format
//! 3. Builds the appropriate instruction (FrcLinearMotion, FrcJointMotion, etc.)
//! 4. Sends the instruction via the driver
//! 5. Updates DeviceStatus to signal completion

use bevy::prelude::*;

use fanuc_replica_execution::{DeviceStatus, MotionCommandEvent, MotionType};
use fanuc_replica_robotics::RobotPose;

/// Marker component for FANUC robot entities that can receive motion commands.
#[derive(Component, Debug, Clone, Default, Reflect)]
#[reflect(Component)]
pub struct FanucMotionDevice;

/// System that processes MotionCommandEvents for FANUC robots.
///
/// This system:
/// 1. Listens for MotionCommandEvents
/// 2. Looks up the FANUC driver for the target entity
/// 3. Converts quaternion pose to WPR
/// 4. Sends the motion instruction
///
/// Note: This is a placeholder implementation. The actual implementation
/// would need to integrate with the RmiDriver component from the plugins crate.
pub fn fanuc_motion_handler_system(
    mut motion_events: MessageReader<MotionCommandEvent>,
    mut device_query: Query<&mut DeviceStatus, With<FanucMotionDevice>>,
) {
    for event in motion_events.read() {
        // Look up the device
        let Ok(mut status) = device_query.get_mut(event.device) else {
            warn!(
                "MotionCommandEvent for entity {:?} but no FanucMotionDevice found",
                event.device
            );
            continue;
        };

        // Get translation and WPR from the pose
        let (x, y, z) = event.target_pose.translation();
        let (w, p, r) = event.target_pose.to_wpr_degrees();

        // Build the instruction based on motion type
        let instruction_type = match &event.motion.motion_type {
            MotionType::Linear => "FrcLinearMotion",
            MotionType::Joint => "FrcJointMotion",
            MotionType::Circular { .. } => "FrcCircularMotion",
        };

        info!(
            "FANUC motion: {} to ({:.2}, {:.2}, {:.2}) WPR({:.2}, {:.2}, {:.2}) @ {:.1} mm/s (point {})",
            instruction_type,
            x, y, z,
            w, p, r,
            event.motion.speed,
            event.point.index
        );

        // In a real implementation, we would:
        // 1. Get the RmiDriver component
        // 2. Build the appropriate SendPacket
        // 3. Send via driver.send_packet()
        // 4. Track the sequence number for completion feedback

        // For now, just mark as ready for next command
        // In real impl, this would be set by feedback from the robot
        status.ready_for_next = true;
        status.completed_count += 1;
    }
}

/// Convert a RobotPose to FANUC Position format (x, y, z, w, p, r).
///
/// This is a helper function that would be used when building
/// the actual FANUC instruction packets.
pub fn robot_pose_to_fanuc_position(pose: &RobotPose) -> (f64, f64, f64, f64, f64, f64) {
    let (x, y, z) = pose.translation();
    let (w, p, r) = pose.to_wpr_degrees();

    (x, y, z, w, p, r)
}

