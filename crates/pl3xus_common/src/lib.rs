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
#[cfg_attr(feature = "ecs", derive(bevy::prelude::Message))]
pub enum ControlRequest {
    /// Request to take control of the specified entity (entity.to_bits()).
    Take(u64),
    /// Request to release control of the specified entity (entity.to_bits()).
    Release(u64),
}

/// Response to a control request.
///
/// Each response includes a unique `sequence` number to ensure that identical
/// responses (e.g., multiple "AlreadyControlled" for repeated requests) are
/// treated as distinct messages by the client's message deduplication logic.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "ecs", derive(bevy::prelude::Message))]
pub struct ControlResponse {
    /// Unique sequence number to distinguish otherwise identical responses.
    pub sequence: u64,
    /// The actual response variant.
    pub kind: ControlResponseKind,
}

/// The kind of control response.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub enum ControlResponseKind {
    /// No response yet (default state).
    #[default]
    None,
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
    /// Another client is requesting control of an entity you control.
    /// Sent to the controlling client when someone else requests control.
    ControlRequested {
        /// The client that is requesting control.
        by_client: ConnectionId,
    },
    /// An error occurred.
    Error(String),
}

/// Component that tracks which client has control of an entity.
///
/// This is a default control component that can be used with `ExclusiveControlPlugin`.
/// Applications can also define their own control components with additional fields.
///
/// When the `ecs` feature is enabled, this type also derives `Component`.
///
/// # Sub-connections
///
/// Clients can have "sub-connections" - related connections like additional browser tabs
/// or related services that should share the same control permissions. When a client takes
/// control of an entity, their sub-connections are also authorized to send commands.
///
/// Use `AssociateSubConnection` to register a sub-connection with a parent connection.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ecs", derive(bevy::prelude::Component))]
pub struct EntityControl {
    /// The client that currently has control.
    pub client_id: ConnectionId,
    /// Sub-connections that share control with the primary client.
    /// These are related connections (like additional browser tabs) that
    /// are authorized to send commands on behalf of the controlling client.
    #[serde(default)]
    pub sub_connection_ids: Vec<ConnectionId>,
    /// Timestamp of last activity (for timeout detection).
    pub last_activity: f32,
}

impl Default for EntityControl {
    fn default() -> Self {
        Self {
            client_id: ConnectionId { id: 0 },
            sub_connection_ids: Vec::new(),
            last_activity: 0.0,
        }
    }
}

impl EntityControl {
    /// Check if the given connection has control (either as primary or sub-connection).
    pub fn has_control(&self, connection_id: ConnectionId) -> bool {
        // client_id 0 means "no controller" - nobody has control
        if self.client_id.id == 0 {
            return false;
        }
        self.client_id == connection_id || self.sub_connection_ids.contains(&connection_id)
    }

    /// Check if any client has control of this entity.
    pub fn is_controlled(&self) -> bool {
        self.client_id.id != 0
    }
}

// ============================================================================
// Sub-Connection Types (for related connections like multiple browser tabs)
// ============================================================================

/// Request to associate a sub-connection with a parent connection.
///
/// When a client opens a related connection (like a second browser tab),
/// it can associate itself with the parent connection to share control
/// permissions. Messages from sub-connections are authorized as if they
/// came from the parent connection.
///
/// # Example Flow
///
/// 1. User opens main app in Tab 1, gets ConnectionId 5
/// 2. User opens a second tab (Tab 2), gets ConnectionId 7
/// 3. Tab 2 sends `AssociateSubConnection { parent_connection_id: 5 }`
/// 4. Server updates SubConnections resource for connection 5 to include 7
/// 5. When Tab 1 takes control of an entity, Tab 2 can also send commands
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ecs", derive(bevy::prelude::Message))]
pub struct AssociateSubConnection {
    /// The parent connection that this sub-connection should be associated with.
    pub parent_connection_id: ConnectionId,
}

/// Response to a sub-connection association request.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ecs", derive(bevy::prelude::Message))]
pub struct AssociateSubConnectionResponse {
    /// Whether the association was successful.
    pub success: bool,
    /// Error message if the association failed.
    pub error: Option<String>,
    /// The parent connection that was associated with.
    pub parent_connection_id: ConnectionId,
}

// ============================================================================
// Server Notification Types (shared between server and client)
// ============================================================================

/// Severity level for server notifications.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub enum NotificationLevel {
    /// Informational message.
    #[default]
    Info,
    /// Success message.
    Success,
    /// Warning message.
    Warning,
    /// Error message.
    Error,
}

/// Global sequence counter for server notifications.
/// Starts at 1 so that 0 can be used as the "unset" default value.
static NOTIFICATION_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// Get the next notification sequence number.
fn next_notification_sequence() -> u64 {
    NOTIFICATION_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// A notification from the server to be displayed to the user.
///
/// This is used to communicate authorization denials, errors, and other
/// important messages that should be shown as toasts or in a console.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[cfg_attr(feature = "ecs", derive(bevy::prelude::Message))]
pub struct ServerNotification {
    /// Unique sequence number for deduplication.
    /// Each notification gets a unique, monotonically increasing sequence number.
    pub sequence: u64,
    /// The notification message.
    pub message: String,
    /// The severity level.
    pub level: NotificationLevel,
    /// Optional context (e.g., the message type that was rejected).
    pub context: Option<String>,
}

impl ServerNotification {
    /// Create an info notification.
    pub fn info(message: impl Into<String>) -> Self {
        Self {
            sequence: next_notification_sequence(),
            message: message.into(),
            level: NotificationLevel::Info,
            context: None,
        }
    }

    /// Create a success notification.
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            sequence: next_notification_sequence(),
            message: message.into(),
            level: NotificationLevel::Success,
            context: None,
        }
    }

    /// Create a warning notification.
    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            sequence: next_notification_sequence(),
            message: message.into(),
            level: NotificationLevel::Warning,
            context: None,
        }
    }

    /// Create an error notification.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            sequence: next_notification_sequence(),
            message: message.into(),
            level: NotificationLevel::Error,
            context: None,
        }
    }

    /// Add context to the notification.
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }
}
