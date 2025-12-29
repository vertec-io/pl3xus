//! FANUC database query functions.
//!
//! This module provides plain functions for database operations.
//! Each function takes a rusqlite Connection, making them easy to test
//! and use from any context.

use rusqlite::Connection;
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

// Note: Program CRUD operations have been moved to the programs crate.
// See fanuc_replica_programs::queries for list_programs, get_program, create_program, etc.

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
    // Delete all data from fanuc-specific tables
    // Note: Program data is managed by the programs crate database
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