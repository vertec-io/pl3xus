use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::error::SyncError;
use crate::traits::SyncComponent;

/// Function type for deserializing bincode bytes to JSON.
type JsonDeserializeFn = fn(&[u8]) -> Result<serde_json::Value, bincode::error::DecodeError>;

/// Function type for serializing JSON to bincode bytes.
type JsonSerializeFn = fn(&serde_json::Value) -> Result<Vec<u8>, bincode::error::EncodeError>;

/// Unified client-side type registry for component deserialization and DevTools support.
///
/// This registry provides two modes of operation:
/// 1. **Concrete type deserialization** - For reactive hooks and normal client usage
/// 2. **JSON conversion** - For DevTools UI (enabled via `.with_devtools_support()`)
///
/// The JSON converters are only kept if `.with_devtools_support()` is called on the builder,
/// ensuring zero overhead for applications that don't use DevTools.
///
/// # Example
///
/// ```rust,ignore
/// use pl3xus_client::{ClientTypeRegistry, SyncComponent};
///
/// // Without DevTools
/// let registry = ClientTypeRegistry::builder()
///     .register::<Position>()
///     .register::<Velocity>()
///     .build();
///
/// // With DevTools - just add .with_devtools_support()
/// let registry = ClientTypeRegistry::builder()
///     .register::<Position>()
///     .register::<Velocity>()
///     .with_devtools_support()  // Enable DevTools
///     .build();
/// ```
#[derive(Clone)]
pub struct ClientTypeRegistry {
    /// Map from component type name to deserializer function (for concrete types)
    deserializers: Arc<HashMap<String, Arc<dyn Fn(&[u8]) -> Result<Box<dyn Any + Send + Sync>, bincode::error::DecodeError> + Send + Sync>>>,

    /// Map from component type name to TypeId (for type checking)
    type_ids: Arc<HashMap<String, TypeId>>,

    /// JSON converters (only present if .with_devtools_support() was called on builder)
    json_converters: Arc<RwLock<HashMap<String, (JsonDeserializeFn, JsonSerializeFn)>>>,

    /// Whether JSON support is enabled (set by .with_devtools_support() on builder)
    json_support_enabled: bool,
}

impl ClientTypeRegistry {
    /// Create a new empty registry.
    ///
    /// Most users should use `ClientTypeRegistry::builder()` instead.
    pub fn new() -> Self {
        Self {
            deserializers: Arc::new(HashMap::new()),
            type_ids: Arc::new(HashMap::new()),
            json_converters: Arc::new(RwLock::new(HashMap::new())),
            json_support_enabled: false,
        }
    }

    /// Create a new builder for constructing a registry.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let registry = ClientTypeRegistry::builder()
    ///     .register::<Position>()
    ///     .register::<Velocity>()
    ///     .build();
    /// ```
    pub fn builder() -> ClientTypeRegistryBuilder {
        ClientTypeRegistryBuilder::new()
    }

    /// Deserialize component data into the concrete type T.
    ///
    /// This is used by reactive hooks and normal client code.
    ///
    /// # Errors
    ///
    /// Returns `SyncError::TypeNotRegistered` if the component type is not registered.
    /// Returns `SyncError::DeserializationFailed` if deserialization fails.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let position: Position = registry.deserialize("Position", &bytes)?;
    /// ```
    pub fn deserialize<T: 'static>(&self, name: &str, data: &[u8]) -> Result<T, SyncError> {
        let deserializer = self.deserializers.get(name)
            .ok_or_else(|| SyncError::TypeNotRegistered {
                component_name: name.to_string(),
            })?;

        let boxed = deserializer(data)
            .map_err(|e: bincode::error::DecodeError| SyncError::DeserializationFailed {
                component_name: name.to_string(),
                error: format!("{:?}", e),
            })?;

