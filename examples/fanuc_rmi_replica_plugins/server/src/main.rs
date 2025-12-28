//! FANUC RMI Replica Server
//!
//! This is a minimal server that imports the plugins crate and runs the Bevy app.
//! All domain logic is in the plugins crate.
//!
//! # Plugin Architecture
//!
//! The server loads plugins in this order:
//! 1. `CorePlugin` - Networking, database, ActiveSystem
//! 2. `RobotPlugin` - Robot state, connections, programs, I/O
//! 3. `ExecutionPlugin` - Coordinated multi-device toolpath execution
//! 4. `FanucPlugin` - FANUC-specific motion handling
//! 5. `DuetPlugin` (optional) - Duet extruder support

fn main() {
    let mut app = fanuc_replica_plugins::build();

    app.run();
}

