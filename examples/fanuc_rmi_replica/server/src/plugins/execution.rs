//! Program execution plugin.
//!
//! Handles program execution with buffered streaming:
//! - Loads programs from database
//! - Sends instructions to robot via driver
//! - Tracks progress and updates ExecutionState
//! - Handles pause/resume/stop
//! - Broadcasts ProgramNotification when program completes

use bevy::prelude::*;
use bevy_tokio_tasks::TokioTasksRuntime;
use std::collections::{VecDeque, HashMap};
use std::sync::atomic::{AtomicU64, Ordering};
use fanuc_rmi::packets::{SendPacket, ResponsePacket, SentInstructionInfo};
use fanuc_rmi::instructions::FrcLinearMotion;
use fanuc_rmi::{TermType, SpeedType, Configuration, Position};
use fanuc_replica_types::{
    ExecutionState, Instruction, ProgramNotification, ProgramNotificationKind,
    ConsoleLogEntry, ConsoleDirection, ConsoleMsgType,
};
use tokio::sync::broadcast;
use pl3xus::Network;
use pl3xus_websockets::WebSocketProvider;

use super::connection::{FanucRobot, RmiDriver, RmiResponseChannel, RobotConnectionState};

/// Global sequence counter for program notifications.
/// Each notification gets a unique sequence number to ensure identical notifications
/// are treated as distinct messages by clients.
static NOTIFICATION_SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// Create a new ProgramNotification with a unique sequence number.
fn new_notification(kind: ProgramNotificationKind) -> ProgramNotification {
    ProgramNotification {
        sequence: NOTIFICATION_SEQUENCE.fetch_add(1, Ordering::Relaxed),
        kind,
    }
}

/// Create a console log entry with current timestamp.
pub fn console_entry(content: impl Into<String>, direction: ConsoleDirection, msg_type: ConsoleMsgType) -> ConsoleLogEntry {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let ms = now.as_millis() as u64;
    let secs = now.as_secs();
    let hours = (secs / 3600) % 24;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;
    let millis = (ms % 1000) as u32;

    ConsoleLogEntry {
        timestamp: format!("{:02}:{:02}:{:02}.{:03}", hours, minutes, seconds, millis),
        timestamp_ms: ms,
        direction,
        msg_type,
        content: content.into(),
        sequence_id: None,
    }
}

/// Maximum instructions to send ahead (conservative: use 5 of 8 available slots).
pub const MAX_BUFFER: usize = 5;

// ============================================================================
// Resources
// ============================================================================

/// Loaded program data - stores all information needed to execute a program.
/// This is populated by LoadProgram and used by StartProgram.
#[derive(Clone, Default)]
pub struct LoadedProgramData {
    /// Program instructions.
    pub instructions: Vec<Instruction>,
    /// Approach position (start_x, start_y, start_z, start_w, start_p, start_r).
    pub approach: Option<(f64, f64, f64, f64, f64, f64)>,
    /// Retreat position (end_x, end_y, end_z, end_w, end_p, end_r).
    pub retreat: Option<(f64, f64, f64, f64, f64, f64)>,
    /// Move speed for approach/retreat.
    pub move_speed: f64,
}

/// Executor run state - prevents invalid state combinations.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExecutorRunState {
    /// No program running.
    #[default]
    Idle,
    /// Program is actively executing.
    Running,
    /// Program is paused (can be resumed).
    Paused,
}

/// Program executor state - manages buffered execution.
#[derive(Resource, Default)]
pub struct ProgramExecutor {
    /// Currently loaded program ID.
    pub loaded_program_id: Option<i64>,
    /// Program name for display.
    pub loaded_program_name: Option<String>,
    /// Loaded program data (instructions, approach/retreat positions).
    /// Populated by LoadProgram, used by StartProgram.
    pub loaded_program_data: Option<LoadedProgramData>,
    /// Total instructions (including approach/retreat).
    pub total_instructions: usize,
    /// Instructions waiting to be sent (line_number, packet).
    pub pending_queue: VecDeque<(usize, SendPacket)>,
    /// In-flight instructions by request_id (from driver) -> line_number.
    /// Used before we know the sequence_id.
    pub in_flight_by_request: HashMap<u64, usize>,
    /// In-flight instructions by sequence_id (from robot) -> line_number.
    /// Used after SentInstructionInfo maps request_id -> sequence_id.
    pub in_flight_by_sequence: HashMap<u32, usize>,
    /// Highest completed line number.
    pub completed_line: usize,
    /// Current run state.
    pub run_state: ExecutorRunState,
    /// Program defaults for building packets.
    pub defaults: ProgramDefaults,
}

