//! Robot status polling plugin.
//!
//! Periodically polls the robot for position, joint angles, and status updates.

use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use std::time::Duration;
use fanuc_rmi::packets::{SendPacket, ResponsePacket, CommandResponse, PacketPriority};
use fanuc_replica_types::*;

use super::connection::{FanucRobot, RmiDriver, RmiResponseChannel, RobotConnectionState};

// ============================================================================
// Plugin
// ============================================================================

pub struct RobotPollingPlugin;

impl Plugin for RobotPollingPlugin {
    fn build(&self, app: &mut App) {
        // Poll every 100ms for position/status updates
        app.add_systems(Update, (
            poll_robot_status.run_if(on_timer(Duration::from_millis(100))),
            process_poll_responses,
        ));
    }
}

// ============================================================================
// Systems
// ============================================================================

/// Send polling commands to the robot.
fn poll_robot_status(
    robots: Query<(&RmiDriver, &RobotConnectionState), With<FanucRobot>>,
) {
    use fanuc_rmi::dto as raw_dto;

    for (driver, state) in robots.iter() {
        if *state != RobotConnectionState::Connected {
            continue;
        }

        // Send position query using dto types
        let pos_cmd = raw_dto::Command::FrcReadCartesianPosition(
            raw_dto::FrcReadCartesianPosition { group: 1 }
        );
        let pos_packet: SendPacket = raw_dto::SendPacket::Command(pos_cmd).into();
        let _ = driver.0.send_command(pos_packet, PacketPriority::High);

        // Send joint angles query
        let joint_cmd = raw_dto::Command::FrcReadJointAngles(
            raw_dto::FrcReadJointAngles { group: 1 }
        );
        let joint_packet: SendPacket = raw_dto::SendPacket::Command(joint_cmd).into();
        let _ = driver.0.send_command(joint_packet, PacketPriority::High);

        // Send status query
        let status_cmd = raw_dto::Command::FrcGetStatus;
        let status_packet: SendPacket = raw_dto::SendPacket::Command(status_cmd).into();
        let _ = driver.0.send_command(status_packet, PacketPriority::High);
    }
}

/// Process responses from polling commands.
fn process_poll_responses(
    mut robots: Query<(
        &mut RmiResponseChannel,
        &mut RobotPosition,
        &mut JointAngles,
        &mut RobotStatus,
        &RobotConnectionState,
    ), With<FanucRobot>>,
) {
    for (mut response_channel, mut position, mut joints, mut status, state) in robots.iter_mut() {
        if *state != RobotConnectionState::Connected {
            continue;
        }

        // Process all available responses
        while let Ok(response) = response_channel.0.try_recv() {
            match response {
                ResponsePacket::CommandResponse(CommandResponse::FrcReadCartesianPosition(pos_resp)) => {
                    // Update position from response (pos field, f32 values)
                    position.0.x = pos_resp.pos.x;
                    position.0.y = pos_resp.pos.y;
                    position.0.z = pos_resp.pos.z;
                    position.0.w = pos_resp.pos.w;
                    position.0.p = pos_resp.pos.p;
                    position.0.r = pos_resp.pos.r;
                }
                ResponsePacket::CommandResponse(CommandResponse::FrcReadJointAngles(joint_resp)) => {
                    // Update joint angles from response (f32 values)
                    joints.0.j1 = joint_resp.joint_angles.j1;
                    joints.0.j2 = joint_resp.joint_angles.j2;
                    joints.0.j3 = joint_resp.joint_angles.j3;
                    joints.0.j4 = joint_resp.joint_angles.j4;
                    joints.0.j5 = joint_resp.joint_angles.j5;
                    joints.0.j6 = joint_resp.joint_angles.j6;
                }
                ResponsePacket::CommandResponse(CommandResponse::FrcGetStatus(status_resp)) => {
                    // Update status from response
                    // servo_ready: 1 = ready, 0 = not ready
                    // tp_mode: 1 = TP enabled, 0 = disabled
                    // rmi_motion_status: 1 = in motion, 0 = not in motion
                    status.servo_ready = status_resp.servo_ready == 1;
                    status.tp_enabled = status_resp.tp_mode == 1;
                    status.in_motion = status_resp.rmi_motion_status == 1;
                    // Store error message if there's an error
                    if status_resp.error_id != 0 {
                        status.error_message = Some(format!("Error ID: {}", status_resp.error_id));
                    } else {
                        status.error_message = None;
                    }
                }
                _ => {
                    // Ignore other response types (instruction responses, etc.)
                }
            }
        }
    }
}

