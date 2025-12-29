//! ToolpathBuffer and BufferState components.

use super::ExecutionPoint;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::Duration;

/// Validation timeout - 30 seconds
pub const VALIDATION_TIMEOUT: Duration = Duration::from_secs(30);

#[cfg(feature = "ecs")]
use bevy::prelude::*;

/// The main execution buffer containing toolpath points.
///
/// This is a VecDeque that the orchestrator consumes from.
/// Points are pushed by importers/producers and popped by the orchestrator.
///
/// ## Static vs Streaming Execution
///
/// - **Static programs**: Use `new_static(total)` - buffer is sealed immediately,
///   total is known upfront, UI can show percentage progress.
/// - **Streaming programs**: Use `new_streaming()` - buffer starts unsealed,
///   call `seal()` when producer is done adding points.
///
/// ## Completion Detection
///
/// Use `is_execution_complete(completed_count)` to check if execution is done.
/// This properly handles both static and streaming modes.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "ecs", derive(Component))]
pub struct ToolpathBuffer {
    /// The queue of points to execute
    points: VecDeque<ExecutionPoint>,

    /// Original points for re-running (only for static programs)
    /// This allows the program to be run multiple times without reloading
    original_points: Option<Vec<ExecutionPoint>>,

    /// Total number of points ever added to this buffer
    total_added: u32,

    /// Expected total for static programs (None for streaming until sealed)
    expected_total: Option<u32>,

    /// True when no more points will be added
    /// - Static programs: true immediately after creation
    /// - Streaming: becomes true when producer calls seal()
    sealed: bool,
}

impl Default for ToolpathBuffer {
    fn default() -> Self {
        Self {
            points: VecDeque::new(),
            original_points: None,
            total_added: 0,
            expected_total: None,
            sealed: false,
        }
    }
}

impl ToolpathBuffer {
    /// Create a new empty buffer (defaults to unsealed/streaming mode).
    ///
    /// For static programs with known total, prefer `new_static()`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a buffer for a static program with known total.
    ///
    /// The buffer is immediately sealed - no need to call `seal()`.
    /// Use this when all points are known upfront (e.g., loaded from database).
    pub fn new_static(expected_total: u32) -> Self {
        Self {
            points: VecDeque::with_capacity(expected_total as usize),
            original_points: Some(Vec::with_capacity(expected_total as usize)),
            total_added: 0,
            expected_total: Some(expected_total),
            sealed: true, // Static = sealed from start
        }
    }

    /// Create a buffer for streaming execution.
    ///
    /// Call `seal()` when the producer is done adding points.
    /// Until sealed, the buffer will transition to AwaitingPoints when empty
    /// instead of Complete.
    pub fn new_streaming() -> Self {
        Self {
            points: VecDeque::new(),
            original_points: None, // Streaming doesn't support re-run
            total_added: 0,
            expected_total: None,
            sealed: false,
        }
    }

    /// Create a buffer with expected capacity (legacy API).
    #[deprecated(since = "0.2.0", note = "Use new_static() for static programs")]
    pub fn with_expected(expected_total: u32) -> Self {
        Self::new_static(expected_total)
    }

    /// Push a point to the back of the buffer.
    ///
    /// Also increments the `total_added` counter and stores in original_points
    /// for static programs (to support re-running).
    pub fn push(&mut self, point: ExecutionPoint) {
        // Store a copy in original_points for re-run support (static programs only)
        if let Some(ref mut originals) = self.original_points {
            originals.push(point.clone());
        }
        self.points.push_back(point);
        self.total_added += 1;
    }

    /// Pop a point from the front of the buffer.
    pub fn pop(&mut self) -> Option<ExecutionPoint> {
        self.points.pop_front()
    }

    /// Peek at the front point without removing it.
    pub fn peek(&self) -> Option<&ExecutionPoint> {
        self.points.front()
    }

    /// Get the number of points currently in the buffer.
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Check if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Clear all points from the buffer (full reset).
    ///
    /// Note: This also resets `total_added` to 0, clears original_points,
    /// and unseals the buffer. Use `reset_for_rerun()` to keep original points.
    pub fn clear(&mut self) {
        self.points.clear();
        self.original_points = None;
        self.total_added = 0;
        self.expected_total = None;
        self.sealed = false;
    }

