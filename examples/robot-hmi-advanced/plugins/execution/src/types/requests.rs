//! Message types for execution control.
//!
//! Simple verb names - these are system commands, not "program" commands.
//! The execution system is buffer-centric: we start/pause/resume/stop the buffer.

use pl3xus_common::{ErrorResponse, RequestMessage};
use serde::{Deserialize, Serialize};

// ============================================================================
// Start
// ============================================================================

/// Request to start execution.
///
/// Transitions: Ready/Completed/Stopped → Validating → Executing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Start;

/// Response to Start request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartResponse {
    pub success: bool,
    pub error: Option<String>,
}

impl RequestMessage for Start {
    type ResponseMessage = StartResponse;
}

impl ErrorResponse for Start {
    fn error_response(error: String) -> Self::ResponseMessage {
        StartResponse {
            success: false,
            error: Some(error),
        }
    }
}

// ============================================================================
// Pause
// ============================================================================

/// Request to pause execution.
///
/// Transitions: Running → Paused
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pause;

/// Response to Pause request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PauseResponse {
    pub success: bool,
    pub error: Option<String>,
}

impl RequestMessage for Pause {
    type ResponseMessage = PauseResponse;
}

impl ErrorResponse for Pause {
    fn error_response(error: String) -> Self::ResponseMessage {
        PauseResponse {
            success: false,
            error: Some(error),
        }
    }
}

// ============================================================================
// Resume
// ============================================================================

/// Request to resume execution.
///
/// Transitions: Paused → Running
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resume;

/// Response to Resume request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeResponse {
    pub success: bool,
    pub error: Option<String>,
}

impl RequestMessage for Resume {
    type ResponseMessage = ResumeResponse;
}

impl ErrorResponse for Resume {
    fn error_response(error: String) -> Self::ResponseMessage {
        ResumeResponse {
            success: false,
            error: Some(error),
        }
    }
}

// ============================================================================
// Stop
// ============================================================================

/// Request to stop execution.
///
/// Transitions: Running/Paused/Validating → Stopped
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stop;

/// Response to Stop request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopResponse {
    pub success: bool,
    pub error: Option<String>,
}

impl RequestMessage for Stop {
    type ResponseMessage = StopResponse;
}

impl ErrorResponse for Stop {
    fn error_response(error: String) -> Self::ResponseMessage {
        StopResponse {
            success: false,
            error: Some(error),
        }
    }
}

