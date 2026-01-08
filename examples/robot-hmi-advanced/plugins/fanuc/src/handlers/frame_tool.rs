//! Frame and tool data handlers.
//!
//! Handles operations for:
//! - Active frame/tool selection
//! - Frame data read/write
//! - Tool data read/write

use bevy::prelude::*;
use bevy::ecs::message::MessageReader;
use pl3xus::managers::network_request::Request;
use pl3xus_async::TokioTasksRuntime;
use pl3xus_sync::authorization::AuthorizedRequest;
use pl3xus_sync::TargetedRequest;
use fanuc_rmi::packets::PacketPriority;

use crate::connection::{FanucRobot, RmiDriver, RobotConnectionState};
use crate::types::*;

/// Handle GetActiveFrameTool request - returns current active frame and tool.
pub fn handle_get_active_frame_tool(
    mut requests: MessageReader<Request<TargetedRequest<GetActiveFrameTool>>>,
    robots: Query<&FrameToolDataState, With<FanucRobot>>,
) {
    for request in requests.read() {
        let targeted = request.get_request();
        info!("📋 Handling GetActiveFrameTool for target {}", targeted.target_id);

        let target = match targeted.target_id.parse::<u64>() {
            Ok(bits) => Entity::from_bits(bits),
            Err(_) => {
                error!("Invalid target entity: {}", targeted.target_id);
                continue;
            }
        };

        let (uframe, utool) = if let Ok(ft_state) = robots.get(target) {
            (ft_state.active_frame, ft_state.active_tool)
        } else {
            (1, 1)
        };

        let response = GetActiveFrameToolResponse { uframe, utool };
        if let Err(e) = request.clone().respond(response) {
            error!("Failed to send response: {:?}", e);
        }
    }
}

/// Handle SetActiveFrameTool request - sets active frame and tool on robot.
pub fn handle_set_active_frame_tool(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut requests: MessageReader<AuthorizedRequest<SetActiveFrameTool>>,
    mut robots: Query<(
        &mut FrameToolDataState,
        &mut ActiveConfigState,
        &RobotConnectionState,
        Option<&RmiDriver>,
    ), With<FanucRobot>>,
) {
    use fanuc_rmi::dto as raw_dto;

    let _guard = tokio_runtime.runtime().enter();

    for request in requests.read() {
        let inner = request.get_request();
        info!("📋 Handling SetActiveFrameTool: uframe={}, utool={}", inner.uframe, inner.utool);

        let Some((ft_state, mut active_config, _, driver)) = robots.iter_mut()
            .find(|(_, _, state, driver)| **state == RobotConnectionState::Connected && driver.is_some())
        else {
            warn!("SetActiveFrameTool rejected: No connected robot");
            let _ = request.clone().respond(SetActiveFrameToolResponse {
                success: false,
                error: Some("No connected robot".to_string()),
            });
            continue;
        };

        let driver = driver.expect("Checked above");
        let old_frame = ft_state.active_frame;
        let old_tool = ft_state.active_tool;

        let command = raw_dto::Command::FrcSetUFrameUTool(raw_dto::FrcSetUFrameUTool {
            group: 1,
            u_frame_number: inner.uframe as u8,
            u_tool_number: inner.utool as u8,
        });
        let send_packet: fanuc_rmi::packets::SendPacket = raw_dto::SendPacket::Command(command).into();

        match driver.0.send_packet(send_packet, PacketPriority::Immediate) {
            Ok(seq) => {
                info!("✅ Sent FrcSetUFrameUTool with sequence {}", seq);

                if old_frame != inner.uframe {
                    active_config.u_frame_number = inner.uframe;
                    active_config.changes_count += 1;
                    active_config.change_log.push(ConfigChangeEntry {
                        field_name: "UFrame".to_string(),
                        old_value: format!("{}", old_frame),
                        new_value: format!("{}", inner.uframe),
                    });
                }
                if old_tool != inner.utool {
                    active_config.u_tool_number = inner.utool;
                    active_config.changes_count += 1;
                    active_config.change_log.push(ConfigChangeEntry {
                        field_name: "UTool".to_string(),
                        old_value: format!("{}", old_tool),
                        new_value: format!("{}", inner.utool),
                    });
                }

                let _ = request.clone().respond(SetActiveFrameToolResponse { success: true, error: None });
            }
            Err(e) => {
                error!("Failed to send FrcSetUFrameUTool: {:?}", e);
                let _ = request.clone().respond(SetActiveFrameToolResponse {
                    success: false,
                    error: Some(format!("Failed to send command: {:?}", e)),
                });
            }
        }
    }
}

