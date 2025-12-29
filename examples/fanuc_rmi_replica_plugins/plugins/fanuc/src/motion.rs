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

use fanuc_rmi::instructions::FrcLinearMotion;
use fanuc_rmi::packets::{Instruction, PacketPriority, ResponsePacket, SendPacket};
use fanuc_rmi::{Configuration, Position, SpeedType, TermType};

use fanuc_replica_execution::{
    BufferState, DeviceStatus, ExecutionCoordinator, ExecutionPoint, MotionCommand,
    MotionCommandEvent, MotionType, MotionType as ExecMotionType, PointMetadata, PrimaryMotion,
    ToolpathBuffer,
};
use fanuc_replica_robotics::RobotPose;

use crate::connection::{
    FanucRobot, RmiDriver, RmiExecutionResponseChannel, RmiSentInstructionChannel,
    RobotConnectionState,
};
use crate::types::ActiveConfigState;

/// Marker component for FANUC robot entities that can receive motion commands.
#[derive(Component, Debug, Clone, Default, Reflect)]
#[reflect(Component)]
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
// Instruction to ExecutionPoint Conversion
// ============================================================================

use crate::program::ProgramDefaults;
use crate::types::Instruction as ProgramInstruction;
// Note: ExecutionPoint, MotionCommand, etc. are imported at the top of the file
use fanuc_replica_robotics::FrameId;

/// Convert a legacy program Instruction to an ExecutionPoint for the new orchestrator.
///
/// This handles:
/// - Converting XYZ + optional WPR to RobotPose (quaternion-based)
/// - Extracting speed and blend radius from instruction or defaults
/// - Setting appropriate metadata
///
/// # Arguments
/// * `instruction` - The legacy instruction from the database
/// * `defaults` - Program defaults for missing values
/// * `index` - The 0-based index in the toolpath
/// * `is_last` - Whether this is the last instruction (uses FINE termination)
pub fn instruction_to_execution_point(
    instruction: &ProgramInstruction,
    defaults: &ProgramDefaults,
    index: u32,
    is_last: bool,
) -> ExecutionPoint {
    // Get WPR with fallback to defaults
    let w = instruction.w.unwrap_or(defaults.w);
    let p = instruction.p.unwrap_or(defaults.p);
    let r = instruction.r.unwrap_or(defaults.r);

    // Create robot-agnostic pose from XYZ + WPR
    // Note: Instructions are always in world frame (frame 0)
    let target_pose = RobotPose::from_xyz_wpr(
        instruction.x,
        instruction.y,
        instruction.z,
        w,
        p,
        r,
        FrameId::World,
    );

    // Get speed with fallback to defaults
    let speed = instruction.speed.unwrap_or(defaults.speed) as f32;

    // Determine blend radius based on termination type
    // CNT with value > 0 means continuous motion (blend)
    // FINE or CNT with value 0 means stop at point
    let blend_radius = if is_last {
        // Last instruction always uses FINE (stop at point)
        0.0
    } else {
        // Use instruction's term_value if it's CNT, otherwise use program default
        let term_type = instruction.term_type.as_deref().unwrap_or(&defaults.term_type);
        let term_value = instruction.term_value.unwrap_or(defaults.term_value);

        if term_type.eq_ignore_ascii_case("CNT") && term_value > 0 {
            term_value as f32
        } else {
            0.0
        }
    };

    // Build the motion command
    let motion = MotionCommand {
        speed,
        motion_type: ExecMotionType::Linear, // All instructions are linear for now
        blend_radius,
    };

    // Build metadata with line number for tracking
    let metadata = PointMetadata {
        comment: Some(format!("Line {}", instruction.line_number)),
        ..Default::default()
    };

    ExecutionPoint {
        index,
        target_pose,
        motion,
        aux_commands: HashMap::new(),
        metadata,
    }
}

