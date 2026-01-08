//! Frame identification types.
//!
//! Frames define coordinate systems in which positions are expressed.
//! This is conceptually similar to ROS TF2's frame_id strings.

use serde::{Deserialize, Serialize};

/// Identifies a coordinate frame.
///
/// Frames can be:
/// - World (base/global frame)
/// - UserFrame (FANUC UFrame, numbered 0-9)
/// - Tool (FANUC UTool, numbered 0-10)
/// - Named (custom/user-defined)
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FrameId {
    /// World/base coordinate frame (UFrame 0 on FANUC)
    World,
    
    /// User-defined frame by number (FANUC UFrame 1-9)
    UserFrame(u8),
    
    /// Tool center point frame by number (FANUC UTool 1-10)
    Tool(u8),
    
    /// Named frame for custom coordinate systems
    Named(String),
}

impl FrameId {
    /// Create a UserFrame from a number.
    /// Returns World for frame 0.
    pub fn from_uframe_number(n: u8) -> Self {
        if n == 0 {
            FrameId::World
        } else {
            FrameId::UserFrame(n)
        }
    }
    
    /// Get the numeric ID for FANUC UFrame.
    /// Returns 0 for World, the frame number for UserFrame, None for others.
    pub fn as_uframe_number(&self) -> Option<u8> {
        match self {
            FrameId::World => Some(0),
            FrameId::UserFrame(n) => Some(*n),
            _ => None,
        }
    }
    
    /// Get the numeric ID for FANUC UTool.
    pub fn as_utool_number(&self) -> Option<u8> {
        match self {
            FrameId::Tool(n) => Some(*n),
            _ => None,
        }
    }
}

impl Default for FrameId {
    fn default() -> Self {
        FrameId::World
    }
}

impl std::fmt::Display for FrameId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameId::World => write!(f, "world"),
            FrameId::UserFrame(n) => write!(f, "uframe_{}", n),
            FrameId::Tool(n) => write!(f, "utool_{}", n),
            FrameId::Named(name) => write!(f, "{}", name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_uframe_number() {
        assert_eq!(FrameId::from_uframe_number(0), FrameId::World);
        assert_eq!(FrameId::from_uframe_number(1), FrameId::UserFrame(1));
        assert_eq!(FrameId::from_uframe_number(9), FrameId::UserFrame(9));
    }

    #[test]
    fn test_as_uframe_number() {
        assert_eq!(FrameId::World.as_uframe_number(), Some(0));
        assert_eq!(FrameId::UserFrame(5).as_uframe_number(), Some(5));
        assert_eq!(FrameId::Tool(1).as_uframe_number(), None);
        assert_eq!(FrameId::Named("custom".into()).as_uframe_number(), None);
    }
}

