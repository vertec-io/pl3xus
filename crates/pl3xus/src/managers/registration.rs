//! Convenience registration functions for network messages.
//!
//! These functions bundle multiple registration steps into single calls,
//! eliminating boilerplate and ensuring correct setup.
//!
//! # Example
//!
//! ```rust,ignore
//! use pl3xus::register_message;
//! use pl3xus_websockets::WebSocketProvider;
//!
//! // Complete registration with system-set scheduling
//! register_message::<JogCommand, WebSocketProvider, _>(&mut app, MySchedule::Notify);
//!
//! // Simple incoming-only registration
//! register_message_unscheduled::<Ping, WebSocketProvider>(&mut app);
//! ```

use bevy::prelude::*;
use bevy::ecs::schedule::SystemSet;

use crate::NetworkProvider;
use crate::managers::network::AppNetworkMessage;
use pl3xus_common::Pl3xusMessage;

/// Register a complete bidirectional message with system-set controlled sending.
///
/// This bundles:
/// - `register_network_message` (incoming plain `T`)
/// - `register_targeted_message` (incoming `TargetedMessage<T>`)
/// - `register_outbound_message` (outgoing in system set)
///
/// For authorization middleware, see `pl3xus_sync::register_authorized_message`.
///
/// # Example
///
/// ```rust,ignore
/// use pl3xus::register_message;
/// use pl3xus_websockets::WebSocketProvider;
///
/// register_message::<JogCommand, WebSocketProvider, _>(&mut app, MySchedule::Notify);
/// ```
pub fn register_message<T, NP, S>(app: &mut App, system_set: S)
where
    T: Pl3xusMessage + Clone + 'static,
    NP: NetworkProvider,
    S: SystemSet + Clone,
{
    // Register incoming plain message
    app.register_network_message::<T, NP>();

    // Register incoming targeted message
    app.register_targeted_message::<T, NP>();

    // Register outbound with system-set control
    app.register_outbound_message::<T, NP, S>(system_set);
}

/// Register an incoming-only message (no outbound, no targeting).
///
/// Use this when you only need to receive messages and don't need:
/// - System-set controlled sending
/// - Targeted message support
///
/// # Example
///
/// ```rust,ignore
/// use pl3xus::register_message_unscheduled;
///
/// register_message_unscheduled::<Ping, WebSocketProvider>(&mut app);
/// ```
pub fn register_message_unscheduled<T, NP>(app: &mut App)
where
    T: Pl3xusMessage + Clone + 'static,
    NP: NetworkProvider,
{
    app.register_network_message::<T, NP>();
}

