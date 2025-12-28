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
        // Programs table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS programs (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                default_w REAL NOT NULL DEFAULT 0.0,
                default_p REAL NOT NULL DEFAULT 0.0,
                default_r REAL NOT NULL DEFAULT 0.0,
                default_speed REAL,
                default_speed_type TEXT NOT NULL DEFAULT 'mmSec',
                default_term_type TEXT NOT NULL DEFAULT 'CNT',
                default_term_value INTEGER NOT NULL DEFAULT 100,
                default_uframe INTEGER,
                default_utool INTEGER,
                start_x REAL,
                start_y REAL,
                start_z REAL,
                start_w REAL,
                start_p REAL,
                start_r REAL,
                end_x REAL,
                end_y REAL,
                end_z REAL,
                end_w REAL,
                end_p REAL,
                end_r REAL,
                move_speed REAL NOT NULL DEFAULT 100.0,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        // Program instructions table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS program_instructions (
                id INTEGER PRIMARY KEY,
                program_id INTEGER NOT NULL,
                line_number INTEGER NOT NULL,
                x REAL NOT NULL,
                y REAL NOT NULL,
                z REAL NOT NULL,
                w REAL,
                p REAL,
                r REAL,
                ext1 REAL,
                ext2 REAL,
                ext3 REAL,
                speed REAL,
                speed_type TEXT,
                term_type TEXT,
                term_value INTEGER,
                uframe INTEGER,
                utool INTEGER,
                FOREIGN KEY (program_id) REFERENCES programs(id) ON DELETE CASCADE
            )",
            [],
        )?;

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

    fn run_migrations(&self, conn: &Connection) -> anyhow::Result<()> {
        // Migration: Add default_speed_type column to programs table if missing
        let has_speed_type: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM pragma_table_info('programs') WHERE name = 'default_speed_type'",
            [],
            |row| row.get(0),
        )?;

        if !has_speed_type {
            conn.execute(
                "ALTER TABLE programs ADD COLUMN default_speed_type TEXT DEFAULT 'mmSec'",
                [],
            )?;
        }

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

