//! Bridges `lifegraph-remote-capability` onto the Lifegraph mesh.
//!
//! Provides:
//! - **`MeshCapabilityTransport`** — wraps the mesh to send/receive
//!   `CapabilityRemoteEnvelope` protobuf messages routed by `NodeID`.
//! - **`MeshCapabilityServer`** — hosts local `RemoteCapabilityProvider`
//!   instances and serves inbound requests routed to this node.
//! - **`MeshCapabilityClient`** — initiates sessions with remote nodes
//!   by NodeID, sending invocations and receiving results.
//!
//! The mesh handles delivery — this layer only deals in capability
//! protocol messages wrapped in mesh frames.

use lifegraph_capabilities::CapabilityError;
use lifegraph_hardware_signing::NodeID;
#[allow(unused_imports)] // used in tests
use lifegraph_mesh::{FrameType, MeshFrame, MeshFrameHeader};
use lifegraph_mesh_link::MeshLink;
use lifegraph_proto::lifegraph::v0::capability::{CapabilityInvocation, CapabilityResult};
use lifegraph_proto::lifegraph::v0::capability_runtime::{
    capability_remote_envelope, CapabilityRemoteEnvelope,
};
use lifegraph_remote_capability::RemoteCapabilityProvider;
use prost::Message;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

/// Shared outbound queue: (destination NodeID, serialized protobuf payload).
/// Used by `MeshCapabilityTransport::send()` to push frames that the daemon
/// will drain, encrypt, and send.
pub type OutboundQueue = Rc<RefCell<VecDeque<(NodeID, Vec<u8>)>>>;

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
// MeshCapabilityTransport — RemoteCapabilityTransport over the mesh
// ---------------------------------------------------------------------------

/// A capability transport that sends and receives `CapabilityRemoteEnvelope`
/// messages through the mesh, addressed by `NodeID`.
///
/// **Outbound**: `send()` pushes serialized protobuf payloads to a shared
/// `OutboundQueue` (an `Rc<RefCell<VecDeque>>`). The daemon drains this
/// queue, encrypts through the session manager, signs, and transmits.
///
/// **Inbound**: Decrypted capability envelopes are delivered to the inbox
/// by the daemon after session decryption.
pub struct MeshCapabilityTransport {
    remote_id: NodeID,
    /// Inbound envelopes queued for this session.
    inbox: EnvelopeInbox,
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

impl lifegraph_remote_capability::RemoteCapabilityTransport for MeshCapabilityTransport {
    fn send(&mut self, envelope: CapabilityRemoteEnvelope) -> Result<(), CapabilityError> {
        // Encode the envelope as protobuf and push to the shared outbound queue.
        // The daemon will drain this queue, encrypt through the session manager,
        // sign with hardware, and transmit.
        let payload = envelope.encode_to_vec();
        self.outbound.borrow_mut().push_back((self.remote_id, payload));
        Ok(())
    }

