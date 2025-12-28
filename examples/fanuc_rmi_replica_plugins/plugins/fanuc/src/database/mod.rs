//! FANUC plugin database module.
//!
//! This module provides:
//! - Database schema initialization
//! - Query functions for FANUC-specific data
//! - Migrations
//!
//! Query functions are plain functions that take a rusqlite Connection,
//! making them easy to test and use from any context.

mod schema;
mod queries;

pub use schema::FanucDatabaseInit;

// Re-export all query functions
pub use queries::{
    // Robot Connections
    list_robot_connections,
    get_robot_connection,
    create_robot_connection,
    update_robot_connection,
    delete_robot_connection,
    // Robot Configurations
    get_configurations_for_robot,
    get_configuration,
    get_default_configuration_for_robot,
    create_configuration,
    update_configuration,
    delete_configuration,
    set_default_configuration,
    save_current_configuration,
    // Programs
    list_programs,
    get_program,
    create_program,
    delete_program,
    update_program_settings,
    insert_instructions,
    // Settings
    get_settings,
    update_settings,
    reset_database,
    // I/O Config
    get_io_config,
    update_io_config,
};

