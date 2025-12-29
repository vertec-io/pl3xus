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
//! 5. Tracks in-flight instructions for completion feedback
//!
//! # Response Handling
//!
//! A separate system (`fanuc_motion_response_system`) processes instruction
//! responses from the robot, mapping sequence IDs back to point indices
//! and updating DeviceStatus accordingly.

use bevy::ecs::message::MessageReader;
use bevy::prelude::*;
use bevy_tokio_tasks::TokioTasksRuntime;
use std::collections::HashMap;

use fanuc_rmi::dto as raw_dto;
use fanuc_rmi::instructions::FrcLinearMotion;
use fanuc_rmi::packets::{Instruction, PacketPriority, ResponsePacket, SendPacket};
use fanuc_rmi::{Configuration, Position, SpeedType, TermType};

use fanuc_replica_execution::{BufferState, DeviceStatus, MotionCommandEvent, MotionType};
use fanuc_replica_robotics::RobotPose;

use crate::connection::{
    FanucRobot, RmiDriver, RmiExecutionResponseChannel, RmiSentInstructionChannel,
    RobotConnectionState,
};
use crate::types::ActiveConfigState;

/// Marker component for FANUC robot entities that can receive motion commands.
#[derive(Component, Debug, Clone, Default)]
pub struct FanucMotionDevice;

/// Resource to track in-flight motion instructions.
///
/// This maps request IDs (from send_packet) and sequence IDs (from driver)
/// back to point indices so we can update DeviceStatus on completion.
#[derive(Resource, Default)]
pub struct FanucInFlightInstructions {
    /// Map request_id (from send_packet) -> (entity, point_index)
    pub by_request: HashMap<u64, (Entity, usize)>,
    /// Map sequence_id (from SentInstructionInfo) -> (entity, point_index)
    pub by_sequence: HashMap<u32, (Entity, usize)>,
}

impl FanucInFlightInstructions {
    /// Record that an instruction was sent with the given request_id.
    pub fn record_sent(&mut self, request_id: u64, entity: Entity, point_index: usize) {
        self.by_request.insert(request_id, (entity, point_index));
    }

    /// Map a request_id to its sequence_id when SentInstructionInfo arrives.
    pub fn map_sequence(&mut self, request_id: u64, sequence_id: u32) {
        if let Some(info) = self.by_request.remove(&request_id) {
            self.by_sequence.insert(sequence_id, info);
        }
    }

    /// Handle instruction completion by sequence_id.
    /// Returns (entity, point_index) if found.
    pub fn handle_completion(&mut self, sequence_id: u32) -> Option<(Entity, usize)> {
        self.by_sequence.remove(&sequence_id)
    }

    /// Check if there are any instructions in flight.
    pub fn is_empty(&self) -> bool {
        self.by_request.is_empty() && self.by_sequence.is_empty()
    }

    /// Clear all tracking (e.g., on abort or disconnect).
    pub fn clear(&mut self) {
        self.by_request.clear();
        self.by_sequence.clear();
    }
}

