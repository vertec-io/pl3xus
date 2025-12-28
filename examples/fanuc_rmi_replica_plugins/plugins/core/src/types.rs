//! Core types - ActiveSystem marker component.

use serde::{Deserialize, Serialize};

#[cfg(feature = "ecs")]
use bevy::prelude::Component;

#[cfg(feature = "stores")]
use reactive_stores::Store;

/// Marker component for the active/current System entity.
///
/// This entity is the control root - clients request control of this entity
/// to gain control over the entire apparatus including all child robots.
/// The System represents the overall application/cell and is the parent
/// entity in the hierarchy.
#[cfg_attr(feature = "ecs", derive(Component))]
#[cfg_attr(feature = "stores", derive(Store))]
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct ActiveSystem;

