//! Robot status polling plugin.
//!
//! Periodically polls the robot for position, joint angles, and status updates.
//! Also handles active config sync detection and retry logic.

use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use bevy_tokio_tasks::TokioTasksRuntime;
use std::time::Duration;
use fanuc_rmi::packets::{SendPacket, ResponsePacket, CommandResponse, PacketPriority};
use crate::types::*;

use crate::connection::{FanucRobot, RmiDriver, RmiResponseChannel, RobotConnectionState};

/// Marker component to indicate a sync retry is in progress (prevents concurrent retries).
#[derive(Component)]
pub struct ConfigSyncInProgress;

// ============================================================================
// Plugin
// ============================================================================

pub struct RobotPollingPlugin;

impl Plugin for RobotPollingPlugin {
    fn build(&self, app: &mut App) {
        // Poll every 100ms for position/status updates
        // Detect config mismatches and trigger resync system
        app.add_systems(Update, (
            poll_robot_status.run_if(on_timer(Duration::from_millis(100))),
            process_poll_responses,
            handle_config_sync_retry,
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
///
/// Also detects mismatches between robot's active frame/tool and ActiveConfigState,
/// triggering resync attempts when needed.
fn process_poll_responses(
    mut robots: Query<(
        &mut RmiResponseChannel,
        &mut RobotPosition,
        &mut JointAngles,
        &mut RobotStatus,
        &mut FrameToolDataState,
        &ActiveConfigState,
        &mut ActiveConfigSyncState,
        &RobotConnectionState,
    ), With<FanucRobot>>,
) {
    for (
        mut response_channel,
        mut position,
        mut joints,
        mut status,
        mut frame_tool_state,
        active_config,
        mut sync_state,
        state
    ) in robots.iter_mut() {
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
                    status.servo_ready = status_resp.servo_ready == 1;
                    status.tp_enabled = status_resp.tp_mode == 1;
                    status.in_motion = status_resp.rmi_motion_status == 1;
                    status.speed_override = status_resp.override_value as u8;

                    if status_resp.error_id != 0 {
                        status.error_message = Some(format!("Error ID: {}", status_resp.error_id));
                    } else {
                        status.error_message = None;
                    }

                    // Update active frame/tool numbers in RobotStatus
                    let robot_uframe = status_resp.number_uframe as i32;
                    let robot_utool = status_resp.number_utool as i32;
                    status.active_uframe = robot_uframe as u8;
                    status.active_utool = robot_utool as u8;

                    // Update FrameToolDataState so the UI gets the actual robot values
                    frame_tool_state.active_frame = robot_uframe;
                    frame_tool_state.active_tool = robot_utool;

                    // Check for mismatch between robot and ActiveConfigState
                    let config_uframe = active_config.u_frame_number;
                    let config_utool = active_config.u_tool_number;
                    let has_mismatch = robot_uframe != config_uframe || robot_utool != config_utool;

                    if has_mismatch {
                        // Only update status if we're not already retrying or failed
                        match sync_state.status {
                            ConfigSyncStatus::Synced => {
                                // Detected mismatch - trigger resync
                                warn!(
                                    "🔄 Config mismatch detected: robot has UFrame={}, UTool={} but config wants UFrame={}, UTool={}",
                                    robot_uframe, robot_utool, config_uframe, config_utool
                                );
                                sync_state.status = ConfigSyncStatus::Mismatch;
                                sync_state.retry_count = 0;
                                sync_state.error_message = None;
                            }
                            ConfigSyncStatus::Retrying => {
                                // Still retrying, don't change status
                            }
                            ConfigSyncStatus::Mismatch | ConfigSyncStatus::Failed => {
                                // Already aware of mismatch or failed, don't spam logs
                            }
                        }
                    } else {
                        // Values match - mark as synced if we were in mismatch/retrying state
                        if sync_state.status != ConfigSyncStatus::Synced {
                            info!("✅ Config sync successful: robot now has UFrame={}, UTool={}", robot_uframe, robot_utool);
                            sync_state.status = ConfigSyncStatus::Synced;
                            sync_state.retry_count = 0;
                            sync_state.sync_in_progress = false;
                            sync_state.error_message = None;
                        }
                    }

                    // NOTE: We intentionally do NOT sync the driver's sequence counter here.
                    // The driver's counter is authoritative.
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


/// Handle config sync retries when mismatch is detected.
///
/// When `ActiveConfigSyncState.status` is `Mismatch`, this system:
/// 1. Sends FrcSetUFrameUTool command to the robot
/// 2. Transitions to `Retrying` status
/// 3. On next poll, if values match, transitions to `Synced`
/// 4. If max retries exceeded, transitions to `Failed`
fn handle_config_sync_retry(
    tokio: Res<TokioTasksRuntime>,
    mut commands: Commands,
    mut robots: Query<(
        Entity,
        &ActiveConfigState,
        &mut ActiveConfigSyncState,
        &RobotConnectionState,
        Option<&RmiDriver>,
    ), (With<FanucRobot>, Without<ConfigSyncInProgress>)>,
) {
    use fanuc_rmi::dto as raw_dto;

    // Enter the Tokio runtime context so send_packet can use tokio::spawn
    let _guard = tokio.runtime().enter();

    for (entity, active_config, mut sync_state, conn_state, driver) in robots.iter_mut() {
        // Only process if connected and in Mismatch state
        if *conn_state != RobotConnectionState::Connected {
            continue;
        }

        if sync_state.status != ConfigSyncStatus::Mismatch {
            continue;
        }

        // Check if we've exceeded max retries
        if sync_state.retry_count >= sync_state.max_retries {
            error!(
                "❌ Config sync failed after {} retries. Robot may need manual intervention.",
                sync_state.retry_count
            );
            sync_state.status = ConfigSyncStatus::Failed;
            sync_state.error_message = Some(format!(
                "Failed to sync frame/tool after {} attempts. Robot may be in an incompatible state.",
                sync_state.retry_count
            ));
            continue;
        }

        // Get driver to send command
        let Some(driver) = driver else {
            warn!("No driver available for config sync retry");
            continue;
        };

        // Mark as in-progress to prevent concurrent retries
        commands.entity(entity).insert(ConfigSyncInProgress);
        sync_state.status = ConfigSyncStatus::Retrying;
        sync_state.retry_count += 1;
        sync_state.sync_in_progress = true;

        let uframe = active_config.u_frame_number as u8;
        let utool = active_config.u_tool_number as u8;
        let retry_count = sync_state.retry_count;

        info!(
            "🔄 Config sync retry {}/{}: Setting UFrame={}, UTool={}",
            retry_count, sync_state.max_retries, uframe, utool
        );

        // Build the command packet (same pattern as handlers.rs)
        let command = raw_dto::Command::FrcSetUFrameUTool(raw_dto::FrcSetUFrameUTool {
            group: 1,
            u_frame_number: uframe,
            u_tool_number: utool,
        });
        let send_packet: SendPacket = raw_dto::SendPacket::Command(command).into();

        // Send synchronously (send_packet is not async, it queues the packet)
        match driver.0.send_packet(send_packet, PacketPriority::High) {
            Ok(seq) => {
                info!("✅ Config sync command queued successfully (retry {}, seq={})", retry_count, seq);
                // Remove the in-progress marker - next poll will verify
                commands.entity(entity).remove::<ConfigSyncInProgress>();
                sync_state.sync_in_progress = false;
                // Stay in Retrying - next poll will verify and set to Synced or back to Mismatch
            }
            Err(e) => {
                error!("❌ Config sync command failed to queue: {}", e);
                commands.entity(entity).remove::<ConfigSyncInProgress>();
                sync_state.sync_in_progress = false;
                // Go back to Mismatch to trigger another retry
                sync_state.status = ConfigSyncStatus::Mismatch;
                sync_state.error_message = Some(format!("Command failed: {}", e));
            }
        }
    }
}