/// System that processes MotionCommandEvents for FANUC robots.
///
/// This system:
/// 1. Listens for MotionCommandEvents
/// 2. Looks up the FANUC driver and ActiveConfigState for the target entity
/// 3. Converts quaternion pose to WPR
/// 4. Builds and sends the motion instruction via RMI driver
/// 5. Tracks the request_id for completion feedback
pub fn fanuc_motion_handler_system(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut motion_events: MessageReader<MotionCommandEvent>,
    mut in_flight: ResMut<FanucInFlightInstructions>,
    mut device_query: Query<&mut DeviceStatus, With<FanucMotionDevice>>,
    driver_query: Query<(&RmiDriver, &RobotConnectionState, &ActiveConfigState), With<FanucRobot>>,
) {
    // Enter the Tokio runtime context so send_packet can use tokio::spawn
    let _guard = tokio_runtime.runtime().enter();

    for event in motion_events.read() {
        // Look up the device status
        let Ok(mut status) = device_query.get_mut(event.device) else {
            warn!(
                "MotionCommandEvent for entity {:?} but no FanucMotionDevice found",
                event.device
            );
            continue;
        };

        // Get the connected driver and active configuration
        let Ok((driver, conn_state, active_config)) = driver_query.single() else {
            warn!("MotionCommandEvent but no FanucRobot with driver found");
            status.error = Some("No FANUC robot driver found".to_string());
            continue;
        };

        if *conn_state != RobotConnectionState::Connected {
            warn!("MotionCommandEvent but robot not connected");
            status.error = Some("Robot not connected".to_string());
            continue;
        }

        // Get translation and WPR from the pose
        let (x, y, z) = event.target_pose.translation();
        let (w, p, r) = event.target_pose.to_wpr_degrees();

        // Build the FANUC Position
        let position = Position {
            x,
            y,
            z,
            w,
            p,
            r,
            ext1: 0.0,
            ext2: 0.0,
            ext3: 0.0,
        };

        // Build configuration from the robot's active configuration state
        // This ensures motion uses the current UFrame/UTool and arm configuration
        let configuration = Configuration {
            u_tool_number: active_config.u_tool_number as i8,
            u_frame_number: active_config.u_frame_number as i8,
            front: active_config.front as i8,
            up: active_config.up as i8,
            left: active_config.left as i8,
            flip: active_config.flip as i8,
            turn4: active_config.turn4 as i8,
            turn5: active_config.turn5 as i8,
            turn6: active_config.turn6 as i8,
        };

        // Determine termination type based on blend_radius
        // blend_radius > 0 means CNT (continuous), 0 means FINE (stop at point)
        let (term_type, term_value) = if event.motion.blend_radius > 0.0 {
            // CNT value is typically 0-100, we'll use blend_radius as a percentage
            let cnt_value = (event.motion.blend_radius.min(100.0)) as u8;
            (TermType::CNT, cnt_value)
        } else {
            (TermType::FINE, 0)
        };

        // Build the instruction based on motion type
        let packet = match &event.motion.motion_type {
            MotionType::Linear => {
                let motion = FrcLinearMotion::new(
                    event.point.index, // sequence_id placeholder, driver will assign real one
                    configuration,
                    position,
                    SpeedType::MMSec,
                    event.motion.speed as f64,
                    term_type,
                    term_value,
                );
                SendPacket::Instruction(Instruction::FrcLinearMotion(motion))
            }
            MotionType::Joint => {
                // For joint motion, we'd use FrcJointMotion
                // For now, fall back to linear motion
                warn!("Joint motion not yet implemented, using linear motion");
                let motion = FrcLinearMotion::new(
                    event.point.index,
                    configuration,
                    position,
                    SpeedType::MMSec,
                    event.motion.speed as f64,
                    term_type,
                    term_value,
                );
                SendPacket::Instruction(Instruction::FrcLinearMotion(motion))
            }
            MotionType::Circular => {
                // For circular motion, we'd use FrcCircularMotion
                // For now, fall back to linear motion
                warn!("Circular motion not yet implemented, using linear motion");
                let motion = FrcLinearMotion::new(
                    event.point.index,
                    configuration,
                    position,
                    SpeedType::MMSec,
                    event.motion.speed as f64,
                    term_type,
                    term_value,
                );
                SendPacket::Instruction(Instruction::FrcLinearMotion(motion))
            }
        };

        // Send the instruction via the driver
        match driver.0.send_packet(packet, PacketPriority::Standard) {
            Ok(request_id) => {
                // Track the request for completion feedback
                in_flight.record_sent(request_id, event.device, event.point.index as usize);

                info!(
                    "📤 FANUC motion sent: Linear to ({:.2}, {:.2}, {:.2}) WPR({:.2}, {:.2}, {:.2}) @ {:.1} mm/s (point {}, request_id {})",
                    x, y, z, w, p, r, event.motion.speed, event.point.index, request_id
                );

                // Don't mark ready_for_next yet - wait for response
                status.ready_for_next = false;
            }
            Err(e) => {
                error!(
                    "Failed to send motion instruction for point {}: {}",
                    event.point.index, e
                );
                status.error = Some(format!("Failed to send motion: {}", e));
            }
        }
    }
}

