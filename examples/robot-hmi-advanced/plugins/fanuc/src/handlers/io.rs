//! I/O handlers.
//!
//! Handles operations for:
//! - Digital I/O (DIN/DOUT)
//! - Analog I/O (AIN/AOUT)
//! - Group I/O (GIN/GOUT)
//! - I/O configuration

use bevy::prelude::*;
use bevy::ecs::message::MessageReader;
use pl3xus::managers::network_request::Request;
use pl3xus_async::TokioTasksRuntime;
use pl3xus_sync::authorization::AuthorizedRequest;
use pl3xus_sync::TargetedRequest;
use fanuc_rmi::packets::PacketPriority;
use robot_hmi_core::database::DatabaseResource;

use crate::connection::{FanucRobot, RmiDriver, RobotConnectionState};
use crate::database;
use crate::types::*;

/// Handle ReadDin request - reads digital input value.
pub fn handle_read_din(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut requests: MessageReader<Request<TargetedRequest<ReadDin>>>,
    robots: Query<(Option<&RmiDriver>, &RobotConnectionState), With<FanucRobot>>,
) {
    use fanuc_rmi::packets::{SendPacket, Command, ResponsePacket, CommandResponse};
    use fanuc_rmi::commands::FrcReadDIN;
    use std::time::Duration;

    let _guard = tokio_runtime.runtime().enter();

    for request in requests.read() {
        let targeted = request.get_request();
        let port_number = targeted.request.port_number;
        info!("📋 Handling ReadDin for port {}", port_number);

        let robot_info = robots.iter()
            .find(|(driver, state)| **state == RobotConnectionState::Connected && driver.is_some());

        if let Some((Some(driver), _)) = robot_info {
            let driver = driver.0.clone();
            let request = request.clone();

            tokio_runtime.spawn_background_task(move |_ctx| async move {
                let packet = SendPacket::Command(Command::FrcReadDIN(FrcReadDIN { port_number }));
                let mut response_rx = driver.response_tx.subscribe();

                if let Err(e) = driver.send_packet(packet, PacketPriority::Standard) {
                    bevy::log::error!("Failed to send FrcReadDIN: {}", e);
                    let _ = request.respond(DinValueResponse { port_number, port_value: false });
                    return;
                }

                let result = tokio::time::timeout(Duration::from_secs(5), async {
                    while let Ok(response) = response_rx.recv().await {
                        if let ResponsePacket::CommandResponse(CommandResponse::FrcReadDIN(resp)) = response {
                            return Some(resp);
                        }
                    }
                    None
                }).await;

                match result {
                    Ok(Some(resp)) if resp.error_id == 0 => {
                        let _ = request.respond(DinValueResponse {
                            port_number,
                            port_value: resp.port_value != 0,
                        });
                    }
                    _ => {
                        let _ = request.respond(DinValueResponse { port_number, port_value: false });
                    }
                }
            });
        } else {
            let _ = request.clone().respond(DinValueResponse { port_number, port_value: false });
        }
    }
}

/// Handle ReadDinBatch request - reads multiple digital input values.
pub fn handle_read_din_batch(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut requests: MessageReader<Request<TargetedRequest<ReadDinBatch>>>,
    robots: Query<(Option<&RmiDriver>, &RobotConnectionState), With<FanucRobot>>,
) {
    use fanuc_rmi::packets::{SendPacket, Command, ResponsePacket, CommandResponse};
    use fanuc_rmi::commands::FrcReadDIN;
    use std::time::Duration;

    let _guard = tokio_runtime.runtime().enter();

    for request in requests.read() {
        let targeted = request.get_request();
        let port_numbers = targeted.request.port_numbers.clone();
        info!("📋 Handling ReadDinBatch for {} ports", port_numbers.len());

        let robot_info = robots.iter()
            .find(|(driver, state)| **state == RobotConnectionState::Connected && driver.is_some());

        if let Some((Some(driver), _)) = robot_info {
            let driver = driver.0.clone();
            let request = request.clone();

            tokio_runtime.spawn_background_task(move |_ctx| async move {
                let mut values: Vec<(u16, bool)> = Vec::new();

                for port_number in port_numbers {
                    let packet = SendPacket::Command(Command::FrcReadDIN(FrcReadDIN { port_number }));
                    let mut response_rx = driver.response_tx.subscribe();

                    if let Err(_) = driver.send_packet(packet, PacketPriority::Standard) {
                        values.push((port_number, false));
                        continue;
                    }

                    let result = tokio::time::timeout(Duration::from_secs(5), async {
                        while let Ok(response) = response_rx.recv().await {
                            if let ResponsePacket::CommandResponse(CommandResponse::FrcReadDIN(resp)) = response {
                                if resp.port_number == port_number {
                                    return Some(resp);
                                }
                            }
                        }
                        None
                    }).await;

                    match result {
                        Ok(Some(resp)) if resp.error_id == 0 => {
                            values.push((port_number, resp.port_value != 0));
                        }
                        _ => {
                            values.push((port_number, false));
                        }
                    }
                }

                let _ = request.respond(DinBatchResponse { values });
            });
        } else {
            let values: Vec<(u16, bool)> = port_numbers.iter().map(|&p| (p, false)).collect();
            let _ = request.clone().respond(DinBatchResponse { values });
        }
    }
}

