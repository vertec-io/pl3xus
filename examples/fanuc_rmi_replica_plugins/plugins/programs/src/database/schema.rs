//! Programs database schema initialization.

use fanuc_replica_core::DatabaseInit;
use rusqlite::Connection;

/// Programs plugin database initializer.
pub struct ProgramsDatabaseInit;

impl DatabaseInit for ProgramsDatabaseInit {
    fn name(&self) -> &'static str {
        "programs"
    }

    fn init_schema(&self, conn: &Connection) -> anyhow::Result<()> {
        // Programs table - device-agnostic program metadata
        conn.execute(
            "CREATE TABLE IF NOT EXISTS programs (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                
                -- Default values for missing instruction fields
                default_speed REAL,
                default_term_type TEXT,
                default_term_value INTEGER,
                
                -- Approach/retreat move speed
                move_speed REAL NOT NULL DEFAULT 100.0,
                
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        // Program sequences table - supports multiple approach/retreat sequences
        conn.execute(
            "CREATE TABLE IF NOT EXISTS program_sequences (
                id INTEGER PRIMARY KEY,
                program_id INTEGER NOT NULL,
                sequence_type TEXT NOT NULL CHECK (sequence_type IN ('approach', 'main', 'retreat')),
                name TEXT,
                order_index INTEGER NOT NULL DEFAULT 0,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (program_id) REFERENCES programs(id) ON DELETE CASCADE,
                UNIQUE(program_id, sequence_type, order_index)
            )",
            [],
        )?;

        // Create index for faster sequence lookups
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_sequences_program 
             ON program_sequences(program_id, sequence_type, order_index)",
            [],
        )?;

        // Program instructions table - device-agnostic instructions
        // Note: Frame references (uframe/utool) are intentionally NOT stored here.
        // Programs are device-agnostic; frame selection happens at execution time
        // using the device's active configuration.
        conn.execute(
            "CREATE TABLE IF NOT EXISTS program_instructions (
                id INTEGER PRIMARY KEY,
                sequence_id INTEGER NOT NULL,
                line_number INTEGER NOT NULL,

                -- Required position
                x REAL NOT NULL,
                y REAL NOT NULL,
                z REAL NOT NULL,

                -- Optional rotations
                w REAL,
                p REAL,
                r REAL,

                -- Optional extra axes
                ext1 REAL,
                ext2 REAL,
                ext3 REAL,

                -- Motion parameters
                speed REAL,
                term_type TEXT,
                term_value INTEGER,

                FOREIGN KEY (sequence_id) REFERENCES program_sequences(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Create index for faster instruction lookups
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_instructions_sequence 
             ON program_instructions(sequence_id, line_number)",
            [],
        )?;

        Ok(())
    }

    fn run_migrations(&self, _conn: &Connection) -> anyhow::Result<()> {
        // No migrations needed - uframe/utool removed as part of device-agnostic design
        // Frame selection happens at execution time using device's active configuration
        Ok(())
    }

    fn seed_data(&self, _conn: &Connection) -> anyhow::Result<()> {
        // No seed data needed
        Ok(())
    }
}

