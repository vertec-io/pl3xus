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

// ============================================================================
// TYPES - Always available (ECS derives gated by "ecs" feature)
// ============================================================================
pub mod types;

// ============================================================================
// SERVER-ONLY - Plugin, database, handlers
// ============================================================================
cfg_if! {
    if #[cfg(feature = "server")] {
        pub mod database;
        pub mod handlers;
        mod plugin;
        mod plugin_schedule;

        pub use plugin::CorePlugin;
        pub use plugin::init_database;
        pub use plugin_schedule::PluginSchedule;
    }
}

