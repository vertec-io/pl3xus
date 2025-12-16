pub mod messages;
pub use messages::*;

// Explicitly export Pl3xusMessage for clarity
pub use messages::Pl3xusMessage;

pub mod codec;

pub mod error;

use serde::{Deserialize, Serialize};

use std::fmt::Debug;
use std::fmt::Display;

pub use pl3xus_macros::SubscribeById;

#[derive(Serialize, Deserialize, Clone)]
/// [`NetworkPacket`]s are untyped packets to be sent over the wire
///
/// The packet contains both a human-readable type name (for debugging) and
/// a schema hash (for matching). The system tries to match by type_name first,
/// then falls back to schema_hash for resilience against module refactoring.
pub struct NetworkPacket {
    /// Full type name including module path (for debugging)
    /// Example: "my_crate::messages::PlayerPosition"
    pub type_name: String,
    /// Schema hash computed from short type name (for matching)
    /// This provides stability across module refactoring
    pub schema_hash: u64,
    /// The serialized message data from bincode
    pub data: Vec<u8>,
}

impl Debug for NetworkPacket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NetworkPacket")
            .field("type_name", &self.type_name)
            .field("schema_hash", &format_args!("0x{:016x}", self.schema_hash))
            .field("data_len", &self.data.len())
            .finish()
    }
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq, Clone, Copy, Debug)]
/// A [`ConnectionId`] denotes a single connection
pub struct ConnectionId {
    /// The key of the connection.
    pub id: u32,
}

impl ConnectionId {
    /// Represents the server's connection ID
    pub const SERVER: Self = ConnectionId { id: 0 };

    /// Returns true if this ConnectionId represents the server
    pub fn is_server(&self) -> bool {
        self.id == Self::SERVER.id
    }
}

impl Display for ConnectionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Connection with ID={0}", self.id))
    }
}

// ============================================================================
// Control Types (shared between server and client)
// ============================================================================

/// Request to take or release control of an entity.
///
/// Used with `ExclusiveControlPlugin` on the server.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ControlRequest {
    /// Request to take control of the specified entity (entity.to_bits()).
    Take(u64),
    /// Request to release control of the specified entity (entity.to_bits()).
    Release(u64),
}

/// Response to a control request.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ControlResponse {
    /// Control was successfully taken.
    Taken,
    /// Control was successfully released.
    Released,
    /// Entity is already controlled by another client.
    AlreadyControlled {
        /// The client that currently has control.
        by_client: ConnectionId,
    },
    /// Entity is not currently controlled (when trying to release).
    NotControlled,
    /// An error occurred.
    Error(String),
}

/// Component that tracks which client has control of an entity.
///
/// This is a default control component that can be used with `ExclusiveControlPlugin`.
/// Applications can also define their own control components with additional fields.
///
/// When the `ecs` feature is enabled, this type also derives `Component`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ecs", derive(bevy::prelude::Component))]
pub struct EntityControl {
    /// The client that currently has control.
    pub client_id: ConnectionId,
    /// Timestamp of last activity (for timeout detection).
    pub last_activity: f32,
}

impl Default for EntityControl {
    fn default() -> Self {
        Self {
            client_id: ConnectionId { id: 0 },
            last_activity: 0.0,
        }
    }
}
