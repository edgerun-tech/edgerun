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

pub struct MeshCapabilityTransport {
    remote_id: NodeID,
    /// Inbound envelopes queued for this session.
    pub(crate) inbox: EnvelopeInbox,
    /// Shared outbound queue — pushed by `send()`, drained by the daemon.
    outbound: OutboundQueue,
}

impl MeshCapabilityTransport {
    /// Creates a new transport targeting the given remote node.
    pub fn new(remote_id: NodeID, outbound: OutboundQueue) -> Self {
        Self {
            remote_id,
            inbox: EnvelopeInbox::default(),
            outbound,
        }
    }

    /// Returns the remote NodeID this transport communicates with.
    #[must_use]
    pub fn remote_id(&self) -> &NodeID {
        &self.remote_id
    }

    /// Access the inbound inbox (for the server side to poll).
    #[must_use]
    pub fn inbox(&self) -> &EnvelopeInbox {
        &self.inbox
    }

    /// Access the inbound inbox mutably (for the server side to consume).
    #[must_use]
    pub fn inbox_mut(&mut self) -> &mut EnvelopeInbox {
        &mut self.inbox
    }
}

impl edgerun_remote_capability::RemoteCapabilityTransport for MeshCapabilityTransport {
    fn send(&mut self, envelope: CapabilityRemoteEnvelope) -> Result<(), CapabilityError> {
        // Encode the envelope as protobuf and push to the shared outbound queue.
        // The daemon will drain this queue, encrypt through the session manager,
        // sign with hardware, and transmit.
        let payload = envelope.native_encode_to_vec();
        self.outbound
            .lock()
            .expect("outbound queue poisoned")
            .push_back((self.remote_id, payload));
        Ok(())
    }

    fn recv(&mut self) -> Result<Option<CapabilityRemoteEnvelope>, CapabilityError> {
        Ok(self.inbox.pop())
    }
}

// ---------------------------------------------------------------------------

trait NativeCapabilityEnvelopeEncode {
    fn native_encode_to_vec(&self) -> Vec<u8>;
}

impl NativeCapabilityEnvelopeEncode for CapabilityRemoteEnvelope {
    fn native_encode_to_vec(&self) -> Vec<u8> {
        // Transitional native mesh-capability frame. Replace with edgerun-wire.
        let tag: u8 = if self.message.is_some() { 1 } else { 0 };
        let mut out = Vec::new();
        out.extend_from_slice(b"ERMC");
        out.push(tag);
        out
    }
}

fn decode_capability_remote_envelope(
    _bytes: &[u8],
) -> Result<CapabilityRemoteEnvelope, &'static str> {
    Ok(CapabilityRemoteEnvelope { message: None })
}
