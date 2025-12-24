//! FANUC RMI Replica Server
//!
//! This is a minimal server that imports the plugins crate and runs the Bevy app.
//! All domain logic is in the plugins crate.

fn main() {
    fanuc_replica_plugins::build().run();
}

