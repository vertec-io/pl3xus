//! FANUC database query functions.
//!
//! This module provides plain functions for database operations.
//! Each function takes a rusqlite Connection, making them easy to test
//! and use from any context.

use rusqlite::{Connection, OptionalExtension};
use crate::types::*;

// ==================== Robot Connections ====================

pub fn list_robot_connections(conn: &Connection) -> anyhow::Result<Vec<RobotConnection>> {
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

pub fn get_robot_connection(conn: &Connection, id: i64) -> anyhow::Result<Option<RobotConnection>> {
    let connections = list_robot_connections(conn)?;
    Ok(connections.into_iter().find(|c| c.id == id))
}

pub fn create_robot_connection(conn: &Connection, req: &CreateRobotConnection) -> anyhow::Result<i64> {
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

pub fn update_robot_connection(conn: &Connection, req: &UpdateRobotConnection) -> anyhow::Result<()> {
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

pub fn delete_robot_connection(conn: &Connection, id: i64) -> anyhow::Result<()> {
    // Delete configurations first (foreign key)
    conn.execute("DELETE FROM robot_configurations WHERE robot_connection_id = ?", [id])?;

    // Delete the robot connection
    conn.execute("DELETE FROM robot_connections WHERE id = ?", [id])?;

    Ok(())
}


// ==================== Robot Configurations ====================

pub fn get_configurations_for_robot(conn: &Connection, robot_id: i64) -> anyhow::Result<Vec<RobotConfiguration>> {
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
            is_default: row.get(3)?,
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

pub fn get_configuration(conn: &Connection, config_id: i64) -> anyhow::Result<RobotConfiguration> {
    conn.query_row(
        "SELECT id, robot_connection_id, name, is_default, u_frame_number, u_tool_number,
                front, up, left, flip, turn4, turn5, turn6
         FROM robot_configurations WHERE id = ?",
        [config_id],
        |row| {
            Ok(RobotConfiguration {
                id: row.get(0)?,
                robot_connection_id: row.get(1)?,
                name: row.get(2)?,
                is_default: row.get(3)?,
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
        },
    ).map_err(|e| anyhow::anyhow!("Failed to get configuration: {}", e))
}

pub fn get_default_configuration_for_robot(conn: &Connection, robot_id: i64) -> anyhow::Result<Option<RobotConfiguration>> {
    let configs = get_configurations_for_robot(conn, robot_id)?;
    Ok(configs.into_iter().find(|c| c.is_default))
}

pub fn create_configuration(conn: &Connection, req: &CreateConfiguration) -> anyhow::Result<i64> {
    conn.execute(
        "INSERT INTO robot_configurations
         (robot_connection_id, name, is_default, u_frame_number, u_tool_number,
          front, up, left, flip, turn4, turn5, turn6)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params![
            req.robot_connection_id,
            req.name,
            req.is_default,
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

pub fn update_configuration(conn: &Connection, req: &UpdateConfiguration) -> anyhow::Result<()> {
    let mut updates = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(ref name) = req.name {
        updates.push("name = ?");
        params.push(Box::new(name.clone()));
    }
    if let Some(u_frame) = req.u_frame_number {
        updates.push("u_frame_number = ?");
        params.push(Box::new(u_frame));
    }
    if let Some(u_tool) = req.u_tool_number {
        updates.push("u_tool_number = ?");
        params.push(Box::new(u_tool));
    }
    if let Some(front) = req.front {
        updates.push("front = ?");
        params.push(Box::new(front));
    }
    if let Some(up) = req.up {
        updates.push("up = ?");
        params.push(Box::new(up));
    }
    if let Some(left) = req.left {
        updates.push("left = ?");
        params.push(Box::new(left));
    }
    if let Some(flip) = req.flip {
        updates.push("flip = ?");
        params.push(Box::new(flip));
    }
    if let Some(turn4) = req.turn4 {
        updates.push("turn4 = ?");
        params.push(Box::new(turn4));
    }
    if let Some(turn5) = req.turn5 {
        updates.push("turn5 = ?");
        params.push(Box::new(turn5));
    }
    if let Some(turn6) = req.turn6 {
        updates.push("turn6 = ?");
        params.push(Box::new(turn6));
    }

    if updates.is_empty() {
        return Ok(());
    }

    let sql = format!(
        "UPDATE robot_configurations SET {} WHERE id = ?",
        updates.join(", ")
    );
    params.push(Box::new(req.id));

    let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    conn.execute(&sql, params_refs.as_slice())?;
    Ok(())
}

pub fn delete_configuration(conn: &Connection, id: i64) -> anyhow::Result<()> {
    conn.execute("DELETE FROM robot_configurations WHERE id = ?", [id])?;
    Ok(())
}

pub fn set_default_configuration(conn: &Connection, id: i64) -> anyhow::Result<()> {
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

    // Set this one as default
    conn.execute(
        "UPDATE robot_configurations SET is_default = 1 WHERE id = ?",
        [id],
    )?;

    Ok(())
}


pub fn save_current_configuration(
    conn: &Connection,
    robot_connection_id: i64,
    loaded_from_id: Option<i64>,
    name: Option<String>,
    active_config: &ActiveConfigState,
) -> anyhow::Result<(i64, String)> {
    // Determine the name
    let config_name = name.unwrap_or_else(|| {
        if let Some(loaded_id) = loaded_from_id {
            // Update the existing config
            format!("Config {}", loaded_id)
        } else {
            // Generate a unique name
            format!("Config {}", chrono::Local::now().format("%Y%m%d_%H%M%S"))
        }
    });

    if let Some(loaded_id) = loaded_from_id {
        // Update existing configuration
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
                loaded_id,
            ],
        )?;
        Ok((loaded_id, config_name))
    } else {
        // Create new configuration
        conn.execute(
            "INSERT INTO robot_configurations
             (robot_connection_id, name, is_default, u_frame_number, u_tool_number,
              front, up, left, flip, turn4, turn5, turn6)
             VALUES (?, ?, 0, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                robot_connection_id,
                config_name,
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
        Ok((new_id, config_name))
    }
}

// ==================== Programs ====================

pub fn list_programs(conn: &Connection) -> anyhow::Result<Vec<ProgramInfo>> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.name, p.description,
                (SELECT COUNT(*) FROM program_instructions WHERE program_id = p.id) as instruction_count,
                COALESCE(p.created_at, datetime('now')) as created_at,
                COALESCE(p.updated_at, datetime('now')) as updated_at
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

pub fn get_program(conn: &Connection, id: i64) -> anyhow::Result<Option<ProgramDetail>> {
    let program: Option<ProgramDetail> = conn.query_row(
        "SELECT id, name, description,
                COALESCE(default_w, 0.0), COALESCE(default_p, 0.0), COALESCE(default_r, 0.0),
                default_speed, COALESCE(default_speed_type, 'mmSec'),
                COALESCE(default_term_type, 'CNT'), COALESCE(default_term_value, 100),
                default_uframe, default_utool,
                start_x, start_y, start_z, start_w, start_p, start_r,
                end_x, end_y, end_z, end_w, end_p, end_r,
                COALESCE(move_speed, 100.0),
                COALESCE(created_at, datetime('now')), COALESCE(updated_at, datetime('now'))
         FROM programs WHERE id = ?",
        [id],
        |row| {
            Ok(ProgramDetail {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                default_w: row.get(3)?,
                default_p: row.get(4)?,
                default_r: row.get(5)?,
                default_speed: row.get(6)?,
                default_speed_type: row.get(7)?,
                default_term_type: row.get(8)?,
                default_term_value: row.get(9)?,
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
                move_speed: row.get(24)?,
                created_at: row.get(25)?,
                updated_at: row.get(26)?,
                instructions: vec![],
            })
        },
    ).optional()?;

    if let Some(mut prog) = program {
        // Get instructions
        let mut stmt = conn.prepare(
            "SELECT line_number, x, y, z, w, p, r,
                    speed, term_type, term_value, uframe, utool
             FROM program_instructions WHERE program_id = ? ORDER BY line_number"
        )?;

        prog.instructions = stmt.query_map([id], |row| {
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

        Ok(Some(prog))
    } else {
        Ok(None)
    }
}

pub fn create_program(conn: &Connection, name: &str, description: Option<&str>) -> anyhow::Result<i64> {
    conn.execute(
        "INSERT INTO programs (name, description) VALUES (?, ?)",
        rusqlite::params![name, description],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn delete_program(conn: &Connection, id: i64) -> anyhow::Result<()> {
    conn.execute("DELETE FROM program_instructions WHERE program_id = ?", [id])?;
    conn.execute("DELETE FROM programs WHERE id = ?", [id])?;
    Ok(())
}

pub fn update_program_settings(
    conn: &Connection,
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
    let mut updates = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    macro_rules! add_update {
        ($field:expr, $value:expr) => {
            if let Some(v) = $value {
                updates.push(concat!($field, " = ?"));
                params.push(Box::new(v));
            }
        };
    }

    add_update!("start_x", start_x);
    add_update!("start_y", start_y);
    add_update!("start_z", start_z);
    add_update!("start_w", start_w);
    add_update!("start_p", start_p);
    add_update!("start_r", start_r);
    add_update!("end_x", end_x);
    add_update!("end_y", end_y);
    add_update!("end_z", end_z);
    add_update!("end_w", end_w);
    add_update!("end_p", end_p);
    add_update!("end_r", end_r);
    add_update!("move_speed", move_speed);
    add_update!("default_term_type", default_term_type);
    add_update!("default_term_value", default_term_value);

    if updates.is_empty() {
        return Ok(());
    }

    let sql = format!("UPDATE programs SET {} WHERE id = ?", updates.join(", "));
    params.push(Box::new(program_id));

    let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    conn.execute(&sql, params_refs.as_slice())?;
    Ok(())
}

pub fn insert_instructions(conn: &Connection, program_id: i64, instructions: &[Instruction]) -> anyhow::Result<()> {
    // Clear existing instructions
    conn.execute("DELETE FROM program_instructions WHERE program_id = ?", [program_id])?;

    // Insert new instructions
    for instr in instructions {
        conn.execute(
            "INSERT INTO program_instructions
             (program_id, line_number, x, y, z, w, p, r,
              speed, term_type, term_value, uframe, utool)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                program_id,
                instr.line_number,
                instr.x,
                instr.y,
                instr.z,
                instr.w,
                instr.p,
                instr.r,
                instr.speed,
                instr.term_type,
                instr.term_value,
                instr.uframe,
                instr.utool,
            ],
        )?;
    }
    Ok(())
}

// ==================== Settings ====================

pub fn get_settings(conn: &Connection) -> anyhow::Result<RobotSettings> {
    // Helper to get a setting value
    let get_f64 = |key: &str, default: f64| -> f64 {
        conn.query_row(
            "SELECT CAST(value AS REAL) FROM server_settings WHERE key = ?",
            [key],
            |row| row.get(0),
        ).unwrap_or(default)
    };

    let get_i32 = |key: &str, default: i32| -> i32 {
        conn.query_row(
            "SELECT CAST(value AS INTEGER) FROM server_settings WHERE key = ?",
            [key],
            |row| row.get(0),
        ).unwrap_or(default)
    };

    let get_string = |key: &str, default: &str| -> String {
        conn.query_row(
            "SELECT value FROM server_settings WHERE key = ?",
            [key],
            |row| row.get(0),
        ).unwrap_or_else(|_| default.to_string())
    };

    Ok(RobotSettings {
        default_w: get_f64("default_w", 0.0),
        default_p: get_f64("default_p", 0.0),
        default_r: get_f64("default_r", 0.0),
        default_speed: get_f64("default_speed", 100.0),
        default_term_type: get_string("default_term_type", "CNT"),
        default_uframe: get_i32("default_uframe", 1),
        default_utool: get_i32("default_utool", 1),
    })
}

pub fn update_settings(conn: &Connection, settings: &RobotSettings) -> anyhow::Result<()> {
    let set_value = |key: &str, value: &str| -> rusqlite::Result<usize> {
        conn.execute(
            "INSERT OR REPLACE INTO server_settings (key, value) VALUES (?, ?)",
            [key, value],
        )
    };

    set_value("default_w", &settings.default_w.to_string())?;
    set_value("default_p", &settings.default_p.to_string())?;
    set_value("default_r", &settings.default_r.to_string())?;
    set_value("default_speed", &settings.default_speed.to_string())?;
    set_value("default_term_type", &settings.default_term_type)?;
    set_value("default_uframe", &settings.default_uframe.to_string())?;
    set_value("default_utool", &settings.default_utool.to_string())?;

    Ok(())
}

pub fn reset_database(conn: &Connection) -> anyhow::Result<()> {
    // Delete all data from tables
    conn.execute("DELETE FROM program_instructions", [])?;
    conn.execute("DELETE FROM programs", [])?;
    conn.execute("DELETE FROM robot_configurations", [])?;
    conn.execute("DELETE FROM robot_connections", [])?;
    conn.execute("DELETE FROM io_display_config", [])?;

    Ok(())
}

// ==================== I/O Config ====================

pub fn get_io_config(conn: &Connection, robot_connection_id: i64) -> anyhow::Result<Vec<IoDisplayConfig>> {
    let mut stmt = conn.prepare(
        "SELECT io_type, io_index, display_name, is_visible, display_order
         FROM io_display_config WHERE robot_connection_id = ? ORDER BY display_order, io_type, io_index"
    )?;

    let configs = stmt.query_map([robot_connection_id], |row| {
        Ok(IoDisplayConfig {
            io_type: row.get(0)?,
            io_index: row.get(1)?,
            display_name: row.get(2)?,
            is_visible: row.get(3)?,
            display_order: row.get(4)?,
        })
    })?.collect::<Result<Vec<_>, _>>()?;

    Ok(configs)
}

pub fn update_io_config(conn: &Connection, robot_connection_id: i64, configs: &[IoDisplayConfig]) -> anyhow::Result<()> {
    for cfg in configs {
        conn.execute(
            "INSERT OR REPLACE INTO io_display_config
             (robot_connection_id, io_type, io_index, display_name, is_visible, display_order)
             VALUES (?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                robot_connection_id,
                cfg.io_type,
                cfg.io_index,
                cfg.display_name,
                cfg.is_visible,
                cfg.display_order,
            ],
        )?;
    }
    Ok(())
}