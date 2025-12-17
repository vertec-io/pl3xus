//! Server plugins for the Fanuc RMI Replica application.
//!
//! Following Bevy's plugin architecture, each major feature is its own plugin:
//! - `connection`: Robot connection state machine
//! - `sync`: Robot state synchronization from driver
//! - `requests`: Request/Response handlers for database queries

pub mod connection;
pub mod sync;
pub mod requests;

pub use connection::RobotConnectionPlugin;
pub use sync::RobotSyncPlugin;
pub use requests::RequestHandlerPlugin;