        boxed.downcast::<T>()
            .map(|b| *b)
            .map_err(|_| SyncError::DeserializationFailed {
                component_name: name.to_string(),
                error: "Type mismatch".to_string(),
            })
    }

    /// Check if a component type is registered.
    pub fn is_registered(&self, name: &str) -> bool {
        self.deserializers.contains_key(name)
    }

    /// Get the TypeId for a registered component type.
    pub fn get_type_id(&self, name: &str) -> Option<TypeId> {
        self.type_ids.get(name).copied()
    }

    /// Get all registered type names.
    ///
    /// This is useful for DevTools to enumerate available component types.
    pub fn registered_types(&self) -> Vec<String> {
        self.deserializers.keys().cloned().collect()
    }

    /// Deserialize component data to JSON for DevTools display.
    ///
    /// This method lazily initializes the JSON converter for the given type name
    /// on first use. Subsequent calls reuse the cached converter.
    ///
    /// # Errors
    ///
    /// Returns error if the type is not registered or deserialization fails.
    pub fn deserialize_to_json(&self, name: &str, data: &[u8]) -> Result<serde_json::Value, SyncError> {
        // Check if JSON converter exists
        {
            let converters = self.json_converters.read().unwrap();
            if let Some((deserializer, _)) = converters.get(name) {
                return deserializer(data)
                    .map_err(|e| SyncError::DeserializationFailed {
                        component_name: name.to_string(),
                        error: format!("{:?}", e),
                    });
            }
        }

        // Type not registered at all
        Err(SyncError::TypeNotRegistered {
            component_name: name.to_string(),
        })
    }

    /// Serialize JSON data to bincode bytes for DevTools mutations.
    ///
    /// This method lazily initializes the JSON converter for the given type name
    /// on first use. Subsequent calls reuse the cached converter.
    ///
    /// # Errors
    ///
    /// Returns error if the type is not registered or serialization fails.
    pub fn serialize_from_json(&self, name: &str, json: &serde_json::Value) -> Result<Vec<u8>, SyncError> {
        // Check if JSON converter exists
        {
            let converters = self.json_converters.read().unwrap();
            if let Some((_, serializer)) = converters.get(name) {
                return serializer(json)
                    .map_err(|e| SyncError::SerializationFailed {
                        component_name: name.to_string(),
                        error: format!("{:?}", e),
                    });
            }
        }

        // Type not registered at all
        Err(SyncError::TypeNotRegistered {
            component_name: name.to_string(),
        })
    }

    /// Check if a type has JSON converters registered.
    ///
    /// This is useful for DevTools to determine if a component can be edited.
    pub fn has_json_support(&self, name: &str) -> bool {
        let converters = self.json_converters.read().unwrap();
        converters.contains_key(name)
    }

    /// Check if DevTools support is enabled for this registry.
    ///
    /// Returns true if `.with_devtools_support()` was called on the builder.
    /// DevTools should check this and log an error if false.
    pub fn is_devtools_support_enabled(&self) -> bool {
        self.json_support_enabled
    }
}

impl Default for ClientTypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing a `ClientTypeRegistry`.
///
/// This provides a fluent API for registering multiple component types.
///
/// # Example
///
/// ```rust,ignore
/// use pl3xus_client::ClientTypeRegistry;
///
/// // Without DevTools
/// let registry = ClientTypeRegistry::builder()
///     .register::<Position>()
///     .register::<Velocity>()
///     .build();
///
/// // With DevTools - just add .with_devtools_support()
/// let registry = ClientTypeRegistry::builder()
///     .register::<Position>()
///     .register::<Velocity>()
///     .with_devtools_support()
///     .build();
/// ```
pub struct ClientTypeRegistryBuilder {
    deserializers: HashMap<String, Arc<dyn Fn(&[u8]) -> Result<Box<dyn Any + Send + Sync>, bincode::error::DecodeError> + Send + Sync>>,
    type_ids: HashMap<String, TypeId>,
    json_converters: HashMap<String, (JsonDeserializeFn, JsonSerializeFn)>,
    json_support_enabled: bool,
}

