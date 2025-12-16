use bevy::prelude::*;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

use fanuc_replica_types::*;

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
        conn.execute(
            "CREATE TABLE IF NOT EXISTS programs (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                default_w REAL DEFAULT 0.0,
                default_p REAL DEFAULT 0.0,
                default_r REAL DEFAULT 0.0,
                default_speed REAL,
                default_term_type TEXT DEFAULT 'CNT',
                default_term_value INTEGER DEFAULT 100,
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
                move_speed REAL DEFAULT 100.0,
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
                default_joint_jog_speed REAL DEFAULT 0.1,
                default_joint_jog_step REAL DEFAULT 0.25,
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

        Ok(())
    }

    // ==================== Robot Connections ====================
    pub fn list_robot_connections(&self) -> anyhow::Result<Vec<RobotConnectionDto>> {
        let conn = self.0.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, ip_address, port,
                    COALESCE(default_speed, 100.0), COALESCE(default_speed_type, 'mmSec'),
                    COALESCE(default_term_type, 'CNT'),
                    COALESCE(default_w, 0.0), COALESCE(default_p, 0.0), COALESCE(default_r, 0.0),
                    COALESCE(default_cartesian_jog_speed, 10.0), COALESCE(default_cartesian_jog_step, 1.0),
                    COALESCE(default_joint_jog_speed, 0.1), COALESCE(default_joint_jog_step, 0.25)
             FROM robot_connections ORDER BY name"
        )?;

        let connections = stmt.query_map([], |row| {
            Ok(RobotConnectionDto {
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
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(connections)
    }

    pub fn get_robot_connection(&self, id: i64) -> anyhow::Result<Option<RobotConnectionDto>> {
        let connections = self.list_robot_connections()?;
        Ok(connections.into_iter().find(|c| c.id == id))
    }

    // ==================== Robot Configurations ====================
    pub fn get_configurations_for_robot(&self, robot_id: i64) -> anyhow::Result<Vec<RobotConfigurationDto>> {
        let conn = self.0.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, robot_connection_id, name, is_default, u_frame_number, u_tool_number,
                    front, up, left, flip, turn4, turn5, turn6
             FROM robot_configurations WHERE robot_connection_id = ? ORDER BY name"
        )?;

        let configs = stmt.query_map([robot_id], |row| {
            Ok(RobotConfigurationDto {
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

        let program = conn.query_row(
            "SELECT id, name, description, default_term_type, default_term_value,
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
                    default_term_type: row.get::<_, Option<String>>(3)?.unwrap_or("CNT".to_string()),
                    default_term_value: row.get(4)?,
                    start_x: row.get(5)?,
                    start_y: row.get(6)?,
                    start_z: row.get(7)?,
                    start_w: row.get(8)?,
                    start_p: row.get(9)?,
                    start_r: row.get(10)?,
                    end_x: row.get(11)?,
                    end_y: row.get(12)?,
                    end_z: row.get(13)?,
                    end_w: row.get(14)?,
                    end_p: row.get(15)?,
                    end_r: row.get(16)?,
                    move_speed: row.get(17)?,
                    created_at: row.get(18)?,
                    updated_at: row.get(19)?,
                })
            },
        ).ok();

        if let Some(mut prog) = program {
            let mut stmt = conn.prepare(
                "SELECT line_number, x, y, z, w, p, r, speed, term_type, term_value, uframe, utool
                 FROM program_instructions WHERE program_id = ? ORDER BY line_number"
            )?;

            let instructions: Vec<InstructionDto> = stmt.query_map([id], |row| {
                Ok(InstructionDto {
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

    pub fn create_program(&self, name: &str, description: Option<&str>) -> anyhow::Result<i64> {
        let conn = self.0.lock().unwrap();
        conn.execute(
            "INSERT INTO programs (name, description) VALUES (?, ?)",
            [name, description.unwrap_or("")],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn delete_program(&self, id: i64) -> anyhow::Result<()> {
        let conn = self.0.lock().unwrap();
        conn.execute("DELETE FROM program_instructions WHERE program_id = ?", [id])?;
        conn.execute("DELETE FROM programs WHERE id = ?", [id])?;
        Ok(())
    }

    pub fn insert_instructions(&self, program_id: i64, instructions: &[InstructionDto]) -> anyhow::Result<()> {
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
}
