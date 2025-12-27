//! Robot status polling plugin.
//!
//! Periodically polls the robot for position, joint angles, and status updates.

use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use bevy_tokio_tasks::TokioTasksRuntime;
use std::time::Duration;
use fanuc_rmi::packets::{SendPacket, ResponsePacket, CommandResponse, PacketPriority};
use super::types::*;

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
    tokio_runtime: Res<TokioTasksRuntime>,
    robots: Query<(&RmiDriver, &RobotConnectionState), With<FanucRobot>>,
) {
    use fanuc_rmi::dto as raw_dto;

    // Enter the Tokio runtime context so send_packet can use tokio::spawn
    let _guard = tokio_runtime.runtime().enter();

    for (driver, state) in robots.iter() {
        if *state != RobotConnectionState::Connected {
            continue;
        }
        trace!("poll_robot_status: sending poll commands to connected robot");

        // Send position query using dto types
        let pos_cmd = raw_dto::Command::FrcReadCartesianPosition(
            raw_dto::FrcReadCartesianPosition { group: 1 }
        );
        let pos_packet: SendPacket = raw_dto::SendPacket::Command(pos_cmd).into();
        let _ = driver.0.send_packet(pos_packet, PacketPriority::High);

        // Send joint angles query
        let joint_cmd = raw_dto::Command::FrcReadJointAngles(
            raw_dto::FrcReadJointAngles { group: 1 }
        );
        let joint_packet: SendPacket = raw_dto::SendPacket::Command(joint_cmd).into();
        let _ = driver.0.send_packet(joint_packet, PacketPriority::High);

        // Send status query
        let status_cmd = raw_dto::Command::FrcGetStatus;
        let status_packet: SendPacket = raw_dto::SendPacket::Command(status_cmd).into();
        let _ = driver.0.send_packet(status_packet, PacketPriority::High);
    }
}

/// Process responses from polling commands.
fn process_poll_responses(
    mut robots: Query<(
        &mut RmiResponseChannel,
        &mut RobotPosition,
        &mut JointAngles,
        &mut RobotStatus,
        &mut FrameToolDataState,
        &RobotConnectionState,
    ), With<FanucRobot>>,
) {
    for (mut response_channel, mut position, mut joints, mut status, mut frame_tool_state, state) in robots.iter_mut() {
        if *state != RobotConnectionState::Connected {
            continue;
        }

        // Process all available responses
        let mut response_count = 0;
        while let Ok(response) = response_channel.0.try_recv() {
            response_count += 1;
            match response {
                ResponsePacket::CommandResponse(CommandResponse::FrcReadCartesianPosition(pos_resp)) => {
                    // Update position from response (pos field, f64 values)
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
                    // Update speed override from robot (this is the actual value the robot is using)
                    status.speed_override = status_resp.override_value as u8;
                    // Store error message if there's an error
                    if status_resp.error_id != 0 {
                        status.error_message = Some(format!("Error ID: {}", status_resp.error_id));
                    } else {
                        status.error_message = None;
                    }
                    // Update active frame/tool numbers in RobotStatus
                    status.active_uframe = status_resp.number_uframe as u8;
                    status.active_utool = status_resp.number_utool as u8;

                    // Also update FrameToolDataState so the UI gets the actual robot values
                    frame_tool_state.active_frame = status_resp.number_uframe as i32;
                    frame_tool_state.active_tool = status_resp.number_utool as i32;

                    // NOTE: We intentionally do NOT sync the driver's sequence counter here.
                    // The driver's counter is authoritative - it knows what sequence IDs it has
                    // assigned to packets. The robot's next_sequence_id is what it expects to
                    // receive next, which may lag behind what the driver has sent due to in-flight
                    // packets. Syncing here would cause duplicate sequence IDs to be sent.
                    // The driver already initializes its counter to 1 on connect, matching the
                    // robot's expectation after FRC_Initialize.
                }
                _ => {
                    // Ignore other response types (instruction responses, etc.)
                }
            }
        }
        if response_count > 0 {
            trace!("process_poll_responses: processed {} responses", response_count);
        }
    }
}

