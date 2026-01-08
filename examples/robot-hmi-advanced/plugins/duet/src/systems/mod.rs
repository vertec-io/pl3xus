//! Duet ECS systems.
//!
//! This module contains systems for:
//! - Command handling (processing AuxiliaryCommandEvents)
//! - HTTP communication with Duet controllers

mod command;

pub use command::{duet_command_handler_system, duet_http_sender_system};