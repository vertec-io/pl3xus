//! Program execution plugin.
//!
//! Handles program execution with buffered streaming:
//! - Loads programs from database
//! - Sends instructions to robot via driver
//! - Tracks progress and updates ExecutionState
//! - Handles pause/resume/stop

use bevy::prelude::*;
use bevy_tokio_tasks::TokioTasksRuntime;
use std::collections::{VecDeque, HashMap};
use fanuc_rmi::packets::{SendPacket, ResponsePacket, SentInstructionInfo};
use fanuc_rmi::instructions::FrcLinearMotion;
use fanuc_rmi::{TermType, SpeedType, Configuration, Position};
use fanuc_replica_types::{ExecutionState, Instruction};
use tokio::sync::broadcast;

use super::connection::{FanucRobot, RmiDriver, RmiResponseChannel, RobotConnectionState};

/// Maximum instructions to send ahead (conservative: use 5 of 8 available slots).
pub const MAX_BUFFER: usize = 5;

// ============================================================================
// Resources
// ============================================================================

/// Program executor state - manages buffered execution.
#[derive(Resource, Default)]
pub struct ProgramExecutor {
    /// Currently loaded program ID.
    pub loaded_program_id: Option<i64>,
    /// Program name for display.
    pub loaded_program_name: Option<String>,
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
    /// Whether execution is running (not paused).
    pub running: bool,
    /// Whether execution is paused.
    pub paused: bool,
    /// Program defaults for building packets.
    pub defaults: ProgramDefaults,
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
    /// Reset the executor to idle state.
    pub fn reset(&mut self) {
        self.loaded_program_id = None;
        self.loaded_program_name = None;
        self.total_instructions = 0;
        self.pending_queue.clear();
        self.in_flight_by_request.clear();
        self.in_flight_by_sequence.clear();
        self.completed_line = 0;
        self.running = false;
        self.paused = false;
    }

    /// Check if there are more instructions to send.
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

    /// Check if execution is complete.
    pub fn is_complete(&self) -> bool {
        self.running && self.pending_queue.is_empty() && self.in_flight_by_sequence.is_empty() && self.in_flight_by_request.is_empty()
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
            process_instruction_responses,
            update_execution_state,
        ));
    }
}

// ============================================================================
// Systems
// ============================================================================

/// Process instruction responses and update executor state.
fn process_instruction_responses(
    tokio_runtime: Res<TokioTasksRuntime>,
    mut executor: ResMut<ProgramExecutor>,
    mut robots: Query<(&RmiDriver, &mut RmiResponseChannel, Option<&mut RmiSentInstructionChannel>, &RobotConnectionState), With<FanucRobot>>,
) {
    if !executor.running || executor.paused {
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
                            error!("Instruction {} failed with error {}", line, instr_resp.get_error_id());
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
            info!("✅ Program execution complete: {} instructions", executor.total_instructions);
            executor.running = false;
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
        // Only update if something changed to avoid unnecessary syncs
        let needs_update =
            state.running != executor.running ||
            state.paused != executor.paused ||
            state.current_line != executor.completed_line ||
            state.total_lines != executor.total_instructions ||
            state.loaded_program_id != executor.loaded_program_id;

        if needs_update {
            state.loaded_program_id = executor.loaded_program_id;
            state.loaded_program_name = executor.loaded_program_name.clone();
            state.running = executor.running;
            state.paused = executor.paused;
            state.current_line = executor.completed_line;
            state.total_lines = executor.total_instructions;
            // NOTE: program_lines are only set by handle_load_program, not here
        }
    }
}

