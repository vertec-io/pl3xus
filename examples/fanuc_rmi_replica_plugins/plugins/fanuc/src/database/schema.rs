//! FANUC database schema initialization.

use fanuc_replica_core::DatabaseInit;
use rusqlite::Connection;

/// FANUC plugin database initializer.
pub struct FanucDatabaseInit;

impl DatabaseInit for FanucDatabaseInit {
    fn name(&self) -> &'static str {
        "fanuc"
    }

    fn init_schema(&self, conn: &Connection) -> anyhow::Result<()> {
        // Note: Program tables (programs, program_instructions) have been moved to
        // the programs crate. See fanuc_replica_programs::database::schema.

        // Robot connections table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS robot_connections (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                ip_address TEXT NOT NULL DEFAULT '127.0.0.1',
                port INTEGER NOT NULL DEFAULT 16001,
                default_speed REAL,
                default_speed_type TEXT,
                default_term_type TEXT,
                default_w REAL,
                default_p REAL,
                default_r REAL,
                default_cartesian_jog_speed REAL DEFAULT 10.0,
                default_cartesian_jog_step REAL DEFAULT 1.0,
                default_joint_jog_speed REAL DEFAULT 10.0,
                default_joint_jog_step REAL DEFAULT 1.0,
                default_rotation_jog_speed REAL DEFAULT 5.0,
                default_rotation_jog_step REAL DEFAULT 1.0,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        // Robot configurations table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS robot_configurations (
                id INTEGER PRIMARY KEY,
                robot_connection_id INTEGER NOT NULL,
                name TEXT NOT NULL,
                is_default INTEGER NOT NULL DEFAULT 0,
                u_frame_number INTEGER NOT NULL DEFAULT 1,
                u_tool_number INTEGER NOT NULL DEFAULT 1,
                front INTEGER NOT NULL DEFAULT 1,
                up INTEGER NOT NULL DEFAULT 1,
                left INTEGER NOT NULL DEFAULT 0,
                flip INTEGER NOT NULL DEFAULT 0,
                turn4 INTEGER NOT NULL DEFAULT 0,
                turn5 INTEGER NOT NULL DEFAULT 0,
                turn6 INTEGER NOT NULL DEFAULT 0,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (robot_connection_id) REFERENCES robot_connections(id) ON DELETE CASCADE,
                UNIQUE(robot_connection_id, name)
            )",
            [],
        )?;

        // I/O display configuration
        conn.execute(
            "CREATE TABLE IF NOT EXISTS io_display_config (
                id INTEGER PRIMARY KEY,
                robot_connection_id INTEGER NOT NULL,
                io_type TEXT NOT NULL,
                io_index INTEGER NOT NULL,
                display_name TEXT,
                is_visible INTEGER DEFAULT 1,
                display_order INTEGER,
                FOREIGN KEY (robot_connection_id) REFERENCES robot_connections(id) ON DELETE CASCADE,
                UNIQUE(robot_connection_id, io_type, io_index)
            )",
            [],
        )?;

        // Server settings
        conn.execute(
            "CREATE TABLE IF NOT EXISTS server_settings (
                id INTEGER PRIMARY KEY,
                key TEXT NOT NULL UNIQUE,
                value TEXT,
                description TEXT
            )",
            [],
        )?;

        Ok(())
    }

    fn run_migrations(&self, _conn: &Connection) -> anyhow::Result<()> {
        // Note: Program table migrations have been moved to the programs crate.
        // No fanuc-specific migrations needed at this time.
        Ok(())
    }

    fn seed_data(&self, conn: &Connection) -> anyhow::Result<()> {
        // Insert default settings if not exists
        conn.execute(
            "INSERT OR IGNORE INTO server_settings (key, value, description) VALUES
                ('theme', 'dark', 'UI theme: dark or light'),
                ('default_robot_id', NULL, 'Default robot connection on startup'),
                ('auto_connect', 'false', 'Auto-connect to default robot')",
            [],
        )?;

        // Insert default robot connection for testing
        conn.execute(
            "INSERT OR IGNORE INTO robot_connections (name, description, ip_address, port) VALUES
                ('Local Test Robot', 'Local test robot connection', '127.0.0.1', 16001),
                ('Shop Floor Robot 1', 'Main production robot', '192.168.1.100', 16001)",
            [],
        )?;

        // Insert default configuration for the first robot
        conn.execute(
            "INSERT OR IGNORE INTO robot_configurations (robot_connection_id, name, is_default, u_frame_number, u_tool_number)
             SELECT id, 'Default Config', 1, 1, 1 FROM robot_connections WHERE name = 'Local Test Robot'",
            [],
        )?;

        Ok(())
    }
}

