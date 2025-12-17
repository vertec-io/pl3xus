use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Network message with automatic type name generation and schema hashing
///
/// This trait is automatically implemented for all types that are
/// `Serialize + DeserializeOwned + Send + Sync + 'static`.
///
/// The type name is generated from `std::any::type_name()` and cached
/// for performance. The first access incurs a ~500ns cost, subsequent
/// accesses are ~50-100ns.
///
/// The schema hash is computed from the short type name (without module path)
/// to provide a stable identifier that survives module refactoring.
///
/// ## Example
///
/// ```rust
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize, Clone)]
/// struct PlayerPosition {
///     x: f32,
///     y: f32,
///     z: f32,
/// }
///
/// // No trait implementation needed!
/// // Pl3xusMessage is automatically implemented.
/// // Use with app.register_network_message::<PlayerPosition, Provider>();
/// ```
pub trait Pl3xusMessage: Serialize + DeserializeOwned + Send + Sync + 'static {
    /// Returns the full type name for this message type (includes module path).
    ///
    /// The name is generated from `std::any::type_name()` and cached
    /// in a global static for performance.
    ///
    /// Example: `"my_crate::messages::PlayerPosition"`
    fn type_name() -> &'static str {
        use std::any::{TypeId, type_name};
        use std::collections::HashMap;
        use std::sync::{Mutex, OnceLock};

        static CACHE: OnceLock<Mutex<HashMap<TypeId, &'static str>>> = OnceLock::new();
        let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));

        let type_id = TypeId::of::<Self>();

        // Fast path: check cache without holding lock long
        {
            let cache_guard = cache.lock().unwrap();
            if let Some(&name) = cache_guard.get(&type_id) {
                return name;
            }
        }

        // Slow path: generate and cache
        let full_type_name = type_name::<Self>();
        let static_name = Box::leak(full_type_name.to_string().into_boxed_str());

        {
            let mut cache_guard = cache.lock().unwrap();
            cache_guard.insert(type_id, static_name);
        }

        static_name
    }

    /// Returns the short type name (just the struct name, no module path).
    ///
    /// This is used for schema hashing to provide stability across module refactoring.
    ///
    /// Example: `"PlayerPosition"` (from `"my_crate::messages::PlayerPosition"`)
    fn short_name() -> &'static str {
        use std::any::TypeId;
        use std::collections::HashMap;
        use std::sync::{Mutex, OnceLock};

        static CACHE: OnceLock<Mutex<HashMap<TypeId, &'static str>>> = OnceLock::new();
        let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));

        let type_id = TypeId::of::<Self>();

        // Fast path: check cache without holding lock long
        {
            let cache_guard = cache.lock().unwrap();
            if let Some(&name) = cache_guard.get(&type_id) {
                return name;
            }
        }

        // Slow path: extract short name from full type name
        let full_name = Self::type_name();
        let short = full_name.rsplit("::").next().unwrap_or(full_name);
        let static_name = Box::leak(short.to_string().into_boxed_str());

        {
            let mut cache_guard = cache.lock().unwrap();
            cache_guard.insert(type_id, static_name);
        }

        static_name
    }

    /// Returns a hash of the message schema.
    ///
    /// The hash is computed from the short type name (without module path).
    /// This provides a stable identifier that survives module refactoring
    /// while still being unique enough to avoid most collisions.
    ///
    /// Note: If two types have the same short name (e.g., `foo::Message` and
    /// `bar::Message`), they will have the same schema hash. This is intentional
    /// and will be caught during registration if both are used in the same binary.
    fn schema_hash() -> u64 {
        use std::any::TypeId;
        use std::collections::HashMap;
        use std::hash::{Hash, Hasher};
        use std::sync::{Mutex, OnceLock};

        static CACHE: OnceLock<Mutex<HashMap<TypeId, u64>>> = OnceLock::new();
        let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));

        let type_id = TypeId::of::<Self>();

        // Fast path: check cache without holding lock long
        {
            let cache_guard = cache.lock().unwrap();
            if let Some(&hash) = cache_guard.get(&type_id) {
                return hash;
            }
        }

        // Slow path: compute hash from short name
        let short = Self::short_name();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        short.hash(&mut hasher);
        let hash = hasher.finish();

        {
            let mut cache_guard = cache.lock().unwrap();
            cache_guard.insert(type_id, hash);
        }

        hash
    }
}

// Blanket implementation for all serializable types
impl<T> Pl3xusMessage for T
where
    T: Serialize + DeserializeOwned + Send + Sync + 'static
{}

/// Marks a type as a request message with an associated response type.
///
/// This trait extends `Pl3xusMessage` to add request/response semantics.
/// The request name is automatically derived from the type name.
///
/// # Example
///
/// ```rust
/// use serde::{Serialize, Deserialize};
/// use pl3xus_common::RequestMessage;
///
/// #[derive(Clone, Debug, Serialize, Deserialize)]
/// struct ListRobots;
///
/// #[derive(Clone, Debug, Serialize, Deserialize)]
/// struct RobotList {
///     robots: Vec<String>,
/// }
///
/// impl RequestMessage for ListRobots {
///     type ResponseMessage = RobotList;
/// }
///
/// // The request name is automatically derived as "ListRobots"
/// assert_eq!(ListRobots::request_name(), "ListRobots");
/// ```
pub trait RequestMessage: Pl3xusMessage + Clone + Debug {
    /// The response type for the request.
    type ResponseMessage: Pl3xusMessage + Clone + Debug;

