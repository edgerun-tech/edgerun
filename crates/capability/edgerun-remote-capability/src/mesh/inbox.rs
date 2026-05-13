use super::collections::{HashMap, VecDeque};
use super::sync::{Arc, Mutex};
use edgerun_hardware_signing::NodeID;
use edgerun_protocols::core_protocol::protocol::capability::{
    CapabilityInvocation, CapabilityResult,
};
use edgerun_protocols::core_protocol::protocol::capability_runtime::CapabilityRemoteEnvelope;

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
