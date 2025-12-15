//! Shared types for pl3xus_client basic example
//!
//! This crate defines component types that are shared between the Bevy server
//! and the Leptos web client.

use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use bevy::prelude::*;

#[cfg(feature = "stores")]
use reactive_stores::Store;

// SyncComponent is automatically implemented for all Serialize + Deserialize types.
// No manual implementation needed!

/// 2D position component
#[cfg_attr(feature = "server", derive(Component))]
#[cfg_attr(feature = "stores", derive(Store))]
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

/// 2D velocity component
#[cfg_attr(feature = "server", derive(Component))]
#[cfg_attr(feature = "stores", derive(Store))]
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Velocity {
    pub x: f32,
    pub y: f32,
}

/// Name component
#[cfg_attr(feature = "server", derive(Component))]
#[cfg_attr(feature = "stores", derive(Store))]
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct EntityName {
    pub name: String,
}

