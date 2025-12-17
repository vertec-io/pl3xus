//! Fanuc RMI Replica Client
//!
//! A demonstration client for the pl3xus real-time synchronization framework.
//!
//! # Key Features
//! - **Zero boilerplate sync**: `use_sync_component::<T>()` handles subscription lifecycle
//! - **Clean RPC**: `ctx.send(msg)` sends typed messages with automatic serialization
//! - **Request/Response**: `use_request::<T>()` for database queries with React Query-style API

mod app;
mod components;
mod layout;
mod pages;

fn main() {
    console_error_panic_hook::set_once();
    _ = console_log::init_with_level(log::Level::Debug);

    leptos::mount::mount_to_body(|| leptos::view! { <app::App/> });
}
