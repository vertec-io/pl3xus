//! Orchestrator system - the main execution loop.
//!
//! # Architecture Note
//!
//! Bevy ECS doesn't support querying trait objects directly. Instead, device
//! plugins register their systems that listen for `MotionCommandEvent` and
//! `AuxiliaryCommandEvent` events. The orchestrator emits these events, and
//! device-specific systems handle them.
//!
//! This creates a clean decoupling where:
//! - The orchestrator knows nothing about specific device implementations
//! - Device plugins listen for events addressed to them
//! - Device plugins update their `DeviceStatus` component for feedback

use bevy::prelude::*;

use crate::components::{
    BufferState, ExecutionCoordinator, ExecutionPoint, MotionCommand, PrimaryMotion,
    ToolpathBuffer,
};
use crate::traits::AuxiliaryCommand;
use fanuc_replica_robotics::RobotPose;

/// Event sent when the orchestrator needs to dispatch a motion command.
///
/// Device plugins with a `PrimaryMotion` marker should listen for this
/// and check if the `coordinator` entity matches their parent.
#[derive(bevy::prelude::Message, Debug, Clone)]
pub struct MotionCommandEvent {
    /// The coordinator entity that owns this execution
    pub coordinator: Entity,
    /// The motion device entity (child with PrimaryMotion marker)
    pub device: Entity,
    /// Target pose (quaternion-based, device converts to native format)
    pub target_pose: RobotPose,
    /// Motion parameters
    pub motion: MotionCommand,
    /// Full execution point for additional context
    pub point: ExecutionPoint,
}

/// Event sent when the orchestrator needs to dispatch an auxiliary command.
///
/// Device plugins should listen for this and check if the `device` entity
/// matches one they manage.
#[derive(bevy::prelude::Message, Debug, Clone)]
pub struct AuxiliaryCommandEvent {
    /// The coordinator entity that owns this execution
    pub coordinator: Entity,
    /// The auxiliary device entity
    pub device: Entity,
    /// The device type string (for routing)
    pub device_type: String,
    /// The command to execute
    pub command: AuxiliaryCommand,
    /// Point index for tracking
    pub point_index: u32,
}

/// Component added to motion devices to report their status.
///
/// Device plugins update this component, and the orchestrator reads it
/// to determine if the device is ready for the next command.
#[derive(Component, Debug, Clone, Default)]
pub struct DeviceStatus {
    /// True if the device is connected and operational
    pub is_connected: bool,
    /// True if the device can accept another command
    pub ready_for_next: bool,
    /// Number of motions confirmed complete (monotonically increasing)
    pub completed_count: u32,
    /// Error message if device is in error state
    pub error: Option<String>,
}

/// System that updates buffer state based on buffer contents.
///
/// Handles transitions:
/// - Idle + points received → Buffering
/// - Buffering + min threshold reached → Ready
/// - Executing + buffer empty + all complete → Complete
pub fn update_buffer_state_system(
    mut query: Query<(&mut BufferState, &ToolpathBuffer), With<ExecutionCoordinator>>,
) {
    for (mut state, buffer) in query.iter_mut() {
        match &*state {
            BufferState::Idle => {
                // Transition to Buffering when points are received
                if !buffer.is_empty() {
                    *state = BufferState::Buffering { min_threshold: 3 };
                    info!("Execution: Idle → Buffering");
                }
            }

            BufferState::Buffering { min_threshold } => {
                // Transition to Ready when minimum threshold is reached
                if buffer.len() >= *min_threshold as usize {
                    *state = BufferState::Ready;
                    info!("Execution: Buffering → Ready ({} points)", buffer.len());
                }
            }

            BufferState::Executing { completed_count, .. } => {
                // Transition to Complete when all points are executed
                // Uses is_execution_complete() which checks: sealed + empty + confirmed
                let count = *completed_count;
                if buffer.is_execution_complete(count) {
                    *state = BufferState::Complete {
                        total_executed: count,
                    };
                    info!("Execution: Executing → Complete ({} points)", count);
                }
                // Note: AwaitingPoints transition is handled in sync_device_status_to_buffer_state
                // when buffer is empty but not sealed (streaming mode)
            }

            // Other states don't auto-transition
            _ => {}
        }
    }
}

/// Main orchestrator system - dispatches commands to devices via events.
///
/// For each coordinator in Executing state:
/// 1. Find the primary motion device (child with PrimaryMotion + DeviceStatus)
/// 2. Check if it's ready for the next command
/// 3. Pop a point from the buffer
/// 4. Send MotionCommandEvent for the primary device
/// 5. Send AuxiliaryCommandEvent for each auxiliary device
/// 6. Update buffer state
pub fn orchestrator_system(
    mut coordinator_query: Query<
        (Entity, &mut BufferState, &mut ToolpathBuffer),
        With<ExecutionCoordinator>,
    >,
    children_query: Query<&Children>,
    device_status_query: Query<(Entity, &DeviceStatus, Option<&DeviceType>), With<PrimaryMotion>>,
    aux_device_query: Query<(Entity, &DeviceType), Without<PrimaryMotion>>,
    mut motion_events: MessageWriter<MotionCommandEvent>,
    mut aux_events: MessageWriter<AuxiliaryCommandEvent>,
) {
    for (coordinator_entity, mut state, mut buffer) in coordinator_query.iter_mut() {
        // Only process coordinators in Executing state
        let completed_count = match &*state {
            BufferState::Executing { completed_count, .. } => *completed_count,
            _ => continue,
        };

        // Get children of this coordinator
        let Ok(children) = children_query.get(coordinator_entity) else {
            warn!(
                "Coordinator {:?} has no children (no devices attached)",
                coordinator_entity
            );
            continue;
        };

        // Find the primary motion device among children
        let mut primary_device: Option<(Entity, &DeviceStatus)> = None;
        for child in children.iter() {
            if let Ok((entity, status, _)) = device_status_query.get(child) {
                primary_device = Some((entity, status));
                break;
            }
        }

        let Some((motion_entity, motion_status)) = primary_device else {
            warn!(
                "Coordinator {:?} has no PrimaryMotion device with DeviceStatus",
                coordinator_entity
            );
            continue;
        };

        // Check if motion device is ready
        if !motion_status.ready_for_next {
            trace!("Motion device not ready, waiting...");
            continue;
        }

        // Pop the next point from the buffer
        let Some(point) = buffer.pop() else {
            // Buffer empty, waiting for completion feedback
            continue;
        };

        // Send motion command event
        motion_events.write(MotionCommandEvent {
            coordinator: coordinator_entity,
            device: motion_entity,
            target_pose: point.target_pose.clone(),
            motion: point.motion.clone(),
            point: point.clone(),
        });

        trace!("Sent motion event for point {}", point.index);

        // Send auxiliary command events
        for child in children.iter() {
            if let Ok((aux_entity, device_type)) = aux_device_query.get(child) {
                if let Some(cmd) = point.aux_commands.get(&device_type.0) {
                    aux_events.write(AuxiliaryCommandEvent {
                        coordinator: coordinator_entity,
                        device: aux_entity,
                        device_type: device_type.0.clone(),
                        command: cmd.clone(),
                        point_index: point.index,
                    });
                }
            }
        }

        // Update state with new current index
        *state = BufferState::Executing {
            current_index: point.index,
            completed_count,
        };
    }
}

/// Component that identifies a device type for command routing.
///
/// Added to devices so the orchestrator can match them with aux_commands.
#[derive(Component, Debug, Clone)]
pub struct DeviceType(pub String);

impl DeviceType {
    pub fn new(device_type: impl Into<String>) -> Self {
        Self(device_type.into())
    }
}

