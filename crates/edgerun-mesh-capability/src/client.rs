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

pub struct MeshCapabilityClient {
    remote_id: NodeID,
    transport: MeshCapabilityTransport,
}

impl MeshCapabilityClient {
    /// Creates a client targeting the given remote node.
    pub fn new(remote_id: NodeID, outbound: OutboundQueue, dispatcher: &mut MeshEnvelopeDispatcher) -> Self {
        let transport = MeshCapabilityTransport::new(remote_id, outbound);
        // Register this client's inbox with the dispatcher
        let _idx = dispatcher.register_inbox(remote_id);
        Self { remote_id, transport }
    }

    #[must_use]
    pub fn remote_id(&self) -> &NodeID {
        &self.remote_id
    }

    /// Returns a mutable reference to the underlying transport
    /// for use with `edgerun_remote_capability::serve_one`.
    pub fn transport_mut(&mut self) -> &mut MeshCapabilityTransport {
        &mut self.transport
    }
}