/// Convert an approach/retreat position to an ExecutionPoint.
///
/// # Arguments
/// * `x`, `y`, `z` - Position coordinates
/// * `w`, `p`, `r` - Orientation (optional, uses defaults if None)
/// * `defaults` - Program defaults for missing values
/// * `index` - The 0-based index in the toolpath
/// * `speed` - Move speed for approach/retreat
/// * `is_last` - Whether this is the last point (uses FINE termination)
pub fn approach_retreat_to_execution_point(
    x: f64,
    y: f64,
    z: f64,
    w: Option<f64>,
    p: Option<f64>,
    r: Option<f64>,
    defaults: &ProgramDefaults,
    index: u32,
    speed: f64,
    is_last: bool,
) -> ExecutionPoint {
    let w = w.unwrap_or(defaults.w);
    let p = p.unwrap_or(defaults.p);
    let r = r.unwrap_or(defaults.r);

    let target_pose = RobotPose::from_xyz_wpr(x, y, z, w, p, r, FrameId::World);

    let blend_radius = if is_last { 0.0 } else { defaults.term_value as f32 };

    let motion = MotionCommand {
        speed: speed as f32,
        motion_type: ExecMotionType::Linear,
        blend_radius,
    };

    let metadata = PointMetadata {
        comment: Some(if is_last {
            "Retreat".to_string()
        } else {
            "Approach".to_string()
        }),
        is_travel: true, // Approach/retreat are travel moves
        ..Default::default()
    };

    ExecutionPoint {
        index,
        target_pose,
        motion,
        aux_commands: HashMap::new(),
        metadata,
    }
}

// ============================================================================
// ExecutionState Sync System (New Execution Architecture)
// ============================================================================

use crate::types::{ExecutionState, ProgramExecutionState};
use fanuc_replica_core::ActiveSystem;
// Note: BufferState is imported at the top of the file

/// Sync BufferState (on System entity) to ExecutionState (on robot entity).
///
/// This system bridges the new execution architecture with the existing client sync.
/// It runs alongside the legacy `update_execution_state` system during migration.
///
/// The system:
/// 1. Reads BufferState from the System entity
/// 2. Maps BufferState to ProgramExecutionState
/// 3. Updates current_line based on BufferState::Executing
/// 4. Computes available actions based on state
/// 5. Updates ExecutionState on robot entity (which is synced to clients)
pub fn sync_buffer_state_to_execution_state(
    system_query: Query<&BufferState, With<ActiveSystem>>,
    mut execution_states: Query<&mut ExecutionState, With<FanucRobot>>,
) {
    // Get BufferState from System entity
    let Ok(buffer_state) = system_query.single() else {
        return; // No BufferState means no new execution system active
    };

    // Map BufferState to ProgramExecutionState
    let (new_state, _current_line, completed_count) = match buffer_state {
        BufferState::Idle => (ProgramExecutionState::Idle, 0, 0),
        BufferState::Buffering { .. } => (ProgramExecutionState::Idle, 0, 0),
        BufferState::Ready => (ProgramExecutionState::Idle, 0, 0),
        BufferState::Executing { current_index, completed_count } => {
            (ProgramExecutionState::Running, *current_index as usize, *completed_count as usize)
        }
        BufferState::Paused { paused_at_index } => {
            (ProgramExecutionState::Paused, *paused_at_index as usize, *paused_at_index as usize)
        }
        BufferState::AwaitingPoints { completed_count } => {
            // Streaming mode: waiting for more points - show as running
            (ProgramExecutionState::Running, *completed_count as usize, *completed_count as usize)
        }
        BufferState::WaitingForFeedback { .. } => {
            // Treat waiting as running for UI purposes
            (ProgramExecutionState::Running, 0, 0)
        }
        BufferState::Complete { total_executed } => {
            (ProgramExecutionState::Completed, *total_executed as usize, *total_executed as usize)
        }
        BufferState::Error { .. } => (ProgramExecutionState::Error, 0, 0),
        BufferState::Stopped { at_index, completed_count } => {
            // Stopped by user - map to Idle for UI (program can be restarted)
            (ProgramExecutionState::Idle, *at_index as usize, *completed_count as usize)
        }
    };

    // Compute available actions based on state machine
    let (can_load, can_start, can_pause, can_resume, can_stop, can_unload) = match new_state {
        ProgramExecutionState::NoProgram => (true, false, false, false, false, false),
        ProgramExecutionState::Idle => (false, true, false, false, false, true),
        ProgramExecutionState::Running => (false, false, true, false, true, false),
        ProgramExecutionState::Paused => (false, false, false, true, true, false),
        ProgramExecutionState::Completed => (false, true, false, false, false, true),
        ProgramExecutionState::Error => (false, true, false, false, false, true),
    };

    // Update ExecutionState on robot entity
    for mut exec_state in execution_states.iter_mut() {
        // Skip if in NoProgram state (unload was just called)
        if exec_state.state == ProgramExecutionState::NoProgram {
            continue;
        }

        // Only update if something changed
        let needs_update =
            exec_state.state != new_state ||
            exec_state.current_line != completed_count ||
            exec_state.can_load != can_load ||
            exec_state.can_start != can_start ||
            exec_state.can_pause != can_pause ||
            exec_state.can_resume != can_resume ||
            exec_state.can_stop != can_stop ||
            exec_state.can_unload != can_unload;

        if needs_update {
            exec_state.state = new_state.clone();
            exec_state.current_line = completed_count;
            exec_state.can_load = can_load;
            exec_state.can_start = can_start;
            exec_state.can_pause = can_pause;
            exec_state.can_resume = can_resume;
            exec_state.can_stop = can_stop;
            exec_state.can_unload = can_unload;
        }
    }
}