    /// Returns the request name, derived from the short type name.
    ///
    /// This is automatically implemented using `Pl3xusMessage::short_name()`.
    fn request_name() -> &'static str {
        Self::short_name()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(bound = "T: Pl3xusMessage")]
pub struct TargetedMessage<T: Pl3xusMessage> {
    pub target_id: String,
    pub message: T,
}

impl<T: Pl3xusMessage> TargetedMessage<T> {
    pub fn name() -> &'static str {
        // Use a global cache with lazy initialization
        use std::any::TypeId;
        use std::collections::HashMap;
        use std::sync::Mutex;
        use std::sync::OnceLock;

        static CACHE: OnceLock<Mutex<HashMap<TypeId, &'static str>>> = OnceLock::new();
        let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));

        let type_id = TypeId::of::<T>();

        // Try to get from cache first
        {
            let cache_guard = cache.lock().unwrap();
            if let Some(&name) = cache_guard.get(&type_id) {
                return name;
            }
        }

        // Not in cache, create it once and leak it (only once per type)
        // Use the message_kind() method which works for both NetworkMessage and Pl3xusMessage
        let inner_name = T::type_name();
        let formatted_name = format!("Targeted({})", inner_name);
        let static_name = Box::leak(formatted_name.into_boxed_str());

        // Store in cache for future use
        {
            let mut cache_guard = cache.lock().unwrap();
            cache_guard.insert(type_id, static_name);
        }

        static_name
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(bound = "T: Pl3xusMessage")]
pub struct PreviousMessage<T: Pl3xusMessage> {
    // Empty struct - only used for type information
    #[serde(skip)]
    _phantom: std::marker::PhantomData<T>,
    // Add a marker field that will actually be serialized
    #[serde(default)]
    _marker: bool,
}

impl<T: Pl3xusMessage> Default for PreviousMessage<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Pl3xusMessage> PreviousMessage<T> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
            _marker: false,
        }
    }

    pub fn name() -> &'static str {
        // Use a global cache with lazy initialization
        use std::any::TypeId;
        use std::collections::HashMap;
        use std::sync::Mutex;
        use std::sync::OnceLock;

        static CACHE: OnceLock<Mutex<HashMap<TypeId, &'static str>>> = OnceLock::new();
        let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));

        let type_id = TypeId::of::<T>();

        // Try to get from cache first
        {
            let cache_guard = cache.lock().unwrap();
            if let Some(&name) = cache_guard.get(&type_id) {
                return name;
            }
        }

        // Not in cache, create it once and leak it (only once per type)
        // Use type_name() which works for all Pl3xusMessage types
        let inner_name = T::type_name();
        let formatted_name = format!("PreviousMessage({})", inner_name);
        let static_name = Box::leak(formatted_name.into_boxed_str());

        // Store in cache for future use
        {
            let mut cache_guard = cache.lock().unwrap();
            cache_guard.insert(type_id, static_name);
        }

        static_name
    }
}

/// Marks a type as a subscription message that can be used in a pub/sub pattern.
///
/// This trait works with `Pl3xusMessage` types, using automatic type name generation.
///
/// # Type Parameters
/// * `Request` - The message type used to initiate a subscription
/// * `Unsubscribe` - The message type used to terminate a subscription
/// * `SubscriptionParams` - Parameters needed to create a subscription request
///
/// # Examples
/// ```rust
/// use pl3xus_common::SubscriptionMessage;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize, Debug)]
/// struct SessionUpdate {
///     session_id: String,
///     state: Vec<u8>,
/// }
///
/// #[derive(Serialize, Deserialize, Debug)]
/// struct SubscribeToSession {
///     session_id: String,
/// }
///
/// #[derive(Serialize, Deserialize, Debug)]
/// struct UnsubscribeFromSession {
///     session_id: String,
/// }
///
/// impl SubscriptionMessage for SessionUpdate {
///     type SubscribeRequest = SubscribeToSession;
///     type UnsubscribeRequest = UnsubscribeFromSession;
///     type SubscriptionParams = String;
///
///     fn get_subscription_params(&self) -> Self::SubscriptionParams {
///         self.session_id.clone()
///     }
///
///     fn create_subscription_request(params: Self::SubscriptionParams) -> Self::SubscribeRequest {
///         SubscribeToSession { session_id: params }
///     }
///
///     fn create_unsubscribe_request(params: Self::SubscriptionParams) -> Self::UnsubscribeRequest {
///         UnsubscribeFromSession { session_id: params }
///     }
/// }
/// ```
///
pub trait SubscriptionMessage: Pl3xusMessage {
    /// The message type used to request a subscription
    type SubscribeRequest: Pl3xusMessage
        + Serialize
        + DeserializeOwned
        + Send
        + Sync
        + Debug
        + 'static;

