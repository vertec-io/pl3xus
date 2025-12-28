//! ToolpathBuffer and BufferState components.

use super::ExecutionPoint;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[cfg(feature = "ecs")]
use bevy::prelude::*;

/// The main execution buffer containing toolpath points.
///
/// This is a VecDeque that the orchestrator consumes from.
/// Points are pushed by importers/producers and popped by the orchestrator.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "ecs", derive(Component))]
pub struct ToolpathBuffer {
    /// The queue of points to execute
    points: VecDeque<ExecutionPoint>,

    /// Total number of points expected (set when toolpath is loaded)
    expected_total: u32,
}

impl ToolpathBuffer {
    /// Create a new empty buffer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a buffer with expected capacity.
    pub fn with_expected(expected_total: u32) -> Self {
        Self {
            points: VecDeque::with_capacity(expected_total as usize),
            expected_total,
        }
    }

    /// Push a point to the back of the buffer.
    pub fn push(&mut self, point: ExecutionPoint) {
        self.points.push_back(point);
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

    /// Clear all points from the buffer.
    pub fn clear(&mut self) {
        self.points.clear();
    }

    /// Get the expected total number of points.
    pub fn expected_total(&self) -> u32 {
        self.expected_total
    }

    /// Set the expected total number of points.
    pub fn set_expected_total(&mut self, total: u32) {
        self.expected_total = total;
    }

    /// Extend the buffer with an iterator of points.
    pub fn extend(&mut self, points: impl IntoIterator<Item = ExecutionPoint>) {
        self.points.extend(points);
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
    /// Check if execution is active (Executing or WaitingForFeedback).
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            BufferState::Executing { .. } | BufferState::WaitingForFeedback { .. }
        )
    }

    /// Check if execution can be started (Ready state).
    pub fn can_start(&self) -> bool {
        matches!(self, BufferState::Ready)
    }

    /// Check if execution is complete.
    pub fn is_complete(&self) -> bool {
        matches!(self, BufferState::Complete { .. })
    }

    /// Check if an error occurred.
    pub fn is_error(&self) -> bool {
        matches!(self, BufferState::Error { .. })
    }
}

