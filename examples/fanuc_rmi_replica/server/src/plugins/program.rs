//! Program execution plugin with orchestrator pattern.
//!
//! Architecture (following meteorite pattern):
//! 1. LoadProgram: Creates Program component on System entity with instructions
//! 2. StartProgram: Sets Program state to Running
//! 3. Orchestrator: Watches for Running state, dispatches instructions in batches
//! 4. Robot Executor: Receives instructions, sends to driver, tracks completions
//! 5. Completion: Orchestrator detects all instructions complete, sets state to Completed

use bevy::prelude::*;
use fanuc_rmi::packets::{SendPacket, ResponsePacket, PacketPriority};
use fanuc_rmi::instructions::FrcLinearMotion;
use fanuc_rmi::{TermType, SpeedType, Configuration, Position};
use fanuc_replica_types::{
    Instruction, ProgramNotification, ProgramNotificationKind,
    ConsoleLogEntry, ConsoleDirection, ConsoleMsgType,
    ExecutionState, ProgramExecutionState,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use super::connection::{FanucRobot, RmiDriver, RobotConnectionState, RmiExecutionResponseChannel};

/// Global sequence counter for program notifications.
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

// ============================================================================
// Program State Machine
// ============================================================================

/// Program execution state (like meteorite's PrinterStatus).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProgramState {
    /// Program is loaded but not running.
    #[default]
    Idle,
    /// Program is actively executing.
    Running,
    /// Program is paused (can be resumed).
    Paused,
    /// Program completed successfully.
    Completed,
    /// Program encountered an error.
    Error,
}

/// Program defaults for building motion packets.
/// Required fields (term_type, term_value) are non-optional - they come from DB.
#[derive(Clone, Default)]
pub struct ProgramDefaults {
    pub w: f64,
    pub p: f64,
    pub r: f64,
    pub speed: f64,
    pub speed_type: String,
    pub term_type: String,
    pub term_value: u8,  // Required: 0-100 for CNT, 0 for FINE
    pub uframe: Option<i32>,
    pub utool: Option<i32>,
}

/// Approach/retreat position.
#[derive(Clone, Default)]
pub struct ApproachRetreat {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: Option<f64>,
    pub p: Option<f64>,
    pub r: Option<f64>,
}

/// Program component - attached to System entity when a program is loaded.
/// This is the single source of truth for program execution state.
#[derive(Component)]
pub struct Program {
    /// Program ID from database.
    pub id: i64,
    /// Program name for display.
    pub name: String,
    /// All instructions (including approach/retreat as first/last).
    pub instructions: Vec<Instruction>,
    /// Current instruction index (next to send).
    pub current_index: usize,
    /// Number of instructions completed (responses received).
    pub completed_count: usize,
    /// Current execution state.
    pub state: ProgramState,
    /// Program defaults for motion.
    pub defaults: ProgramDefaults,
    /// Approach position (optional).
    pub approach: Option<ApproachRetreat>,
    /// Retreat position (optional).
    pub retreat: Option<ApproachRetreat>,
    /// Move speed for approach/retreat.
    pub move_speed: f64,
}

impl Program {
    /// Total number of instructions (including approach/retreat).
    pub fn total_instructions(&self) -> usize {
        let mut count = self.instructions.len();
        if self.approach.is_some() { count += 1; }
        if self.retreat.is_some() { count += 1; }
        count
    }

    /// Check if all instructions have been sent.
    pub fn all_sent(&self) -> bool {
        self.current_index >= self.total_instructions()
    }

    /// Check if all instructions have been completed.
    #[allow(dead_code)]
    pub fn all_completed(&self) -> bool {
        self.completed_count >= self.total_instructions()
    }
}

/// Execution buffer on the Robot entity - tracks in-flight instructions.
/// Similar to meteorite's PrinterExecutionBuffer.
#[derive(Component, Default)]
pub struct ExecutionBuffer {
    /// Maximum instructions to keep in flight (conservative: 5 of 8 available slots).
    pub max_in_flight: usize,
    /// In-flight instructions by request_id -> (line_number, instruction_index).
    pub in_flight_by_request: HashMap<u64, (usize, usize)>,
    /// In-flight instructions by sequence_id -> (line_number, instruction_index).
    pub in_flight_by_sequence: HashMap<u32, (usize, usize)>,
}

impl ExecutionBuffer {
    pub fn new() -> Self {
        Self {
            max_in_flight: 5,
            in_flight_by_request: HashMap::new(),
            in_flight_by_sequence: HashMap::new(),
        }
    }

    /// Number of instructions currently in flight.
    pub fn in_flight_count(&self) -> usize {
        self.in_flight_by_request.len() + self.in_flight_by_sequence.len()
    }

    /// Check if we can send more instructions.
    pub fn can_send(&self) -> bool {
        self.in_flight_count() < self.max_in_flight
    }

