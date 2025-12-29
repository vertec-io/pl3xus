//! Database queries for programs.

use rusqlite::{Connection, OptionalExtension};
use crate::types::{
    ProgramInfo, ProgramDetail, Instruction, InstructionSequence, SequenceType,
};

// ============================================================================
// Programs CRUD
// ============================================================================

/// List all programs with instruction counts.
pub fn list_programs(conn: &Connection) -> anyhow::Result<Vec<ProgramInfo>> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.name, p.description,
                (SELECT COUNT(*) FROM program_instructions pi 
                 JOIN program_sequences ps ON pi.sequence_id = ps.id 
                 WHERE ps.program_id = p.id AND ps.sequence_type = 'main') as instruction_count,
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

/// Get a program with all its sequences and instructions.
pub fn get_program(conn: &Connection, id: i64) -> anyhow::Result<Option<ProgramDetail>> {
    let program: Option<(i64, String, Option<String>, Option<f64>, Option<String>, Option<u8>, f64, String, String)> = 
        conn.query_row(
            "SELECT id, name, description, default_speed, default_term_type, default_term_value,
                    COALESCE(move_speed, 100.0),
                    COALESCE(created_at, datetime('now')), COALESCE(updated_at, datetime('now'))
             FROM programs WHERE id = ?",
            [id],
            |row| Ok((
                row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?,
                row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?
            )),
        ).optional()?;

    let Some((id, name, description, default_speed, default_term_type, default_term_value, move_speed, created_at, updated_at)) = program else {
        return Ok(None);
    };

    // Get all sequences for this program
    let approach_sequences = get_sequences(conn, id, SequenceType::Approach)?;
    let main_sequences = get_sequences(conn, id, SequenceType::Main)?;
    let retreat_sequences = get_sequences(conn, id, SequenceType::Retreat)?;

    // Main sequence - there should be exactly one, create empty if missing
    let main_sequence = main_sequences.into_iter().next().unwrap_or_else(|| {
        InstructionSequence {
            id: 0,
            sequence_type: SequenceType::Main,
            name: None,
            order_index: 0,
            instructions: vec![],
        }
    });

    Ok(Some(ProgramDetail {
        id,
        name,
        description,
        default_speed,
        default_term_type,
        default_term_value,
        move_speed,
        approach_sequences,
        main_sequence,
        retreat_sequences,
        created_at,
        updated_at,
    }))
}

/// Get sequences of a specific type for a program.
fn get_sequences(conn: &Connection, program_id: i64, seq_type: SequenceType) -> anyhow::Result<Vec<InstructionSequence>> {
    let type_str = match seq_type {
        SequenceType::Approach => "approach",
        SequenceType::Main => "main",
        SequenceType::Retreat => "retreat",
    };

    let mut stmt = conn.prepare(
        "SELECT id, name, order_index FROM program_sequences 
         WHERE program_id = ? AND sequence_type = ?
         ORDER BY order_index"
    )?;

    let sequences: Vec<(i64, Option<String>, i32)> = stmt.query_map(
        rusqlite::params![program_id, type_str],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    )?.collect::<Result<Vec<_>, _>>()?;

    let mut result = Vec::new();
    for (seq_id, name, order_index) in sequences {
        let instructions = get_instructions(conn, seq_id)?;
        result.push(InstructionSequence {
            id: seq_id,
            sequence_type: seq_type,
            name,
            order_index,
            instructions,
        });
    }

    Ok(result)
}

/// Get instructions for a sequence.
fn get_instructions(conn: &Connection, sequence_id: i64) -> anyhow::Result<Vec<Instruction>> {
    let mut stmt = conn.prepare(
        "SELECT line_number, x, y, z, w, p, r, ext1, ext2, ext3, speed, term_type, term_value
         FROM program_instructions WHERE sequence_id = ? ORDER BY line_number"
    )?;

    let instructions = stmt.query_map([sequence_id], |row| {
        Ok(Instruction {
            line_number: row.get(0)?,
            x: row.get(1)?,
            y: row.get(2)?,
            z: row.get(3)?,
            w: row.get(4)?,
            p: row.get(5)?,
            r: row.get(6)?,
            ext1: row.get(7)?,
            ext2: row.get(8)?,
            ext3: row.get(9)?,
            speed: row.get(10)?,
            term_type: row.get(11)?,
            term_value: row.get(12)?,
        })
    })?.collect::<Result<Vec<_>, _>>()?;

    Ok(instructions)
}

// ============================================================================
// Create / Delete / Update
// ============================================================================

