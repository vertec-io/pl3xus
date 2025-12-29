//! Core request handlers.

use bevy::ecs::message::MessageReader;
use bevy::prelude::*;
use pl3xus::managers::network_request::Request;

use crate::{DatabaseInitRegistry, DatabaseResource, ResetDatabase, ResetDatabaseResponse};

/// Handle ResetDatabase request - drops all tables and reinitializes.
pub fn handle_reset_database(
    mut requests: MessageReader<Request<ResetDatabase>>,
    db: Option<Res<DatabaseResource>>,
    registry: Res<DatabaseInitRegistry>,
) {
    for request in requests.read() {
        info!("📋 Handling ResetDatabase - dropping and reinitializing all tables");

        let result = match db.as_ref() {
            Some(db_res) => {
                // Get list of all tables except sqlite internals
                let conn = db_res.connection();
                let conn = conn.lock().unwrap();
                
                // Get all table names
                let tables: Vec<String> = {
                    let mut stmt = conn.prepare(
                        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'"
                    ).unwrap();
                    stmt.query_map([], |row| row.get(0))
                        .unwrap()
                        .filter_map(|r| r.ok())
                        .collect()
                };
                
                // Drop all tables
                for table in &tables {
                    if let Err(e) = conn.execute(&format!("DROP TABLE IF EXISTS \"{}\"", table), []) {
                        error!("Failed to drop table {}: {}", table, e);
                    }
                }
                info!("Dropped {} tables", tables.len());
                drop(conn);
                
                // Reinitialize all schemas
                db_res.init_all(&registry)
            }
            None => Err(anyhow::anyhow!("Database not available")),
        };

        let response = match result {
            Ok(_) => {
                info!("✅ Database reset complete");
                ResetDatabaseResponse { success: true, error: None }
            }
            Err(e) => {
                error!("❌ Database reset failed: {}", e);
                ResetDatabaseResponse { success: false, error: Some(e.to_string()) }
            }
        };

        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