impl ProgramExecutor {
    /// Check if the executor is currently running (not idle or paused).
    pub fn is_running(&self) -> bool {
        self.run_state == ExecutorRunState::Running
    }

    /// Check if the executor is paused.
    pub fn is_paused(&self) -> bool {
        self.run_state == ExecutorRunState::Paused
    }

    /// Check if a program is actively being executed (running or paused).
    pub fn is_active(&self) -> bool {
        self.run_state != ExecutorRunState::Idle
    }
}

/// Component to receive SentInstructionInfo from the driver.
/// This is used to map request_id -> sequence_id.
#[derive(Component)]
pub struct RmiSentInstructionChannel(pub broadcast::Receiver<SentInstructionInfo>);

/// Program defaults for building motion packets.
#[derive(Default, Clone)]
pub struct ProgramDefaults {
    pub w: f64,
    pub p: f64,
    pub r: f64,
    pub speed: f64,
    pub speed_type: String,
    pub term_type: String,
    pub term_value: Option<u8>,
    pub uframe: Option<i32>,
    pub utool: Option<i32>,
}

impl ProgramExecutor {
    /// Reset the executor to idle state (clears everything including loaded program).
    pub fn reset(&mut self) {
        self.loaded_program_id = None;
        self.loaded_program_name = None;
        self.loaded_program_data = None;
        self.total_instructions = 0;
        self.pending_queue.clear();
        self.in_flight_by_request.clear();
        self.in_flight_by_sequence.clear();
        self.completed_line = 0;
        self.run_state = ExecutorRunState::Idle;
    }

    /// Reset execution state but keep the loaded program.
    /// Used when stopping execution - allows re-running the same program.
    pub fn reset_execution(&mut self) {
        self.pending_queue.clear();
        self.in_flight_by_request.clear();
        self.in_flight_by_sequence.clear();
        self.completed_line = 0;
        self.run_state = ExecutorRunState::Idle;
    }

    /// Check if there are more instructions to send.
    #[allow(dead_code)]
    pub fn has_pending(&self) -> bool {
        !self.pending_queue.is_empty()
    }

    /// Get the number of in-flight instructions (both pending mapping and mapped).
    pub fn in_flight_count(&self) -> usize {
        self.in_flight_by_request.len() + self.in_flight_by_sequence.len()
    }

    /// Get the next batch of instructions to send (up to MAX_BUFFER - in_flight).
    pub fn get_next_batch(&mut self) -> Vec<(usize, SendPacket)> {
        let can_send = MAX_BUFFER.saturating_sub(self.in_flight_count());
        let mut batch = Vec::new();

        for _ in 0..can_send {
            if let Some((line, packet)) = self.pending_queue.pop_front() {
                batch.push((line, packet));
            } else {
                break;
            }
        }

        batch
    }

    /// Record that an instruction was sent (by request_id from driver).
    pub fn record_sent(&mut self, request_id: u64, line_number: usize) {
        self.in_flight_by_request.insert(request_id, line_number);
    }

    /// Map request_id to sequence_id when SentInstructionInfo arrives.
    /// This moves the entry from in_flight_by_request to in_flight_by_sequence.
    pub fn map_sequence(&mut self, request_id: u64, sequence_id: u32) {
        if let Some(line) = self.in_flight_by_request.remove(&request_id) {
            self.in_flight_by_sequence.insert(sequence_id, line);
        }
    }

    /// Handle instruction completion by sequence_id.
    /// Returns the line number if found.
    pub fn handle_completion(&mut self, sequence_id: u32) -> Option<usize> {
        if let Some(line) = self.in_flight_by_sequence.remove(&sequence_id) {
            self.completed_line = self.completed_line.max(line);
            Some(line)
        } else {
            None
        }
    }

    /// Check if execution is complete (was running and all instructions done).
    #[allow(dead_code)]
    pub fn is_complete(&self) -> bool {
        self.is_running() && self.pending_queue.is_empty() && self.in_flight_by_sequence.is_empty() && self.in_flight_by_request.is_empty()
    }

