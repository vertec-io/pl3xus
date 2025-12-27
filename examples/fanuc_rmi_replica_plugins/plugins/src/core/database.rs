//! Core database module - provides the database resource and all database operations.
//!
//! This module provides the DatabaseResource used by both core and robot plugins.

use bevy::prelude::*;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

use crate::robot::types::*;

/// Database resource providing SQLite connection pool.
#[derive(Resource)]
pub struct DatabaseResource(pub Arc<Mutex<Connection>>);

impl DatabaseResource {
    pub fn open(path: &str) -> anyhow::Result<Self> {
        let conn = Connection::open(path)?;
        Ok(Self(Arc::new(Mutex::new(conn))))
    }

    pub fn init_schema(&self) -> anyhow::Result<()> {
        let conn = self.0.lock().unwrap();

        // Programs table - complete schema matching Fanuc_RMI_API
        // Required fields are NOT NULL - no silent defaults, values must be explicitly set
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
                default_joint_jog_speed REAL DEFAULT 10.0,  -- °/s
                default_joint_jog_step REAL DEFAULT 1.0,    -- degrees
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

        // Run migrations for existing databases
        Self::run_migrations(&conn)?;

        Ok(())
    }

    /// Run schema migrations for existing databases
    fn run_migrations(conn: &Connection) -> anyhow::Result<()> {
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

    // ==================== Robot Connections ====================
    pub fn list_robot_connections(&self) -> anyhow::Result<Vec<RobotConnection>> {
        let conn = self.0.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, ip_address, port,
                    COALESCE(default_speed, 100.0), COALESCE(default_speed_type, 'mmSec'),
                    COALESCE(default_term_type, 'CNT'),
                    COALESCE(default_w, 0.0), COALESCE(default_p, 0.0), COALESCE(default_r, 0.0),
                    COALESCE(default_cartesian_jog_speed, 10.0), COALESCE(default_cartesian_jog_step, 1.0),
                    COALESCE(default_joint_jog_speed, 0.1), COALESCE(default_joint_jog_step, 0.25),
                    COALESCE(default_rotation_jog_speed, 5.0), COALESCE(default_rotation_jog_step, 1.0)
             FROM robot_connections ORDER BY name"
        )?;

        let connections = stmt.query_map([], |row| {
            Ok(RobotConnection {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                ip_address: row.get(3)?,
                port: row.get(4)?,
                default_speed: row.get(5)?,
                default_speed_type: row.get(6)?,
                default_term_type: row.get(7)?,
                default_w: row.get(8)?,
                default_p: row.get(9)?,
                default_r: row.get(10)?,
                default_cartesian_jog_speed: row.get(11)?,
                default_cartesian_jog_step: row.get(12)?,
                default_joint_jog_speed: row.get(13)?,
                default_joint_jog_step: row.get(14)?,
                default_rotation_jog_speed: row.get(15)?,
                default_rotation_jog_step: row.get(16)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(connections)
    }

    #[allow(dead_code)]
    pub fn get_robot_connection(&self, id: i64) -> anyhow::Result<Option<RobotConnection>> {
        let connections = self.list_robot_connections()?;
        Ok(connections.into_iter().find(|c| c.id == id))
    }

    pub fn create_robot_connection(&self, req: &CreateRobotConnection) -> anyhow::Result<i64> {
        let conn = self.0.lock().unwrap();

        // Insert robot connection
        conn.execute(
            "INSERT INTO robot_connections (name, description, ip_address, port,
             default_speed, default_speed_type, default_term_type,
             default_w, default_p, default_r,
             default_cartesian_jog_speed, default_cartesian_jog_step,
             default_joint_jog_speed, default_joint_jog_step,
             default_rotation_jog_speed, default_rotation_jog_step)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                req.name,
                req.description,
                req.ip_address,
                req.port,
                req.default_speed,
                req.default_speed_type,
                req.default_term_type,
                req.default_w,
                req.default_p,
                req.default_r,
                req.default_cartesian_jog_speed,
                req.default_cartesian_jog_step,
                req.default_joint_jog_speed,
                req.default_joint_jog_step,
                req.default_rotation_jog_speed,
                req.default_rotation_jog_step,
            ],
        )?;

        let robot_id = conn.last_insert_rowid();

        // Insert default configuration
        let cfg = &req.configuration;
        conn.execute(
            "INSERT INTO robot_configurations
             (robot_connection_id, name, is_default, u_frame_number, u_tool_number,
              front, up, left, flip, turn4, turn5, turn6)
             VALUES (?, ?, 1, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                robot_id,
                cfg.name,
                cfg.u_frame_number,
                cfg.u_tool_number,
                cfg.front,
                cfg.up,
                cfg.left,
                cfg.flip,
                cfg.turn4,
                cfg.turn5,
                cfg.turn6,
            ],
        )?;

        Ok(robot_id)
    }

    pub fn update_robot_connection(&self, req: &UpdateRobotConnection) -> anyhow::Result<()> {
        let conn = self.0.lock().unwrap();

        // Build dynamic update query based on which fields are provided
        let mut updates = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref name) = req.name {
            updates.push("name = ?");
            params.push(Box::new(name.clone()) as Box<dyn rusqlite::ToSql>);
        }
        if let Some(ref description) = req.description {
            updates.push("description = ?");
            params.push(Box::new(description.clone()) as Box<dyn rusqlite::ToSql>);
        }
        if let Some(ref ip_address) = req.ip_address {
            updates.push("ip_address = ?");
            params.push(Box::new(ip_address.clone()) as Box<dyn rusqlite::ToSql>);
        }
        if let Some(port) = req.port {
            updates.push("port = ?");
            params.push(Box::new(port) as Box<dyn rusqlite::ToSql>);
        }

        if updates.is_empty() {
            return Ok(());
        }

        let sql = format!(
            "UPDATE robot_connections SET {} WHERE id = ?",
            updates.join(", ")
        );
        params.push(Box::new(req.id));

        let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, params_refs.as_slice())?;

        Ok(())
    }

