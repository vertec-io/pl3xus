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
///
/// ## Capacity-Based In-Flight Queue
///
/// For continuous motion (like FANUC), we need to keep multiple commands
/// in-flight simultaneously. The controller needs ~5 points ahead to
/// calculate smooth blending/lookahead. This is modeled with:
/// - `in_flight_capacity`: Maximum commands that can be in-flight (device-specific)
/// - `in_flight_count`: Current number of sent-but-not-confirmed commands
///
/// The orchestrator can send commands while `ready_for_next()` returns true.
#[derive(Component, Debug, Clone)]
pub struct DeviceStatus {
    /// True if the device is connected and operational
    pub is_connected: bool,

    /// Maximum number of commands that can be in-flight simultaneously.
    /// For FANUC continuous motion: typically 5-10.
    /// For simple devices (e.g., Duet extruder): typically 1.
    pub in_flight_capacity: u32,

    /// Current number of commands in-flight (sent but not confirmed complete).
    /// Incremented when command is sent, decremented when completion confirmed.
    pub in_flight_count: u32,

    /// Number of motions confirmed complete (monotonically increasing)
    pub completed_count: u32,

    /// Error message if device is in error state
    pub error: Option<String>,
}

impl Default for DeviceStatus {
    fn default() -> Self {
        Self {
            is_connected: false,
            in_flight_capacity: 1, // Conservative default
            in_flight_count: 0,
            completed_count: 0,
            error: None,
        }
    }
}

impl DeviceStatus {
    /// Create a new DeviceStatus with specified capacity.
    pub fn with_capacity(capacity: u32) -> Self {
        Self {
            is_connected: false,
            in_flight_capacity: capacity,
            in_flight_count: 0,
            completed_count: 0,
            error: None,
        }
    }

    /// Check if device can accept another command.
    ///
    /// Returns true if:
    /// - Device is connected
    /// - No error state
    /// - In-flight count is below capacity
    pub fn ready_for_next(&self) -> bool {
        self.is_connected && self.error.is_none() && self.in_flight_count < self.in_flight_capacity
    }

    /// Check if queue needs more commands for smooth motion.
    ///
    /// For continuous motion, we want to keep the queue filled to a minimum
    /// depth. This helps maintain smooth motion by giving the controller
    /// lookahead data.
    pub fn queue_needs_fill(&self, min_queue_depth: u32) -> bool {
        self.ready_for_next() && self.in_flight_count < min_queue_depth
    }

    /// Record that a command was sent.
    pub fn command_sent(&mut self) {
        self.in_flight_count += 1;
    }

    /// Record that a command completed.
    pub fn command_completed(&mut self) {
        if self.in_flight_count > 0 {
            self.in_flight_count -= 1;
        }
        self.completed_count += 1;
    }

    /// Reset in-flight tracking (e.g., on stop or error).
    pub fn reset_in_flight(&mut self) {
        self.in_flight_count = 0;
    }
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

/// Maximum commands to dispatch in a single tick.
///
/// This prevents the orchestrator from blocking too long in one frame
/// when filling a large in-flight queue.
const MAX_DISPATCH_PER_TICK: u32 = 10;

/// Main orchestrator system - dispatches commands to devices via events.
///
/// For each coordinator in Executing state:
/// 1. Find the primary motion device (child with PrimaryMotion + DeviceStatus)
/// 2. Loop while device can accept commands (up to burst limit)
/// 3. Pop points from buffer and send MotionCommandEvent
/// 4. Send AuxiliaryCommandEvent for each auxiliary device
/// 5. Update buffer state
///
/// ## In-Flight Queue Filling
///
/// This system loops while `ready_for_next()` returns true, allowing multiple
/// commands to be dispatched in a single tick. This fills the device's
/// in-flight queue, which is critical for smooth continuous motion.
pub fn orchestrator_system(
    mut coordinator_query: Query<
        (Entity, &mut BufferState, &mut ToolpathBuffer),
        With<ExecutionCoordinator>,
    >,
    children_query: Query<&Children>,
    mut device_status_query: Query<
        (Entity, &mut DeviceStatus, Option<&DeviceType>),
        With<PrimaryMotion>,
    >,
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
        let mut motion_entity: Option<Entity> = None;
        for child in children.iter() {
            if device_status_query.get(child).is_ok() {
                motion_entity = Some(child);
                break;
            }
        }

        let Some(motion_entity) = motion_entity else {
            warn!(
                "Coordinator {:?} has no PrimaryMotion device with DeviceStatus",
                coordinator_entity
            );
            continue;
        };

        // Dispatch loop - fill in-flight queue up to capacity
        let mut dispatched = 0u32;
        let mut last_point_index = 0u32;

        loop {
            // Check burst limit
            if dispatched >= MAX_DISPATCH_PER_TICK {
                trace!(
                    "Hit dispatch limit ({} commands), continuing next tick",
                    dispatched
                );
                break;
            }

            // Get current device status (must re-query since we mutate it)
            let Ok((_, mut motion_status, _)) = device_status_query.get_mut(motion_entity) else {
                break;
            };

            // Check if motion device can accept more commands
            if !motion_status.ready_for_next() {
                if dispatched == 0 {
                    trace!(
                        "Motion device not ready (in_flight: {}/{})",
                        motion_status.in_flight_count,
                        motion_status.in_flight_capacity
                    );
                }
                break;
            }

            // Pop the next point from the buffer
            let Some(point) = buffer.pop() else {
                // Buffer empty, waiting for more points or completion
                break;
            };

            last_point_index = point.index;

            // Mark command as sent (increment in-flight count)
            motion_status.command_sent();

            // Send motion command event
            motion_events.write(MotionCommandEvent {
                coordinator: coordinator_entity,
                device: motion_entity,
                target_pose: point.target_pose.clone(),
                motion: point.motion.clone(),
                point: point.clone(),
            });

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

            dispatched += 1;
        }

        // Update state with new current index if we dispatched anything
        if dispatched > 0 {
            trace!("Dispatched {} commands, last index {}", dispatched, last_point_index);
            *state = BufferState::Executing {
                current_index: last_point_index,
                completed_count,
            };
        }
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

