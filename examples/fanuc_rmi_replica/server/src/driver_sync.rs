use bevy::prelude::*;
use fanuc_replica_types::*;
use fanuc_rmi::packets::{ResponsePacket, CommandResponse};
use crate::plugins::connection::{FanucRobot, RmiDriver, RmiResponseChannel, RobotConnectionState};

// Interval for polling status (seconds)
const POLL_RATE: f64 = 0.1;

/// Sync robot state from driver - entity-based, supports multiple robots
pub fn sync_robot_state(
    mut robot_query: Query<(
        Entity,
        &RobotConnectionState,
        Option<&RmiDriver>,
        Option<&mut RmiResponseChannel>,
        &mut RobotPosition,
        &mut RobotStatus,
        &mut JointAngles,
    ), With<FanucRobot>>,
    time: Res<Time>,
    mut elapsed: Local<f64>,
) {
    // 1. Poll Logic - request state from all connected robots
    *elapsed += time.delta_secs_f64();
    let should_poll = *elapsed >= POLL_RATE;
    if should_poll {
        *elapsed = 0.0;
    }

    // 2. Process each robot entity
    for (entity, conn_state, driver, response_rx, mut pos, mut status, mut joints) in robot_query.iter_mut() {
        // Skip disconnected robots
        if *conn_state != RobotConnectionState::Connected {
            continue;
        }

        let Some(_driver) = driver else { continue };

        // Poll commands would go here if we had the proper driver API
        if should_poll {
            // TODO: Send poll commands via driver
            // let _ = driver.0.send_packet(cmd_pos.into(), PacketPriority::Low).await;
            trace!("Would poll robot {:?} for state", entity);
        }

        // 3. Consume responses from this robot's response channel
        if let Some(mut rx) = response_rx {
            while let Ok(response) = rx.0.try_recv() {
                match response {
                    ResponsePacket::CommandResponse(CommandResponse::FrcReadCartesianPosition(res)) => {
                        // Convert packets response to dto::Position
                        pos.0 = fanuc_rmi::dto::Position {
                            x: res.pos.x, y: res.pos.y, z: res.pos.z,
                            w: res.pos.w, p: res.pos.p, r: res.pos.r,
                            ext1: res.pos.ext1, ext2: res.pos.ext2, ext3: res.pos.ext3,
                        };
                    }
                    ResponsePacket::CommandResponse(CommandResponse::FrcReadJointAngles(res)) => {
                        joints.0 = fanuc_rmi::dto::JointAngles {
                            j1: res.joint_angles.j1, j2: res.joint_angles.j2, j3: res.joint_angles.j3,
                            j4: res.joint_angles.j4, j5: res.joint_angles.j5, j6: res.joint_angles.j6,
                            j7: res.joint_angles.j7, j8: res.joint_angles.j8, j9: res.joint_angles.j9,
                        };
                    }
                    ResponsePacket::CommandResponse(CommandResponse::FrcGetStatus(res)) => {
                        status.servo_ready = res.servo_ready != 0;
                        status.tp_enabled = res.tp_mode != 0;
                        status.in_motion = res.rmi_motion_status != 0;
                        status.error_message = if res.error_id != 0 {
                            Some(format!("Error ID: {}", res.error_id))
                        } else {
                            None
                        };
                    }
                    _ => {}
                }
            }
        }
    }
}