    /// How many more instructions we can send.
    #[allow(dead_code)]
    pub fn available_slots(&self) -> usize {
        self.max_in_flight.saturating_sub(self.in_flight_count())
    }

    /// Record that an instruction was sent (by request_id from driver).
    pub fn record_sent(&mut self, request_id: u64, line_number: usize, instruction_index: usize) {
        self.in_flight_by_request.insert(request_id, (line_number, instruction_index));
    }

    /// Map request_id to sequence_id when SentInstructionInfo arrives.
    pub fn map_sequence(&mut self, request_id: u64, sequence_id: u32) {
        if let Some(info) = self.in_flight_by_request.remove(&request_id) {
            self.in_flight_by_sequence.insert(sequence_id, info);
        }
    }

    /// Handle instruction completion by sequence_id.
    /// Returns (line_number, instruction_index) if found.
    pub fn handle_completion(&mut self, sequence_id: u32) -> Option<(usize, usize)> {
        self.in_flight_by_sequence.remove(&sequence_id)
    }

    /// Clear all in-flight tracking.
    pub fn clear(&mut self) {
        self.in_flight_by_request.clear();
        self.in_flight_by_sequence.clear();
    }
}

// ============================================================================
// Packet Building
// ============================================================================

/// Build a motion instruction packet from a program instruction.
pub fn build_motion_packet(instruction: &Instruction, defaults: &ProgramDefaults, is_last: bool) -> SendPacket {
    let w = instruction.w.unwrap_or(defaults.w);
    let p = instruction.p.unwrap_or(defaults.p);
    let r = instruction.r.unwrap_or(defaults.r);
    let speed = instruction.speed.unwrap_or(defaults.speed);

    let speed_type = match defaults.speed_type.as_str() {
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
        match instruction.term_type.as_deref().unwrap_or(&defaults.term_type) {
            "FINE" => TermType::FINE,
            _ => TermType::CNT,
        }
    };

    // Use instruction's term_value if set, otherwise use program default
    let term_value = instruction.term_value.unwrap_or(defaults.term_value);

    let position = Position {
        x: instruction.x,
        y: instruction.y,
        z: instruction.z,
        w, p, r,
        ext1: 0.0,
        ext2: 0.0,
        ext3: 0.0,
    };

    let uframe = instruction.uframe.or(defaults.uframe).unwrap_or(1) as i8;
    let utool = instruction.utool.or(defaults.utool).unwrap_or(1) as i8;

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
    pos: &ApproachRetreat,
    defaults: &ProgramDefaults,
    line_number: usize,
    speed: f64,
    is_last: bool,
) -> SendPacket {
    let w = pos.w.unwrap_or(defaults.w);
    let p = pos.p.unwrap_or(defaults.p);
    let r = pos.r.unwrap_or(defaults.r);

    let (term_type, term_value) = if is_last {
        (TermType::FINE, 0)
    } else {
        (TermType::CNT, defaults.term_value)
    };

    let position = Position {
        x: pos.x, y: pos.y, z: pos.z, w, p, r,
        ext1: 0.0, ext2: 0.0, ext3: 0.0,
    };

    let uframe = defaults.uframe.unwrap_or(1) as i8;
    let utool = defaults.utool.unwrap_or(1) as i8;

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

// ============================================================================
// Plugin
// ============================================================================

pub struct ProgramPlugin;

impl Plugin for ProgramPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            orchestrator_dispatch,
            process_instruction_responses,
            update_execution_state,
            reset_on_disconnect,
        ).chain());
    }
}

// ============================================================================
// Systems
// ============================================================================

use bevy_tokio_tasks::TokioTasksRuntime;
use pl3xus::Network;
use pl3xus_websockets::WebSocketProvider;
use super::connection::RmiSentInstructionChannel;
use super::system::ActiveSystem;