    /// Reset the buffer for re-running the same program.
    ///
    /// For static programs, this restores all original points to the queue,
    /// allowing the program to be executed again without reloading.
    ///
    /// Returns true if reset was successful, false if no original points available.
    pub fn reset_for_rerun(&mut self) -> bool {
        if let Some(ref originals) = self.original_points {
            self.points.clear();
            self.points.extend(originals.iter().cloned());
            self.total_added = originals.len() as u32;
            // Keep sealed and expected_total as they were
            true
        } else {
            false
        }
    }

    /// Check if this buffer supports re-running (has stored original points).
    pub fn can_rerun(&self) -> bool {
        self.original_points.is_some()
    }

    /// Seal the buffer - no more points will be added.
    ///
    /// Sets `expected_total` to current `total_added` if not already set.
    /// After sealing, when the buffer is empty and all points are confirmed,
    /// execution will complete.
    pub fn seal(&mut self) {
        self.sealed = true;
        if self.expected_total.is_none() {
            self.expected_total = Some(self.total_added);
        }
    }

    /// Check if buffer is sealed (no more points will be added).
    pub fn is_sealed(&self) -> bool {
        self.sealed
    }

    /// Get total number of points ever added to this buffer.
    pub fn total_added(&self) -> u32 {
        self.total_added
    }

    /// Get the expected total number of points.
    ///
    /// Returns `Some(total)` for static programs or after `seal()` is called.
    /// Returns `None` for streaming programs that haven't been sealed yet.
    pub fn expected_total(&self) -> Option<u32> {
        self.expected_total
    }

    /// Check if execution is logically complete.
    ///
    /// Returns true only if:
    /// - Buffer is sealed (no more points coming)
    /// - Buffer is empty (all points dispatched)
    /// - All dispatched points are confirmed complete
    pub fn is_execution_complete(&self, completed_count: u32) -> bool {
        self.sealed && self.points.is_empty() && completed_count >= self.total_added
    }

    /// Check if buffer should transition to AwaitingPoints state.
    ///
    /// Returns true if:
    /// - Buffer is empty (all points dispatched)
    /// - Buffer is NOT sealed (more points expected)
    pub fn should_await_points(&self) -> bool {
        self.points.is_empty() && !self.sealed
    }

    /// Calculate execution progress as a percentage (0.0 to 100.0).
    ///
    /// ## For static programs (sealed with known total):
    /// Returns `Some(percentage)` based on `completed_count / expected_total`
    ///
    /// ## For streaming programs (not sealed):
    /// Returns `None` - percentage is indeterminate since total is unknown
    ///
    /// ## Edge cases:
    /// - Returns `Some(100.0)` if expected_total is 0 (empty program)
    /// - Returns `Some(100.0)` if completed_count >= expected_total
    pub fn progress_percent(&self, completed_count: u32) -> Option<f32> {
        self.expected_total.map(|total| {
            if total == 0 {
                100.0
            } else {
                ((completed_count as f32 / total as f32) * 100.0).min(100.0)
            }
        })
    }

    /// Set the expected total number of points (legacy API).
    #[deprecated(since = "0.2.0", note = "Use new_static() or seal() instead")]
    pub fn set_expected_total(&mut self, total: u32) {
        self.expected_total = Some(total);
        self.sealed = true;
    }

    /// Extend the buffer with an iterator of points.
    ///
    /// Also increments `total_added` for each point added.
    pub fn extend(&mut self, points: impl IntoIterator<Item = ExecutionPoint>) {
        for point in points {
            self.push(point);
        }
    }
}

/// State machine for execution coordination.
///
/// This is a separate component from ToolpathBuffer for clean ECS queries.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ecs", derive(Component))]
pub enum BufferState {
    /// No execution active, buffer empty or not initialized
    #[default]
    Idle,

    /// Receiving points, waiting for minimum buffer threshold
    Buffering {
        /// Minimum points needed before execution can start
        min_threshold: u32,
    },

    /// Minimum buffer reached, ready to start execution
    Ready,

    /// Validating subsystems before execution starts.
    ///
    /// When user clicks "Start", we enter this state and wait for all
    /// subsystems to report Ready. If all ready, transition to Executing.
    /// If any error or timeout, transition to Error.
    ///
    /// Note: started_at is only available on server (uses Instant).
    /// For serialization, we don't include the timestamp.
    Validating,

