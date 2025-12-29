//! Core systems - startup, ActiveSystem entity management.

use bevy::prelude::*;
use pl3xus_sync::control::EntityControl;

use super::types::ActiveSystem;

/// Spawn the ActiveSystem entity on startup.
///
/// This entity serves as the control root for the entire application.
/// Clients request exclusive control of this entity to control all robots.
///
/// Note: ExecutionCoordinator, ToolpathBuffer, and BufferState components
/// are added dynamically by LoadProgram and removed by UnloadProgram.
/// The new orchestrator only activates when a program is loaded.
pub fn spawn_active_system(mut commands: Commands) {
    info!("🏭 Spawning ActiveSystem entity");
    commands.spawn((
        ActiveSystem,
        EntityControl::default(),
        Name::new("System"),
    ));
}

