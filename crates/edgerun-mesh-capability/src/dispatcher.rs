use crate::prelude::v1::*;
use edgerun_capabilities::CapabilityError;
use edgerun_hardware_signing::NodeID;
#[allow(unused_imports)] // used in tests
use edgerun_mesh::{FrameType, MeshFrame, MeshFrameHeader};
use edgerun_mesh_link::MeshLink;
use edgerun_proto::edgerun::v0::capability::{CapabilityInvocation, CapabilityResult};
use edgerun_proto::edgerun::v0::capability_runtime::{
    capability_remote_envelope, CapabilityRemoteEnvelope,
};
use edgerun_remote_capability::RemoteCapabilityProvider;
use prost::Message;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use super::*;

// Mesh envelope dispatcher — inbound frame handler
// ---------------------------------------------------------------------------

/// Dispatches inbound mesh data frames to the correct `MeshCapabilityTransport`.
///
/// Maintains a map of `NodeID → Vec<EnvelopeInbox>` so that multiple
/// concurrent sessions with the same remote node each get their own queue.
pub struct MeshEnvelopeDispatcher {
    /// Inbound envelopes keyed by sender NodeID.
    /// Multiple sessions can exist with the same remote, so we store a
    /// list of inboxes — the first matching one that isn't for a closed
    /// session gets the envelope.
    inboxes: HashMap<NodeID, Vec<EnvelopeInbox>>,
}

impl Default for MeshEnvelopeDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl MeshEnvelopeDispatcher {
    pub fn new() -> Self {
        Self {
            inboxes: HashMap::new(),
        }
    }

    /// Registers a new inbox for a remote NodeID, returning a handle to it.
    pub fn register_inbox(&mut self, remote_id: NodeID) -> usize {
        let inboxes = self.inboxes.entry(remote_id).or_default();
        let idx = inboxes.len();
        inboxes.push(EnvelopeInbox::default());
        idx
    }

    /// Removes an inbox by index for a given remote NodeID.
    pub fn unregister_inbox(&mut self, remote_id: NodeID, idx: usize) {
        if let Some(inboxes) = self.inboxes.get_mut(&remote_id) {
            if idx < inboxes.len() {
                inboxes.remove(idx);
            }
            if inboxes.is_empty() {
                self.inboxes.remove(&remote_id);
            }
        }
    }

    /// Removes all inboxes for a given remote NodeID (called when a peer dies).
    pub fn remove_peer(&mut self, peer_id: &NodeID) {
        self.inboxes.remove(peer_id);
    }

    /// Returns a mutable reference to the inboxes map for direct access.
    pub fn inboxes_mut(&mut self) -> &mut HashMap<NodeID, Vec<EnvelopeInbox>> {
        &mut self.inboxes
    }

    /// Delivers an inbound mesh frame's payload to the appropriate inbox.
    ///
    /// Returns `true` if the envelope was delivered, `false` if no inbox
    /// was registered for the sender.
    pub fn deliver(&mut self, frame: &MeshFrame) -> bool {
        // The payload is a protobuf CapabilityRemoteEnvelope
        let envelope = match CapabilityRemoteEnvelope::decode(frame.payload.as_slice()) {
            Ok(e) => e,
            Err(_) => return false,
        };

        let sender = frame.header.src;
        if let Some(inboxes) = self.inboxes.get_mut(&sender) {
            if let Some(inbox) = inboxes.first_mut() {
                inbox.push(envelope);
                return true;
            }
        }
        false
    }
}

// ---------------------------------------------------------------------------
