//! Bevy plugin registration for the execution system.

use bevy::prelude::*;

/// System set for subsystem validation.
///
/// Subsystem plugins should add their validation systems to `SubsystemValidation`
/// so they run before `coordinate_validation`.
///
/// Example:
/// ```rust,ignore
/// app.add_systems(Update, validate_fanuc_subsystem.in_set(SubsystemValidation));
/// ```
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubsystemValidation;

#[cfg(feature = "server")]
use pl3xus_sync::{AppBatchRequestRegistrationExt, AppPl3xusSyncExt, ComponentSyncConfig};
#[cfg(feature = "server")]
use pl3xus_websockets::WebSocketProvider;

#[cfg(feature = "server")]
use crate::components::{BufferDisplayData, ExecutionState, Subsystems};

#[cfg(feature = "server")]
use fanuc_replica_core::ActiveSystem;

#[cfg(feature = "server")]
use crate::handlers::{handle_pause, handle_resume, handle_start, handle_stop};

#[cfg(feature = "server")]
use crate::types::{Pause, Resume, Start, Stop};

#[cfg(feature = "server")]
use crate::systems::{
    coordinate_validation, orchestrator_system, reset_on_disconnect_system,
    sync_buffer_state_to_execution_state, sync_device_status_to_buffer_state,
    update_buffer_state_system, AuxiliaryCommandEvent, MotionCommandEvent,
};

/// Plugin for the execution system.
///
/// This plugin registers:
/// - Events for device command dispatch
/// - State management systems (buffer state transitions)
/// - Orchestrator system (command dispatch to devices)
///
/// # Usage
///
/// ```rust,ignore
/// app.add_plugins(ExecutionPlugin);
/// ```
///
/// # Schedule
///
/// Systems run in Update schedule in this order:
/// 1. `update_buffer_state_system` - Handle state transitions
/// 2. `orchestrator_system` - Dispatch commands to devices
///
/// Device plugins should add their own systems that run after these
/// to consume the `MotionCommandEvent` and `AuxiliaryCommandEvent` events.
///
/// # Device Plugins
///
/// Device-specific handlers are in their respective plugin crates:
/// - FANUC: `fanuc_replica_plugins::RobotPlugin` (includes motion handler)
/// - Duet: `fanuc_replica_duet::DuetPlugin`
pub struct ExecutionPlugin;

impl Plugin for ExecutionPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "server")]
        {
            // =====================================================================
            // SYNCED COMPONENTS
            // =====================================================================
            // ExecutionState - synced to all clients for UI display
            app.sync_component::<ExecutionState>(Some(ComponentSyncConfig::read_only_with_message(
                "ExecutionState is read-only. Use Start, Pause, Resume, Stop commands."
            )));

            // BufferDisplayData - synced to all clients for buffer table display
            app.sync_component::<BufferDisplayData>(Some(ComponentSyncConfig::read_only_with_message(
                "BufferDisplayData is read-only. Updated by Load/Unload commands."
            )));

            // =====================================================================
            // TARGETED REQUESTS (require entity control)
            // =====================================================================
            // Execution control commands - these target the ActiveSystem entity
            app.requests::<(
                Start,
                Pause,
                Resume,
                Stop,
            ), WebSocketProvider>()
                .targeted()
                .with_default_entity_policy()
                .with_error_response();

            // =====================================================================
            // EVENTS
            // =====================================================================
            app.add_message::<MotionCommandEvent>();
            app.add_message::<AuxiliaryCommandEvent>();

            // =====================================================================
            // SYSTEMS
            // =====================================================================
            // Execution control handlers
            app.add_systems(Update, (
                handle_start,
                handle_pause,
                handle_resume,
                handle_stop,
            ));

            // Configure SubsystemValidation set to run before coordinate_validation
            // Subsystem plugins (programs, fanuc, duet) add their validation systems
            // to SubsystemValidation set so they run first.
            app.configure_sets(
                Update,
                SubsystemValidation.before(coordinate_validation),
            );

            // Validation coordinator runs after subsystem validations
            // It checks Subsystems.all_ready() and transitions Validating → Executing
            app.add_systems(Update, coordinate_validation);

            // Buffer state management runs first, then orchestrator, then lifecycle, then sync
            // Order:
            // 1. update_buffer_state_system - Handle internal state transitions
            // 2. orchestrator_system - Dispatch commands to devices
            // 3. reset_on_disconnect_system - Clean up when devices disconnect
            // 4. sync_device_status_to_buffer_state - Sync device status back to buffer
            // 5. sync_buffer_state_to_execution_state - Sync buffer state to synced ExecutionState
            app.add_systems(
                Update,
                (
                    update_buffer_state_system,
                    orchestrator_system,
                    reset_on_disconnect_system,
                    sync_device_status_to_buffer_state,
                    sync_buffer_state_to_execution_state,
                )
                    .chain(),
            );

            // Add ExecutionState and Subsystems to System entity
            // Run in First to ensure it runs before Update systems, but check for entity existence
            app.add_systems(First, add_execution_components_to_system);

            info!("Execution plugin loaded");
        }
    }
}

/// Track whether we've initialized execution components
#[cfg(feature = "server")]
#[derive(Resource, Default)]
struct ExecutionComponentsInitialized(bool);

/// Add execution components to the System entity.
///
/// This runs in First schedule and waits until the System entity exists.
/// It uses a resource to track whether it's already done.
/// - ExecutionState: synced to all clients for UI state display
/// - BufferDisplayData: synced to all clients for buffer table display
/// - Subsystems: internal subsystem tracking (not synced)
#[cfg(feature = "server")]
fn add_execution_components_to_system(
    mut commands: Commands,
    system_query: Query<Entity, (With<ActiveSystem>, Without<ExecutionState>)>,
    mut initialized: Local<bool>,
) {
    if *initialized {
        return;
    }

    if let Ok(system_entity) = system_query.single() {
        commands.entity(system_entity).insert((
            ExecutionState::no_source(),
            BufferDisplayData::new(),
            Subsystems::default(),
        ));
        *initialized = true;
        info!("📡 Added ExecutionState, BufferDisplayData, and Subsystems to System entity");
    }
}