/// Orchestrator system - dispatches instructions from Program to Robot.
/// Watches for Program in Running state and sends batches of instructions.
fn orchestrator_dispatch(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut systems: Query<&mut Program, With<ActiveSystem>>,
    mut robots: Query<(&RmiDriver, &mut ExecutionBuffer, &RobotConnectionState), With<FanucRobot>>,
) {
    // Get the program (if any)
    let Ok(mut program) = systems.single_mut() else {
        return;
    };

    // Only dispatch if running
    if program.state != ProgramState::Running {
        return;
    }

    // Get the connected robot
    let Ok((driver, mut buffer, state)) = robots.single_mut() else {
        return;
    };

    if *state != RobotConnectionState::Connected {
        return;
    }

    // Enter the Tokio runtime context so send_packet can use tokio::spawn
    let _guard = tokio_runtime.runtime().enter();

    // Send instructions while we have slots and instructions to send
    let total = program.total_instructions();
    let has_approach = program.approach.is_some();
    let has_retreat = program.retreat.is_some();

    while buffer.can_send() && !program.all_sent() {
        let idx = program.current_index;
        let is_last = idx == total - 1;

        // Determine what instruction to send based on index
        let (packet, line_number) = if has_approach && idx == 0 {
            // First instruction is approach
            let approach = program.approach.as_ref().unwrap();
            info!("📤 Sending approach move to ({:.2}, {:.2}, {:.2})", approach.x, approach.y, approach.z);
            (build_approach_retreat_packet(approach, &program.defaults, 0, program.move_speed, false), 0)
        } else if has_retreat && idx == total - 1 {
            // Last instruction is retreat
            let retreat = program.retreat.as_ref().unwrap();
            info!("📤 Sending retreat move to ({:.2}, {:.2}, {:.2})", retreat.x, retreat.y, retreat.z);
            (build_approach_retreat_packet(retreat, &program.defaults, idx, program.move_speed, true), idx)
        } else {
            // Regular instruction
            let instr_idx = if has_approach { idx - 1 } else { idx };
            if instr_idx >= program.instructions.len() {
                error!("Instruction index {} out of bounds (len={})", instr_idx, program.instructions.len());
                break;
            }
            let instruction = &program.instructions[instr_idx];
            let is_last_instr = if has_retreat { idx == total - 2 } else { is_last };
            (build_motion_packet(instruction, &program.defaults, is_last_instr), instruction.line_number as usize)
        };

        // Send the packet
        match driver.0.send_packet(packet, PacketPriority::Standard) {
            Ok(request_id) => {
                buffer.record_sent(request_id, line_number, idx);
                info!("📤 Sent instruction {} (request_id {}, idx {})", line_number, request_id, idx);
                program.current_index += 1;
            }
            Err(e) => {
                error!("Failed to send instruction {}: {}", line_number, e);
                program.state = ProgramState::Error;
                break;
            }
        }
    }
}


/// Process instruction responses and update completion tracking.
/// Uses RmiExecutionResponseChannel (separate from polling channel) to avoid contention.
fn process_instruction_responses(
    mut systems: Query<&mut Program, With<ActiveSystem>>,
    mut robots: Query<(&mut ExecutionBuffer, &mut RmiExecutionResponseChannel, Option<&mut RmiSentInstructionChannel>, &RobotConnectionState), With<FanucRobot>>,
    net: Res<Network<WebSocketProvider>>,
) {
    // Get the program (if any)
    let Ok(mut program) = systems.single_mut() else {
        return;
    };

    // Only process if running
    if program.state != ProgramState::Running {
        return;
    }

    for (mut buffer, mut execution_response_channel, sent_channel, state) in robots.iter_mut() {
        if *state != RobotConnectionState::Connected {
            continue;
        }

        // Process SentInstructionInfo to map request_id -> sequence_id
        if let Some(mut sent_rx) = sent_channel {
            while let Ok(sent_info) = sent_rx.0.try_recv() {
                buffer.map_sequence(sent_info.request_id, sent_info.sequence_id);
                debug!("🔗 Mapped request {} -> sequence {}", sent_info.request_id, sent_info.sequence_id);
            }
        }

        // Check for instruction responses (using dedicated execution channel)
        while let Ok(response) = execution_response_channel.0.try_recv() {
            match &response {
                ResponsePacket::InstructionResponse(instr_resp) => {
                    let seq_id = instr_resp.get_sequence_id();
                    if let Some((line, _idx)) = buffer.handle_completion(seq_id) {
                        program.completed_count += 1;
                        info!("📍 Instruction {} completed (seq_id {}, {}/{})",
                            line, seq_id, program.completed_count, program.total_instructions());

                        // Check for error
                        if instr_resp.get_error_id() != 0 {
                            let error_msg = format!("Instruction error: {}", instr_resp.get_error_id());
                            error!("Instruction {} failed with error {}", line, instr_resp.get_error_id());

                            // Broadcast error notification
                            let notification = new_notification(ProgramNotificationKind::Error {
                                program_name: program.name.clone(),
                                at_line: line,
                                error_message: error_msg.clone(),
                            });
                            net.broadcast(notification);

                            let console_msg = console_entry(
                                format!("Program '{}' error at line {}: {}", program.name, line, error_msg),
                                ConsoleDirection::System,
                                ConsoleMsgType::Error,
                            );
                            net.broadcast(console_msg);

                            program.state = ProgramState::Error;
                            buffer.clear();
                            return;
                        }
                    }
                }
                _ => {}
            }
        }

        // Check if execution is complete
        if program.all_sent() && buffer.in_flight_count() == 0 {
            let total = program.total_instructions();
            info!("✅ Program '{}' execution complete: {} instructions", program.name, total);
            program.state = ProgramState::Completed;

            // Broadcast completion notification
            let notification = new_notification(ProgramNotificationKind::Completed {
                program_name: program.name.clone(),
                total_instructions: total,
            });
            net.broadcast(notification);

            let console_msg = console_entry(
                format!("Program '{}' completed ({} instructions)", program.name, total),
                ConsoleDirection::System,
                ConsoleMsgType::Status,
            );
            net.broadcast(console_msg);
        }
    }
}

