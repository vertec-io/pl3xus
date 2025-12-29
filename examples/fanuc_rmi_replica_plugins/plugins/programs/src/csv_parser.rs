//! Flexible CSV parser for program instructions.
//!
//! Supports multiple column formats:
//! - Required: X, Y, Z (any capitalization)
//! - Optional: W, P, R, EXT1, EXT2, EXT3, SPEED, TERM_TYPE, TERM_VALUE

use crate::types::Instruction;
use std::collections::HashMap;

/// Result of parsing a CSV file.
#[derive(Debug)]
pub struct ParseResult {
    pub instructions: Vec<Instruction>,
    pub warnings: Vec<ParseWarning>,
}

/// A warning during parsing (non-fatal).
#[derive(Debug, Clone)]
pub struct ParseWarning {
    pub line: usize,
    pub message: String,
}

/// A parsing error (fatal).
#[derive(Debug)]
pub struct ParseError {
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ParseError {}

/// Parse CSV content into instructions.
///
/// # Arguments
/// * `content` - CSV content as string
///
/// # Returns
/// * `ParseResult` with instructions and any warnings
pub fn parse_csv(content: &str) -> Result<ParseResult, ParseError> {
    let mut lines = content.lines();
    let mut warnings = Vec::new();
    
    // Parse header line
    let header_line = lines.next().ok_or_else(|| ParseError {
        message: "CSV is empty".to_string(),
    })?;
    
    let column_map = parse_header(header_line)?;
    
    // Validate required columns
    if !column_map.contains_key("x") || !column_map.contains_key("y") || !column_map.contains_key("z") {
        return Err(ParseError {
            message: "CSV must contain X, Y, and Z columns".to_string(),
        });
    }
    
    let mut instructions = Vec::new();
    let mut line_number = 1;
    
    for (row_index, line) in lines.enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        
        let values: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        
        match parse_row(&values, &column_map, line_number) {
            Ok(instruction) => {
                instructions.push(instruction);
                line_number += 1;
            }
            Err(msg) => {
                warnings.push(ParseWarning {
                    line: row_index + 2, // +2 for 1-indexing and header
                    message: msg,
                });
            }
        }
    }
    
    if instructions.is_empty() && warnings.is_empty() {
        return Err(ParseError {
            message: "No valid instructions found in CSV".to_string(),
        });
    }
    
    Ok(ParseResult {
        instructions,
        warnings,
    })
}

/// Parse header line and return column name -> index mapping.
fn parse_header(line: &str) -> Result<HashMap<String, usize>, ParseError> {
    let columns: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
    
    let mut map = HashMap::new();
    for (index, col) in columns.iter().enumerate() {
        let normalized = col.to_lowercase().replace([' ', '-', '_'], "");
        map.insert(normalized, index);
    }
    
    Ok(map)
}

/// Parse a single row into an instruction.
fn parse_row(
    values: &[&str],
    column_map: &HashMap<String, usize>,
    line_number: i32,
) -> Result<Instruction, String> {
    // Helper to get a value by column name
    let get_f64 = |name: &str| -> Option<f64> {
        column_map.get(name)
            .and_then(|&idx| values.get(idx))
            .and_then(|v| v.parse().ok())
    };
    
    let get_opt_f64 = |name: &str| -> Option<f64> {
        get_f64(name)
    };
    
    let get_opt_u8 = |name: &str| -> Option<u8> {
        column_map.get(name)
            .and_then(|&idx| values.get(idx))
            .and_then(|v| v.parse().ok())
    };
    
    let get_opt_string = |name: &str| -> Option<String> {
        column_map.get(name)
            .and_then(|&idx| values.get(idx))
            .filter(|v| !v.is_empty())
            .map(|v| v.to_string())
    };
    
    // Required columns
    let x = get_f64("x").ok_or_else(|| format!("Missing or invalid X value"))?;
    let y = get_f64("y").ok_or_else(|| format!("Missing or invalid Y value"))?;
    let z = get_f64("z").ok_or_else(|| format!("Missing or invalid Z value"))?;
    
    Ok(Instruction {
        line_number,
        x,
        y,
        z,
        w: get_opt_f64("w"),
        p: get_opt_f64("p"),
        r: get_opt_f64("r"),
        ext1: get_opt_f64("ext1"),
        ext2: get_opt_f64("ext2"),
        ext3: get_opt_f64("ext3"),
        speed: get_opt_f64("speed"),
        term_type: get_opt_string("termtype").or_else(|| get_opt_string("term_type")),
        term_value: get_opt_u8("termvalue").or_else(|| get_opt_u8("term_value")),
    })
}

