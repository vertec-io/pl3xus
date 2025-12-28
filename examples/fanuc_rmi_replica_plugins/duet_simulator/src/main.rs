//! Duet RepRapFirmware HTTP API Simulator
//!
//! This simulates the Duet's HTTP endpoints for testing the extruder integration:
//! - `/rr_gcode?gcode=...` - Execute G-code command
//! - `/rr_model?key=...&flags=...` - Get object model (machine state)
//!
//! Usage:
//!   cargo run -p duet_simulator
//!   # Then connect from the application with IP 127.0.0.1:8080

use axum::{
    extract::{Query, State},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use parking_lot::RwLock;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, Level};

/// Simulator state - tracks machine position and command history
#[derive(Debug, Default)]
pub struct SimulatorState {
    /// Current Y-axis position (extruder piston)
    pub y_position: f32,
    /// Current feedrate (mm/min)
    pub feedrate: f32,
    /// Total commands received
    pub commands_received: u64,
    /// Last 10 commands for debugging
    pub command_history: Vec<String>,
}

impl SimulatorState {
    fn add_command(&mut self, cmd: String) {
        self.commands_received += 1;
        self.command_history.push(cmd);
        if self.command_history.len() > 10 {
            self.command_history.remove(0);
        }
    }
}

type SharedState = Arc<RwLock<SimulatorState>>;

/// Query parameters for /rr_gcode endpoint
#[derive(Deserialize)]
struct GcodeQuery {
    gcode: String,
}

/// Query parameters for /rr_model endpoint
#[derive(Deserialize)]
struct ModelQuery {
    #[serde(default)]
    key: String,
    #[serde(default)]
    flags: String,
}

/// Response from /rr_gcode - Duet returns empty object on success
#[derive(Serialize)]
struct GcodeResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    err: Option<u32>,
}

/// Object model response (simplified)
#[derive(Serialize)]
struct ObjectModelResponse {
    result: ObjectModelResult,
}

#[derive(Serialize)]
struct ObjectModelResult {
    r#move: MoveModel,
}

#[derive(Serialize)]
struct MoveModel {
    axes: Vec<AxisModel>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AxisModel {
    machine_position: f32,
}

/// Handle /rr_gcode - process G-code commands
async fn handle_gcode(
    Query(query): Query<GcodeQuery>,
    State(state): State<SharedState>,
) -> impl IntoResponse {
    // URL-decode the gcode
    let gcode = urlencoding::decode(&query.gcode)
        .unwrap_or_else(|_| query.gcode.clone().into())
        .to_string();

    info!("Received G-code: {}", gcode);

    // Parse G1 commands: G1 Y{position} F{feedrate}
    let g1_regex = Regex::new(r"G1\s*Y([\d.]+)(?:\s*F([\d.]+))?").unwrap();

    let mut state = state.write();
    state.add_command(gcode.clone());

    if let Some(caps) = g1_regex.captures(&gcode) {
        if let Some(y_match) = caps.get(1) {
            if let Ok(y_pos) = y_match.as_str().parse::<f32>() {
                // Duet uses relative positioning by default for Y in this context
                state.y_position += y_pos;
                info!("Y position now: {:.4} mm", state.y_position);
            }
        }
        if let Some(f_match) = caps.get(2) {
            if let Ok(feedrate) = f_match.as_str().parse::<f32>() {
                state.feedrate = feedrate;
            }
        }
    }

    // Parse M220 speed override: M220 S{percent}
    if gcode.contains("M220") {
        info!("Speed override command received");
    }

    Json(GcodeResponse { err: None })
}

/// Handle /rr_model - return object model with current state
async fn handle_model(
    Query(query): Query<ModelQuery>,
    State(state): State<SharedState>,
) -> impl IntoResponse {
    info!("Model query: key={}, flags={}", query.key, query.flags);

    let state = state.read();

    Json(ObjectModelResponse {
        result: ObjectModelResult {
            r#move: MoveModel {
                axes: vec![AxisModel {
                    machine_position: state.y_position,
                }],
            },
        },
    })
}

/// Handle /rr_status - return basic status (for compatibility)
async fn handle_status(State(state): State<SharedState>) -> impl IntoResponse {
    let state = state.read();
    Json(serde_json::json!({
        "status": "I",  // I = Idle
        "coords": {
            "xyz": [0.0, state.y_position, 0.0]
        },
        "seq": state.commands_received
    }))
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    let state = Arc::new(RwLock::new(SimulatorState::default()));

    let app = Router::new()
        .route("/rr_gcode", get(handle_gcode))
        .route("/rr_model", get(handle_model))
        .route("/rr_status", get(handle_status))
        .with_state(state.clone());

    let addr = "0.0.0.0:8080";
    info!("╔════════════════════════════════════════════════════╗");
    info!("║       Duet Simulator v0.1.0                       ║");
    info!("║       Listening on http://{}                  ║", addr);
    info!("╠════════════════════════════════════════════════════╣");
    info!("║ Endpoints:                                        ║");
    info!("║   GET /rr_gcode?gcode=<url-encoded-gcode>         ║");
    info!("║   GET /rr_model?key=<key>&flags=<flags>           ║");
    info!("║   GET /rr_status                                  ║");
    info!("╚════════════════════════════════════════════════════╝");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
