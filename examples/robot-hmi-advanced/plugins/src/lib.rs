//! FANUC RMI Replica Plugins
//!
//! This crate provides namespaced access to all plugin crates:
//!
//! - `robot_hmi_plugins::core` - Networking, database, ActiveSystem (control root)
//! - `robot_hmi_plugins::fanuc` - FANUC robot state, connections, I/O, motion
//! - `robot_hmi_plugins::programs` - Device-agnostic program management
//! - `robot_hmi_plugins::execution` - Toolpath execution orchestration
//! - `robot_hmi_plugins::duet` - Duet extruder support (optional)
//!
//! # Usage
//!
//! Import types from their source plugin for clarity:
//! ```rust,ignore
//! use robot_hmi_plugins::fanuc::types::{JogCommand, ConnectionState};
//! use robot_hmi_plugins::programs::types::{ProgramInfo, ListPrograms};
//! use robot_hmi_plugins::execution::types::{Start, Stop};
//! ```
//!
//! # Feature Flags
//!
//! - `ecs`: Enables `Component` derive for Bevy ECS (server-side)
//! - `server`: Enables server-only functionality (driver, database, systems)
//! - `stores`: Enables `Store` derive for reactive stores (client-side)
//! - `duet`: Enables Duet extruder plugin (requires server)
//!
//! # Cargo.toml
//!
//! Server:
//! ```toml
//! robot_hmi_plugins = { path = "../plugins", features = ["ecs", "server"] }
//! ```
//!
//! Client:
//! ```toml
//! robot_hmi_plugins = { path = "../plugins", default-features = false, features = ["stores"] }
//! ```

use cfg_if::cfg_if;

// =============================================================================
// Plugin module re-exports (namespaced access)
// =============================================================================
//
// Each plugin is re-exported as a module, preserving its internal structure.
// Access types via: robot_hmi_plugins::<plugin>::types::<Type>
//
// Example:
//   use robot_hmi_plugins::fanuc::types::ConnectionState;
//   use robot_hmi_plugins::programs::types::ProgramInfo;

/// Core plugin: networking, database, ActiveSystem
pub use robot_hmi_core as core;

/// FANUC plugin: robot state, connections, I/O, motion, jog commands
pub use robot_hmi_fanuc as fanuc;

/// Programs plugin: device-agnostic program storage and CRUD
pub use robot_hmi_programs as programs;

/// Execution plugin: toolpath execution orchestration
pub use robot_hmi_execution as execution;

/// Robotics plugin: robot-agnostic coordinate types
pub use robot_hmi_robotics as robotics;

// =============================================================================
// Convenience re-exports for common external dependencies
// =============================================================================

/// FANUC RMI protocol types (DTO layer)
pub use fanuc_rmi as fanuc_rmi_types;

/// FANUC RMI DTO types
pub use fanuc_rmi::dto;

/// Common speed and termination types
pub use fanuc_rmi::{SpeedType, TermType};

/// Common request/response types
pub use pl3xus_common::{RequestMessage, ErrorResponse};

// Feature-gated re-exports - only for types that require feature-specific derives/deps
cfg_if! {
    if #[cfg(feature = "server")] {
        // Re-export commonly used server infrastructure
        pub use robot_hmi_core::{CorePlugin, init_database, PluginSchedule};
        pub use robot_hmi_core::database::{DatabaseResource, DatabaseInit, DatabaseInitRegistry};
        pub use robot_hmi_core::types::ActiveSystem;

        // Plugin types for app.add_plugins()
        pub use robot_hmi_fanuc::FanucPlugin;
        pub use robot_hmi_programs::ProgramsPlugin;
        pub use robot_hmi_execution::ExecutionPlugin;

        // Macro re-exports
        pub use pl3xus_macros::{Invalidates, HasSuccess};
    } else if #[cfg(feature = "ecs")] {
        // ECS-only (no server): types for testing/shared code

        // Core types
        pub use robot_hmi_core::types::ActiveSystem;

        // FANUC types
        pub use robot_hmi_fanuc::*;
    }
}

cfg_if! {
    if #[cfg(feature = "duet")] {
        // Duet plugin (requires server feature)
        pub use robot_hmi_duet::DuetPlugin;
    }
}



cfg_if! {
    if #[cfg(all(feature = "stores", not(feature = "server"), not(feature = "ecs")))] {
        // Stores feature (standalone): types with Store derives for client-side reactivity
        // Base types are already exported above - this adds Store-derived component types

        // Core types with Store derives
        pub use robot_hmi_core::types::{
            ActiveSystem,
            ConsoleLogEntry, ConsoleDirection, ConsoleMsgType, console_entry,
        };

        // FANUC types with Store derives
        pub use robot_hmi_fanuc::*;
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
///     robot_hmi_plugins::build().run();
/// }
/// ```
#[cfg(feature = "server")]
pub fn build() -> bevy::app::App {
    use bevy::prelude::*;
    use robot_hmi_core::CorePlugin;
    use robot_hmi_fanuc::FanucPlugin;
    use robot_hmi_programs::ProgramsPlugin;
    use robot_hmi_execution::ExecutionPlugin;

    let mut app = App::new();

    // Core plugin: networking, database, ActiveSystem
    app.add_plugins(CorePlugin);

    // Execution plugin: toolpath orchestration (must come before FanucPlugin)
    app.add_plugins(ExecutionPlugin);

    // Programs plugin: device-agnostic program storage and CRUD
    app.add_plugins(ProgramsPlugin);

    // FANUC plugin: robot state, connections, I/O, motion
    app.add_plugins(FanucPlugin);

    // Duet plugin: extruder support (optional)
    cfg_if! {
        if #[cfg(feature = "duet")] {
            app.add_plugins(robot_hmi_duet::DuetPlugin);
        }
    }

    app
}

