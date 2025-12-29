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

use cfg_if::cfg_if;

mod types;

// Types always available
pub use types::{
    ActiveSystem, ConsoleLogEntry, ConsoleDirection, ConsoleMsgType, console_entry,
    ResetDatabase, ResetDatabaseResponse,
};

cfg_if! {
    if #[cfg(feature = "server")] {
        mod database;
        mod handlers;
        mod plugin;
        mod plugin_schedule;

        pub use database::{DatabaseResource, DatabaseInit, DatabaseInitRegistry};
        pub use handlers::handle_reset_database;
        pub use plugin::{CorePlugin, init_database};
        pub use plugin_schedule::PluginSchedule;
    }
}

