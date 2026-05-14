use edgerun_hardware_signing::{MESH_PUBLIC_KEY_LENGTH, MESH_SIGNATURE_LENGTH, NodeID};

use super::*;

// ---------------------------------------------------------------------------
// Frame types
// ---------------------------------------------------------------------------

/// The type of a mesh frame, encoded as a single byte on the wire.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FrameType {
    Data = 0,
    Discovery = 1,
    RouteAdv = 2,
    HandshakeInit = 3,
    HandshakeAccept = 4,
    MetricsReport = 5,
    MigrationOrder = 6,
    MigrationComplete = 7,
    Unknown(u8),
}

impl FrameType {
    #[must_use]
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Data,
            1 => Self::Discovery,
            2 => Self::RouteAdv,
            3 => Self::HandshakeInit,
            4 => Self::HandshakeAccept,
            5 => Self::MetricsReport,
            6 => Self::MigrationOrder,
            7 => Self::MigrationComplete,
            other => Self::Unknown(other),
        }
    }
}

// ---------------------------------------------------------------------------
use crate::prelude::v1::*;
