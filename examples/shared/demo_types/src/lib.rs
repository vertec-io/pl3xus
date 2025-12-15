//! Shared component types for the pl3xus devtools demo.
//!
//! This crate contains component type definitions that are shared between
//! the Bevy server and the Leptos WASM client. These types are:
//! - Serde-compatible for serialization/deserialization
//! - Optionally include Bevy Component trait when "server" feature is enabled
//! - Used by both server and client for type-safe communication

use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use bevy::prelude::*;

/// A simple counter component for demonstration purposes.
#[cfg_attr(feature = "server", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DemoCounter {
    pub value: i32,
}

/// A flag component with a label and enabled state.
#[cfg_attr(feature = "server", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DemoFlag {
    pub label: String,
    pub enabled: bool,
}

/// Serializable representation of an entity's parent.
/// This mirrors Bevy's ChildOf component but is serializable for sync.
#[cfg_attr(feature = "server", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ParentEntity {
    /// The entity bits of the parent entity.
    pub parent_bits: u64,
}

/// Serializable representation of an entity's children.
/// This mirrors Bevy's Children component but is serializable for sync.
#[cfg_attr(feature = "server", derive(Component))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ChildEntities {
    /// The entity bits of all child entities.
    pub children_bits: Vec<u64>,
}
