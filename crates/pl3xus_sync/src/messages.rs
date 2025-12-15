#[cfg(feature = "runtime")]
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Bevy-agnostic entity identifier used on the wire.
///
/// Internally this just uses Bevy's opaque `Entity` bits representation so that
/// we don't rely on any particular layout (row/generation, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SerializableEntity {
    pub bits: u64,
}

impl SerializableEntity {
    /// A dangling entity that can be used to signal "spawn a new entity" in mutations.
    /// This uses the same bit pattern as Bevy's Entity::PLACEHOLDER.
    pub const DANGLING: Self = Self { bits: u64::MAX };
}

#[cfg(feature = "runtime")]
impl From<Entity> for SerializableEntity {
    fn from(e: Entity) -> Self {
        Self { bits: e.to_bits() }
    }
}

#[cfg(feature = "runtime")]
impl SerializableEntity {
    pub fn to_entity(self) -> Entity {
        Entity::from_bits(self.bits)
    }
}

/// Client -> server sync messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncClientMessage {
    /// Request to subscribe to components/entities.
    Subscription(SubscriptionRequest),
    /// Cancel an existing subscription.
    Unsubscribe(UnsubscribeRequest),
    /// Mutate a component value.
    Mutate(MutateComponent),
    /// Database/ECS-backed query request.
    Query(QueryRequest),
    /// Cancel an ongoing query-based subscription.
    QueryCancel(QueryCancel),
}

/// Server -> client sync messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncServerMessage {
    /// A batch of component snapshot/update events.
    SyncBatch(SyncBatch),
    /// Response to a mutation request.
    MutationResponse(MutationResponse),
    /// Response to a query request.
    QueryResponse(QueryResponse),
}

/// Subscribe to component data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionRequest {
    /// Logical client-side identifier for this subscription.
    pub subscription_id: u64,
    /// Component type name (as registered in Bevy's type registry).
    pub component_type: String,
    /// Optional specific entity to subscribe to.
    pub entity: Option<SerializableEntity>,
}

/// Cancel an existing subscription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsubscribeRequest {
    pub subscription_id: u64,
}

/// One batch of sync events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncBatch {
    pub items: Vec<SyncItem>,
}

/// A single sync event.
///
/// Note: `value` fields are raw bytes (bincode-encoded component data).
/// The component type is identified by the `component_type` field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncItem {
    /// Full snapshot for (entity, component_type).
    Snapshot {
        subscription_id: u64,
        entity: SerializableEntity,
        component_type: String,
        /// Bincode-encoded component value
        value: Vec<u8>,
    },
    /// Updated value for (entity, component_type).
    Update {
        subscription_id: u64,
        entity: SerializableEntity,
        component_type: String,
        /// Bincode-encoded component value
        value: Vec<u8>,
    },
    /// Component removed from entity.
    ComponentRemoved {
        subscription_id: u64,
        entity: SerializableEntity,
        component_type: String,
    },
    /// Entity despawned.
    EntityRemoved {
        subscription_id: u64,
        entity: SerializableEntity,
    },
}

/// Request to mutate a component value on the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutateComponent {
    /// Optional correlation ID chosen by the client.
    pub request_id: Option<u64>,
    pub entity: SerializableEntity,
    /// Component type name.
    pub component_type: String,
    /// New value for the component (full value, no patch/diff in v1).
    /// Bincode-encoded component value.
    pub value: Vec<u8>,
}

/// Response to a mutation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationResponse {
    pub request_id: Option<u64>,
    pub status: MutationStatus,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MutationStatus {
    Ok,
    Forbidden,
    NotFound,
    ValidationError,
    InternalError,
}

/// Simple, non-DSL query protocol for DB/ECS-backed queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRequest {
    pub query_id: u64,
    /// Logical query name, e.g. "saved_robot_connections".
    pub namespace: String,
    /// Arbitrary parameters encoded as JSON (interpreted by server-side handler).
    /// JSON-encoded parameters.
    pub params: String,
    pub mode: QueryMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryMode {
    OneShot,
    Subscribe,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse {
    pub query_id: u64,
    pub status: QueryStatus,
    /// Rows for one-shot queries; for subscriptions, servers will usually
    /// materialize ECS entities instead and rely on normal sync messages.
    /// JSON-encoded rows.
    pub rows: Option<Vec<String>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryStatus {
    Ok,
    NotFound,
    Forbidden,
    InvalidParams,
    InternalError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryCancel {
    pub query_id: u64,
}

