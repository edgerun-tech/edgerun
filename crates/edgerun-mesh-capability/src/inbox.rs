use crate::collections::{HashMap, VecDeque};
use crate::prelude::v1::*;
use crate::sync::{Arc, Mutex};
use edgerun_capabilities::CapabilityError;
use edgerun_core::protocol::capability::{CapabilityInvocation, CapabilityResult};
use edgerun_core::protocol::capability_runtime::{
    capability_remote_envelope, CapabilityRemoteEnvelope,
};
use edgerun_hardware_signing::NodeID;
#[allow(unused_imports)] // used in tests
use edgerun_mesh::{FrameType, MeshFrame, MeshFrameHeader};
use edgerun_mesh_link::MeshLink;
use edgerun_remote_capability::RemoteCapabilityProvider;

use super::*;

// ---------------------------------------------------------------------------
// Envelope channel — maps NodeID to a queue of inbound envelopes
// ---------------------------------------------------------------------------

/// A bounded inbox for capability envelopes from a specific remote node.
#[derive(Debug, Default)]
pub struct EnvelopeInbox {
    queue: VecDeque<CapabilityRemoteEnvelope>,
}

impl EnvelopeInbox {
    pub fn push(&mut self, envelope: CapabilityRemoteEnvelope) {
        self.queue.push_back(envelope);
    }

    pub fn pop(&mut self) -> Option<CapabilityRemoteEnvelope> {
        self.queue.pop_front()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

// ---------------------------------------------------------------------------