    /// Actively sending points to devices
    Executing {
        /// Index of the current point being executed
        current_index: u32,
        /// Number of points confirmed complete by motion device
        completed_count: u32,
    },

    /// Execution paused (can resume)
    Paused {
        /// Index we paused at
        paused_at_index: u32,
    },

    /// Buffer empty but not sealed - waiting for more points (streaming mode)
    ///
    /// This state is used during streaming execution when:
    /// - All buffered points have been dispatched
    /// - The producer hasn't called seal() yet
    /// - More points are expected to arrive
    ///
    /// Transitions:
    /// - → Executing: when new points are pushed
    /// - → Complete: when buffer is sealed (after points confirmed)
    AwaitingPoints {
        /// Number of points completed so far
        completed_count: u32,
    },

    /// Waiting for external condition (sensor, feedback, etc.)
    WaitingForFeedback {
        /// Description of what we're waiting for
        reason: WaitReason,
    },

    /// All points executed successfully
    Complete {
        /// Total points that were executed
        total_executed: u32,
    },

    /// Error occurred during execution
    Error {
        /// Error message
        message: String,
    },

    /// Execution stopped by user (not an error, not complete)
    Stopped {
        /// Index at which execution was stopped
        at_index: u32,
        /// Number of points that were completed before stop
        completed_count: u32,
    },
}

/// Reason for waiting during execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WaitReason {
    /// Waiting for sensor value
    SensorThreshold,
    /// Waiting for user confirmation
    UserConfirmation,
    /// Waiting for device to become ready
    DeviceReady,
    /// Waiting for refill operation
    Refill,
}

impl BufferState {
    /// Check if execution is active (Executing, AwaitingPoints, or WaitingForFeedback).
    ///
    /// Note: AwaitingPoints is considered active because execution hasn't finished,
    /// we're just temporarily out of points in streaming mode.
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            BufferState::Executing { .. }
                | BufferState::AwaitingPoints { .. }
                | BufferState::WaitingForFeedback { .. }
        )
    }

    /// Check if execution can be started (Ready state).
    pub fn can_start(&self) -> bool {
        matches!(self, BufferState::Ready)
    }

    /// Check if awaiting more points in streaming mode.
    pub fn is_awaiting_points(&self) -> bool {
        matches!(self, BufferState::AwaitingPoints { .. })
    }

    /// Check if execution is complete.
    pub fn is_complete(&self) -> bool {
        matches!(self, BufferState::Complete { .. })
    }

    /// Check if an error occurred.
    pub fn is_error(&self) -> bool {
        matches!(self, BufferState::Error { .. })
    }

    /// Check if execution was stopped by user.
    pub fn is_stopped(&self) -> bool {
        matches!(self, BufferState::Stopped { .. })
    }

    /// Check if currently validating subsystems.
    pub fn is_validating(&self) -> bool {
        matches!(self, BufferState::Validating)
    }

    /// Check if execution is in a terminal state (Complete, Error, or Stopped).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            BufferState::Complete { .. } | BufferState::Error { .. } | BufferState::Stopped { .. }
        )
    }

    /// Get the current completed count from the state, if available.
    pub fn completed_count(&self) -> Option<u32> {
        match self {
            BufferState::Executing { completed_count, .. } => Some(*completed_count),
            BufferState::AwaitingPoints { completed_count } => Some(*completed_count),
            BufferState::Complete { total_executed } => Some(*total_executed),
            BufferState::Stopped { completed_count, .. } => Some(*completed_count),
            _ => None,
        }
    }

    /// Convert to SystemState for the ExecutionState component.
    ///
    /// Maps internal buffer states to the SystemState enum used by
    /// the ExecutionState component that is synced to clients.
    pub fn to_system_state(&self) -> super::SystemState {
        use super::SystemState;
        match self {
            BufferState::Idle => SystemState::NoSource,
            BufferState::Buffering { .. } => SystemState::Ready,
            BufferState::Ready => SystemState::Ready,
            BufferState::Validating => SystemState::Validating,
            BufferState::Executing { .. } => SystemState::Running,
            BufferState::Paused { .. } => SystemState::Paused,
            BufferState::AwaitingPoints { .. } => SystemState::AwaitingPoints,
            BufferState::WaitingForFeedback { .. } => SystemState::Running,
            BufferState::Complete { .. } => SystemState::Completed,
            BufferState::Error { .. } => SystemState::Error,
            BufferState::Stopped { .. } => SystemState::Stopped,
        }
    }

    /// Compute available UI actions based on current state.
    ///
    /// Returns flags for what actions are available in the current state.
    /// Note: can_load is always false from BufferState because that depends on
    /// whether a program is loaded, which BufferState doesn't track.
    pub fn available_actions(&self) -> UiActions {
        use super::SystemState;
        match self.to_system_state() {
            SystemState::NoSource => UiActions {
                can_load: true,
                can_start: false,
                can_pause: false,
                can_resume: false,
                can_stop: false,
                can_unload: false,
            },
            SystemState::Ready => UiActions {
                can_load: false,
                can_start: true,
                can_pause: false,
                can_resume: false,
                can_stop: false,
                can_unload: true,
            },
            SystemState::Validating => UiActions {
                can_load: false,
                can_start: false,
                can_pause: false,
                can_resume: false,
                can_stop: true, // Can cancel validation
                can_unload: false,
            },
            SystemState::Running | SystemState::AwaitingPoints => UiActions {
                can_load: false,
                can_start: false,
                can_pause: true,
                can_resume: false,
                can_stop: true,
                can_unload: false,
            },
            SystemState::Paused => UiActions {
                can_load: false,
                can_start: false,
                can_pause: false,
                can_resume: true,
                can_stop: true,
                can_unload: false,
            },
            SystemState::Completed | SystemState::Stopped => UiActions {
                can_load: false,
                can_start: true, // Can restart
                can_pause: false,
                can_resume: false,
                can_stop: false,
                can_unload: true,
            },
            SystemState::Error => UiActions {
                can_load: false,
                can_start: true,
                can_pause: false,
                can_resume: false,
                can_stop: false,
                can_unload: true,
            },
        }
    }
}

