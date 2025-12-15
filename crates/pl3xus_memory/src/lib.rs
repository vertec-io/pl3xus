//! # Pl3xus Memory
//!
//! This crate provides memory leak detection and prevention tools for the pl3xus networking library.
//! It helps identify and fix memory leaks in Bevy applications that use pl3xus for networking.
//!
//! ## Features
//!
//! - Memory usage monitoring
//! - Connection cleanup
//! - Message queue monitoring
//! - Resource cleanup
//!
//! ## Usage
//!
//! ```rust,no_run
//! use bevy::prelude::*;
//! use pl3xus_memory::NetworkMemoryPlugin;
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(NetworkMemoryPlugin)
//!         .run();
//! }
//! ```

mod connection_cleanup;
mod memory_diagnostic;
mod memory_monitor;
mod message_cleanup;
mod plugin;

pub use connection_cleanup::*;
pub use memory_diagnostic::*;
pub use memory_monitor::*;
pub use message_cleanup::*;
pub use plugin::*;

/// Re-export the main plugin
pub use plugin::NetworkMemoryPlugin;