    /// The message type used to terminate a subscription
    type UnsubscribeRequest: Pl3xusMessage
        + Serialize
        + DeserializeOwned
        + Send
        + Sync
        + Debug
        + 'static;

    /// Parameters needed to create subscription/unsubscribe requests
    type SubscriptionParams: Serialize
        + DeserializeOwned
        + Send
        + Sync
        + Debug
        + PartialEq
        + Clone
        + 'static;

    /// Returns the subscription parameters associated with this message
    /// This allows clients to match incoming messages with their original subscription parameters
    fn get_subscription_params(&self) -> Self::SubscriptionParams;

    /// Creates a subscription request from the given parameters
    fn create_subscription_request(
        subscription_params: Self::SubscriptionParams,
    ) -> Self::SubscribeRequest;

    /// Creates an unsubscribe request from the given parameters
    fn create_unsubscribe_request(
        subscription_params: Self::SubscriptionParams,
    ) -> Self::UnsubscribeRequest;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pl3xus_message_caching() {
        #[derive(Serialize, Deserialize)]
        struct TestMessage {
            data: String
        }

        let name1 = TestMessage::type_name();
        let name2 = TestMessage::type_name();

        // Should return same pointer (cached)
        assert_eq!(name1 as *const str, name2 as *const str);
        assert!(name1.contains("TestMessage"));
    }

    #[test]
    fn test_different_types_different_names() {
        #[derive(Serialize, Deserialize)]
        struct TypeA {
            x: i32
        }

        #[derive(Serialize, Deserialize)]
        struct TypeB {
            x: i32
        }

        let name_a = TypeA::type_name();
        let name_b = TypeB::type_name();

        assert_ne!(name_a, name_b);
        assert!(name_a.contains("TypeA"));
        assert!(name_b.contains("TypeB"));
    }

    #[test]
    fn test_generic_types() {
        #[derive(Serialize, Deserialize)]
        struct Generic<T> {
            value: T
        }

        let name_i32 = Generic::<i32>::type_name();
        let name_string = Generic::<String>::type_name();

        assert_ne!(name_i32, name_string);
        assert!(name_i32.contains("Generic"));
        assert!(name_string.contains("Generic"));
    }

    #[test]
    fn test_pl3xus_message_type_name() {
        #[derive(Serialize, Deserialize)]
        struct AutoMsg {
            data: String
        }

        // Pl3xusMessage uses type_name() automatically
        let name = AutoMsg::type_name();
        assert!(name.contains("AutoMsg"));
    }

    #[test]
    fn test_short_name() {
        #[derive(Serialize, Deserialize)]
        struct MyMessage {
            data: String
        }

        let short = MyMessage::short_name();
        let full = MyMessage::type_name();

        // Short name should be just the struct name
        assert_eq!(short, "MyMessage");
        // Full name should contain module path
        assert!(full.contains("MyMessage"));
        assert!(full.len() > short.len());
    }

    #[test]
    fn test_schema_hash() {
        #[derive(Serialize, Deserialize)]
        struct MessageA {
            data: String
        }

        #[derive(Serialize, Deserialize)]
        struct MessageB {
            data: String
        }

        let hash_a1 = MessageA::schema_hash();
        let hash_a2 = MessageA::schema_hash();
        let hash_b = MessageB::schema_hash();

        // Same type should have same hash (cached)
        assert_eq!(hash_a1, hash_a2);
        // Different types should have different hashes
        assert_ne!(hash_a1, hash_b);
    }

    #[test]
    fn test_schema_hash_stability() {
        // Simulate types from different modules with same name
        mod module1 {
            use serde::{Serialize, Deserialize};
            #[derive(Serialize, Deserialize)]
            pub struct UserMessage {
                pub message: String
            }
        }

        mod module2 {
            use serde::{Serialize, Deserialize};
            #[derive(Serialize, Deserialize)]
            pub struct UserMessage {
                pub user_id: u32
            }
        }

        // Both should have the same hash (same short name)
        let hash1 = module1::UserMessage::schema_hash();
        let hash2 = module2::UserMessage::schema_hash();

        assert_eq!(hash1, hash2, "Types with same short name should have same schema hash");

        // But different full type names
        let name1 = module1::UserMessage::type_name();
        let name2 = module2::UserMessage::type_name();
        assert_ne!(name1, name2, "Types should have different full type names");
    }

    #[test]
    fn test_request_message_auto_name() {
        use super::*;

        #[derive(Clone, Debug, Serialize, Deserialize)]
        struct ListRobots;

        #[derive(Clone, Debug, Serialize, Deserialize)]
        struct RobotList {
            robots: Vec<String>,
        }

        impl RequestMessage for ListRobots {
            type ResponseMessage = RobotList;
        }

        // Request name should be automatically derived from the type name
        assert_eq!(ListRobots::request_name(), "ListRobots");

        // It should also work with the Pl3xusMessage trait methods
        assert!(ListRobots::type_name().contains("ListRobots"));
        assert_eq!(ListRobots::short_name(), "ListRobots");
    }
}