/// Handle WriteDout request - writes digital output value.
pub fn handle_write_dout(
    mut requests: MessageReader<AuthorizedRequest<WriteDout>>,
    driver_query: Query<&RmiDriver, With<FanucRobot>>,
    runtime: Option<Res<TokioTasksRuntime>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        let target = request.target_entity;
        let port = inner.port_number;
        let value = inner.port_value;
        info!("📋 Handling WriteDout for port {} = {}", port, value);

        let driver = match driver_query.get(target) {
            Ok(d) => d.0.clone(),
            Err(_) => {
                let _ = request.clone().respond(DoutValueResponse {
                    port_number: port, port_value: value, success: false,
                    error: Some("No robot connected".to_string()),
                });
                continue;
            }
        };

        let Some(runtime) = runtime.as_ref() else {
            let _ = request.clone().respond(DoutValueResponse {
                port_number: port, port_value: value, success: false,
                error: Some("Runtime not available".to_string()),
            });
            continue;
        };

        let request = request.clone();
        runtime.spawn_background_task(move |mut ctx| async move {
            use fanuc_rmi::packets::{SendPacket, Command, ResponsePacket, CommandResponse};
            use fanuc_rmi::commands::FrcWriteDOUT;
            use std::time::Duration;

            let packet = SendPacket::Command(Command::FrcWriteDOUT(FrcWriteDOUT {
                port_number: port,
                port_value: if value { 1 } else { 0 },
            }));

            let mut response_rx = driver.response_tx.subscribe();

            if let Err(e) = driver.send_packet(packet, PacketPriority::Standard) {
                let _ = request.respond(DoutValueResponse {
                    port_number: port, port_value: value, success: false,
                    error: Some(format!("Failed to send: {}", e)),
                });
                return;
            }

            let result = tokio::time::timeout(Duration::from_secs(5), async {
                while let Ok(response) = response_rx.recv().await {
                    if let ResponsePacket::CommandResponse(CommandResponse::FrcWriteDOUT(resp)) = response {
                        return Some(resp);
                    }
                }
                None
            }).await;

            match result {
                Ok(Some(resp)) if resp.error_id == 0 => {
                    ctx.run_on_main_thread(move |ctx| {
                        if let Some(mut io_status) = ctx.world.get_mut::<IoStatus>(target) {
                            let word_index = (port as usize - 1) / 16;
                            let bit_index = (port as usize - 1) % 16;
                            while io_status.digital_outputs.len() <= word_index {
                                io_status.digital_outputs.push(0);
                            }
                            if value {
                                io_status.digital_outputs[word_index] |= 1 << bit_index;
                            } else {
                                io_status.digital_outputs[word_index] &= !(1 << bit_index);
                            }
                        }
                    }).await;
                    let _ = request.respond(DoutValueResponse {
                        port_number: port, port_value: value, success: true, error: None,
                    });
                }
                Ok(Some(resp)) => {
                    let _ = request.respond(DoutValueResponse {
                        port_number: port, port_value: value, success: false,
                        error: Some(format!("Robot error: {}", resp.error_id)),
                    });
                }
                _ => {
                    let _ = request.respond(DoutValueResponse {
                        port_number: port, port_value: value, success: false,
                        error: Some("Timeout or no response".to_string()),
                    });
                }
            }
        });
    }
}

