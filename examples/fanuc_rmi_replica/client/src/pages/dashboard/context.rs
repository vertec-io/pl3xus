//! Workspace context and shared types.
//!
//! Contains the WorkspaceContext for sharing UI-LOCAL state between workspace views.
//!
//! IMPORTANT: This context should ONLY contain UI-local state like:
//! - Modal visibility
//! - Accordion expansion states
//! - Dropdown selections
//! - Console/log messages
//!
//! Server-owned state (program execution, robot position, connection status,
//! active frame/tool, etc.) should be read directly from synced components
//! using use_sync_component<T>(). See ARCHITECTURE_SPECIFICATION.md for details.
//!
//! Examples of server-owned state (DO NOT put in this context):
//! - Active UFrame/UTool -> use_sync_component::<FrameToolDataState>()
//! - Program execution -> use_sync_component::<ExecutionState>()
//! - Robot position -> use_sync_component::<RobotPosition>()
//! - Connection state -> use_sync_component::<ConnectionState>()

use leptos::prelude::*;
use std::collections::HashSet;
use js_sys;

// ============================================================================
// System Entity Context
// ============================================================================

/// Context providing the System entity ID.
///
/// This is provided at the layout level by subscribing to `SystemMarker` components.
/// Child components can use this to subscribe to entity-specific components without
/// needing to look up the entity ID themselves.
///
/// # Example
///
/// ```rust,ignore
/// // At layout level (provided automatically by DesktopLayout)
/// let systems = use_sync_component::<SystemMarker>();
/// let system_entity_id = Signal::derive(move || systems.get().keys().next().copied());
/// provide_context(SystemEntityContext { entity_id: system_entity_id });
///
/// // In child components
/// let system_ctx = expect_context::<SystemEntityContext>();
/// let (exec, exists) = use_sync_entity_component_store::<ExecutionState, _>(
///     move || system_ctx.entity_id.get()
/// );
/// ```
#[derive(Clone, Copy)]
pub struct SystemEntityContext {
    /// The reactive entity ID of the System. Returns `None` if no system exists yet.
    pub entity_id: Signal<Option<u64>>,
}

impl SystemEntityContext {
    /// Create a new SystemEntityContext with the given reactive entity ID.
    pub fn new(entity_id: Signal<Option<u64>>) -> Self {
        Self { entity_id }
    }
}

/// Hook to get the SystemEntityContext.
///
/// Panics if called outside of a context that provides SystemEntityContext
/// (i.e., must be a descendant of DesktopLayout).
pub fn use_system_entity() -> SystemEntityContext {
    expect_context::<SystemEntityContext>()
}

// ============================================================================
// Workspace Context (UI-local state)
// ============================================================================

/// Shared context for UI-LOCAL state only.
///
/// IMPORTANT: This context does NOT contain server-owned state.
/// - Active frame/tool comes from FrameToolDataState synced component
/// - Program execution state comes from ExecutionState synced component
/// - Robot position comes from RobotPosition synced component
/// - Connection status comes from ConnectionState synced component
#[derive(Clone, Copy)]
pub struct WorkspaceContext {
    /// Expanded frames in accordion (set of frame numbers) - UI-local
    pub expanded_frames: RwSignal<HashSet<i32>>,
    /// Expanded tools in accordion (set of tool numbers) - UI-local
    pub expanded_tools: RwSignal<HashSet<i32>>,
    /// Recent commands that can be re-run - UI-local
    pub recent_commands: RwSignal<Vec<RecentCommand>>,
    /// Currently selected command ID in the dropdown (None = no selection) - UI-local
    pub selected_command_id: RwSignal<Option<usize>>,
    /// Show command composer modal - UI-local
    pub show_composer: RwSignal<bool>,
    /// Console messages for the command log - UI-local
    pub console_messages: RwSignal<Vec<ConsoleMessage>>,
    /// Error log entries - UI-local
    pub error_log: RwSignal<Vec<String>>,
}

impl WorkspaceContext {
    pub fn new() -> Self {
        Self {
            expanded_frames: RwSignal::new(HashSet::new()),
            expanded_tools: RwSignal::new(HashSet::new()),
            recent_commands: RwSignal::new(Vec::new()),
            selected_command_id: RwSignal::new(None),
            show_composer: RwSignal::new(false),
            console_messages: RwSignal::new(Vec::new()),
            error_log: RwSignal::new(Vec::new()),
        }
    }

    /// Add a console message
    pub fn add_console_message(&self, content: String, direction: MessageDirection, msg_type: MessageType) {
        let now = js_sys::Date::new_0();
        let timestamp = now.to_locale_time_string("en-US").as_string().unwrap_or_default();
        let timestamp_ms = now.get_time() as u64;

        self.console_messages.update(|msgs| {
            msgs.push(ConsoleMessage {
                timestamp,
                timestamp_ms,
                content,
                direction,
                msg_type,
                sequence_id: None,
            });
            // Keep only last 500 messages
            if msgs.len() > 500 {
                msgs.remove(0);
            }
        });
    }

    /// Add an error to the error log
    pub fn add_error(&self, error: String) {
        self.error_log.update(|errors| {
            errors.push(error);
            // Keep only last 100 errors
            if errors.len() > 100 {
                errors.remove(0);
            }
        });
    }
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct CommandLogEntry {
    pub timestamp: String,
    pub command: String,
    pub status: CommandStatus,
}

/// A recently executed command that can be re-run
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct RecentCommand {
    pub id: usize,
    pub name: String,
    pub command_type: String,
    pub description: String,
    // Motion parameters
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
    pub p: f64,
    pub r: f64,
    pub speed: f64,
    pub term_type: String,
    pub uframe: u8,
    pub utool: u8,
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub enum CommandStatus {
    Pending,
    Success,
    Error(String),
}

// NOTE: ProgramLine type has been removed. Use ProgramLineInfo from fanuc_replica_types instead.
// Program execution state comes from the synced ExecutionState component.

/// Console message for the command log
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct ConsoleMessage {
    pub timestamp: String,
    pub timestamp_ms: u64,
    pub content: String,
    pub direction: MessageDirection,
    pub msg_type: MessageType,
    pub sequence_id: Option<u32>,
}

/// Direction of the message
#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub enum MessageDirection {
    Sent,
    Received,
    System,
}

/// Type of the message
#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub enum MessageType {
    Command,
    Response,
    Error,
    Status,
    Config,
}

