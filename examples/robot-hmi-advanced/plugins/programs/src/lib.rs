//! Device-agnostic program management plugin.
//!
//! This plugin provides:
//! - Program storage and retrieval
//! - CSV import with flexible column support
//! - Multiple approach/retreat sequence support
//! - Device-agnostic instruction types
//!
//! # Features
//!
//! - `ecs` - Enables Bevy ECS integration
//! - `server` - Enables database and handler functionality
//!
//! # Usage
//!
//! ```rust,ignore
//! use robot_hmi_programs::types::*;
//! use robot_hmi_programs::database;
//! ```

use cfg_if::cfg_if;

// ============================================================================
// TYPES - Always available (ECS derives gated by "ecs" feature)
// ============================================================================
pub mod types;

// ============================================================================
// SERVER-ONLY - Plugin, systems, handlers, database
// ============================================================================
cfg_if! {
    if #[cfg(feature = "server")] {
        pub mod csv_parser;
        pub mod database;
        mod handlers;
        mod notifications;
        mod plugin;
        mod sync_plugin;
        mod validation;
        pub mod systems;

        pub use handlers::ProgramHandlerPlugin;
        pub use notifications::ProgramNotificationsPlugin;
        pub use plugin::ProgramsPlugin;
        pub use sync_plugin::ProgramsSyncPlugin;
    }
}