/// Handle GetFrameData request - reads frame data from robot.
pub fn handle_get_frame_data(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut requests: MessageReader<Request<TargetedRequest<GetFrameData>>>,
    robots: Query<(&FrameToolDataState, Option<&RmiDriver>, &RobotConnectionState), With<FanucRobot>>,
) {
    use fanuc_rmi::packets::{SendPacket, Command, ResponsePacket, CommandResponse};
    use fanuc_rmi::commands::FrcReadUFrameData;
    use std::time::Duration;

    let _guard = tokio_runtime.runtime().enter();

    for request in requests.read() {
        let targeted = request.get_request();
        let frame_number = targeted.request.frame_number;
        info!("📋 Handling GetFrameData for frame {}", frame_number);

        // UFrame 0 cannot be queried - world coordinates
        if frame_number == 0 {
            let response = FrameDataResponse {
                frame_number: 0,
                x: 0.0, y: 0.0, z: 0.0,
                w: 0.0, p: 0.0, r: 0.0,
            };
            let _ = request.clone().respond(response);
            continue;
        }

        let robot_info = robots.iter()
            .find(|(_, driver, state)| **state == RobotConnectionState::Connected && driver.is_some());

        if let Some((_, Some(driver), _)) = robot_info {
            let driver = driver.0.clone();
            let request = request.clone();

            tokio_runtime.spawn_background_task(move |mut ctx| async move {
                let packet = SendPacket::Command(Command::FrcReadUFrameData(FrcReadUFrameData {
                    frame_number: frame_number as i8,
                    group: 1,
                }));

                let mut response_rx = driver.response_tx.subscribe();

                if let Err(e) = driver.send_packet(packet, PacketPriority::Standard) {
                    bevy::log::error!("Failed to send FrcReadUFrameData: {}", e);
                    let _ = request.respond(FrameDataResponse {
                        frame_number, x: 0.0, y: 0.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0,
                    });
                    return;
                }

                let result = tokio::time::timeout(Duration::from_secs(5), async {
                    while let Ok(response) = response_rx.recv().await {
                        if let ResponsePacket::CommandResponse(CommandResponse::FrcReadUFrameData(resp)) = response {
                            return Some(resp);
                        }
                    }
                    None
                }).await;

                match result {
                    Ok(Some(resp)) if resp.error_id == 0 => {
                        let frame_data = FrameToolData {
                            x: resp.frame.x, y: resp.frame.y, z: resp.frame.z,
                            w: resp.frame.w, p: resp.frame.p, r: resp.frame.r,
                        };
                        let frame_data_clone = frame_data.clone();
                        let frame_num = frame_number;
                        ctx.run_on_main_thread(move |ctx| {
                            let mut query = ctx.world.query_filtered::<&mut FrameToolDataState, With<FanucRobot>>();
                            for mut ft_state in query.iter_mut(ctx.world) {
                                ft_state.frames.insert(frame_num, frame_data_clone.clone());
                            }
                        }).await;
                        let _ = request.respond(FrameDataResponse {
                            frame_number,
                            x: frame_data.x, y: frame_data.y, z: frame_data.z,
                            w: frame_data.w, p: frame_data.p, r: frame_data.r,
                        });
                    }
                    _ => {
                        let _ = request.respond(FrameDataResponse {
                            frame_number, x: 0.0, y: 0.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0,
                        });
                    }
                }
            });
        } else {
            let _ = request.clone().respond(FrameDataResponse {
                frame_number, x: 0.0, y: 0.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0,
            });
        }
    }
}

