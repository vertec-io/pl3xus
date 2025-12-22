//! Fanuc RMI Replica Client
//!
//! A demonstration client for the pl3xus real-time synchronization framework.
//!
//! # Key Features
//! - **Zero boilerplate sync**: `use_components::<T>()` handles subscription lifecycle
//! - **Clean RPC**: `ctx.send(msg)` sends typed messages with automatic serialization
//! - **Queries**: `use_query::<T>()` for read operations with auto-fetch and server-side invalidation
//! - **Mutations**: `use_mutation::<T>()` for write operations with response handling

mod app;
mod components;
mod layout;
mod pages;

fn main() {
    console_error_panic_hook::set_once();
    _ = console_log::init_with_level(log::Level::Debug);

    leptos::mount::mount_to_body(|| leptos::view! { <app::App/> });
}