impl ClientTypeRegistryBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            deserializers: HashMap::new(),
            type_ids: HashMap::new(),
            json_converters: HashMap::new(),
            json_support_enabled: false,
        }
    }

    /// Register a component type for deserialization.
    ///
    /// This registers the type for both concrete deserialization (used by reactive hooks)
    /// and JSON conversion (used by DevTools if `.with_devtools_support()` is called).
    ///
    /// The JSON converters are always created during registration, but will be dropped
    /// during `.build()` unless `.with_devtools_support()` was called.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Without DevTools
    /// let registry = ClientTypeRegistry::builder()
    ///     .register::<Position>()
    ///     .register::<Velocity>()
    ///     .build();
    ///
    /// // With DevTools - just add .with_devtools_support()
    /// let registry = ClientTypeRegistry::builder()
    ///     .register::<Position>()
    ///     .register::<Velocity>()
    ///     .with_devtools_support()
    ///     .build();
    /// ```
    pub fn register<T>(mut self) -> Self
    where
        T: SyncComponent + serde::Serialize + for<'de> serde::Deserialize<'de> + 'static,
    {
        let name = T::component_name().to_string();

        self.type_ids.insert(name.clone(), TypeId::of::<T>());

        // Register concrete deserializer (always needed)
        self.deserializers.insert(
            name.clone(),
            Arc::new(|data| {
                let (component, _) = bincode::serde::decode_from_slice::<T, _>(data, bincode::config::standard())?;
                Ok(Box::new(component) as Box<dyn Any + Send + Sync>)
            }),
        );

        // Register JSON converters (will be dropped if .with_devtools_support() not called)
        let deserializer: JsonDeserializeFn = |data: &[u8]| {
            let (value, _): (T, _) = bincode::serde::decode_from_slice(data, bincode::config::standard())?;
            serde_json::to_value(&value)
                .map_err(|e| bincode::error::DecodeError::OtherString(format!("JSON serialization failed: {}", e)))
        };

        let serializer: JsonSerializeFn = |json: &serde_json::Value| {
            let value: T = serde_json::from_value(json.clone())
                .map_err(|e| bincode::error::EncodeError::OtherString(format!("JSON deserialization failed: {}", e)))?;
            bincode::serde::encode_to_vec(&value, bincode::config::standard())
        };

        self.json_converters.insert(name, (deserializer, serializer));

        self
    }

    /// Enable DevTools support for this registry.
    ///
    /// Call this method to keep the JSON converters that were registered during `.register::<T>()`.
    /// If this method is not called, the JSON converters will be dropped during `.build()`.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let registry = ClientTypeRegistry::builder()
    ///     .register::<Position>()
    ///     .register::<Velocity>()
    ///     .with_devtools_support()  // Enable DevTools
    ///     .build();
    /// ```
    pub fn with_devtools_support(mut self) -> Self {
        self.json_support_enabled = true;
        self
    }

    /// Build the final `ClientTypeRegistry` wrapped in an `Arc`.
    ///
    /// The registry is wrapped in an Arc because it needs to be shared
    /// across multiple reactive contexts and potentially with DevTools.
    ///
    /// If `.with_devtools_support()` was not called, the JSON converters will be dropped
    /// to avoid overhead for applications that don't use DevTools.
    pub fn build(self) -> Arc<ClientTypeRegistry> {
        let json_converters = if self.json_support_enabled {
            self.json_converters
        } else {
            HashMap::new()
        };

        Arc::new(ClientTypeRegistry {
            deserializers: Arc::new(self.deserializers),
            type_ids: Arc::new(self.type_ids),
            json_converters: Arc::new(RwLock::new(json_converters)),
            json_support_enabled: self.json_support_enabled,
        })
    }
}

impl Default for ClientTypeRegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

