//! Automatic query invalidation infrastructure.
//!
//! This module provides a declarative way to specify which queries should be
//! invalidated when mutations succeed. Instead of manually broadcasting
//! `QueryInvalidation` messages in every handler, you declare the relationships
//! at registration time.
//!
//! # Example
//!
//! ```rust,ignore
//! use pl3xus_sync::{AppInvalidationExt, MutationResponse};
//!
//! // Implement MutationResponse for your response types
//! impl MutationResponse for CreateProgramResponse {
//!     fn is_success(&self) -> bool {
//!         self.success
//!     }
//! }
//!
//! // Register invalidation rules in your plugin
//! impl Plugin for RequestHandlerPlugin {
//!     fn build(&self, app: &mut App) {
//!         app.invalidation_rules()
//!             .on_success::<CreateProgram>().invalidate("ListPrograms")
//!             .on_success::<DeleteProgram>().invalidate("ListPrograms");
//!     }
//! }
//! ```

use bevy::prelude::*;
use pl3xus_common::RequestMessage;
use std::collections::HashMap;
use std::sync::Arc;

use crate::messages::{QueryInvalidation, SyncServerMessage};



/// A single invalidation rule.
#[derive(Clone)]
pub struct InvalidationRule {
    /// The query type name to invalidate (e.g., "ListPrograms").
    pub query_type: String,
    /// Optional: function to extract keys from the request for keyed invalidation.
    pub key_extractor: Option<Arc<dyn Fn(&[u8]) -> Option<Vec<String>> + Send + Sync>>,
}

/// Storage for invalidation rules, keyed by request type.
#[derive(Resource, Default)]
pub struct InvalidationRules {
    /// Maps request type name -> list of invalidation rules
    rules: HashMap<String, Vec<InvalidationRule>>,
}

impl InvalidationRules {
    /// Get rules for a specific request type.
    pub fn get_rules(&self, request_type: &str) -> Option<&Vec<InvalidationRule>> {
        self.rules.get(request_type)
    }

    /// Add a rule for a request type.
    pub fn add_rule(&mut self, request_type: String, rule: InvalidationRule) {
        self.rules.entry(request_type).or_default().push(rule);
    }
}

/// Builder for configuring invalidation rules.
pub struct InvalidationRulesBuilder<'a> {
    app: &'a mut App,
}

impl<'a> InvalidationRulesBuilder<'a> {
    /// Start configuring invalidation for a specific request type.
    pub fn on_success<T: RequestMessage>(self) -> InvalidationRuleBuilder<'a, T> {
        InvalidationRuleBuilder {
            app: self.app,
            _marker: std::marker::PhantomData,
        }
    }
}

/// Builder for a single request type's invalidation rules.
pub struct InvalidationRuleBuilder<'a, T: RequestMessage> {
    app: &'a mut App,
    _marker: std::marker::PhantomData<T>,
}

impl<'a, T: RequestMessage> InvalidationRuleBuilder<'a, T> {
    /// Invalidate a query type when this request succeeds.
    pub fn invalidate(self, query_type: &str) -> InvalidationRulesBuilder<'a> {
        let request_type = std::any::type_name::<T>().to_string();
        
        // Ensure the resource exists
        if !self.app.world().contains_resource::<InvalidationRules>() {
            self.app.insert_resource(InvalidationRules::default());
        }
        
        // Add the rule
        self.app.world_mut().resource_mut::<InvalidationRules>().add_rule(
            request_type,
            InvalidationRule {
                query_type: query_type.to_string(),
                key_extractor: None,
            },
        );
        
        InvalidationRulesBuilder { app: self.app }
    }

    /// Invalidate a keyed query type when this request succeeds.
    /// The key_fn extracts the key from the request.
    pub fn invalidate_keyed<F>(self, query_type: &str, key_fn: F) -> InvalidationRulesBuilder<'a>
    where
        F: Fn(&T) -> String + Send + Sync + 'static,
    {
        let request_type = std::any::type_name::<T>().to_string();
        
        // Ensure the resource exists
        if !self.app.world().contains_resource::<InvalidationRules>() {
            self.app.insert_resource(InvalidationRules::default());
        }
        
        // Create a key extractor that deserializes the request
        let key_extractor: Arc<dyn Fn(&[u8]) -> Option<Vec<String>> + Send + Sync> = 
            Arc::new(move |bytes: &[u8]| {
                // Try to deserialize the request
                if let Ok((request, _)) = bincode::serde::decode_from_slice::<T, _>(
                    bytes,
                    bincode::config::standard(),
                ) {
                    Some(vec![key_fn(&request)])
                } else {
                    None
                }
            });
        
        // Add the rule
        self.app.world_mut().resource_mut::<InvalidationRules>().add_rule(
            request_type,
            InvalidationRule {
                query_type: query_type.to_string(),
                key_extractor: Some(key_extractor),
            },
        );
        
        InvalidationRulesBuilder { app: self.app }
    }
}

/// Extension trait for App to configure invalidation rules.
pub trait AppInvalidationExt {
    /// Start building invalidation rules.
    fn invalidation_rules(&mut self) -> InvalidationRulesBuilder<'_>;
}

impl AppInvalidationExt for App {
    fn invalidation_rules(&mut self) -> InvalidationRulesBuilder<'_> {
        // Ensure the resource exists
        if !self.world().contains_resource::<InvalidationRules>() {
            self.insert_resource(InvalidationRules::default());
        }
        InvalidationRulesBuilder { app: self }
    }
}

// =============================================================================
// Helper Functions for Handlers
// =============================================================================

use pl3xus::managers::NetworkProvider;
use pl3xus::Network;

/// Broadcast query invalidations for a successful mutation.
///
/// Call this after successfully responding to a mutation request.
/// This is a simpler alternative to `respond_with_invalidation` when you
/// need more control over the response flow.
///
/// # Example
///
/// ```rust,ignore
/// fn handle_create_program(...) {
///     for request in requests.read() {
///         let response = CreateProgramResponse { success: true, ... };
///         if let Ok(()) = request.respond(response.clone()) {
///             if response.success {
///                 broadcast_invalidations::<CreateProgram, _>(&net, &rules, None);
///             }
///         }
///     }
/// }
/// ```
pub fn broadcast_invalidations<T, NP>(
    net: &Network<NP>,
    rules: &InvalidationRules,
    keys: Option<Vec<String>>,
) where
    T: RequestMessage,
    NP: NetworkProvider,
{
    let request_type = std::any::type_name::<T>().to_string();

    if let Some(rule_list) = rules.get_rules(&request_type) {
        for rule in rule_list {
            let invalidation = QueryInvalidation {
                query_types: vec![rule.query_type.clone()],
                keys: keys.clone(),
            };
            net.broadcast(SyncServerMessage::QueryInvalidation(invalidation));
            debug!(
                "📢 Auto-invalidated query '{}' after successful {}",
                rule.query_type,
                request_type
            );
        }
    }
}

