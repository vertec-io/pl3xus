//! Coordinator and marker components.

use serde::{Deserialize, Serialize};

#[cfg(feature = "ecs")]
use bevy::prelude::*;

/// Marker component for an entity that coordinates execution.
///
/// An entity with this marker:
/// - Has a ToolpathBuffer and BufferState
/// - Has child entities with ExecutionTarget markers
/// - Is processed by the orchestrator system
///
/// # Example
/// ```rust,ignore
/// commands.spawn((
///     ExecutionCoordinator::new("printer_1"),
///     ToolpathBuffer::new(),
///     BufferState::Idle,
/// )).with_children(|parent| {
///     parent.spawn((
///         FanucDriver::new(config),
///         ExecutionTarget,
///         PrimaryMotion,
///     ));
///     parent.spawn((
///         DuetExtruder::new(config),
///         ExecutionTarget,
///     ));
/// });
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ecs", derive(Component))]
pub struct ExecutionCoordinator {
    /// Unique identifier for this coordinator
    pub id: String,

    /// Human-readable name
    pub name: String,
}

impl ExecutionCoordinator {
    /// Create a new coordinator with the given ID.
    pub fn new(id: impl Into<String>) -> Self {
        let id = id.into();
        Self {
            name: id.clone(),
            id,
        }
    }

    /// Create a coordinator with a custom name.
    pub fn with_name(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
        }
    }
}

/// Marker component for entities that receive execution commands.
///
/// Add this to any entity that should receive commands from the orchestrator.
/// The entity must be a child of an ExecutionCoordinator entity.
///
/// The orchestrator queries for children with this marker and calls the
/// appropriate device trait methods (MotionDevice or AuxiliaryDevice).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ecs", derive(Component))]
pub struct ExecutionTarget;

/// Marker component for the primary motion device.
///
/// Exactly one child of an ExecutionCoordinator should have this marker.
/// The primary motion device:
/// - Controls execution timing (its `ready_for_next()` gates new commands)
/// - Provides motion completion feedback
/// - Is typically a robot arm
///
/// If no entity has this marker, the orchestrator will log an error.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ecs", derive(Component))]
pub struct PrimaryMotion;

/// Marker component for entities that provide feedback.
///
/// Add this to sensors or other entities that provide feedback
/// used by the orchestrator for WaitingForFeedback states.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ecs", derive(Component))]
pub struct FeedbackProvider;