    fn recv(&mut self) -> Result<Option<CapabilityRemoteEnvelope>, CapabilityError> {
        Ok(self.inbox.pop())
    }
}

// ---------------------------------------------------------------------------
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
// MeshCapabilityServer — serves local providers over the mesh
// ---------------------------------------------------------------------------

/// Hosts a `RemoteCapabilityProvider` and processes inbound capability
/// requests received via mesh frames.
pub struct MeshCapabilityServer<P> {
    provider: P,
    dispatcher: MeshEnvelopeDispatcher,
}

impl<P: RemoteCapabilityProvider> MeshCapabilityServer<P> {
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            dispatcher: MeshEnvelopeDispatcher::new(),
        }
    }

    pub fn provider(&self) -> &P {
        &self.provider
    }

    pub fn provider_mut(&mut self) -> &mut P {
        &mut self.provider
    }

    pub fn dispatcher(&mut self) -> &mut MeshEnvelopeDispatcher {
        &mut self.dispatcher
    }

    /// Processes one inbound envelope for the server, if available.
    pub fn serve_one(
        &mut self,
        _link: &mut MeshLink,
    ) -> Result<bool, CapabilityError> {
        let senders: Vec<NodeID> = self
            .dispatcher
            .inboxes_mut()
            .keys()
            .copied()
            .collect();

        for sender in senders {
            if let Some(inboxes) = self.dispatcher.inboxes_mut().get_mut(&sender) {
                if let Some(inbox) = inboxes.first_mut() {
                    if let Some(envelope) = inbox.pop() {
                        let response = self.process_envelope(&sender, envelope)?;
                        if let Some(resp_env) = response {
                            if let Some(resp_inboxes) = self.dispatcher.inboxes_mut().get_mut(&sender) {
                                if let Some(resp_inbox) = resp_inboxes.first_mut() {
                                    resp_inbox.push(resp_env);
                                }
                            }
                        }
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
    }

    fn process_envelope(
        &mut self,
        _sender: &NodeID,
        envelope: CapabilityRemoteEnvelope,
    ) -> Result<Option<CapabilityRemoteEnvelope>, CapabilityError> {
        match envelope.message {
            Some(capability_remote_envelope::Message::SessionOpen(open)) => {
                let accept = self.provider.open_session(&open)?;
                Ok(Some(CapabilityRemoteEnvelope {
                    message: Some(capability_remote_envelope::Message::SessionAccept(accept)),
                }))
            }
            Some(capability_remote_envelope::Message::Invocation(invocation)) => {
                let result = self.provider.invoke(&invocation.grant_id, &invocation, None);
                let msg = match result {
                    Ok(resp) => capability_remote_envelope::Message::Result(resp.result),
                    Err(err) => capability_remote_envelope::Message::Result(invocation_error_result(&invocation, err)),
                };
                Ok(Some(CapabilityRemoteEnvelope { message: Some(msg) }))
            }
            Some(capability_remote_envelope::Message::InvocationFrame(frame)) => {
                let Some(invocation) = frame.invocation else {
                    return Err(CapabilityError::InvalidRequest(
                        "invocation frame must contain an invocation",
                    ));
                };
                let result = self.provider.invoke(&invocation.grant_id, &invocation, Some(&frame.inline_parameters));
                let msg = match result {
                    Ok(resp) => capability_remote_envelope::Message::Result(resp.result),
                    Err(err) => capability_remote_envelope::Message::Result(invocation_error_result(&invocation, err)),
                };
                Ok(Some(CapabilityRemoteEnvelope { message: Some(msg) }))
            }
            Some(capability_remote_envelope::Message::SessionClose(close)) => {
                self.provider.close_session(&close)?;
                Ok(None)
            }
            Some(capability_remote_envelope::Message::Request(request)) => {
                if let Some(grant) = self.provider.handle_request(&request)? {
                    Ok(Some(CapabilityRemoteEnvelope {
                        message: Some(capability_remote_envelope::Message::Grant(grant)),
                    }))
                } else {
                    Ok(None)
                }
            }
            Some(capability_remote_envelope::Message::Grant(grant)) => {
                self.provider.handle_grant(&grant)?;
                Ok(None)
            }
            Some(capability_remote_envelope::Message::Revocation(revocation)) => {
                self.provider.handle_revocation(&revocation)?;
                Ok(None)
            }
            Some(capability_remote_envelope::Message::SessionAccept(_))
            | Some(capability_remote_envelope::Message::SessionEvent(_))
            | Some(capability_remote_envelope::Message::Result(_))
            | Some(capability_remote_envelope::Message::ResultFrame(_))
            | None => Ok(None),
        }
    }
}

fn invocation_error_result(
    invocation: &CapabilityInvocation,
    error: CapabilityError,
) -> CapabilityResult {
    CapabilityResult {
        result_version: 1,
        invocation_id: invocation.invocation_id.clone(),
        grant_id: invocation.grant_id.clone(),
        success: false,
        result_access_class: invocation.requested_access_class,
        produced_event_kinds: Vec::new(),
        payload_object: None,
        error_reason: error.to_string(),
        produced_at: None,
        signature: None,
    }
}

// ---------------------------------------------------------------------------
// MeshCapabilityClient — initiates sessions with remote nodes
// ---------------------------------------------------------------------------

/// A client that opens capability sessions on remote nodes via the mesh.
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
    /// for use with `lifegraph_remote_capability::serve_one`.
    pub fn transport_mut(&mut self) -> &mut MeshCapabilityTransport {
        &mut self.transport
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lifegraph_remote_capability::RemoteCapabilityTransport;
    use lifegraph_proto::lifegraph::v0::capability_runtime::{
        capability_remote_envelope, CapabilitySessionMode, CapabilitySessionOpen,
    };
    use std::cell::RefCell;
    use std::rc::Rc;

    fn node_id(v: u8) -> NodeID {
        let mut bytes = [0u8; 64];
        bytes[0] = v;
        NodeID(bytes)
    }

    #[test]
    fn transport_send_pushes_to_outbound_queue() {
        let outbound: OutboundQueue = Rc::new(RefCell::new(VecDeque::new()));
        let mut transport = MeshCapabilityTransport::new(node_id(0xBB), Rc::clone(&outbound));
        let envelope = CapabilityRemoteEnvelope {
            message: Some(capability_remote_envelope::Message::SessionOpen(
                CapabilitySessionOpen {
                    version: 1,
                    session_id: b"test-session".to_vec(),
                    selector: None,
                    mode: CapabilitySessionMode::Unspecified as i32,
                    requested_operations: vec![1],
                    requested_access_class: 0,
                    requested_constraints: vec![],
                    correlation_id: vec![],
                },
            )),
        };
        assert!(transport.send(envelope).is_ok());
        // Verify it was pushed to the outbound queue
        let queue = outbound.borrow();
        assert_eq!(queue.len(), 1);
        assert_eq!(queue[0].0, node_id(0xBB));
    }

    #[test]
    fn transport_recv_returns_queued_envelopes() {
        let outbound: OutboundQueue = Rc::new(RefCell::new(VecDeque::new()));
        let mut transport = MeshCapabilityTransport::new(node_id(0xBB), outbound);
        assert!(transport.recv().unwrap().is_none());

        // Push an envelope directly into the inbox (simulating the mesh
        // link delivering a frame)
        transport.inbox.push(CapabilityRemoteEnvelope {
            message: None,
        });

        let received = transport.recv().unwrap();
        assert!(received.is_some());
        assert!(transport.recv().unwrap().is_none());
    }

    #[test]
    fn dispatcher_delivers_to_registered_inbox() {
        let mut dispatcher = MeshEnvelopeDispatcher::new();
        let remote = node_id(0xCC);
        let idx = dispatcher.register_inbox(remote);

        // Simulate an inbound mesh frame
        let payload = CapabilityRemoteEnvelope {
            message: None,
        }
        .encode_to_vec();

        let frame = MeshFrame {
            header: MeshFrameHeader {
                dest: node_id(0xAA), // destined for us
                src: remote,
                ttl: 10,
                frame_type: FrameType::Data,
            },
            payload,
            signature: [0u8; 64],
        };

        assert!(dispatcher.deliver(&frame));

        // Pop from the inbox to verify delivery
        let inboxes = dispatcher.inboxes_mut();
        let envelope = inboxes.get_mut(&remote).unwrap()[idx].pop();
        assert!(envelope.is_some());
    }

    #[test]
    fn dispatcher_rejects_unknown_sender() {
        let mut dispatcher = MeshEnvelopeDispatcher::new();

        let payload = CapabilityRemoteEnvelope {
            message: None,
        }
        .encode_to_vec();

        let frame = MeshFrame {
            header: MeshFrameHeader {
                dest: node_id(0xAA),
                src: node_id(0xFF), // no inbox registered
                ttl: 10,
                frame_type: FrameType::Data,
            },
            payload,
            signature: [0u8; 64],
        };

        assert!(!dispatcher.deliver(&frame));
    }

    #[test]
    fn client_registers_inbox_on_creation() {
        let outbound: OutboundQueue = Rc::new(RefCell::new(VecDeque::new()));
        let mut dispatcher = MeshEnvelopeDispatcher::new();
        let remote = node_id(0xDD);
        let _client = MeshCapabilityClient::new(remote, outbound, &mut dispatcher);

        // The dispatcher should have registered an inbox
        assert!(dispatcher.inboxes_mut().contains_key(&remote));
    }

    #[test]
    fn envelope_inbox_fifo_ordering() {
        let mut inbox = EnvelopeInbox::default();
        for i in 0..5u32 {
            inbox.push(CapabilityRemoteEnvelope {
                message: Some(capability_remote_envelope::Message::SessionOpen(
                    CapabilitySessionOpen {
                        version: i,
                        session_id: vec![i as u8],
                        selector: None,
                        mode: CapabilitySessionMode::Unspecified as i32,
                        requested_operations: vec![],
                        requested_access_class: 0,
                        requested_constraints: vec![],
                        correlation_id: vec![],
                    },
                )),
            });
        }

        for i in 0..5 {
            let env = inbox.pop().unwrap();
            if let Some(capability_remote_envelope::Message::SessionOpen(open)) = env.message {
                assert_eq!(open.version, i);
            } else {
                panic!("expected SessionOpen");
            }
        }
        assert!(inbox.pop().is_none());
    }
}
