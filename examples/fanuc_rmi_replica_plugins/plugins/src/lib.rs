//! FANUC RMI Replica Plugins
//!
//! This crate re-exports types and plugins from the modular plugin crates:
//! - `fanuc_replica_core`: Networking, database, ActiveSystem (control root)
//! - `fanuc_replica_fanuc`: FANUC robot state, connections, programs, I/O, motion
//! - `fanuc_replica_execution`: Toolpath execution orchestration
//! - `fanuc_replica_duet`: Duet extruder support (optional)
//!
//! Types are defined with conditional derives based on features:
//!
//! - `ecs`: Enables `Component` derive for Bevy ECS (server-side)
//! - `server`: Enables server-only functionality (driver, database, systems)
//! - `stores`: Enables `Store` derive for reactive stores (client-side)
//! - `duet`: Enables Duet extruder plugin (requires server)
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

use cfg_if::cfg_if;

// Re-export FANUC DTO types (always available)
pub use fanuc_rmi as fanuc_rmi_types;
pub use fanuc_rmi::dto;
pub use fanuc_rmi::{SpeedType, TermType};

// Feature-gated re-exports using cfg_if for cleaner organization
cfg_if! {
    if #[cfg(feature = "server")] {
        // Server feature includes everything from ECS plus server-specific types

        // Core plugin exports
        pub use fanuc_replica_core::{
            ActiveSystem, CorePlugin, DatabaseResource, DatabaseInit,
            DatabaseInitRegistry, init_database, PluginSchedule,
        };

        // FANUC plugin exports (all types + plugin)
        pub use fanuc_replica_fanuc::*;

        // Execution plugin exports
        pub use fanuc_replica_execution::{
            ExecutionPlugin, ExecutionCoordinator, ExecutionTarget, ExecutionPoint,
            ToolpathBuffer, BufferState, MotionCommand, MotionType, PointMetadata,
            PrimaryMotion, MotionDevice, AuxiliaryDevice, AuxiliaryCommand, DeviceError,
            MotionCommandEvent, AuxiliaryCommandEvent, DeviceStatus, DeviceType,
        };

        // Common types
        pub use pl3xus_common::{RequestMessage, ErrorResponse};

        // Server-only: automatic query invalidation macros
        pub use pl3xus_macros::{Invalidates, HasSuccess};
    } else if #[cfg(feature = "ecs")] {
        // ECS-only (no server): types for testing/shared code

        // Core types
        pub use fanuc_replica_core::ActiveSystem;

        // FANUC types
        pub use fanuc_replica_fanuc::*;

        // Common types
        pub use pl3xus_common::{RequestMessage, ErrorResponse};
    }
}

cfg_if! {
    if #[cfg(feature = "duet")] {
        // Duet plugin (requires server feature)
        pub use fanuc_replica_duet::DuetPlugin;
    }
}

cfg_if! {
    if #[cfg(feature = "stores")] {
        // Stores feature: re-export types with Store derives for client-side reactivity
        pub use fanuc_replica_fanuc::*;
        pub use pl3xus_common::{RequestMessage, ErrorResponse};
    }
}

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
    use fanuc_replica_core::CorePlugin;
    use fanuc_replica_fanuc::FanucPlugin;
    use fanuc_replica_execution::ExecutionPlugin;

    let mut app = App::new();

    // Core plugin: networking, database, ActiveSystem
    app.add_plugins(CorePlugin);

    // Execution plugin: toolpath orchestration (must come before FanucPlugin)
    app.add_plugins(ExecutionPlugin);

    // FANUC plugin: robot state, connections, programs, I/O, motion
    app.add_plugins(FanucPlugin);

    // Duet plugin: extruder support (optional)
    cfg_if! {
        if #[cfg(feature = "duet")] {
            app.add_plugins(fanuc_replica_duet::DuetPlugin);
        }
    }

    app
}

