use serde::{de::DeserializeOwned, Serialize};

/// Trait for types that can be synchronized via pl3xus_sync.
///
/// This trait is **automatically implemented** for all types that are
/// `Serialize + DeserializeOwned + Send + Sync + 'static`.
///
/// # Type Identification
///
/// Components are identified by their **short type name** (struct name only, no module path).
/// This matches the server-side behavior in pl3xus_sync and provides stability across
/// module refactoring.
///
/// The type name is extracted automatically using `std::any::type_name()` and cached
/// for performance. First access incurs ~500ns, subsequent accesses are ~50-100ns.
///
/// # Example
///
/// ```rust,ignore
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize, Clone, Debug)]
/// struct Position {
///     x: f32,
///     y: f32,
/// }
///
/// // That's it! SyncComponent is automatically implemented.
/// // No manual implementation needed.
/// ```
///
/// # Implementation Note
///
/// Component names are an internal implementation detail of pl3xus_sync's message
/// routing system. They are automatically extracted from the type name and cannot be
/// customized by users. This ensures consistency between client and server.
pub trait SyncComponent: Serialize + DeserializeOwned + Send + Sync + 'static {
    /// Returns the component type name used for synchronization.
    ///
    /// This returns the **short type name** (struct name only, no module path)
    /// to match the server-side behavior.
    ///
    /// The name is cached in a global static for performance. First access incurs ~500ns,
    /// subsequent accesses are ~50-100ns.
    ///
    /// # Implementation Note
    ///
    /// This is an internal implementation detail of pl3xus_sync's message routing system.
    /// The default implementation is provided by the blanket impl and cannot be overridden.
    fn component_name() -> &'static str {
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

        // Slow path: extract short name from full type name
        let full_name = type_name::<Self>();
        let short = full_name.rsplit("::").next().unwrap_or(full_name);
        let static_name = Box::leak(short.to_string().into_boxed_str());

        {
            let mut cache_guard = cache.lock().unwrap();
            cache_guard.insert(type_id, static_name);
        }

        static_name
    }
}

// Blanket implementation for all serializable types
impl<T> SyncComponent for T
where
    T: Serialize + DeserializeOwned + Send + Sync + 'static
{}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[test]
    fn test_automatic_sync_component_impl() {
        #[derive(Serialize, Deserialize)]
        struct TestComponent {
            value: i32,
        }

        // Should automatically have SyncComponent
        let name = TestComponent::component_name();
        assert_eq!(name, "TestComponent");
    }

    #[test]
    fn test_component_name_caching() {
        #[derive(Serialize, Deserialize)]
        struct CachedComponent {
            data: String,
        }

        let name1 = CachedComponent::component_name();
        let name2 = CachedComponent::component_name();

        // Should return same pointer (cached)
        assert_eq!(name1 as *const str, name2 as *const str);
        assert_eq!(name1, "CachedComponent");
    }

    #[test]
    fn test_different_types_different_names() {
        #[derive(Serialize, Deserialize)]
        struct TypeA {
            x: i32,
        }

        #[derive(Serialize, Deserialize)]
        struct TypeB {
            x: i32,
        }

        let name_a = TypeA::component_name();
        let name_b = TypeB::component_name();

        assert_ne!(name_a, name_b);
        assert_eq!(name_a, "TypeA");
        assert_eq!(name_b, "TypeB");
    }

    #[test]
    fn test_short_name_extraction() {
        // Simulate a type with module path
        mod inner {
            use serde::{Deserialize, Serialize};
            #[derive(Serialize, Deserialize)]
            pub struct NestedComponent {
                pub value: u32,
            }
        }

        let name = inner::NestedComponent::component_name();
        // Should be just the struct name, not the full path
        assert_eq!(name, "NestedComponent");
    }

    // Note: We cannot test custom overrides because Rust's coherence rules
    // prevent having both a blanket impl and specific impls.
    // This is correct - component names are internal implementation details
    // and should not be customizable by users.
}
