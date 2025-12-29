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

use cfg_if::cfg_if;

// Always available
mod types;
pub use types::*;

cfg_if! {
    if #[cfg(feature = "server")] {
        mod database;
        mod csv_parser;
        mod handlers;
        mod notifications;
        mod plugin;
        mod validation;

        pub use database::{ProgramsDatabaseInit, queries};
        pub use csv_parser::{parse_csv, ParseResult, ParseError, ParseWarning};
        pub use handlers::ProgramHandlerPlugin;
        pub use notifications::ProgramNotificationsPlugin;
        pub use plugin::ProgramsPlugin;
    }
}

