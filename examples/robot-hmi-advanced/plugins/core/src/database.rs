//! Core database module - provides the database resource and initialization trait.
//!
//! This module provides:
//! - `DatabaseResource` - SQLite connection wrapper
//! - `DatabaseInit` trait - For plugins to register their schemas
//! - `DatabaseInitRegistry` - Resource holding all database initializers

use bevy::prelude::*;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

/// Database resource providing SQLite connection.
#[derive(Resource)]
pub struct DatabaseResource(pub Arc<Mutex<Connection>>);

impl DatabaseResource {
    /// Open a database connection.
    pub fn open(path: &str) -> anyhow::Result<Self> {
        let conn = Connection::open(path)?;
        Ok(Self(Arc::new(Mutex::new(conn))))
    }

    /// Get a reference to the connection for direct queries.
    pub fn connection(&self) -> Arc<Mutex<Connection>> {
        Arc::clone(&self.0)
    }

    /// Initialize all registered plugins' schemas.
    pub fn init_all(&self, registry: &DatabaseInitRegistry) -> anyhow::Result<()> {
        let conn = self.0.lock().unwrap();
        
        // Create core schema version table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS _schema_versions (
                plugin TEXT PRIMARY KEY,
                version INTEGER NOT NULL DEFAULT 1,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        
        // Initialize each plugin's schema
        for init in &registry.initializers {
            info!("📦 Initializing database schema for: {}", init.name());
            init.init_schema(&conn)?;
            init.run_migrations(&conn)?;
            init.seed_data(&conn)?;
            
            // Record schema version
            conn.execute(
                "INSERT OR REPLACE INTO _schema_versions (plugin, version, updated_at) 
                 VALUES (?, 1, CURRENT_TIMESTAMP)",
                [init.name()],
            )?;
        }
        
        Ok(())
    }
}

/// Trait for plugins to register their database schemas and migrations.
pub trait DatabaseInit: Send + Sync + 'static {
    /// Plugin name for logging and error messages.
    fn name(&self) -> &'static str;
    
    /// Initialize schema (CREATE TABLE IF NOT EXISTS).
    fn init_schema(&self, conn: &Connection) -> anyhow::Result<()>;
    
    /// Run migrations for existing databases.
    fn run_migrations(&self, conn: &Connection) -> anyhow::Result<()> {
        let _ = conn; // Suppress unused warning
        Ok(())
    }
    
    /// Insert seed data (INSERT OR IGNORE).
    fn seed_data(&self, conn: &Connection) -> anyhow::Result<()> {
        let _ = conn; // Suppress unused warning
        Ok(())
    }
}

/// Resource holding all database initializers.
/// Plugins add their initializers during plugin build.
#[derive(Resource, Default)]
pub struct DatabaseInitRegistry {
    pub initializers: Vec<Box<dyn DatabaseInit>>,
}

impl DatabaseInitRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self { initializers: Vec::new() }
    }
    
    /// Register a database initializer.
    pub fn register(&mut self, init: impl DatabaseInit) {
        self.initializers.push(Box::new(init));
    }
}