/// System that processes SentInstructionInfo to map request_id -> sequence_id.
///
/// When the driver assigns a sequence ID to an instruction, it broadcasts
/// a SentInstructionInfo. This system updates our tracking so we can
/// correlate responses with the original points.
pub fn fanuc_sent_instruction_system(
    mut in_flight: ResMut<FanucInFlightInstructions>,
    mut channels: Query<&mut RmiSentInstructionChannel, With<FanucRobot>>,
) {
    for mut channel in channels.iter_mut() {
        while let Ok(sent_info) = channel.0.try_recv() {
            in_flight.map_sequence(sent_info.request_id, sent_info.sequence_id);
            debug!(
                "🔗 Mapped request {} -> sequence {}",
                sent_info.request_id, sent_info.sequence_id
            );
        }
    }
}

/// System that processes instruction responses from the robot.
///
/// When the robot completes an instruction, it sends a response with
/// the sequence ID. This system updates DeviceStatus to signal completion.
pub fn fanuc_motion_response_system(
    mut in_flight: ResMut<FanucInFlightInstructions>,
    mut device_query: Query<&mut DeviceStatus, With<FanucMotionDevice>>,
    mut channels: Query<&mut RmiExecutionResponseChannel, With<FanucRobot>>,
) {
    for mut channel in channels.iter_mut() {
        while let Ok(response) = channel.0.try_recv() {
            if let ResponsePacket::InstructionResponse(instr_resp) = &response {
                let seq_id = instr_resp.get_sequence_id();

                if let Some((entity, point_index)) = in_flight.handle_completion(seq_id) {
                    // Check for error
                    let error_id = instr_resp.get_error_id();
                    if error_id != 0 {
                        error!(
                            "Instruction {} (point {}) failed with error {}",
                            seq_id, point_index, error_id
                        );

                        if let Ok(mut status) = device_query.get_mut(entity) {
                            status.error = Some(format!(
                                "Instruction {} failed with error {}",
                                seq_id, error_id
                            ));
                            status.ready_for_next = false;
                        }
                    } else {
                        info!(
                            "📍 Instruction completed: sequence {} (point {})",
                            seq_id, point_index
                        );

                        if let Ok(mut status) = device_query.get_mut(entity) {
                            status.completed_count += 1;
                            // Ready for next if no more in-flight instructions
                            status.ready_for_next = in_flight.is_empty();
                        }
                    }
                }
            }
        }
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

// ============================================================================
// BufferState Change Detection and FRC Command Sending
// ============================================================================
//
// Note: sync_buffer_state_to_execution_state and sync_device_status_to_buffer_state
// have been moved to the execution plugin (not FANUC-specific).
// The react_to_buffer_state_changes system below IS FANUC-specific because it
// sends FRC commands (FrcPause, FrcContinue, FrcAbort) to the robot.

use fanuc_replica_core::ActiveSystem;

use std::sync::atomic::{AtomicU8, Ordering};

/// Tracks the last known BufferState category for change detection.
/// We use a simple enum-to-u8 mapping to detect state category changes.
#[derive(Resource, Default)]
pub struct LastBufferStateCategory(AtomicU8);

const STATE_IDLE: u8 = 0;
const STATE_EXECUTING: u8 = 1;
const STATE_PAUSED: u8 = 2;
const STATE_STOPPED: u8 = 3;
const STATE_COMPLETE: u8 = 4;

fn buffer_state_to_category(state: &BufferState) -> u8 {
    match state {
        BufferState::Idle | BufferState::Buffering { .. } | BufferState::Ready | BufferState::Validating => STATE_IDLE,
        BufferState::Executing { .. } | BufferState::AwaitingPoints { .. } | BufferState::WaitingForFeedback { .. } => STATE_EXECUTING,
        BufferState::Paused { .. } => STATE_PAUSED,
        BufferState::Stopped { .. } | BufferState::Error { .. } => STATE_STOPPED,
        BufferState::Complete { .. } => STATE_COMPLETE,
    }
}

/// React to BufferState changes and send appropriate FRC commands to the robot.
///
/// This system bridges the execution plugin's state machine with the FANUC driver:
/// - Paused → Send FrcPause to halt robot motion
/// - Executing (from Paused) → Send FrcContinue to resume motion
/// - Stopped/Error → Send FrcAbort to abort all motion
/// - Complete → No FRC command needed (program finished normally)
///
/// Note: This uses change detection to only send commands on state transitions.
pub fn react_to_buffer_state_changes(
    tokio_runtime: Res<TokioTasksRuntime>,
    system_query: Query<&BufferState, (With<ActiveSystem>, Changed<BufferState>)>,
    robot_query: Query<&RmiDriver, With<FanucRobot>>,
    last_state: ResMut<LastBufferStateCategory>,
) {
    let Ok(buffer_state) = system_query.single() else {
        return; // No change or no BufferState
    };

    let Ok(driver) = robot_query.single() else {
        return; // No robot driver
    };

    let new_category = buffer_state_to_category(buffer_state);
    let old_category = last_state.0.swap(new_category, Ordering::SeqCst);

    // Only send commands on category transitions
    if old_category == new_category {
        return;
    }

    // Enter the Tokio runtime context so send_packet can use tokio::spawn
    let _guard = tokio_runtime.runtime().enter();

    match (old_category, new_category) {
        // Transition to Paused: send FrcPause
        (STATE_EXECUTING, STATE_PAUSED) => {
            info!("🤖 BufferState changed to Paused - sending FrcPause");
            let command = raw_dto::Command::FrcPause;
            let send_packet: SendPacket = raw_dto::SendPacket::Command(command).into();
            match driver.0.send_packet(send_packet, PacketPriority::Immediate) {
                Ok(seq) => info!("Sent FrcPause with sequence {}", seq),
                Err(e) => error!("Failed to send FrcPause: {:?}", e),
            }
        }

        // Transition from Paused to Executing: send FrcContinue
        (STATE_PAUSED, STATE_EXECUTING) => {
            info!("🤖 BufferState changed from Paused to Executing - sending FrcContinue");
            let command = raw_dto::Command::FrcContinue;
            let send_packet: SendPacket = raw_dto::SendPacket::Command(command).into();
            match driver.0.send_packet(send_packet, PacketPriority::Immediate) {
                Ok(seq) => info!("Sent FrcContinue with sequence {}", seq),
                Err(e) => error!("Failed to send FrcContinue: {:?}", e),
            }
        }

        // Transition to Stopped (from Executing or Paused): send FrcAbort then FrcInitialize
        (STATE_EXECUTING | STATE_PAUSED, STATE_STOPPED) => {
            info!("🤖 BufferState changed to Stopped - sending FrcAbort + FrcInitialize");

            // First send FrcAbort to stop all motion
            let abort_command = raw_dto::Command::FrcAbort;
            let send_packet: SendPacket = raw_dto::SendPacket::Command(abort_command).into();
            match driver.0.send_packet(send_packet, PacketPriority::Immediate) {
                Ok(seq) => info!("Sent FrcAbort with sequence {}", seq),
                Err(e) => error!("Failed to send FrcAbort: {:?}", e),
            }

            // Then send FrcInitialize to reset the controller for next execution
            let init_command = raw_dto::Command::FrcInitialize(raw_dto::FrcInitialize { group_mask: 1 });
            let send_packet: SendPacket = raw_dto::SendPacket::Command(init_command).into();
            match driver.0.send_packet(send_packet, PacketPriority::Immediate) {
                Ok(seq) => info!("Sent FrcInitialize with sequence {}", seq),
                Err(e) => error!("Failed to send FrcInitialize: {:?}", e),
            }
        }

        // Transition to Complete: no FRC command needed, program finished normally
        (STATE_EXECUTING, STATE_COMPLETE) => {
            info!("🎉 BufferState changed to Complete - program finished successfully");
        }

        // Other transitions don't need FRC commands
        _ => {}
    }
}
