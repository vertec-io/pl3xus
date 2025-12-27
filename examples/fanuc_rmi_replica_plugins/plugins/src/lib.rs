//! FANUC RMI Replica Plugins
//!
//! This crate contains all domain logic for the FANUC RMI Replica application.
//! Types are defined with conditional derives based on features:
//!
//! - `ecs`: Enables `Component` derive for Bevy ECS (server-side)
//! - `server`: Enables server-only functionality (driver, database, systems)
//! - `stores`: Enables `Store` derive for reactive stores (client-side)
//!
//! # Architecture
//!
//! The crate is organized into two main plugins:
//! - `core`: Networking, database, ActiveSystem (control root)
//! - `robot`: Robot state, connections, programs, I/O, motion
//!
//! # Usage
//!
//! Server:
//! ```toml
//! fanuc_replica_plugins = { path = "../plugins", features = ["ecs", "server"] }
//! ```
//!
//! Client:
//! ```toml
//! fanuc_replica_plugins = { path = "../plugins", default-features = false, features = ["stores"] }
//! ```

pub mod core;
pub mod robot;

// Re-export core types
pub use core::ActiveSystem;
#[cfg(feature = "server")]
pub use core::{CorePlugin, DatabaseResource, init_database};

// Re-export robot types (all of them)
pub use robot::*;

// Re-export FANUC DTO types (same crate for all features, different feature flags)
pub use fanuc_rmi as fanuc_rmi_types;
pub use fanuc_rmi::dto;
pub use fanuc_rmi::{SpeedType, TermType};

// Re-export pl3xus common types (always available via ecs or stores)
#[cfg(any(feature = "ecs", feature = "stores"))]
pub use pl3xus_common::{RequestMessage, ErrorResponse};

// Server-only: automatic query invalidation macros
#[cfg(feature = "server")]
pub use pl3xus_macros::{Invalidates, HasSuccess};

/// Build the complete Bevy application with all plugins.
///
/// This is the main entry point for the server. It creates a fully configured
/// Bevy App with all domain plugins registered.
///
/// # Example
/// ```rust,ignore
/// fn main() {
///     fanuc_replica_plugins::build().run();
/// }
/// ```
#[cfg(feature = "server")]
pub fn build() -> bevy::app::App {
    use bevy::prelude::*;

    let mut app = App::new();

    // Core plugin: networking, database, ActiveSystem
    app.add_plugins(CorePlugin);

    // Robot plugin: robot state, connections, programs, I/O, motion
    app.add_plugins(RobotPlugin);

    app
}