/// Handle WriteFrameData request - writes frame data to robot.
pub fn handle_write_frame_data(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut requests: MessageReader<AuthorizedRequest<WriteFrameData>>,
    robots: Query<(Option<&RmiDriver>, &RobotConnectionState), With<FanucRobot>>,
) {
    use fanuc_rmi::packets::{SendPacket, Command, ResponsePacket, CommandResponse};
    use fanuc_rmi::commands::FrcWriteUFrameData;
    use fanuc_rmi::FrameData;
    use std::time::Duration;

    let _guard = tokio_runtime.runtime().enter();

    for request in requests.read() {
        let inner = request.get_request();
        let frame_number = inner.frame_number;
        info!("📋 Handling WriteFrameData for frame {}", frame_number);

        if frame_number == 0 {
            let _ = request.clone().respond(WriteFrameDataResponse {
                success: false,
                error: Some("UFrame 0 cannot be modified".to_string()),
            });
            continue;
        }

        let robot_info = robots.iter()
            .find(|(driver, state)| **state == RobotConnectionState::Connected && driver.is_some());

        if let Some((Some(driver), _)) = robot_info {
            let driver = driver.0.clone();
            let request = request.clone();
            let (x, y, z, w, p, r) = (inner.x, inner.y, inner.z, inner.w, inner.p, inner.r);

            tokio_runtime.spawn_background_task(move |mut ctx| async move {
                let packet = SendPacket::Command(Command::FrcWriteUFrameData(FrcWriteUFrameData {
                    frame_number: frame_number as i8,
                    frame: FrameData { x, y, z, w, p, r },
                    group: 1,
                }));

                let mut response_rx = driver.response_tx.subscribe();

                if let Err(e) = driver.send_packet(packet, PacketPriority::Standard) {
                    let _ = request.respond(WriteFrameDataResponse {
                        success: false,
                        error: Some(format!("Failed to send: {}", e)),
                    });
                    return;
                }

                let result = tokio::time::timeout(Duration::from_secs(5), async {
                    while let Ok(response) = response_rx.recv().await {
                        if let ResponsePacket::CommandResponse(CommandResponse::FrcWriteUFrameData(resp)) = response {
                            return Some(resp);
                        }
                    }
                    None
                }).await;

                match result {
                    Ok(Some(resp)) if resp.error_id == 0 => {
                        let frame_data = FrameToolData { x, y, z, w, p, r };
                        let frame_num = frame_number;
                        ctx.run_on_main_thread(move |ctx| {
                            let mut query = ctx.world.query_filtered::<&mut FrameToolDataState, With<FanucRobot>>();
                            for mut ft_state in query.iter_mut(ctx.world) {
                                ft_state.frames.insert(frame_num, frame_data.clone());
                            }
                        }).await;
                        let _ = request.respond(WriteFrameDataResponse { success: true, error: None });
                    }
                    Ok(Some(resp)) => {
                        let _ = request.respond(WriteFrameDataResponse {
                            success: false,
                            error: Some(format!("Robot error: {}", resp.error_id)),
                        });
                    }
                    _ => {
                        let _ = request.respond(WriteFrameDataResponse {
                            success: false,
                            error: Some("Timeout or no response".to_string()),
                        });
                    }
                }
            });
        } else {
            let _ = request.clone().respond(WriteFrameDataResponse {
                success: false,
                error: Some("No connected robot".to_string()),
            });
        }
    }
}

/// Handle GetToolData request - reads tool data from robot.
pub fn handle_get_tool_data(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut requests: MessageReader<Request<TargetedRequest<GetToolData>>>,
    robots: Query<(&FrameToolDataState, Option<&RmiDriver>, &RobotConnectionState), With<FanucRobot>>,
) {
    use fanuc_rmi::packets::{SendPacket, Command, ResponsePacket, CommandResponse};
    use fanuc_rmi::commands::FrcReadUToolData;
    use std::time::Duration;

    let _guard = tokio_runtime.runtime().enter();

    for request in requests.read() {
        let targeted = request.get_request();
        let tool_number = targeted.request.tool_number;
        info!("📋 Handling GetToolData for tool {}", tool_number);

        if tool_number <= 0 {
            let _ = request.clone().respond(ToolDataResponse {
                tool_number, x: 0.0, y: 0.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0,
            });
            continue;
        }

        let robot_info = robots.iter()
            .find(|(_, driver, state)| **state == RobotConnectionState::Connected && driver.is_some());

        if let Some((_, Some(driver), _)) = robot_info {
            let driver = driver.0.clone();
            let request = request.clone();

            tokio_runtime.spawn_background_task(move |mut ctx| async move {
                let packet = SendPacket::Command(Command::FrcReadUToolData(FrcReadUToolData {
                    tool_number: tool_number as i8,
                    group: 1,
                }));

                let mut response_rx = driver.response_tx.subscribe();

                if let Err(e) = driver.send_packet(packet, PacketPriority::Standard) {
                    bevy::log::error!("Failed to send FrcReadUToolData: {}", e);
                    let _ = request.respond(ToolDataResponse {
                        tool_number, x: 0.0, y: 0.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0,
                    });
                    return;
                }

                let result = tokio::time::timeout(Duration::from_secs(5), async {
                    while let Ok(response) = response_rx.recv().await {
                        if let ResponsePacket::CommandResponse(CommandResponse::FrcReadUToolData(resp)) = response {
                            return Some(resp);
                        }
                    }
                    None
                }).await;

                match result {
                    Ok(Some(resp)) if resp.error_id == 0 => {
                        let tool_data = FrameToolData {
                            x: resp.frame.x, y: resp.frame.y, z: resp.frame.z,
                            w: resp.frame.w, p: resp.frame.p, r: resp.frame.r,
                        };
                        let tool_data_clone = tool_data.clone();
                        let tool_num = tool_number;
                        ctx.run_on_main_thread(move |ctx| {
                            let mut query = ctx.world.query_filtered::<&mut FrameToolDataState, With<FanucRobot>>();
                            for mut ft_state in query.iter_mut(ctx.world) {
                                ft_state.tools.insert(tool_num, tool_data_clone.clone());
                            }
                        }).await;
                        let _ = request.respond(ToolDataResponse {
                            tool_number,
                            x: tool_data.x, y: tool_data.y, z: tool_data.z,
                            w: tool_data.w, p: tool_data.p, r: tool_data.r,
                        });
                    }
                    _ => {
                        let _ = request.respond(ToolDataResponse {
                            tool_number, x: 0.0, y: 0.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0,
                        });
                    }
                }
            });
        } else {
            let _ = request.clone().respond(ToolDataResponse {
                tool_number, x: 0.0, y: 0.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0,
            });
        }
    }
}