/// Handle ReadAin request - reads analog input value.
pub fn handle_read_ain(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut requests: MessageReader<Request<TargetedRequest<ReadAin>>>,
    robots: Query<(Option<&RmiDriver>, &RobotConnectionState), With<FanucRobot>>,
) {
    use fanuc_rmi::packets::{SendPacket, Command, ResponsePacket, CommandResponse};
    use fanuc_rmi::commands::FrcReadAIN;
    use std::time::Duration;

    let _guard = tokio_runtime.runtime().enter();

    for request in requests.read() {
        let targeted = request.get_request();
        let port_number = targeted.request.port_number;
        info!("📋 Handling ReadAin for port {}", port_number);

        let robot_info = robots.iter()
            .find(|(driver, state)| **state == RobotConnectionState::Connected && driver.is_some());

        if let Some((Some(driver), _)) = robot_info {
            let driver = driver.0.clone();
            let request = request.clone();

            tokio_runtime.spawn_background_task(move |_ctx| async move {
                let packet = SendPacket::Command(Command::FrcReadAIN(FrcReadAIN { port_number }));
                let mut response_rx = driver.response_tx.subscribe();

                if let Err(_) = driver.send_packet(packet, PacketPriority::Standard) {
                    let _ = request.respond(AinValueResponse { port_number, port_value: 0.0 });
                    return;
                }

                let result = tokio::time::timeout(Duration::from_secs(5), async {
                    while let Ok(response) = response_rx.recv().await {
                        if let ResponsePacket::CommandResponse(CommandResponse::FrcReadAIN(resp)) = response {
                            return Some(resp);
                        }
                    }
                    None
                }).await;

                match result {
                    Ok(Some(resp)) if resp.error_id == 0 => {
                        let _ = request.respond(AinValueResponse { port_number, port_value: resp.port_value });
                    }
                    _ => {
                        let _ = request.respond(AinValueResponse { port_number, port_value: 0.0 });
                    }
                }
            });
        } else {
            let _ = request.clone().respond(AinValueResponse { port_number, port_value: 0.0 });
        }
    }
}

/// Handle WriteAout request - writes analog output value.
pub fn handle_write_aout(
    mut requests: MessageReader<AuthorizedRequest<WriteAout>>,
    mut io_query: Query<&mut IoStatus, With<FanucRobot>>,
    driver_query: Query<&RmiDriver, With<FanucRobot>>,
    runtime: Option<Res<TokioTasksRuntime>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        let target = request.target_entity;
        let port = inner.port_number;
        let value = inner.port_value;
        info!("📋 Handling WriteAout for port {} = {}", port, value);

        if let Ok(mut io_status) = io_query.get_mut(target) {
            io_status.analog_outputs.insert(port, value);
        }

        if let (Ok(driver), Some(runtime)) = (driver_query.get(target), runtime.as_ref()) {
            let driver = driver.0.clone();
            runtime.spawn_background_task(move |_ctx| async move {
                use fanuc_rmi::packets::{SendPacket, Command};
                use fanuc_rmi::commands::FrcWriteAOUT;
                let packet = SendPacket::Command(Command::FrcWriteAOUT(FrcWriteAOUT { port_number: port, port_value: value }));
                let _ = driver.send_packet(packet, PacketPriority::Standard);
            });
        }

        let _ = request.clone().respond(AoutValueResponse {
            port_number: port, port_value: value, success: true, error: None,
        });
    }
}

