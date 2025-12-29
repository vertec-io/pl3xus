//! Program execution notifications.
//!
//! This module provides systems that watch for execution state changes
//! and broadcast notifications to all connected clients.

use bevy::prelude::*;
use std::sync::atomic::{AtomicU8, Ordering};

use fanuc_replica_core::{
    console_entry, ActiveSystem, ConsoleDirection, ConsoleMsgType,
};
use fanuc_replica_execution::{BufferState, ExecutionCoordinator, ExecutionState, SystemState};
use pl3xus::Network;
use pl3xus_common::ServerNotification;
use pl3xus_websockets::WebSocketProvider;

/// Tracks the last known SystemState for change detection.
#[derive(Resource, Default)]
pub struct LastNotifiedState(AtomicU8);

const STATE_NONE: u8 = 0;
const STATE_RUNNING: u8 = 1;
const STATE_COMPLETE: u8 = 2;
const STATE_STOPPED: u8 = 3;
const STATE_ERROR: u8 = 4;

fn system_state_to_category(state: SystemState) -> u8 {
    match state {
        SystemState::Running => STATE_RUNNING,
        SystemState::Completed => STATE_COMPLETE,
        SystemState::Stopped => STATE_STOPPED,
        SystemState::Error => STATE_ERROR,
        _ => STATE_NONE,
    }
}

/// System that watches for execution state changes and broadcasts notifications.
///
/// Sends ServerNotification messages when:
/// - Execution starts (Running)
/// - Execution completes (Completed)
/// - Execution is stopped (Stopped)
/// - Execution encounters an error (Error)
pub fn send_program_notifications(
    net: Res<Network<WebSocketProvider>>,
    last_state: ResMut<LastNotifiedState>,
    system_query: Query<
        (&ExecutionState, Option<&ExecutionCoordinator>, Option<&BufferState>),
        (With<ActiveSystem>, Changed<ExecutionState>),
    >,
) {
    let Ok((exec_state, coordinator_opt, buffer_state_opt)) = system_query.single() else {
        return; // No change
    };

    let new_category = system_state_to_category(exec_state.state);
    let old_category = last_state.0.swap(new_category, Ordering::SeqCst);

    // Only notify on category transitions
    if old_category == new_category {
        return;
    }

    // Get program name from coordinator or execution state
    let program_name = coordinator_opt
        .map(|c| c.name.clone())
        .or_else(|| exec_state.source_name.clone())
        .unwrap_or_else(|| "Unknown".to_string());

    let total_points = exec_state.total_points.unwrap_or(0);

    let (notification, console) = match (old_category, new_category) {
        // Transition to Running: program started
        (_, STATE_RUNNING) => {
            let msg = format!(
                "Program '{}' started execution ({} points)",
                program_name, total_points
            );
            info!("📢 {}", msg);
            (
                Some(
                    ServerNotification::info(&msg).with_context("ProgramExecution"),
                ),
                Some(console_entry(
                    &msg,
                    ConsoleDirection::System,
                    ConsoleMsgType::Status,
                )),
            )
        }

        // Transition to Complete: program finished successfully
        (STATE_RUNNING, STATE_COMPLETE) => {
            let msg = format!(
                "Program '{}' completed ({} points executed)",
                program_name, exec_state.points_executed
            );
            info!("📢 {}", msg);
            (
                Some(
                    ServerNotification::success(&msg).with_context("ProgramExecution"),
                ),
                Some(console_entry(
                    &msg,
                    ConsoleDirection::System,
                    ConsoleMsgType::Status,
                )),
            )
        }

        // Transition to Stopped: program was stopped by user
        (STATE_RUNNING, STATE_STOPPED) => {
            let at_line = exec_state.current_index;
            let completed = exec_state.points_executed;
            let msg = format!(
                "Program '{}' stopped at line {} ({} completed)",
                program_name, at_line, completed
            );
            info!("📢 {}", msg);
            (
                Some(
                    ServerNotification::warning(&msg).with_context("ProgramExecution"),
                ),
                Some(console_entry(
                    &msg,
                    ConsoleDirection::System,
                    ConsoleMsgType::Status,
                )),
            )
        }

        // Transition to Error: program encountered an error
        (_, STATE_ERROR) => {
            let at_line = exec_state.current_index;
            let error_message = buffer_state_opt
                .and_then(|bs| {
                    if let BufferState::Error { message } = bs {
                        Some(message.clone())
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| "Unknown error".to_string());

            let msg = format!(
                "Program '{}' error at line {}: {}",
                program_name, at_line, error_message
            );
            error!("📢 {}", msg);
            (
                Some(
                    ServerNotification::error(&msg).with_context("ProgramExecution"),
                ),
                Some(console_entry(
                    &msg,
                    ConsoleDirection::System,
                    ConsoleMsgType::Error,
                )),
            )
        }

        // Other transitions don't need notifications
        _ => (None, None),
    };

    // Broadcast the notification to all clients (for toasts)
    if let Some(notif) = notification {
        net.broadcast(notif);
    }

    // Broadcast the console entry (for console log)
    if let Some(entry) = console {
        net.broadcast(entry);
    }
}

/// Plugin that adds program notification systems.
pub struct ProgramNotificationsPlugin;

impl Plugin for ProgramNotificationsPlugin {
    fn build(&self, app: &mut App) {
        // Initialize last state tracker
        app.init_resource::<LastNotifiedState>();

        // Add notification system
        app.add_systems(Update, send_program_notifications);
    }
}
