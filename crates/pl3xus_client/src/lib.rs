//! # Pl3xus Client
//!
//! High-level reactive client library for `pl3xus_sync` with Leptos integration.
//!
//! This library provides ergonomic hooks and components for building reactive web applications
//! that synchronize with Bevy ECS servers via `pl3xus_sync`.
//!
//! ## Features
//!
//! - **Automatic Subscription Management**: Subscribe to components with a single hook call
//! - **Subscription Deduplication**: Multiple components share subscriptions automatically
//! - **Lifecycle Management**: Auto-subscribe on mount, auto-unsubscribe on unmount
//! - **Reconnection Handling**: Automatic re-subscription on reconnect
//! - **Type Safety**: Compile-time type checking with Rust's type system
//! - **Dual API**: Support for both signals (atomic) and stores (fine-grained reactivity)
//!
//! ## Quick Start
//!
//! ### Read-Only Display
//!
//! ```rust,ignore
//! use leptos::prelude::*;
//! use pl3xus_client::{SyncProvider, use_sync_component, ClientTypeRegistry};
//!
//! #[component]
//! fn App() -> impl IntoView {
//!     let registry = ClientTypeRegistry::builder()
//!         .register::<Position>()
//!         .register::<Velocity>()
//!         .build();
//!
//!     view! {
//!         <SyncProvider url="ws://localhost:8080" registry=registry>
//!             <AppView/>
//!         </SyncProvider>
//!     }
//! }
//!
//! #[component]
//! fn AppView() -> impl IntoView {
//!     // Automatically subscribes, updates, and unsubscribes
//!     let positions = use_sync_component::<Position>();
//!
//!     view! {
//!         <For
//!             each=move || positions.get().iter().map(|(id, pos)| (*id, pos.clone())).collect::<Vec<_>>()
//!             key=|(id, _)| *id
//!             let:item
//!         >
//!             {
//!                 let (entity_id, position) = item;
//!                 view! {
//!                     <div>"Entity " {entity_id} ": " {format!("{:?}", position)}</div>
//!                 }
//!             }
//!         </For>
//!     }
//! }
//! ```
//!
//! ### Editable Fields with Focus Retention
//!
//! ```rust,ignore
//! use leptos::prelude::*;
//! use pl3xus_client::{SyncFieldInput, SyncComponent};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Clone, Default, Serialize, Deserialize)]
//! struct Position {
//!     x: f32,
//!     y: f32,
//! }
//!
//! // SyncComponent is automatically implemented!
//!
//! #[component]
//! fn PositionEditor(entity_id: u64) -> impl IntoView {
//!     view! {
//!         <div class="position-editor">
//!             <label>
//!                 "X: "
//!                 <SyncFieldInput
//!                     entity_id=entity_id
//!                     field_accessor=|pos: &Position| pos.x
//!                     field_mutator=|pos: &Position, new_x: f32| Position { x: new_x, y: pos.y }
//!                     input_type="number"
//!                     class="number-input"
//!                 />
//!             </label>
//!             <label>
//!                 "Y: "
//!                 <SyncFieldInput
//!                     entity_id=entity_id
//!                     field_accessor=|pos: &Position| pos.y
//!                     field_mutator=|pos: &Position, new_y: f32| Position { x: pos.x, y: new_y }
//!                     input_type="number"
//!                     class="number-input"
//!                 />
//!             </label>
//!         </div>
//!     }
//! }
//! ```
//!
//! The `SyncFieldInput` component implements:
//! - ✅ Focus retention through server updates
//! - ✅ User input preservation while focused
//! - ✅ Enter key to apply mutation
//! - ✅ Blur (click away) to revert to server value

// Module declarations
mod client_type_registry;
mod components;
mod context;
mod error;
mod hooks;
mod provider;
mod traits;

// Re-exports
pub use client_type_registry::{ClientTypeRegistry, ClientTypeRegistryBuilder};
pub use components::SyncFieldInput;
pub use context::{MutationState, RequestState, RequestStatus, SyncConnection, SyncContext, QueryCacheEntry, QueryCacheState};
pub use error::SyncError;

// New hook names (preferred)
pub use hooks::{
    use_components, use_components_where, use_connection, use_sync_context,
    use_entity, use_entity_component, use_entity_reactive,
    use_field_editor, use_message, use_mutations, use_untracked,
    use_request, use_request_with_handler, use_request_state,
    use_targeted_request, use_targeted_request_with_handler,
    UseRequestState, use_send_targeted,
    // TanStack Query-inspired mutation API
    use_mutation, use_mutation_targeted,
    MutationHandle, TargetedMutationHandle,
    // TanStack Query-inspired query API with server-side invalidation
    use_query, use_query_keyed, QueryHandle, QueryState,
};

// Deprecated hook names (for backwards compatibility)
#[allow(deprecated)]
pub use hooks::{
    use_sync_component, use_sync_component_where, use_sync_connection,
    use_sync_entity, use_sync_entity_component, use_sync_entity_reactive,
    use_sync_field_editor, use_sync_message, use_sync_mutations, use_sync_untracked,
};

// New store hook names (preferred)
#[cfg(feature = "stores")]
pub use hooks::{use_message_store, use_component_store, use_entity_component_store};

// Deprecated store hook names (for backwards compatibility)
#[cfg(feature = "stores")]
#[allow(deprecated)]
pub use hooks::{use_sync_message_store, use_sync_component_store, use_sync_entity_component_store};

pub use provider::SyncProvider;
pub use traits::SyncComponent;

// Re-export mutation types from pl3xus_sync for convenience
pub use pl3xus_sync::MutationStatus;

// Re-export control types from pl3xus_common for client-side use
pub use pl3xus_common::{ControlRequest, ControlResponse, EntityControl, ConnectionId};

// Re-export notification types from pl3xus_common for client-side use
pub use pl3xus_common::{NotificationLevel, ServerNotification};

// Re-export ConnectionReadyState for convenience
pub use leptos_use::core::ConnectionReadyState;

#[cfg(feature = "devtools")]
pub mod devtools;

#[cfg(all(feature = "devtools", target_arch = "wasm32"))]
pub use devtools::{DevTools, DevToolsMode};

// Deprecated native client module
#[deprecated(
    since = "0.1.0",
    note = "Use base `pl3xus` for Bevy-to-Bevy networking or `pl3xus_client::SyncProvider` for WASM/Leptos clients"
)]
pub mod native_client;

#[allow(deprecated)]
pub use native_client::{NativeMutationState, NativeSyncClient};