/// Handle ReadGin request - reads group input value.
pub fn handle_read_gin(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut requests: MessageReader<Request<TargetedRequest<ReadGin>>>,
    robots: Query<(Option<&RmiDriver>, &RobotConnectionState), With<FanucRobot>>,
) {
    use fanuc_rmi::packets::{SendPacket, Command, ResponsePacket, CommandResponse};
    use fanuc_rmi::commands::FrcReadGIN;
    use std::time::Duration;

    let _guard = tokio_runtime.runtime().enter();

    for request in requests.read() {
        let targeted = request.get_request();
        let port_number = targeted.request.port_number;
        info!("📋 Handling ReadGin for port {}", port_number);

        let robot_info = robots.iter()
            .find(|(driver, state)| **state == RobotConnectionState::Connected && driver.is_some());

        if let Some((Some(driver), _)) = robot_info {
            let driver = driver.0.clone();
            let request = request.clone();

            tokio_runtime.spawn_background_task(move |_ctx| async move {
                let packet = SendPacket::Command(Command::FrcReadGIN(FrcReadGIN { port_number }));
                let mut response_rx = driver.response_tx.subscribe();

                if let Err(_) = driver.send_packet(packet, PacketPriority::Standard) {
                    let _ = request.respond(GinValueResponse { port_number, port_value: 0 });
                    return;
                }

                let result = tokio::time::timeout(Duration::from_secs(5), async {
                    while let Ok(response) = response_rx.recv().await {
                        if let ResponsePacket::CommandResponse(CommandResponse::FrcReadGIN(resp)) = response {
                            return Some(resp);
                        }
                    }
                    None
                }).await;

                match result {
                    Ok(Some(resp)) if resp.error_id == 0 => {
                        let _ = request.respond(GinValueResponse { port_number, port_value: resp.port_value });
                    }
                    _ => {
                        let _ = request.respond(GinValueResponse { port_number, port_value: 0 });
                    }
                }
            });
        } else {
            let _ = request.clone().respond(GinValueResponse { port_number, port_value: 0 });
        }
    }
}

/// Handle WriteGout request - writes group output value.
pub fn handle_write_gout(
    mut requests: MessageReader<AuthorizedRequest<WriteGout>>,
    mut io_query: Query<&mut IoStatus, With<FanucRobot>>,
    driver_query: Query<&RmiDriver, With<FanucRobot>>,
    runtime: Option<Res<TokioTasksRuntime>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        let target = request.target_entity;
        let port = inner.port_number;
        let value = inner.port_value;
        info!("📋 Handling WriteGout for port {} = {}", port, value);

        if let Ok(mut io_status) = io_query.get_mut(target) {
            io_status.group_outputs.insert(port, value);
        }

        if let (Ok(driver), Some(runtime)) = (driver_query.get(target), runtime.as_ref()) {
            let driver = driver.0.clone();
            runtime.spawn_background_task(move |_ctx| async move {
                use fanuc_rmi::packets::{SendPacket, Command};
                use fanuc_rmi::commands::FrcWriteGOUT;
                let packet = SendPacket::Command(Command::FrcWriteGOUT(FrcWriteGOUT { port_number: port, port_value: value }));
                let _ = driver.send_packet(packet, PacketPriority::Standard);
            });
        }

        let _ = request.clone().respond(GoutValueResponse {
            port_number: port, port_value: value, success: true, error: None,
        });
    }
}

/// Handle GetIoConfig request - gets I/O display configuration.
pub fn handle_get_io_config(
    mut requests: MessageReader<Request<GetIoConfig>>,
    db: Option<Res<DatabaseResource>>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling GetIoConfig for robot_connection_id={}", inner.robot_connection_id);

        let configs = match db.as_ref() {
            Some(db_res) => {
                let conn = db_res.connection();
                let conn = conn.lock().unwrap();
                database::get_io_config(&conn, inner.robot_connection_id).unwrap_or_default()
            }
            None => vec![],
        };

        let _ = request.clone().respond(IoConfigResponse { configs });
    }
}

/// Handle UpdateIoConfig request - updates I/O display configuration.
pub fn handle_update_io_config(
    mut requests: MessageReader<Request<UpdateIoConfig>>,
    db: Option<Res<DatabaseResource>>,
    mut robot_query: Query<(&ConnectionState, &mut IoConfigState)>,
) {
    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling UpdateIoConfig for robot_connection_id={}", inner.robot_connection_id);

        let (success, error) = match db.as_ref() {
            Some(db_res) => {
                let conn = db_res.connection();
                let conn = conn.lock().unwrap();
                match database::update_io_config(&conn, inner.robot_connection_id, &inner.configs) {
                    Ok(_) => {
                        for (conn_state, mut io_config) in robot_query.iter_mut() {
                            if conn_state.active_connection_id == Some(inner.robot_connection_id) {
                                let mut new_configs = std::collections::HashMap::new();
                                for cfg in &inner.configs {
                                    new_configs.insert((cfg.io_type.clone(), cfg.io_index), cfg.clone());
                                }
                                io_config.configs = new_configs;
                                break;
                            }
                        }
                        (true, None)
                    }
                    Err(e) => (false, Some(e.to_string())),
                }
            }
            None => (false, Some("Database not available".to_string())),
        };

        let _ = request.clone().respond(UpdateIoConfigResponse { success, error });
    }
}
