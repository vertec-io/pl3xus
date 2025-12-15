//! DevTools module for pl3xus_client
//!
//! This module provides a drop-in DevTools widget for debugging and inspecting
//! ECS entities synchronized via pl3xus_sync.
//!
//! ## Features
//!
//! - Hierarchical entity inspector
//! - Real-time component editing with mutations
//! - Controlled input pattern (prevents server updates during editing)
//! - Type registry for JSON serialization/deserialization
//!
//! ## Usage
//!
//! Enable the `devtools` feature in your Cargo.toml:
//!
//! ```toml
//! [dependencies]
//! pl3xus_client = { version = "0.1", features = ["devtools"] }
//! ```
//!
//! Then use the DevTools component in your Leptos app:
//!
//! ```rust,ignore
//! use pl3xus_client::devtools::DevTools;
//! use pl3xus_client::ClientTypeRegistry;
//!
//! let registry = ClientTypeRegistry::builder()
//!     .register::<Position>()
//!     .register::<Velocity>()
//!     .with_devtools_support()
//!     .build();
//!
//! view! {
//!     <DevTools ws_url="ws://127.0.0.1:3000/sync" registry=registry />
//! }
//! ```

mod sync;

#[cfg(target_arch = "wasm32")]
mod ui;

// Re-export public API
pub use sync::{DevtoolsSync, use_sync, MutationState};

#[cfg(target_arch = "wasm32")]
pub use ui::{DevTools, DevToolsMode};

// Re-export core wire-level types so downstream tools can depend on this
// crate alone for typical sync workflows.
pub use pl3xus_sync::{
    MutateComponent as SyncMutateComponent,
    MutationResponse as SyncMutationResponse,
    MutationStatus as SyncMutationStatus,
    SerializableEntity as SyncSerializableEntity,
    SyncBatch,
    SyncClientMessage as SyncClientMsg,
    SyncItem,
    SyncServerMessage as SyncServerMsg,
    SubscriptionRequest,
    UnsubscribeRequest,
};

