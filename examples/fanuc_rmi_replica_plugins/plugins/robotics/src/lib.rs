//! Robot-agnostic coordinate types and conversion utilities.
//!
//! This crate provides universal robot pose representations using nalgebra's
//! `Isometry3<f64>` (SE3) as the core type, with conversion utilities for
//! vendor-specific formats.
//!
//! # Architecture
//!
//! The hybrid approach:
//! - **Universal types** (Isometry3) for toolpaths, motion planning, orchestration
//! - **Vendor-specific types** for robot feedback and driver communication
//! - **Clear conversion boundary** at the driver layer
//!
//! Vendor-specific conversions are in their respective plugin crates:
//! - FANUC: `fanuc_replica_fanuc::FanucConversion`
//!
//! # Example
//!
//! ```rust
//! use fanuc_replica_robotics::{RobotPose, FrameId};
//! use nalgebra::{Isometry3, Translation3, UnitQuaternion, Vector3};
//!
//! // Create a pose in world frame
//! let translation = Translation3::new(100.0, 200.0, 300.0);
//! let rotation = UnitQuaternion::from_axis_angle(&Vector3::z_axis(), std::f64::consts::FRAC_PI_2);
//! let pose = RobotPose::new(
//!     Isometry3::from_parts(translation, rotation),
//!     FrameId::World,
//! );
//! ```

pub mod conversion;
pub mod frame;
pub mod pose;

pub use conversion::{euler_zyx_to_quaternion, quaternion_to_euler_zyx};
pub use frame::FrameId;
pub use pose::{RobotPose, TerminationType, ToolpathPoint};

