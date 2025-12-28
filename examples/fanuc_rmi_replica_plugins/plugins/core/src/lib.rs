//! Core infrastructure plugin.
//!
//! This crate provides:
//! - `DatabaseResource` - SQLite connection wrapper
//! - `DatabaseInit` trait - For plugins to register their schemas
//! - `ActiveSystem` - Marker component for the control root entity
//! - `CorePlugin` - Sets up networking, database, and base infrastructure
//! - `PluginSchedule` - System set for ordering plugin systems
//!
//! # Usage
//!
//! Each plugin that needs database access implements `DatabaseInit`:
//!
//! ```rust,ignore
//! impl DatabaseInit for MyPlugin {
//!     fn name(&self) -> &'static str { "my_plugin" }
//!     fn init_schema(&self, conn: &Connection) -> anyhow::Result<()> {
//!         conn.execute("CREATE TABLE IF NOT EXISTS ...", [])?;
//!         Ok(())
//!     }
//! }
//! ```
//!
//! Plugins use `PluginSchedule` to order their systems:
//!
//! ```rust,ignore
//! app.add_systems(
//!     Update,
//!     my_system.in_set(PluginSchedule::MainUpdate),
//! );
//! ```

#[cfg(feature = "server")]
mod database;
mod types;

#[cfg(feature = "server")]
mod plugin;

#[cfg(feature = "server")]
mod plugin_schedule;

// Re-export types
pub use types::ActiveSystem;

#[cfg(feature = "server")]
pub use database::{DatabaseResource, DatabaseInit, DatabaseInitRegistry};

#[cfg(feature = "server")]
pub use plugin::CorePlugin;

#[cfg(feature = "server")]
pub use plugin::init_database;

#[cfg(feature = "server")]
pub use plugin_schedule::PluginSchedule;

