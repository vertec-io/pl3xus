//! Core systems - startup, ActiveSystem entity management.

use bevy::prelude::*;
use pl3xus_sync::control::EntityControl;

use super::types::ActiveSystem;

/// Spawn the ActiveSystem entity on startup.
///
/// This entity serves as the control root for the entire application.
/// Clients request exclusive control of this entity to control all robots.
pub fn spawn_active_system(mut commands: Commands) {
    info!("🏭 Spawning ActiveSystem entity");
    commands.spawn((
        ActiveSystem,
        EntityControl::default(),
        Name::new("System"),
    ));
}

