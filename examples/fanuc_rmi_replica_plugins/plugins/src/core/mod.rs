//! Core plugin - provides foundational infrastructure for the application.
//!
//! This plugin handles:
//! - pl3xus networking setup
//! - Database connection and resource
//! - ActiveSystem entity (control root)
//! - Core synced components

pub mod types;

#[cfg(feature = "server")]
mod plugin;
#[cfg(feature = "server")]
pub mod database;
#[cfg(feature = "server")]
mod systems;

// Re-export types for convenience
pub use types::*;

// Server-only exports
#[cfg(feature = "server")]
pub use plugin::CorePlugin;
#[cfg(feature = "server")]
pub use database::{DatabaseResource, init_database};

