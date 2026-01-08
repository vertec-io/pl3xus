//! Execution handlers for program load/unload operations.
//!
//! This module contains handlers for:
//! - Loading programs into the execution buffer
//! - Unloading programs from the execution buffer

use bevy::prelude::*;
use bevy::ecs::message::MessageReader;
use pl3xus_sync::AuthorizedRequest;

use robot_hmi_core::types::ActiveSystem;
use robot_hmi_core::database::DatabaseResource;
use robot_hmi_execution::types::{
    BufferDisplayData, BufferLineDisplay, BufferState, ExecutionCoordinator, ExecutionPoint,
    ExecutionState, MotionCommand, MotionType, SourceType, SystemState, ToolpathBuffer,
};
use robot_hmi_robotics::frame::FrameId;
use robot_hmi_robotics::pose::RobotPose;
use crate::database::queries;
use crate::types::*;

/// Handle Load request - loads a program into the execution buffer.
///
/// This is the static program loader that:
/// 1. Fetches program from database
/// 2. Converts instructions to ExecutionPoints
/// 3. Populates ToolpathBuffer
/// 4. Updates BufferDisplayData for UI
/// 5. Updates ExecutionState with source info
pub fn handle_load(
    mut commands: Commands,
    mut requests: MessageReader<AuthorizedRequest<Load>>,
    db: Option<Res<DatabaseResource>>,
    system_query: Query<Entity, With<ActiveSystem>>,
    coordinator_query: Query<&ExecutionCoordinator, With<ActiveSystem>>,
    mut execution_states: Query<&mut ExecutionState, With<ActiveSystem>>,
    _buffer_displays: Query<&mut BufferDisplayData, With<ActiveSystem>>,
) {
    for request in requests.read() {
        let request = request.clone();
        let program_id = request.get_request().program_id;
        info!("📋 Handling Load request for program {}", program_id);

        // Get system entity
        let Ok(system_entity) = system_query.single() else {
            let _ = request.respond(LoadResponse {
                success: false,
                program: None,
                error: Some("System not ready".to_string()),
            });
            continue;
        };

        // Check if something is already loaded
        if coordinator_query.get(system_entity).is_ok() {
            let _ = request.respond(LoadResponse {
                success: false,
                program: None,
                error: Some("A program is already loaded. Unload first.".to_string()),
            });
            continue;
        }

        // Get database
        let Some(db_res) = db.as_ref() else {
            let _ = request.respond(LoadResponse {
                success: false,
                program: None,
                error: Some("Database not available".to_string()),
            });
            continue;
        };

        let conn = db_res.connection();
        let conn = conn.lock().unwrap();

        // Fetch program from database
        match queries::get_program(&conn, program_id) {
            Ok(Some(program_detail)) => {
                load_program_into_buffer(
                    &mut commands,
                    &mut execution_states,
                    system_entity,
                    &program_detail,
                    &request,
                );
            }
            Ok(None) => {
                let _ = request.respond(LoadResponse {
                    success: false,
                    program: None,
                    error: Some(format!("Program {} not found", program_id)),
                });
            }
            Err(e) => {
                error!("❌ Database error loading program: {}", e);
                let _ = request.respond(LoadResponse {
                    success: false,
                    program: None,
                    error: Some(format!("Database error: {}", e)),
                });
            }
        }
    }
}