/// Handle WriteToolData request - writes tool data to robot.
pub fn handle_write_tool_data(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut requests: MessageReader<AuthorizedRequest<WriteToolData>>,
    robots: Query<(Option<&RmiDriver>, &RobotConnectionState), With<FanucRobot>>,
) {
    use fanuc_rmi::packets::{SendPacket, Command, ResponsePacket, CommandResponse};
    use fanuc_rmi::commands::FrcWriteUToolData;
    use fanuc_rmi::FrameData;
    use std::time::Duration;

    let _guard = tokio_runtime.runtime().enter();

    for request in requests.read() {
        let inner = request.get_request();
        let tool_number = inner.tool_number;
        info!("📋 Handling WriteToolData for tool {}", tool_number);

        if tool_number <= 0 {
            let _ = request.clone().respond(WriteToolDataResponse {
                success: false,
                error: Some("Tool number must be 1-10".to_string()),
            });
            continue;
        }

        let robot_info = robots.iter()
            .find(|(driver, state)| **state == RobotConnectionState::Connected && driver.is_some());

        if let Some((Some(driver), _)) = robot_info {
            let driver = driver.0.clone();
            let request = request.clone();
            let (x, y, z, w, p, r) = (inner.x, inner.y, inner.z, inner.w, inner.p, inner.r);

            tokio_runtime.spawn_background_task(move |mut ctx| async move {
                let packet = SendPacket::Command(Command::FrcWriteUToolData(FrcWriteUToolData {
                    tool_number: tool_number as i8,
                    frame: FrameData { x, y, z, w, p, r },
                    group: 1,
                }));

                let mut response_rx = driver.response_tx.subscribe();

                if let Err(e) = driver.send_packet(packet, PacketPriority::Standard) {
                    let _ = request.respond(WriteToolDataResponse {
                        success: false,
                        error: Some(format!("Failed to send: {}", e)),
                    });
                    return;
                }

                let result = tokio::time::timeout(Duration::from_secs(5), async {
                    while let Ok(response) = response_rx.recv().await {
                        if let ResponsePacket::CommandResponse(CommandResponse::FrcWriteUToolData(resp)) = response {
                            return Some(resp);
                        }
                    }
                    None
                }).await;

                match result {
                    Ok(Some(resp)) if resp.error_id == 0 => {
                        let tool_data = FrameToolData { x, y, z, w, p, r };
                        let tool_num = tool_number;
                        ctx.run_on_main_thread(move |ctx| {
                            let mut query = ctx.world.query_filtered::<&mut FrameToolDataState, With<FanucRobot>>();
                            for mut ft_state in query.iter_mut(ctx.world) {
                                ft_state.tools.insert(tool_num, tool_data.clone());
                            }
                        }).await;
                        let _ = request.respond(WriteToolDataResponse { success: true, error: None });
                    }
                    Ok(Some(resp)) => {
                        let _ = request.respond(WriteToolDataResponse {
                            success: false,
                            error: Some(format!("Robot error: {}", resp.error_id)),
                        });
                    }
                    _ => {
                        let _ = request.respond(WriteToolDataResponse {
                            success: false,
                            error: Some("Timeout or no response".to_string()),
                        });
                    }
                }
            });
        } else {
            let _ = request.clone().respond(WriteToolDataResponse {
                success: false,
                error: Some("No connected robot".to_string()),
            });
        }
    }
}
