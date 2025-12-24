//! Server plugins for the Fanuc RMI Replica application.
//!
//! Following Bevy's plugin architecture, each major feature is its own plugin:
//! - `system`: System/Apparatus entity (hierarchy root)
//! - `connection`: Robot connection state machine
//! - `sync`: Robot state synchronization from driver
//! - `requests`: Request/Response handlers for database queries
//! - `polling`: Periodic robot status polling
//! - `program`: Program execution with orchestrator pattern

pub mod system;
pub mod connection;
pub mod sync;
pub mod requests;
pub mod polling;
pub mod program;

pub use system::SystemPlugin;
pub use connection::RobotConnectionPlugin;
pub use sync::RobotSyncPlugin;
pub use requests::RequestHandlerPlugin;
pub use polling::RobotPollingPlugin;
pub use program::ProgramPlugin;