/// Update the ExecutionState synced component from Program state.
/// This is the core of the server-driven UI pattern:
/// - Maps internal ProgramState to shared ProgramExecutionState
/// - Computes available actions based on state machine rules
/// - Client simply reflects these values without any logic
fn update_execution_state(
    systems: Query<&Program, With<ActiveSystem>>,
    mut execution_states: Query<&mut ExecutionState>,
) {
    let Ok(program) = systems.single() else {
        return;
    };

    // Map internal state to shared state enum
    let new_state = match program.state {
        ProgramState::Idle => ProgramExecutionState::Idle,
        ProgramState::Running => ProgramExecutionState::Running,
        ProgramState::Paused => ProgramExecutionState::Paused,
        ProgramState::Completed => ProgramExecutionState::Completed,
        ProgramState::Error => ProgramExecutionState::Error,
    };

    // Compute available actions based on state machine
    // This is the single source of truth for what actions are valid
    // can_load is only true when NO program is loaded (NoProgram state)
    let (can_load, can_start, can_pause, can_resume, can_stop, can_unload) = match new_state {
        ProgramExecutionState::NoProgram => (true, false, false, false, false, false),
        ProgramExecutionState::Idle => (false, true, false, false, false, true),
        ProgramExecutionState::Running => (false, false, true, false, true, false),
        ProgramExecutionState::Paused => (false, false, false, true, true, false),
        ProgramExecutionState::Completed => (false, true, false, false, false, true),
        ProgramExecutionState::Error => (false, true, false, false, false, true),
    };

    for mut state in execution_states.iter_mut() {
        // Skip if ExecutionState is in NoProgram state - this means unload was just called
        // and the Program component removal is still pending (deferred commands).
        // We don't want to overwrite the unload handler's changes.
        if state.state == ProgramExecutionState::NoProgram {
            continue;
        }

        // Only update if something changed to avoid unnecessary syncs
        let needs_update =
            state.loaded_program_id != Some(program.id) ||
            state.state != new_state ||
            state.current_line != program.completed_count ||
            state.total_lines != program.total_instructions() ||
            state.can_load != can_load ||
            state.can_start != can_start ||
            state.can_pause != can_pause ||
            state.can_resume != can_resume ||
            state.can_stop != can_stop ||
            state.can_unload != can_unload;

        if needs_update {
            state.loaded_program_id = Some(program.id);
            state.loaded_program_name = Some(program.name.clone());
            state.state = new_state;
            state.current_line = program.completed_count;
            state.total_lines = program.total_instructions();
            state.can_load = can_load;
            state.can_start = can_start;
            state.can_pause = can_pause;
            state.can_resume = can_resume;
            state.can_stop = can_stop;
            state.can_unload = can_unload;
        }
    }
}

/// Reset program state when robot disconnects.
fn reset_on_disconnect(
    mut commands: Commands,
    systems: Query<(Entity, &Program), With<ActiveSystem>>,
    mut robots: Query<(&RobotConnectionState, Option<&mut ExecutionBuffer>), With<FanucRobot>>,
    mut execution_states: Query<&mut ExecutionState>,
) {
    // Check if any robot is connected
    let any_connected = robots.iter().any(|(state, _)| *state == RobotConnectionState::Connected);

    if any_connected {
        return;
    }

    // If no robot connected and we have a running program, reset it
    for (entity, program) in systems.iter() {
        if program.state == ProgramState::Running || program.state == ProgramState::Paused {
            info!("🔌 Robot disconnected while executing - removing Program component");
            commands.entity(entity).remove::<Program>();

            // Clear ExecutionBuffer on all robots
            for (_, buffer_opt) in robots.iter_mut() {
                if let Some(mut buffer) = buffer_opt {
                    buffer.clear();
                    info!("🧹 Cleared ExecutionBuffer on disconnect");
                }
            }

            // Reset execution state to NoProgram
            for mut state in execution_states.iter_mut() {
                state.state = ProgramExecutionState::NoProgram;
                state.loaded_program_id = None;
                state.loaded_program_name = None;
                state.current_line = 0;
                state.total_lines = 0;
                state.program_lines.clear();
                // Reset available actions for NoProgram state
                state.can_load = true;
                state.can_start = false;
                state.can_pause = false;
                state.can_resume = false;
                state.can_stop = false;
                state.can_unload = false;
            }
        }
    }
}