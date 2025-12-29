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

mod types;

#[cfg(feature = "server")]
mod database;

#[cfg(feature = "server")]
mod csv_parser;

#[cfg(feature = "server")]
mod handlers;

#[cfg(feature = "server")]
mod plugin;

// Re-export types (always available)
pub use types::*;

// Re-export server functionality
#[cfg(feature = "server")]
pub use database::{ProgramsDatabaseInit, queries};

#[cfg(feature = "server")]
pub use csv_parser::{parse_csv, ParseResult, ParseError, ParseWarning};

#[cfg(feature = "server")]
pub use handlers::ProgramHandlerPlugin;

#[cfg(feature = "server")]
pub use plugin::ProgramsPlugin;

