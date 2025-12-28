//! Plugin schedule system set for ordering systems within the Update schedule.
//!
//! Defines an ordered sequence of system sets that plugins can use to ensure
//! their systems run at the correct phase of each frame.

use bevy::prelude::*;

/// System sets for ordering plugin systems within the Update schedule.
///
/// These sets are chained in order, ensuring predictable execution:
/// 1. `Load` - Load/spawn entities and components from database/external sources
/// 2. `Despawn` - Clean up entities marked for despawn
/// 3. `ClientConnections` - Handle new/disconnected clients
/// 4. `Authorization` - Authorization checks
/// 5. `ClientRequests` - Process client requests/commands
/// 6. `NetworkConnections` - Manage network connections to external devices
/// 7. `MainUpdate` - Core business logic updates
/// 8. `Notify` - Send notifications/events to clients
/// 9. `Save` - Persist changes to database/external storage
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum PluginSchedule {
    /// Load/spawn entities and components from database or external sources.
    Load,
    /// Clean up entities marked for despawn.
    Despawn,
    /// Handle new client connections and disconnections.
    ClientConnections,
    /// Authorization and permission checks.
    Authorization,
    /// Process incoming client requests and commands.
    ClientRequests,
    /// Manage network connections to external devices (robots, PLCs, etc).
    NetworkConnections,
    /// Core business logic and state updates.
    MainUpdate,
    /// Send notifications and events to clients.
    Notify,
    /// Persist changes to database or external storage.
    Save,
}

/// Configure the plugin schedule system sets in the app.
///
/// This chains all sets in order and adds `ApplyDeferred` at strategic points
/// to ensure commands are applied between phases.
pub fn configure_plugin_schedule(app: &mut App) {
    app.configure_sets(
        Update,
        (
            PluginSchedule::Load,
            PluginSchedule::Despawn,
            PluginSchedule::ClientConnections,
            PluginSchedule::Authorization,
            PluginSchedule::ClientRequests,
            PluginSchedule::NetworkConnections,
            PluginSchedule::MainUpdate,
            PluginSchedule::Notify,
            PluginSchedule::Save,
        )
            .chain(),
    );

    // Apply deferred commands at strategic points
    app.add_systems(
        Update,
        ApplyDeferred
            .after(PluginSchedule::Load)
            .after(PluginSchedule::Despawn)
            .after(PluginSchedule::Authorization)
            .after(PluginSchedule::NetworkConnections)
            .after(PluginSchedule::ClientRequests)
            .before(PluginSchedule::Notify),
    );
}

