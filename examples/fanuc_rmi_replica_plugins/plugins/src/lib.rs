//! FANUC RMI Replica Plugins
//!
//! This crate re-exports types and plugins from the modular plugin crates:
//! - `fanuc_replica_core`: Networking, database, ActiveSystem (control root)
//! - `fanuc_replica_fanuc`: FANUC robot state, connections, I/O, motion
//! - `fanuc_replica_programs`: Device-agnostic program management
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

// =============================================================================
// Always-available types (no feature flags required)
// These are pure data types with Serialize/Deserialize - no ECS/server deps
// =============================================================================

// Execution control types - Start/Pause/Resume/Stop
pub use fanuc_replica_execution::{
    Start, StartResponse, Pause, PauseResponse,
    Resume, ResumeResponse, Stop, StopResponse,
    // Execution state types for UI
    ExecutionState, SystemState, SourceType,
    BufferDisplayData, BufferLineDisplay,
    UiActions,
};

// Program load/unload types
pub use fanuc_replica_programs::{
    Load, LoadResponse, Unload, UnloadResponse,
    // Core program types
    Instruction, SequenceType, InstructionSequence,
    ProgramInfo, ProgramDetail, ProgramWithLines, ProgramLineInfo,
    ProgramNotification, ProgramNotificationKind,
    // CRUD request/response types
    ListPrograms, ListProgramsResponse,
    GetProgram, GetProgramResponse,
    CreateProgram, CreateProgramResponse,
    DeleteProgram, DeleteProgramResponse,
    UpdateProgramSettings, UpdateProgramSettingsResponse,
    UploadCsv, UploadCsvResponse,
    AddSequence, AddSequenceResponse,
    RemoveSequence, RemoveSequenceResponse,
};

// Common types
pub use pl3xus_common::{RequestMessage, ErrorResponse};

// Feature-gated re-exports - only for types that require feature-specific derives/deps
cfg_if! {
    if #[cfg(feature = "server")] {
        // Server feature: plugins, ECS components, database, systems

        // Core plugin exports
        pub use fanuc_replica_core::{
            ActiveSystem, CorePlugin, DatabaseResource, DatabaseInit,
            DatabaseInitRegistry, init_database, PluginSchedule,
        };

        // FANUC plugin exports (all types + plugin)
        pub use fanuc_replica_fanuc::*;

        // Programs plugin (server-only)
        pub use fanuc_replica_programs::ProgramsPlugin;

        // Execution plugin exports (ECS components, traits, systems)
        pub use fanuc_replica_execution::{
            ExecutionPlugin, ExecutionCoordinator, ExecutionTarget, ExecutionPoint,
            ToolpathBuffer, BufferState, MotionCommand, MotionType, PointMetadata,
            PrimaryMotion, MotionDevice, AuxiliaryDevice, AuxiliaryCommand, DeviceError,
            MotionCommandEvent, AuxiliaryCommandEvent, DeviceStatus, DeviceType,
        };

        // Server-only: automatic query invalidation macros
        pub use pl3xus_macros::{Invalidates, HasSuccess};
    } else if #[cfg(feature = "ecs")] {
        // ECS-only (no server): types for testing/shared code

        // Core types
        pub use fanuc_replica_core::ActiveSystem;

        // FANUC types
        pub use fanuc_replica_fanuc::*;
    }
}

cfg_if! {
    if #[cfg(feature = "duet")] {
        // Duet plugin (requires server feature)
        pub use fanuc_replica_duet::DuetPlugin;
    }
}

cfg_if! {
    if #[cfg(all(feature = "stores", not(feature = "server"), not(feature = "ecs")))] {
        // Stores feature (standalone): types with Store derives for client-side reactivity
        // Base types are already exported above - this adds Store-derived component types

        // Core types with Store derives
        pub use fanuc_replica_core::{
            ActiveSystem,
            ConsoleLogEntry, ConsoleDirection, ConsoleMsgType, console_entry,
        };

        // FANUC types with Store derives
        pub use fanuc_replica_fanuc::*;
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
    use fanuc_replica_programs::ProgramsPlugin;
    use fanuc_replica_execution::ExecutionPlugin;

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
            app.add_plugins(fanuc_replica_duet::DuetPlugin);
        }
    }

    app
}