    pub fn delete_robot_connection(&self, id: i64) -> anyhow::Result<()> {
        let conn = self.0.lock().unwrap();

        // Delete configurations first (foreign key)
        conn.execute("DELETE FROM robot_configurations WHERE robot_connection_id = ?", [id])?;

        // Delete the robot connection
        conn.execute("DELETE FROM robot_connections WHERE id = ?", [id])?;

        Ok(())
    }

    // ==================== Robot Configurations ====================
    pub fn get_configurations_for_robot(&self, robot_id: i64) -> anyhow::Result<Vec<RobotConfiguration>> {
        let conn = self.0.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, robot_connection_id, name, is_default, u_frame_number, u_tool_number,
                    front, up, left, flip, turn4, turn5, turn6
             FROM robot_configurations WHERE robot_connection_id = ? ORDER BY name"
        )?;

        let configs = stmt.query_map([robot_id], |row| {
            Ok(RobotConfiguration {
                id: row.get(0)?,
                robot_connection_id: row.get(1)?,
                name: row.get(2)?,
                is_default: row.get::<_, i32>(3)? != 0,
                u_frame_number: row.get(4)?,
                u_tool_number: row.get(5)?,
                front: row.get(6)?,
                up: row.get(7)?,
                left: row.get(8)?,
                flip: row.get(9)?,
                turn4: row.get(10)?,
                turn5: row.get(11)?,
                turn6: row.get(12)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(configs)
    }

    pub fn get_configuration(&self, config_id: i64) -> anyhow::Result<RobotConfiguration> {
        let conn = self.0.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, robot_connection_id, name, is_default, u_frame_number, u_tool_number,
                    front, up, left, flip, turn4, turn5, turn6
             FROM robot_configurations WHERE id = ?"
        )?;

        let config = stmt.query_row([config_id], |row| {
            Ok(RobotConfiguration {
                id: row.get(0)?,
                robot_connection_id: row.get(1)?,
                name: row.get(2)?,
                is_default: row.get::<_, i32>(3)? != 0,
                u_frame_number: row.get(4)?,
                u_tool_number: row.get(5)?,
                front: row.get(6)?,
                up: row.get(7)?,
                left: row.get(8)?,
                flip: row.get(9)?,
                turn4: row.get(10)?,
                turn5: row.get(11)?,
                turn6: row.get(12)?,
            })
        })?;

        Ok(config)
    }

    /// Get the default configuration for a robot connection.
    /// Returns None if no default configuration is set.
    pub fn get_default_configuration_for_robot(&self, robot_connection_id: i64) -> anyhow::Result<Option<RobotConfiguration>> {
        let conn = self.0.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, robot_connection_id, name, is_default, u_frame_number, u_tool_number,
                    front, up, left, flip, turn4, turn5, turn6
             FROM robot_configurations WHERE robot_connection_id = ? AND is_default = 1"
        )?;

        let config = stmt.query_row([robot_connection_id], |row| {
            Ok(RobotConfiguration {
                id: row.get(0)?,
                robot_connection_id: row.get(1)?,
                name: row.get(2)?,
                is_default: row.get::<_, i32>(3)? != 0,
                u_frame_number: row.get(4)?,
                u_tool_number: row.get(5)?,
                front: row.get(6)?,
                up: row.get(7)?,
                left: row.get(8)?,
                flip: row.get(9)?,
                turn4: row.get(10)?,
                turn5: row.get(11)?,
                turn6: row.get(12)?,
            })
        });

        match config {
            Ok(c) => Ok(Some(c)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn create_configuration(&self, req: &CreateConfiguration) -> anyhow::Result<i64> {
        let conn = self.0.lock().unwrap();

        // If setting as default, clear other defaults for this robot
        if req.is_default {
            conn.execute(
                "UPDATE robot_configurations SET is_default = 0 WHERE robot_connection_id = ?",
                [req.robot_connection_id],
            )?;
        }

        conn.execute(
            "INSERT INTO robot_configurations
             (robot_connection_id, name, is_default, u_frame_number, u_tool_number,
              front, up, left, flip, turn4, turn5, turn6)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                req.robot_connection_id,
                req.name,
                req.is_default as i32,
                req.u_frame_number,
                req.u_tool_number,
                req.front,
                req.up,
                req.left,
                req.flip,
                req.turn4,
                req.turn5,
                req.turn6,
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    pub fn update_configuration(&self, req: &UpdateConfiguration) -> anyhow::Result<()> {
        let conn = self.0.lock().unwrap();

        // If setting as default, first get the robot_connection_id and clear others
        if req.is_default == Some(true) {
            let robot_id: i64 = conn.query_row(
                "SELECT robot_connection_id FROM robot_configurations WHERE id = ?",
                [req.id],
                |row| row.get(0),
            )?;
            conn.execute(
                "UPDATE robot_configurations SET is_default = 0 WHERE robot_connection_id = ?",
                [robot_id],
            )?;
        }

        // Build dynamic update
        let mut updates = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref name) = req.name {
            updates.push("name = ?");
            params.push(Box::new(name.clone()));
        }
        if let Some(is_default) = req.is_default {
            updates.push("is_default = ?");
            params.push(Box::new(is_default as i32));
        }
        if let Some(v) = req.u_frame_number {
            updates.push("u_frame_number = ?");
            params.push(Box::new(v));
        }
        if let Some(v) = req.u_tool_number {
            updates.push("u_tool_number = ?");
            params.push(Box::new(v));
        }
        if let Some(v) = req.front {
            updates.push("front = ?");
            params.push(Box::new(v));
        }
        if let Some(v) = req.up {
            updates.push("up = ?");
            params.push(Box::new(v));
        }
        if let Some(v) = req.left {
            updates.push("left = ?");
            params.push(Box::new(v));
        }
        if let Some(v) = req.flip {
            updates.push("flip = ?");
            params.push(Box::new(v));
        }
        if let Some(v) = req.turn4 {
            updates.push("turn4 = ?");
            params.push(Box::new(v));
        }
        if let Some(v) = req.turn5 {
            updates.push("turn5 = ?");
            params.push(Box::new(v));
        }
        if let Some(v) = req.turn6 {
            updates.push("turn6 = ?");
            params.push(Box::new(v));
        }

        if updates.is_empty() {
            return Ok(());
        }

        params.push(Box::new(req.id));
        let sql = format!(
            "UPDATE robot_configurations SET {} WHERE id = ?",
            updates.join(", ")
        );

        let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, params_refs.as_slice())?;

        Ok(())
    }

    pub fn delete_configuration(&self, id: i64) -> anyhow::Result<()> {
        let conn = self.0.lock().unwrap();
        conn.execute("DELETE FROM robot_configurations WHERE id = ?", [id])?;
        Ok(())
    }

    pub fn set_default_configuration(&self, id: i64) -> anyhow::Result<()> {
        let conn = self.0.lock().unwrap();

        // Get the robot_connection_id for this configuration
        let robot_id: i64 = conn.query_row(
            "SELECT robot_connection_id FROM robot_configurations WHERE id = ?",
            [id],
            |row| row.get(0),
        )?;

        // Clear all defaults for this robot
        conn.execute(
            "UPDATE robot_configurations SET is_default = 0 WHERE robot_connection_id = ?",
            [robot_id],
        )?;

        // Set the new default
        conn.execute(
            "UPDATE robot_configurations SET is_default = 1 WHERE id = ?",
            [id],
        )?;

        Ok(())
    }

    /// Save the current active configuration to the database.
    /// If `name` is provided, creates a new configuration with that name.
    /// If `name` is None and `loaded_from_id` is Some, updates the existing configuration.
    /// Returns (config_id, config_name) on success.
    pub fn save_current_configuration(
        &self,
        robot_connection_id: i64,
        loaded_from_id: Option<i64>,
        name: Option<String>,
        active_config: &ActiveConfigState,
    ) -> anyhow::Result<(i64, String)> {
        let conn = self.0.lock().unwrap();

        if let Some(new_name) = name {
            // Create a new configuration
            conn.execute(
                "INSERT INTO robot_configurations
                 (robot_connection_id, name, is_default, u_frame_number, u_tool_number,
                  front, up, left, flip, turn4, turn5, turn6)
                 VALUES (?, ?, 0, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                rusqlite::params![
                    robot_connection_id,
                    new_name,
                    active_config.u_frame_number,
                    active_config.u_tool_number,
                    active_config.front,
                    active_config.up,
                    active_config.left,
                    active_config.flip,
                    active_config.turn4,
                    active_config.turn5,
                    active_config.turn6,
                ],
            )?;
            let new_id = conn.last_insert_rowid();
            Ok((new_id, new_name))
        } else if let Some(existing_id) = loaded_from_id {
            // Update the existing configuration
            conn.execute(
                "UPDATE robot_configurations SET
                 u_frame_number = ?, u_tool_number = ?,
                 front = ?, up = ?, left = ?, flip = ?,
                 turn4 = ?, turn5 = ?, turn6 = ?
                 WHERE id = ?",
                rusqlite::params![
                    active_config.u_frame_number,
                    active_config.u_tool_number,
                    active_config.front,
                    active_config.up,
                    active_config.left,
                    active_config.flip,
                    active_config.turn4,
                    active_config.turn5,
                    active_config.turn6,
                    existing_id,
                ],
            )?;
            // Get the name of the updated configuration
            let config_name: String = conn.query_row(
                "SELECT name FROM robot_configurations WHERE id = ?",
                [existing_id],
                |row| row.get(0),
            )?;
            Ok((existing_id, config_name))
        } else {
            anyhow::bail!("No configuration name provided and no configuration is currently loaded")
        }
    }

    // ==================== Programs ====================
    pub fn list_programs(&self) -> anyhow::Result<Vec<ProgramInfo>> {
        let conn = self.0.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT p.id, p.name, p.description,
                    (SELECT COUNT(*) FROM program_instructions WHERE program_id = p.id) as inst_count,
                    p.created_at, p.updated_at
             FROM programs p ORDER BY p.name"
        )?;

        let programs = stmt.query_map([], |row| {
            Ok(ProgramInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                instruction_count: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(programs)
    }

    pub fn get_program(&self, id: i64) -> anyhow::Result<Option<ProgramDetail>> {
        let conn = self.0.lock().unwrap();

        // All required fields are NOT NULL in DB, so we read them directly
        let program = conn.query_row(
            "SELECT id, name, description,
                    default_w, default_p, default_r,
                    default_speed, default_speed_type, default_term_type, default_term_value,
                    default_uframe, default_utool,
                    start_x, start_y, start_z, start_w, start_p, start_r,
                    end_x, end_y, end_z, end_w, end_p, end_r, move_speed,
                    created_at, updated_at
             FROM programs WHERE id = ?",
            [id],
            |row| {
                Ok(ProgramDetail {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    instructions: Vec::new(), // Will be filled below
                    default_w: row.get(3)?,
                    default_p: row.get(4)?,
                    default_r: row.get(5)?,
                    default_speed: row.get(6)?,
                    default_speed_type: row.get(7)?,  // Required: NOT NULL in DB
                    default_term_type: row.get(8)?,   // Required: NOT NULL in DB
                    default_term_value: row.get(9)?,  // Required: NOT NULL in DB
                    default_uframe: row.get(10)?,
                    default_utool: row.get(11)?,
                    start_x: row.get(12)?,
                    start_y: row.get(13)?,
                    start_z: row.get(14)?,
                    start_w: row.get(15)?,
                    start_p: row.get(16)?,
                    start_r: row.get(17)?,
                    end_x: row.get(18)?,
                    end_y: row.get(19)?,
                    end_z: row.get(20)?,
                    end_w: row.get(21)?,
                    end_p: row.get(22)?,
                    end_r: row.get(23)?,
                    move_speed: row.get(24)?,         // Required: NOT NULL in DB
                    created_at: row.get(25)?,
                    updated_at: row.get(26)?,
                })
            },
        ).ok();

        if let Some(mut prog) = program {
            let mut stmt = conn.prepare(
                "SELECT line_number, x, y, z, w, p, r, speed, term_type, term_value, uframe, utool
                 FROM program_instructions WHERE program_id = ? ORDER BY line_number"
            )?;

            let instructions: Vec<Instruction> = stmt.query_map([id], |row| {
                Ok(Instruction {
                    line_number: row.get(0)?,
                    x: row.get(1)?,
                    y: row.get(2)?,
                    z: row.get(3)?,
                    w: row.get(4)?,
                    p: row.get(5)?,
                    r: row.get(6)?,
                    speed: row.get(7)?,
                    term_type: row.get(8)?,
                    term_value: row.get(9)?,
                    uframe: row.get(10)?,
                    utool: row.get(11)?,
                })
            })?.collect::<Result<Vec<_>, _>>()?;

            prog.instructions = instructions;
            Ok(Some(prog))
        } else {
            Ok(None)
        }
    }

    /// Create a new program with explicit default values.
    /// Required fields are set explicitly - we never rely on silent defaults.
    pub fn create_program(&self, name: &str, description: Option<&str>) -> anyhow::Result<i64> {
        let conn = self.0.lock().unwrap();
        // Explicitly set all required default values - no hidden defaults
        conn.execute(
            "INSERT INTO programs (
                name, description,
                default_w, default_p, default_r,
                default_speed_type, default_term_type, default_term_value,
                move_speed
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                name,
                description.unwrap_or(""),
                0.0_f64,           // default_w
                0.0_f64,           // default_p
                0.0_f64,           // default_r
                "mmSec",           // default_speed_type
                "CNT",             // default_term_type
                100_i32,           // default_term_value
                100.0_f64,         // move_speed
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn delete_program(&self, id: i64) -> anyhow::Result<()> {
        let conn = self.0.lock().unwrap();
        conn.execute("DELETE FROM program_instructions WHERE program_id = ?", [id])?;
        conn.execute("DELETE FROM programs WHERE id = ?", [id])?;
        Ok(())
    }

    pub fn update_program_settings(
        &self,
        program_id: i64,
        start_x: Option<f64>,
        start_y: Option<f64>,
        start_z: Option<f64>,
        start_w: Option<f64>,
        start_p: Option<f64>,
        start_r: Option<f64>,
        end_x: Option<f64>,
        end_y: Option<f64>,
        end_z: Option<f64>,
        end_w: Option<f64>,
        end_p: Option<f64>,
        end_r: Option<f64>,
        move_speed: Option<f64>,
        default_term_type: Option<String>,
        default_term_value: Option<u8>,
    ) -> anyhow::Result<()> {
        let conn = self.0.lock().unwrap();
        // Use COALESCE to preserve existing values when None is passed
        // This prevents accidental overwrites and maintains the "only update what changed" principle
        conn.execute(
            "UPDATE programs SET
                start_x = COALESCE(?, start_x), start_y = COALESCE(?, start_y), start_z = COALESCE(?, start_z),
                start_w = COALESCE(?, start_w), start_p = COALESCE(?, start_p), start_r = COALESCE(?, start_r),
                end_x = COALESCE(?, end_x), end_y = COALESCE(?, end_y), end_z = COALESCE(?, end_z),
                end_w = COALESCE(?, end_w), end_p = COALESCE(?, end_p), end_r = COALESCE(?, end_r),
                move_speed = COALESCE(?, move_speed),
                default_term_type = COALESCE(?, default_term_type),
                default_term_value = COALESCE(?, default_term_value),
                updated_at = CURRENT_TIMESTAMP
             WHERE id = ?",
            rusqlite::params![
                start_x, start_y, start_z,
                start_w, start_p, start_r,
                end_x, end_y, end_z,
                end_w, end_p, end_r,
                move_speed, default_term_type, default_term_value,
                program_id
            ],
        )?;
        Ok(())
    }

    pub fn insert_instructions(&self, program_id: i64, instructions: &[Instruction]) -> anyhow::Result<()> {
        let conn = self.0.lock().unwrap();
        // Clear existing
        conn.execute("DELETE FROM program_instructions WHERE program_id = ?", [program_id])?;

        let mut stmt = conn.prepare(
            "INSERT INTO program_instructions (program_id, line_number, x, y, z, w, p, r, speed, term_type, term_value, uframe, utool)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )?;

        for inst in instructions {
            stmt.execute(rusqlite::params![
                program_id,
                inst.line_number,
                inst.x,
                inst.y,
                inst.z,
                inst.w,
                inst.p,
                inst.r,
                inst.speed,
                inst.term_type,
                inst.term_value,
                inst.uframe,
                inst.utool,
            ])?;
        }

        Ok(())
    }

    // ==================== Settings ====================
    pub fn get_settings(&self) -> anyhow::Result<RobotSettings> {
        let conn = self.0.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT key, value FROM server_settings"
        )?;

        let mut settings = RobotSettings::default();
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })?;

        for row in rows {
            let (key, value) = row?;
            match key.as_str() {
                "default_w" => if let Some(v) = value { settings.default_w = v.parse().unwrap_or(0.0); }
                "default_p" => if let Some(v) = value { settings.default_p = v.parse().unwrap_or(0.0); }
                "default_r" => if let Some(v) = value { settings.default_r = v.parse().unwrap_or(0.0); }
                "default_speed" => if let Some(v) = value { settings.default_speed = v.parse().unwrap_or(100.0); }
                "default_term_type" => if let Some(v) = value { settings.default_term_type = v; }
                "default_uframe" => if let Some(v) = value { settings.default_uframe = v.parse().unwrap_or(1); }
                "default_utool" => if let Some(v) = value { settings.default_utool = v.parse().unwrap_or(1); }
                _ => {}
            }
        }

        Ok(settings)
    }

    pub fn update_settings(&self, settings: &RobotSettings) -> anyhow::Result<()> {
        let conn = self.0.lock().unwrap();

        let settings_pairs = [
            ("default_w", settings.default_w.to_string()),
            ("default_p", settings.default_p.to_string()),
            ("default_r", settings.default_r.to_string()),
            ("default_speed", settings.default_speed.to_string()),
            ("default_term_type", settings.default_term_type.clone()),
            ("default_uframe", settings.default_uframe.to_string()),
            ("default_utool", settings.default_utool.to_string()),
        ];

        for (key, value) in settings_pairs {
            conn.execute(
                "INSERT OR REPLACE INTO server_settings (key, value) VALUES (?, ?)",
                [key, &value],
            )?;
        }

        Ok(())
    }

    pub fn reset_database(&self) -> anyhow::Result<()> {
        let conn = self.0.lock().unwrap();

        // Delete all data from tables
        conn.execute("DELETE FROM program_instructions", [])?;
        conn.execute("DELETE FROM programs", [])?;
        conn.execute("DELETE FROM robot_configurations", [])?;
        conn.execute("DELETE FROM robot_connections", [])?;
        conn.execute("DELETE FROM io_display_config", [])?;
        conn.execute("DELETE FROM server_settings", [])?;

        // Re-insert default settings
        conn.execute(
            "INSERT INTO server_settings (key, value, description) VALUES
                ('theme', 'dark', 'UI theme: dark or light'),
                ('default_robot_id', NULL, 'Default robot connection on startup'),
                ('auto_connect', 'false', 'Auto-connect to default robot')",
            [],
        )?;

        Ok(())
    }

    // ==================== I/O Config ====================
    pub fn get_io_config(&self, robot_connection_id: i64) -> anyhow::Result<Vec<IoDisplayConfig>> {
        let conn = self.0.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT io_type, io_index, display_name, is_visible, display_order
             FROM io_display_config WHERE robot_connection_id = ?
             ORDER BY display_order, io_type, io_index"
        )?;

        let configs = stmt.query_map([robot_connection_id], |row| {
            Ok(IoDisplayConfig {
                io_type: row.get(0)?,
                io_index: row.get(1)?,
                display_name: row.get(2)?,
                is_visible: row.get::<_, i32>(3)? != 0,
                display_order: row.get(4)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(configs)
    }

    pub fn update_io_config(&self, robot_connection_id: i64, configs: &[IoDisplayConfig]) -> anyhow::Result<()> {
        let conn = self.0.lock().unwrap();

        for config in configs {
            conn.execute(
                "INSERT OR REPLACE INTO io_display_config
                 (robot_connection_id, io_type, io_index, display_name, is_visible, display_order)
                 VALUES (?, ?, ?, ?, ?, ?)",
                rusqlite::params![
                    robot_connection_id,
                    config.io_type,
                    config.io_index,
                    config.display_name,
                    config.is_visible as i32,
                    config.display_order,
                ],
            )?;
        }

        Ok(())
    }
}

/// System to initialize the database on startup.
pub fn init_database(mut commands: Commands) {
    // Use a default path - in production this would come from config
    let db_path = std::env::var("DATABASE_PATH")
        .unwrap_or_else(|_| "fanuc_replica.db".to_string());

    match DatabaseResource::open(&db_path) {
        Ok(db) => {
            if let Err(e) = db.init_schema() {
                error!("❌ Failed to initialize DB schema: {}", e);
            }
            info!("✅ Database opened at: {}", db_path);
            commands.insert_resource(db);
        }
        Err(e) => {
            error!("❌ Failed to open database: {}", e);
        }
    }
}