    /// Build a motion instruction packet from a program instruction.
    pub fn build_motion_packet(&self, instruction: &Instruction, is_last: bool) -> SendPacket {
        let w = instruction.w.unwrap_or(self.defaults.w);
        let p = instruction.p.unwrap_or(self.defaults.p);
        let r = instruction.r.unwrap_or(self.defaults.r);
        let speed = instruction.speed.unwrap_or(self.defaults.speed);

        let speed_type = match self.defaults.speed_type.as_str() {
            "mmSec" => SpeedType::MMSec,
            "InchMin" => SpeedType::InchMin,
            "Time" => SpeedType::Time,
            "mSec" => SpeedType::MilliSeconds,
            _ => SpeedType::MMSec,
        };

        // Use FINE for last instruction, otherwise use instruction's term_type or program default
        let term_type = if is_last {
            TermType::FINE
        } else {
            match instruction.term_type.as_deref().unwrap_or(&self.defaults.term_type) {
                "FINE" => TermType::FINE,
                _ => TermType::CNT,
            }
        };

        let term_value = instruction.term_value
            .or(self.defaults.term_value)
            .unwrap_or(if matches!(term_type, TermType::CNT) { 100 } else { 0 });

        let position = Position {
            x: instruction.x,
            y: instruction.y,
            z: instruction.z,
            w, p, r,
            ext1: 0.0,
            ext2: 0.0,
            ext3: 0.0,
        };

        let uframe = instruction.uframe.or(self.defaults.uframe).unwrap_or(1) as i8;
        let utool = instruction.utool.or(self.defaults.utool).unwrap_or(1) as i8;

        let configuration = Configuration {
            u_tool_number: utool,
            u_frame_number: uframe,
            front: 1,
            up: 1,
            left: 0,
            flip: 0,
            turn4: 0,
            turn5: 0,
            turn6: 0,
        };

        let motion = FrcLinearMotion::new(
            instruction.line_number as u32,
            configuration,
            position,
            speed_type,
            speed,
            term_type,
            term_value,
        );

        SendPacket::Instruction(fanuc_rmi::packets::Instruction::FrcLinearMotion(motion))
    }

    /// Build an approach or retreat motion packet.
    pub fn build_approach_retreat_packet(
        &self,
        x: f64, y: f64, z: f64,
        w: Option<f64>, p: Option<f64>, r: Option<f64>,
        line_number: usize,
        speed: f64,
        is_last: bool,
    ) -> SendPacket {
        let w = w.unwrap_or(self.defaults.w);
        let p = p.unwrap_or(self.defaults.p);
        let r = r.unwrap_or(self.defaults.r);

        let (term_type, term_value) = if is_last {
            (TermType::FINE, 0)
        } else {
            (TermType::CNT, self.defaults.term_value.unwrap_or(100))
        };

        let position = Position {
            x, y, z, w, p, r,
            ext1: 0.0, ext2: 0.0, ext3: 0.0,
        };

        let uframe = self.defaults.uframe.unwrap_or(1) as i8;
        let utool = self.defaults.utool.unwrap_or(1) as i8;

        let configuration = Configuration {
            u_tool_number: utool,
            u_frame_number: uframe,
            front: 1,
            up: 1,
            left: 0,
            flip: 0,
            turn4: 0,
            turn5: 0,
            turn6: 0,
        };

        let motion = FrcLinearMotion::new(
            line_number as u32,
            configuration,
            position,
            SpeedType::MMSec,
            speed,
            term_type,
            term_value,
        );

        SendPacket::Instruction(fanuc_rmi::packets::Instruction::FrcLinearMotion(motion))
    }
}

// ============================================================================
// Plugin
// ============================================================================

pub struct ProgramExecutionPlugin;

impl Plugin for ProgramExecutionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ProgramExecutor>();
        app.add_systems(Update, (
            reset_on_disconnect,
            process_instruction_responses,
            update_execution_state,
        ));
    }
}

/// Reset the executor when the robot disconnects.
/// This ensures we don't have stale execution state when reconnecting.
fn reset_on_disconnect(
    mut executor: ResMut<ProgramExecutor>,
    robots: Query<&super::connection::RobotConnectionState, With<super::connection::FanucRobot>>,
) {
    // If executor is active but robot is not connected, reset
    if executor.is_active() {
        let any_connected = robots.iter().any(|state| {
            *state == super::connection::RobotConnectionState::Connected
        });

        if !any_connected {
            info!("🔌 Robot disconnected while executing - resetting executor");
            executor.reset();
        }
    }
}

// ============================================================================
// Systems
// ============================================================================