fn load_program_into_buffer(
    commands: &mut Commands,
    execution_states: &mut Query<&mut ExecutionState, With<ActiveSystem>>,
    system_entity: Entity,
    program_detail: &ProgramDetail,
    request: &AuthorizedRequest<Load>,
) {
    // Build defaults for missing instruction fields
    let default_speed = program_detail.default_speed.unwrap_or(100.0);
    let default_term_type = program_detail.default_term_type.clone()
        .unwrap_or_else(|| "FINE".to_string());

    // Count total instructions
    let approach_count: usize = program_detail.approach_sequences.iter()
        .map(|s| s.instructions.len()).sum();
    let main_count: usize = program_detail.main_sequences.iter()
        .map(|s| s.instructions.len()).sum();
    let retreat_count: usize = program_detail.retreat_sequences.iter()
        .map(|s| s.instructions.len()).sum();
    let total_points = approach_count + main_count + retreat_count;

    // Create static buffer with known total
    let mut toolpath_buffer = ToolpathBuffer::new_static(total_points as u32);
    let mut display_data = BufferDisplayData::new();
    let mut lines: Vec<ProgramLineInfo> = Vec::with_capacity(total_points);
    let mut point_index: u32 = 0;

    // Helper to process an instruction
    let mut process_instruction = |instruction: &Instruction, seq_name: Option<&str>| {
        let speed = instruction.speed.unwrap_or(default_speed);
        let term_type = instruction.term_type.clone()
            .unwrap_or_else(|| default_term_type.clone());

        // Create ExecutionPoint with RobotPose
        let pose = RobotPose::from_xyz_wpr(
            instruction.x,
            instruction.y,
            instruction.z,
            instruction.w.unwrap_or(0.0),
            instruction.p.unwrap_or(0.0),
            instruction.r.unwrap_or(0.0),
            FrameId::World,
        );

        let motion = MotionCommand {
            speed: speed as f32,
            motion_type: MotionType::Linear,
            blend_radius: if term_type == "FINE" { 0.0 } else { 5.0 },
        };

        let exec_point = ExecutionPoint::new(point_index, pose)
            .with_motion(motion);
        toolpath_buffer.push(exec_point);

        // Add to display data
        display_data.push_line(BufferLineDisplay {
            index: point_index as usize,
            line_type: "Move".to_string(),
            description: format!("({:.1}, {:.1}, {:.1})",
                instruction.x, instruction.y, instruction.z),
            sequence_name: seq_name.map(|s| s.to_string()),
            source_line: Some(instruction.line_number as usize),
            x: instruction.x,
            y: instruction.y,
            z: instruction.z,
            w: instruction.w.unwrap_or(0.0),
            p: instruction.p.unwrap_or(0.0),
            r: instruction.r.unwrap_or(0.0),
            speed,
            term_type: term_type.clone(),
        });

        // Add to program lines for response
        lines.push(ProgramLineInfo {
            x: instruction.x,
            y: instruction.y,
            z: instruction.z,
            w: instruction.w.unwrap_or(0.0),
            p: instruction.p.unwrap_or(0.0),
            r: instruction.r.unwrap_or(0.0),
            speed,
            term_type,
        });

        point_index += 1;
    };

    // Process approach sequences
    for seq in &program_detail.approach_sequences {
        let seq_name = seq.name.as_deref().unwrap_or("Approach");
        for instruction in &seq.instructions {
            process_instruction(instruction, Some(seq_name));
        }
    }

    // Process main sequences (concatenate in order)
    for seq in &program_detail.main_sequences {
        let seq_name = seq.name.as_deref().unwrap_or("Main");
        for instruction in &seq.instructions {
            process_instruction(instruction, Some(seq_name));
        }
    }

    // Process retreat sequences
    for seq in &program_detail.retreat_sequences {
        let seq_name = seq.name.as_deref().unwrap_or("Retreat");
        for instruction in &seq.instructions {
            process_instruction(instruction, Some(seq_name));
        }
    }

    info!("📋 Program '{}' loaded: {} points in buffer",
        &program_detail.name, toolpath_buffer.len());

    // Add execution components to System entity
    commands.entity(system_entity).insert((
        ExecutionCoordinator::with_name(
            format!("program_{}", program_detail.id),
            program_detail.name.clone(),
        ),
        toolpath_buffer,
        BufferState::Ready,
        display_data,
    ));

    // Update ExecutionState
    if let Ok(mut exec_state) = execution_states.single_mut() {
        exec_state.state = SystemState::Ready;
        exec_state.source_type = SourceType::StaticProgram;
        exec_state.source_name = Some(program_detail.name.clone());
        exec_state.source_id = Some(program_detail.id);
        exec_state.current_index = 0;
        exec_state.total_points = Some(total_points);
        exec_state.points_executed = 0;
        exec_state.update_execution_actions();
        info!("📡 ExecutionState updated: source='{}', {} points",
            program_detail.name, total_points);
    }

    // Build response
    let program_with_lines = ProgramWithLines {
        id: program_detail.id,
        name: program_detail.name.clone(),
        description: program_detail.description.clone(),
        lines,
        approach_lines: Vec::new(),
        retreat_lines: Vec::new(),
    };

    let _ = request.clone().respond(LoadResponse {
        success: true,
        program: Some(program_with_lines),
        error: None,
    });
}

/// Handle Unload request - unloads the currently loaded program.
pub fn handle_unload(
    mut commands: Commands,
    mut requests: MessageReader<AuthorizedRequest<Unload>>,
    system_query: Query<Entity, With<ActiveSystem>>,
    coordinator_query: Query<&ExecutionCoordinator, With<ActiveSystem>>,
    mut execution_states: Query<&mut ExecutionState, With<ActiveSystem>>,
    mut buffer_displays: Query<&mut BufferDisplayData, With<ActiveSystem>>,
) {
    for request in requests.read() {
        let request = request.clone();
        info!("📋 Handling Unload request");

        // Get system entity
        let Ok(system_entity) = system_query.single() else {
            let _ = request.respond(UnloadResponse {
                success: false,
                error: Some("System not ready".to_string()),
            });
            continue;
        };

        // Check if a program is loaded
        if coordinator_query.get(system_entity).is_err() {
            let _ = request.respond(UnloadResponse {
                success: false,
                error: Some("No program loaded".to_string()),
            });
            continue;
        };

        // Remove execution components (but not synced components like ExecutionState/BufferDisplayData)
        commands.entity(system_entity).remove::<ExecutionCoordinator>();
        commands.entity(system_entity).remove::<ToolpathBuffer>();
        commands.entity(system_entity).remove::<BufferState>();
        info!("📦 Removed execution components from System entity");

        // Reset ExecutionState
        if let Ok(mut exec_state) = execution_states.single_mut() {
            *exec_state = ExecutionState::no_source();
            info!("📡 ExecutionState reset to NoSource");
        }

        // Clear BufferDisplayData
        if let Ok(mut buffer_display) = buffer_displays.single_mut() {
            buffer_display.clear();
            info!("📡 BufferDisplayData cleared");
        }

        let _ = request.respond(UnloadResponse {
            success: true,
            error: None,
        });
    }
}