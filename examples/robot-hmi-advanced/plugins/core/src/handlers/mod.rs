//! Core request handlers.
//!
//! This module contains handlers for core functionality:
//! - Database management (reset)

mod database;

pub use database::handle_reset_database;