use crate::types::{ProgramNotification, ProgramNotificationKind, ConsoleDirection, ConsoleMsgType};
use crate::program::{console_entry, Program, ProgramState};
use pl3xus::Network;
use pl3xus_websockets::WebSocketProvider;
use std::sync::atomic::{AtomicU64, Ordering};

/// Global sequence counter for program notifications (shared with legacy system).
static NOTIFICATION_SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// Create a new ProgramNotification with a unique sequence number.
fn new_notification(kind: ProgramNotificationKind) -> ProgramNotification {
    ProgramNotification {
        sequence: NOTIFICATION_SEQUENCE.fetch_add(1, Ordering::Relaxed),
        kind,
    }
}

/// Sync DeviceStatus changes back to BufferState (and legacy ProgramState).
///
/// This system handles:
/// - Updating BufferState.completed_count from DeviceStatus.completed_count
/// - Transitioning BufferState to Error if device has an error
/// - Transitioning BufferState to Complete when all points are executed
/// - Syncing legacy ProgramState to match BufferState (for "already running" check)
/// - Broadcasting notifications on completion or error
///
/// Note: This runs after fanuc_motion_response_system which updates DeviceStatus.
pub fn sync_device_status_to_buffer_state(
    mut system_query: Query<(&mut BufferState, &ToolpathBuffer, &ExecutionCoordinator, Option<&mut Program>)>,
    device_query: Query<&DeviceStatus, With<PrimaryMotion>>,
    net: Res<Network<WebSocketProvider>>,
) {
    // Get the device status from the primary motion device
    let Ok(device_status) = device_query.single() else {
        return; // No primary motion device
    };

    for (mut buffer_state, toolpath_buffer, coordinator, program_opt) in system_query.iter_mut() {
        // Extract current state info before matching to avoid borrow issues
        let (is_executing, current_index) = match &*buffer_state {
            BufferState::Executing { current_index, .. } => (true, *current_index),
            _ => (false, 0),
        };

        if !is_executing {
            continue;
        }

        // Check for device error
        if let Some(ref error_msg) = device_status.error {
            let error_msg_clone = error_msg.clone();
            let program_name = coordinator.name.clone();

            *buffer_state = BufferState::Error {
                message: error_msg_clone.clone(),
            };
            error!("📛 BufferState -> Error: {}", error_msg_clone);

            // Broadcast error notification
            let notification = new_notification(ProgramNotificationKind::Error {
                program_name: program_name.clone(),
                at_line: current_index as usize,
                error_message: error_msg_clone.clone(),
            });
            net.broadcast(notification);

            let console_msg = console_entry(
                format!("Program '{}' error at line {}: {}", program_name, current_index, error_msg_clone),
                ConsoleDirection::System,
                ConsoleMsgType::Error,
            );
            net.broadcast(console_msg);
            continue;
        }

        // Update completed count from device status
        let new_completed = device_status.completed_count;

        // Check if execution is complete using the sealed buffer pattern
        // is_execution_complete checks: sealed + empty + confirmed >= total_added
        if toolpath_buffer.is_execution_complete(new_completed) {
            let program_name = coordinator.name.clone();

            *buffer_state = BufferState::Complete {
                total_executed: new_completed,
            };
            info!("✅ BufferState -> Complete ({} points)", new_completed);

            // CRITICAL: Also update legacy ProgramState to Completed
            // This ensures StartProgram check (program.state == Running) works correctly
            if let Some(mut program) = program_opt {
                program.state = ProgramState::Completed;
                program.completed_count = new_completed as usize;
                info!("✅ Legacy ProgramState -> Completed");
            }

            // Broadcast completion notification
            let notification = new_notification(ProgramNotificationKind::Completed {
                program_name: program_name.clone(),
                total_instructions: new_completed as usize,
            });
            net.broadcast(notification);

            let console_msg = console_entry(
                format!("Program '{}' completed ({} instructions)", program_name, new_completed),
                ConsoleDirection::System,
                ConsoleMsgType::Status,
            );
            net.broadcast(console_msg);
        } else if new_completed != current_index {
            // Update the state with new counts
            *buffer_state = BufferState::Executing {
                current_index,
                completed_count: new_completed,
            };
        }
        // Note: AwaitingPoints transition for streaming will be added in Phase 3
    }
}
