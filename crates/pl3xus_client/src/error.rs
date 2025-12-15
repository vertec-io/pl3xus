use std::fmt;

/// Errors that can occur when using the pl3xus_client library.
#[derive(Debug, Clone)]
pub enum SyncError {
    /// WebSocket is not connected to the server.
    NotConnected,

    /// Failed to deserialize component data from the server.
    DeserializationFailed {
        /// Component type name that failed to deserialize
        component_name: String,
        /// Error message from the deserializer
        error: String,
    },

    /// Component type is not registered in the ClientRegistry.
    TypeNotRegistered {
        /// Component type name that is not registered
        component_name: String,
    },

    /// Schema hash mismatch between client and server.
    ///
    /// This indicates that the client and server have different versions
    /// of the component type definition.
    SchemaHashMismatch {
        /// Component type name with mismatched schema
        component_name: String,
        /// Expected schema hash (client-side)
        expected: u64,
        /// Actual schema hash (server-side)
        actual: u64,
    },

    /// WebSocket error occurred.
    WebSocketError {
        /// Error message from the WebSocket layer
        message: String,
    },

    /// Failed to serialize component data for mutation.
    SerializationFailed {
        /// Component type name that failed to serialize
        component_name: String,
        /// Error message from the serializer
        error: String,
    },
}

impl fmt::Display for SyncError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyncError::NotConnected => {
                write!(f, "Not connected to server")
            }
            SyncError::DeserializationFailed { component_name, error } => {
                write!(
                    f,
                    "Failed to deserialize component '{}': {}",
                    component_name, error
                )
            }
            SyncError::TypeNotRegistered { component_name } => {
                write!(
                    f,
                    "Component type '{}' is not registered in the ClientRegistry. \
                     Did you forget to call registry.register::<{}>()?",
                    component_name, component_name
                )
            }
            SyncError::SchemaHashMismatch {
                component_name,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "Schema hash mismatch for component '{}': expected {:#x}, got {:#x}. \
                     This usually means the client and server have different versions of the type definition.",
                    component_name, expected, actual
                )
            }
            SyncError::WebSocketError { message } => {
                write!(f, "WebSocket error: {}", message)
            }
            SyncError::SerializationFailed { component_name, error } => {
                write!(
                    f,
                    "Failed to serialize component '{}': {}",
                    component_name, error
                )
            }
        }
    }
}

impl std::error::Error for SyncError {}