/// Process instruction responses and update executor state.
/// Broadcasts ProgramNotification to all clients when program completes.
fn process_instruction_responses(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut executor: ResMut<ProgramExecutor>,
    mut robots: Query<(&RmiDriver, &mut RmiResponseChannel, Option<&mut RmiSentInstructionChannel>, &RobotConnectionState), With<FanucRobot>>,
    net: Res<Network<WebSocketProvider>>,
) {
    if !executor.is_running() {
        return;
    }

    // Enter the Tokio runtime context so send_packet can use tokio::spawn
    let _guard = tokio_runtime.runtime().enter();

    for (driver, mut response_channel, sent_channel, state) in robots.iter_mut() {
        if *state != RobotConnectionState::Connected {
            continue;
        }

        // Process SentInstructionInfo to map request_id -> sequence_id
        if let Some(mut sent_rx) = sent_channel {
            while let Ok(sent_info) = sent_rx.0.try_recv() {
                executor.map_sequence(sent_info.request_id, sent_info.sequence_id);
                debug!("🔗 Mapped request {} -> sequence {}", sent_info.request_id, sent_info.sequence_id);
            }
        }

        // Check for instruction responses
        while let Ok(response) = response_channel.0.try_recv() {
            match &response {
                ResponsePacket::InstructionResponse(instr_resp) => {
                    let seq_id = instr_resp.get_sequence_id();
                    if let Some(line) = executor.handle_completion(seq_id) {
                        info!("📍 Line {} completed (seq_id {})", line, seq_id);

                        // Check for error
                        if instr_resp.get_error_id() != 0 {
                            let program_name = executor.loaded_program_name.clone().unwrap_or_default();
                            let error_msg = format!("Instruction error: {}", instr_resp.get_error_id());
                            error!("Instruction {} failed with error {}", line, instr_resp.get_error_id());

                            // Broadcast error notification to all clients
                            let notification = new_notification(ProgramNotificationKind::Error {
                                program_name: program_name.clone(),
                                at_line: line,
                                error_message: error_msg.clone(),
                            });
                            info!("📢 Broadcasting program error notification");
                            net.broadcast(notification);

                            // Broadcast console entry for error
                            let console_msg = console_entry(
                                format!("Program '{}' error at line {}: {}", program_name, line, error_msg),
                                ConsoleDirection::System,
                                ConsoleMsgType::Error,
                            );
                            net.broadcast(console_msg);

                            executor.reset();
                            return;
                        }
                    }
                }
                _ => {
                    // Other responses - ignore for now
                }
            }
        }

        // Send more instructions if there's room in the buffer
        if executor.in_flight_count() < MAX_BUFFER && !executor.pending_queue.is_empty() {
            let batch = executor.get_next_batch();
            for (line_number, packet) in batch {
                match driver.0.send_packet(packet, fanuc_rmi::packets::PacketPriority::Standard) {
                    Ok(request_id) => {
                        // Record by request_id - will be mapped to sequence_id when SentInstructionInfo arrives
                        executor.record_sent(request_id, line_number);
                        info!("📤 Sent instruction {} (request_id {})", line_number, request_id);
                    }
                    Err(e) => {
                        error!("Failed to send instruction {}: {}", line_number, e);
                        executor.reset();
                        return;
                    }
                }
            }
        }

        // Check if execution is complete
        if executor.pending_queue.is_empty() && executor.in_flight_count() == 0 {
            let program_name = executor.loaded_program_name.clone().unwrap_or_default();
            let total = executor.total_instructions;

            info!("✅ Program '{}' execution complete: {} instructions", program_name, total);
            executor.run_state = ExecutorRunState::Idle;

            // Broadcast completion notification to all connected clients
            let notification = new_notification(ProgramNotificationKind::Completed {
                program_name: program_name.clone(),
                total_instructions: total,
            });
            info!("📢 Broadcasting program completion notification");
            net.broadcast(notification);

            // Broadcast console entry for program completion
            let console_msg = console_entry(
                format!("Program '{}' completed ({} instructions)", program_name, total),
                ConsoleDirection::System,
                ConsoleMsgType::Status,
            );
            net.broadcast(console_msg);
        }
    }
}

/// Update the ExecutionState synced component from executor state.
/// NOTE: Only updates execution-related fields (running, paused, current_line, total_lines).
/// Program loading (loaded_program_id, loaded_program_name, program_lines) is handled
/// by handle_load_program to avoid overwriting the program lines on every frame.
fn update_execution_state(
    executor: Res<ProgramExecutor>,
    mut execution_states: Query<&mut ExecutionState>,
) {
    for mut state in execution_states.iter_mut() {
        let is_running = executor.is_running();
        let is_paused = executor.is_paused();

        // Only update if something changed to avoid unnecessary syncs
        let needs_update =
            state.running != is_running ||
            state.paused != is_paused ||
            state.current_line != executor.completed_line ||
            state.total_lines != executor.total_instructions ||
            state.loaded_program_id != executor.loaded_program_id;

        if needs_update {
            state.loaded_program_id = executor.loaded_program_id;
            state.loaded_program_name = executor.loaded_program_name.clone();
            state.running = is_running;
            state.paused = is_paused;
            state.current_line = executor.completed_line;
            state.total_lines = executor.total_instructions;
        }
    }
}

