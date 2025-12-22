//! System/Apparatus entity plugin.
//!
//! The System entity is the root of the entity hierarchy. It:
//! - Is spawned at server startup
//! - Has EntityControl for exclusive control management
//! - Is the parent of all robot entities
//! - Will hold the ProgramOrchestrator component (Phase 4)
//!
//! Clients must request control of the System entity before they can
//! connect to robots or execute programs.

use bevy::prelude::*;
use pl3xus_sync::control::EntityControl;

// Re-export ActiveSystem from shared types for use throughout the server
pub use fanuc_replica_types::ActiveSystem;

// ============================================================================
// Plugin
// ============================================================================

pub struct SystemPlugin;

impl Plugin for SystemPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_system_entity);
    }
}

// ============================================================================
// Systems
// ============================================================================

/// Spawn the System entity at server startup.
///
/// This is the root entity that clients request control of.
/// Robot entities will be spawned as children of this entity when connections are made.
fn spawn_system_entity(mut commands: Commands) {
    let entity = commands.spawn((
        Name::new("System"),
        ActiveSystem,
        // EntityControl enables exclusive control management
        // Clients can request control via ControlRequest::Take(entity_bits)
        EntityControl::default(),
    )).id();

    info!("🏗️ Spawned System entity {:?} (ready for control requests)", entity);
}