/// Available UI actions based on execution state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UiActions {
    pub can_load: bool,
    pub can_start: bool,
    pub can_pause: bool,
    pub can_resume: bool,
    pub can_stop: bool,
    pub can_unload: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::execution_point::{ExecutionPoint, MotionCommand, MotionType};
    use fanuc_replica_robotics::{FrameId, RobotPose};

    fn make_test_point(index: u32) -> ExecutionPoint {
        ExecutionPoint {
            index,
            target_pose: RobotPose::from_translation(0.0, 0.0, 0.0, FrameId::World),
            motion: MotionCommand {
                motion_type: MotionType::Linear,
                speed: 100.0,
                blend_radius: 0.0,
            },
            aux_commands: Default::default(),
            metadata: Default::default(),
        }
    }

    #[test]
    fn test_new_static_is_sealed() {
        let buffer = ToolpathBuffer::new_static(10);
        assert!(buffer.is_sealed());
        assert_eq!(buffer.expected_total(), Some(10));
        assert_eq!(buffer.total_added(), 0);
    }

    #[test]
    fn test_new_streaming_is_not_sealed() {
        let buffer = ToolpathBuffer::new_streaming();
        assert!(!buffer.is_sealed());
        assert_eq!(buffer.expected_total(), None);
        assert_eq!(buffer.total_added(), 0);
    }

    #[test]
    fn test_seal_sets_expected_total() {
        let mut buffer = ToolpathBuffer::new_streaming();
        buffer.push(make_test_point(0));
        buffer.push(make_test_point(1));
        buffer.push(make_test_point(2));

        assert_eq!(buffer.total_added(), 3);
        assert_eq!(buffer.expected_total(), None);

        buffer.seal();

        assert!(buffer.is_sealed());
        assert_eq!(buffer.expected_total(), Some(3));
    }

    #[test]
    fn test_is_execution_complete_static() {
        let mut buffer = ToolpathBuffer::new_static(3);
        buffer.push(make_test_point(0));
        buffer.push(make_test_point(1));
        buffer.push(make_test_point(2));

        // Not complete - buffer not empty
        assert!(!buffer.is_execution_complete(0));

        // Pop all points
        buffer.pop();
        buffer.pop();
        buffer.pop();

        // Not complete - only 2 confirmed
        assert!(!buffer.is_execution_complete(2));

        // Complete - all 3 confirmed
        assert!(buffer.is_execution_complete(3));
    }

    #[test]
    fn test_is_execution_complete_streaming() {
        let mut buffer = ToolpathBuffer::new_streaming();
        buffer.push(make_test_point(0));
        buffer.push(make_test_point(1));

        // Pop all points
        buffer.pop();
        buffer.pop();

        // Not complete - not sealed
        assert!(!buffer.is_execution_complete(2));

        // Seal the buffer
        buffer.seal();

        // Now complete
        assert!(buffer.is_execution_complete(2));
    }

    #[test]
    fn test_push_increments_total_added() {
        let mut buffer = ToolpathBuffer::new();
        assert_eq!(buffer.total_added(), 0);

        buffer.push(make_test_point(0));
        assert_eq!(buffer.total_added(), 1);

        buffer.push(make_test_point(1));
        assert_eq!(buffer.total_added(), 2);

        // Pop doesn't change total_added
        buffer.pop();
        assert_eq!(buffer.total_added(), 2);
    }

    #[test]
    fn test_clear_resets_all() {
        let mut buffer = ToolpathBuffer::new_static(5);
        buffer.push(make_test_point(0));
        buffer.push(make_test_point(1));

        assert!(buffer.is_sealed());
        assert_eq!(buffer.total_added(), 2);
        assert_eq!(buffer.expected_total(), Some(5));

        buffer.clear();

        assert!(!buffer.is_sealed());
        assert_eq!(buffer.total_added(), 0);
        assert_eq!(buffer.expected_total(), None);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_extend_increments_total_added() {
        let mut buffer = ToolpathBuffer::new_streaming();
        let points = vec![make_test_point(0), make_test_point(1), make_test_point(2)];

        buffer.extend(points);

        assert_eq!(buffer.total_added(), 3);
        assert_eq!(buffer.len(), 3);
    }

    #[test]
    fn test_should_await_points() {
        let mut buffer = ToolpathBuffer::new_streaming();
        buffer.push(make_test_point(0));

        // Not awaiting - buffer has points
        assert!(!buffer.should_await_points());

        buffer.pop();

        // Should await - empty but not sealed
        assert!(buffer.should_await_points());

        buffer.seal();

        // Not awaiting - sealed
        assert!(!buffer.should_await_points());
    }

    #[test]
    fn test_progress_percent_static() {
        let mut buffer = ToolpathBuffer::new_static(10);
        for i in 0..10 {
            buffer.push(make_test_point(i));
        }

        // 0% complete
        assert_eq!(buffer.progress_percent(0), Some(0.0));

        // 50% complete
        assert_eq!(buffer.progress_percent(5), Some(50.0));

        // 100% complete
        assert_eq!(buffer.progress_percent(10), Some(100.0));

        // Capped at 100%
        assert_eq!(buffer.progress_percent(15), Some(100.0));
    }

    #[test]
    fn test_progress_percent_streaming() {
        let buffer = ToolpathBuffer::new_streaming();

        // Streaming - no percentage until sealed
        assert_eq!(buffer.progress_percent(0), None);
        assert_eq!(buffer.progress_percent(5), None);
    }

    #[test]
    fn test_progress_percent_empty_program() {
        let buffer = ToolpathBuffer::new_static(0);

        // Empty program = 100% complete
        assert_eq!(buffer.progress_percent(0), Some(100.0));
    }

    #[test]
    fn test_buffer_state_awaiting_points() {
        let state = BufferState::AwaitingPoints { completed_count: 5 };

        assert!(state.is_active());
        assert!(state.is_awaiting_points());
        assert!(!state.is_complete());
        assert!(!state.is_terminal());
        assert_eq!(state.completed_count(), Some(5));
    }

    #[test]
    fn test_buffer_state_completed_count() {
        assert_eq!(BufferState::Idle.completed_count(), None);
        assert_eq!(BufferState::Ready.completed_count(), None);
        assert_eq!(
            BufferState::Executing { current_index: 3, completed_count: 2 }.completed_count(),
            Some(2)
        );
        assert_eq!(
            BufferState::AwaitingPoints { completed_count: 5 }.completed_count(),
            Some(5)
        );
        assert_eq!(
            BufferState::Complete { total_executed: 10 }.completed_count(),
            Some(10)
        );
        assert_eq!(
            BufferState::Stopped { at_index: 5, completed_count: 4 }.completed_count(),
            Some(4)
        );
    }
}