/// Create a new program with an empty main sequence.
pub fn create_program(conn: &Connection, name: &str, description: Option<&str>) -> anyhow::Result<i64> {
    conn.execute(
        "INSERT INTO programs (name, description) VALUES (?, ?)",
        rusqlite::params![name, description],
    )?;
    let program_id = conn.last_insert_rowid();

    // Create the main sequence automatically
    conn.execute(
        "INSERT INTO program_sequences (program_id, sequence_type, order_index) VALUES (?, 'main', 0)",
        [program_id],
    )?;

    Ok(program_id)
}

/// Delete a program (cascades to sequences and instructions).
pub fn delete_program(conn: &Connection, id: i64) -> anyhow::Result<()> {
    conn.execute("DELETE FROM programs WHERE id = ?", [id])?;
    Ok(())
}

/// Update program settings.
pub fn update_program_settings(
    conn: &Connection,
    program_id: i64,
    name: Option<&str>,
    description: Option<&str>,
    default_speed: Option<f64>,
    default_term_type: Option<&str>,
    default_term_value: Option<u8>,
    move_speed: Option<f64>,
) -> anyhow::Result<()> {
    let mut updates = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(n) = name {
        updates.push("name = ?");
        params.push(Box::new(n.to_string()));
    }
    if let Some(d) = description {
        updates.push("description = ?");
        params.push(Box::new(d.to_string()));
    }
    if let Some(s) = default_speed {
        updates.push("default_speed = ?");
        params.push(Box::new(s));
    }
    if let Some(t) = default_term_type {
        updates.push("default_term_type = ?");
        params.push(Box::new(t.to_string()));
    }
    if let Some(v) = default_term_value {
        updates.push("default_term_value = ?");
        params.push(Box::new(v as i32));
    }
    if let Some(m) = move_speed {
        updates.push("move_speed = ?");
        params.push(Box::new(m));
    }

    if updates.is_empty() {
        return Ok(());
    }

    updates.push("updated_at = datetime('now')");

    let sql = format!("UPDATE programs SET {} WHERE id = ?", updates.join(", "));
    params.push(Box::new(program_id));

    let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    conn.execute(&sql, params_refs.as_slice())?;

    Ok(())
}

// ============================================================================
// Sequences
// ============================================================================

/// Add a sequence (approach or retreat) to a program.
pub fn add_sequence(
    conn: &Connection,
    program_id: i64,
    seq_type: SequenceType,
    name: Option<&str>,
    instructions: &[Instruction],
) -> anyhow::Result<i64> {
    let type_str = match seq_type {
        SequenceType::Approach => "approach",
        SequenceType::Main => "main",
        SequenceType::Retreat => "retreat",
    };

    // Get next order index for this type
    let max_order: Option<i32> = conn.query_row(
        "SELECT MAX(order_index) FROM program_sequences WHERE program_id = ? AND sequence_type = ?",
        rusqlite::params![program_id, type_str],
        |row| row.get(0),
    ).optional()?.flatten();

    let order_index = max_order.map(|m| m + 1).unwrap_or(0);

    conn.execute(
        "INSERT INTO program_sequences (program_id, sequence_type, name, order_index) VALUES (?, ?, ?, ?)",
        rusqlite::params![program_id, type_str, name, order_index],
    )?;
    let sequence_id = conn.last_insert_rowid();

    // Insert instructions
    insert_instructions(conn, sequence_id, instructions)?;

    Ok(sequence_id)
}

/// Remove a sequence.
pub fn remove_sequence(conn: &Connection, sequence_id: i64) -> anyhow::Result<()> {
    conn.execute("DELETE FROM program_sequences WHERE id = ?", [sequence_id])?;
    Ok(())
}

/// Get the main sequence ID for a program.
pub fn get_main_sequence_id(conn: &Connection, program_id: i64) -> anyhow::Result<Option<i64>> {
    let id: Option<i64> = conn.query_row(
        "SELECT id FROM program_sequences WHERE program_id = ? AND sequence_type = 'main' LIMIT 1",
        [program_id],
        |row| row.get(0),
    ).optional()?;
    Ok(id)
}

// ============================================================================
// Instructions
// ============================================================================

/// Insert instructions into a sequence, replacing any existing ones.
pub fn insert_instructions(conn: &Connection, sequence_id: i64, instructions: &[Instruction]) -> anyhow::Result<()> {
    // Clear existing instructions
    conn.execute("DELETE FROM program_instructions WHERE sequence_id = ?", [sequence_id])?;

    // Insert new instructions
    for instr in instructions {
        conn.execute(
            "INSERT INTO program_instructions
             (sequence_id, line_number, x, y, z, w, p, r, ext1, ext2, ext3, speed, term_type, term_value)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                sequence_id,
                instr.line_number,
                instr.x,
                instr.y,
                instr.z,
                instr.w,
                instr.p,
                instr.r,
                instr.ext1,
                instr.ext2,
                instr.ext3,
                instr.speed,
                instr.term_type,
                instr.term_value.map(|v| v as i32),
            ],
        )?;
    }

    Ok(())
}

