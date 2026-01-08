//! Settings handlers.
//!
//! Handles operations for:
//! - Robot settings (speed, orientation defaults)
//! - Jog settings
//! - Connection status

use bevy::prelude::*;
use bevy::ecs::message::MessageReader;
use pl3xus::managers::network_request::Request;
use pl3xus_sync::TargetedRequest;
use robot_hmi_core::database::DatabaseResource;

use crate::connection::{FanucRobot, RobotConnectionState};
use crate::database;
use crate::types::*;

/// Handle GetSettings request.
pub fn handle_get_settings(
    mut requests: MessageReader<Request<GetSettings>>,
    db: Option<Res<DatabaseResource>>,
) {
    for request in requests.read() {
        info!("📋 Handling GetSettings");

        let settings = match db.as_ref() {
            Some(db_res) => {
                let conn = db_res.connection();
                let conn = conn.lock().unwrap();
                database::get_settings(&conn).unwrap_or_default()
            }
            None => RobotSettings::default(),
        };

        let _ = request.clone().respond(SettingsResponse { settings });
    }
}

/// Handle UpdateSettings request.
pub fn handle_update_settings(
    mut requests: MessageReader<Request<UpdateSettings>>,
    db: Option<Res<DatabaseResource>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling UpdateSettings: speed={}", inner.default_speed);

        let settings = RobotSettings {
            default_w: inner.default_w,
            default_p: inner.default_p,
            default_r: inner.default_r,
            default_speed: inner.default_speed,
            default_term_type: inner.default_term_type.clone(),
            default_uframe: inner.default_uframe,
            default_utool: inner.default_utool,
        };

        let (success, error) = match db.as_ref() {
            Some(db_res) => {
                let conn = db_res.connection();
                let conn = conn.lock().unwrap();
                match database::update_settings(&conn, &settings) {
                    Ok(_) => (true, None),
                    Err(e) => (false, Some(e.to_string())),
                }
            }
            None => (false, Some("Database not available".to_string())),
        };

        let _ = request.clone().respond(UpdateSettingsResponse { success, error });
    }
}

/// Handle GetConnectionStatus request.
pub fn handle_get_connection_status(
    mut requests: MessageReader<Request<TargetedRequest<GetConnectionStatus>>>,
    robots: Query<(&RobotConnectionState, &ConnectionState), With<FanucRobot>>,
) {
    for request in requests.read() {
        let targeted = request.get_request();
        info!("📋 Handling GetConnectionStatus for target {}", targeted.target_id);

        let target = match targeted.target_id.parse::<u64>() {
            Ok(bits) => Entity::from_bits(bits),
            Err(_) => {
                error!("Invalid target entity: {}", targeted.target_id);
                continue;
            }
        };

        let response = if let Ok((conn_state, conn_details)) = robots.get(target) {
            ConnectionStatusResponse {
                connected: *conn_state == RobotConnectionState::Connected,
                robot_name: Some(conn_details.robot_name.clone()),
                ip_address: Some(conn_details.robot_addr.clone()),
                port: None,
            }
        } else {
            ConnectionStatusResponse {
                connected: false,
                robot_name: None,
                ip_address: None,
                port: None,
            }
        };

        let _ = request.clone().respond(response);
    }
}

/// Handle UpdateJogSettings request.
pub fn handle_update_jog_settings(
    mut requests: MessageReader<Request<UpdateJogSettings>>,
    db: Option<Res<DatabaseResource>>,
    robots: Query<&ConnectionState, With<FanucRobot>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling UpdateJogSettings: cartesian_speed={}", inner.cartesian_jog_speed);

        let active_connection_id = robots.iter()
            .find_map(|conn_state| {
                if conn_state.robot_connected {
                    conn_state.active_connection_id
                } else {
                    None
                }
            });

        let (success, error) = match (db.as_ref(), active_connection_id) {
            (Some(db_res), Some(connection_id)) => {
                let conn = db_res.connection();
                let conn = conn.lock().unwrap();
                match database::update_jog_settings(
                    &conn, connection_id,
                    inner.cartesian_jog_speed, inner.cartesian_jog_step,
                    inner.joint_jog_speed, inner.joint_jog_step,
                    inner.rotation_jog_speed, inner.rotation_jog_step,
                ) {
                    Ok(_) => (true, None),
                    Err(e) => (false, Some(e.to_string())),
                }
            }
            (None, _) => (false, Some("Database not available".to_string())),
            (_, None) => (false, Some("No active robot connection".to_string())),
        };

        let _ = request.clone().respond(UpdateJogSettingsResponse { success, error });
    }
}

