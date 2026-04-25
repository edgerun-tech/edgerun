use super::*;
use edgerun_mesh::{FrameType, MeshFrame, MeshFrameHeader};
use edgerun_proto::edgerun::v0::capability::{
    CapabilityInvocation, CapabilityRequest, CapabilityResult,
};
use edgerun_proto::edgerun::v0::capability_runtime::{
    capability_remote_envelope, CapabilityInvocationFrame, CapabilitySessionClose,
    CapabilitySessionMode, CapabilitySessionOpen,
};
use edgerun_remote_capability::{RemoteCapabilityProvider, RemoteCapabilityTransport};
use prost::Message;
use std::sync::{Arc, Mutex};

// -----------------------------------------------------------------------
// Test helpers
// -----------------------------------------------------------------------

fn node_id(v: u8) -> NodeID {
    let mut bytes = [0u8; 64];
    bytes[0] = v;
    NodeID(bytes)
}

fn make_envelope(message: Option<capability_remote_envelope::Message>) -> CapabilityRemoteEnvelope {
    CapabilityRemoteEnvelope { message }
}

fn make_session_open(version: u32, session_id: Vec<u8>) -> CapabilitySessionOpen {
    CapabilitySessionOpen {
        version,
        session_id,
        selector: None,
        mode: CapabilitySessionMode::Unspecified as i32,
        requested_operations: vec![1],
        requested_access_class: 0,
        requested_constraints: vec![],
        correlation_id: vec![],
    }
}

fn make_mesh_frame(src: NodeID, payload: Vec<u8>) -> MeshFrame {
    MeshFrame {
        header: MeshFrameHeader {
            dest: node_id(0xAA),
            src,
            ttl: 10,
            frame_type: FrameType::Data,
        },
        payload,
        signature: [0u8; 64],
    }
}

fn make_invocation(invocation_id: Vec<u8>, grant_id: Vec<u8>) -> CapabilityInvocation {
    CapabilityInvocation {
        invocation_version: 1,
        invocation_id,
        grant_id,
        invoker: None,
        operation: 1,
        requested_access_class: 0,
        parameter_object: None,
        correlation_id: vec![],
        invoked_at: None,
        signature: None,
    }
}

// -----------------------------------------------------------------------
// Minimal mock provider for server tests
// -----------------------------------------------------------------------

#[derive(Debug, Default)]
struct MockProvider {
    session_opened: bool,
    session_closed: bool,
    invoke_count: usize,
    fail_on_invoke: bool,
    fail_on_open: bool,
}

impl RemoteCapabilityProvider for MockProvider {
    fn descriptor(&self) -> edgerun_capabilities::CapabilityDescriptor {
        edgerun_capabilities::capability_descriptor(
            "mock",
            "mock-0",
            edgerun_capabilities::CapabilityRole::Input,
            &[edgerun_capabilities::CapabilityModality::Auditory],
            &[edgerun_capabilities::CapabilityEventKind::Auditory],
            &[edgerun_capabilities::CapabilityOperation::Query],
            vec![],
        )
    }

    fn open_session(
        &mut self,
        open: &CapabilitySessionOpen,
    ) -> Result<
        edgerun_proto::edgerun::v0::capability_runtime::CapabilitySessionAccept,
        CapabilityError,
    > {
        if self.fail_on_open {
            return Err(CapabilityError::InvalidRequest("mock open failure"));
        }
        self.session_opened = true;
        Ok(
            edgerun_proto::edgerun::v0::capability_runtime::CapabilitySessionAccept {
                version: open.version,
                session_id: open.session_id.clone(),
                accepted: true,
                granted_operations: open.requested_operations.clone(),
                granted_access_class: open.requested_access_class,
                error_reason: String::new(),
                grant_id: open.session_id.clone(),
            },
        )
    }

    fn invoke(
        &mut self,
        _session_id: &[u8],
        invocation: &CapabilityInvocation,
        _inline_parameters: Option<&[u8]>,
    ) -> Result<edgerun_remote_capability::RemoteInvocationResult, CapabilityError> {
        if self.fail_on_invoke {
            return Err(CapabilityError::InvalidRequest("mock invoke failure"));
        }
        self.invoke_count += 1;
        Ok(edgerun_remote_capability::RemoteInvocationResult {
            result: CapabilityResult {
                result_version: 1,
                invocation_id: invocation.invocation_id.clone(),
                grant_id: invocation.grant_id.clone(),
                success: true,
                result_access_class: invocation.requested_access_class,
                produced_event_kinds: vec![],
                payload_object: None,
                error_reason: String::new(),
                produced_at: None,
                signature: None,
            },
            inline_payload: vec![],
        })
    }

    fn close_session(&mut self, _close: &CapabilitySessionClose) -> Result<(), CapabilityError> {
        self.session_closed = true;
        Ok(())
    }

    fn handle_request(
        &mut self,
        _request: &CapabilityRequest,
    ) -> Result<Option<edgerun_capabilities::CapabilityGrant>, CapabilityError> {
        Ok(None)
    }
}

// ===================================================================
// EnvelopeInbox tests
// ===================================================================

#[test]
fn envelope_inbox_default_is_empty() {
    let mut inbox = EnvelopeInbox::default();
    assert!(inbox.is_empty());
    assert!(inbox.pop().is_none());
}

#[test]
fn envelope_inbox_push_then_pop() {
    let mut inbox = EnvelopeInbox::default();
    let env = make_envelope(None);
    inbox.push(env);
    assert!(!inbox.is_empty());
    let popped = inbox.pop();
    assert!(popped.is_some());
    assert!(inbox.is_empty());
}

#[test]
fn envelope_inbox_fifo_ordering() {
    let mut inbox = EnvelopeInbox::default();
    for i in 0..5u32 {
        inbox.push(make_envelope(Some(
            capability_remote_envelope::Message::SessionOpen(make_session_open(i, vec![i as u8])),
        )));
    }

    for i in 0..5 {
        let env = inbox.pop().unwrap();
        if let Some(capability_remote_envelope::Message::SessionOpen(open)) = env.message {
            assert_eq!(open.version, i);
            assert_eq!(open.session_id, vec![i as u8]);
        } else {
            panic!("expected SessionOpen message");
        }
    }
    assert!(inbox.is_empty());
    assert!(inbox.pop().is_none());
}

#[test]
fn envelope_inbox_multiple_push_pop() {
    let mut inbox = EnvelopeInbox::default();
    inbox.push(make_envelope(None));
    inbox.push(make_envelope(None));
    inbox.push(make_envelope(None));

    assert_eq!(inbox.pop().map(|_| ()), Some(()));
    assert!(!inbox.is_empty());
    assert_eq!(inbox.pop().map(|_| ()), Some(()));
    assert!(!inbox.is_empty());
    assert_eq!(inbox.pop().map(|_| ()), Some(()));
    assert!(inbox.is_empty());
}

// ===================================================================
// OutboundQueue tests
// ===================================================================

#[test]
fn outbound_queue_default_empty() {
    let queue: OutboundQueue = Arc::new(Mutex::new(VecDeque::new()));
    assert!(queue.lock().unwrap().is_empty());
}

#[test]
fn outbound_queue_shared_across_clones() {
    let queue: OutboundQueue = Arc::new(Mutex::new(VecDeque::new()));
    let clone = Arc::clone(&queue);
    queue.lock().unwrap().push_back((node_id(1), vec![1, 2, 3]));
    assert_eq!(clone.lock().unwrap().len(), 1);
}

// ===================================================================
// MeshCapabilityTransport tests
// ===================================================================

#[test]
fn transport_new_sets_remote_id() {
    let outbound: OutboundQueue = Arc::new(Mutex::new(VecDeque::new()));
    let remote = node_id(0x42);
    let transport = MeshCapabilityTransport::new(remote, outbound);
    assert_eq!(*transport.remote_id(), remote);
}

#[test]
fn transport_inbox_is_initially_empty() {
    let outbound: OutboundQueue = Arc::new(Mutex::new(VecDeque::new()));
    let transport = MeshCapabilityTransport::new(node_id(1), outbound);
    assert!(transport.inbox().is_empty());
}

#[test]
fn transport_send_pushes_to_outbound_queue() {
    let outbound: OutboundQueue = Arc::new(Mutex::new(VecDeque::new()));
    let remote = node_id(0xBB);
    let mut transport = MeshCapabilityTransport::new(remote, Arc::clone(&outbound));

    let envelope = make_envelope(Some(capability_remote_envelope::Message::SessionOpen(
        make_session_open(1, b"sess-1".to_vec()),
    )));
    assert!(transport.send(envelope).is_ok());

    let queue = outbound.lock().unwrap();
    assert_eq!(queue.len(), 1);
    assert_eq!(queue[0].0, remote);
    // Payload should be a valid protobuf encoding
    assert!(!queue[0].1.is_empty());
}

#[test]
fn transport_send_multiple_accumulates() {
    let outbound: OutboundQueue = Arc::new(Mutex::new(VecDeque::new()));
    let mut transport = MeshCapabilityTransport::new(node_id(5), Arc::clone(&outbound));

    for i in 0..3u8 {
        let envelope = make_envelope(Some(capability_remote_envelope::Message::SessionOpen(
            make_session_open(i as u32, vec![i]),
        )));
        transport.send(envelope).unwrap();
    }

    let queue = outbound.lock().unwrap();
    assert_eq!(queue.len(), 3);
    for entry in queue.iter() {
        assert_eq!(entry.0, node_id(5));
    }
}

#[test]
fn transport_recv_returns_none_when_inbox_empty() {
    let outbound: OutboundQueue = Arc::new(Mutex::new(VecDeque::new()));
    let mut transport = MeshCapabilityTransport::new(node_id(1), outbound);
    assert!(transport.recv().unwrap().is_none());
}

#[test]
fn transport_recv_returns_queued_envelopes() {
    let outbound: OutboundQueue = Arc::new(Mutex::new(VecDeque::new()));
    let mut transport = MeshCapabilityTransport::new(node_id(0xBB), outbound);

    // Simulate mesh delivering an envelope into the inbox
    transport.inbox.push(make_envelope(None));

    let received = transport.recv().unwrap();
    assert!(received.is_some());
    assert!(transport.recv().unwrap().is_none());
}

#[test]
fn transport_recv_fifo_ordering() {
    let outbound: OutboundQueue = Arc::new(Mutex::new(VecDeque::new()));
    let mut transport = MeshCapabilityTransport::new(node_id(1), outbound);

    for i in 0..4u8 {
        transport.inbox.push(make_envelope(Some(
            capability_remote_envelope::Message::SessionOpen(make_session_open(i as u32, vec![i])),
        )));
    }

    for i in 0..4u8 {
        let env = transport.recv().unwrap().unwrap();
        if let Some(capability_remote_envelope::Message::SessionOpen(open)) = env.message {
            assert_eq!(open.version, i as u32);
        } else {
            panic!("expected SessionOpen");
        }
    }
    assert!(transport.recv().unwrap().is_none());
}

#[test]
fn transport_send_then_recv_independent() {
    // send() pushes to outbound queue; recv() reads from inbox.
    // They are independent paths.
    let outbound: OutboundQueue = Arc::new(Mutex::new(VecDeque::new()));
    let mut transport = MeshCapabilityTransport::new(node_id(1), Arc::clone(&outbound));

    transport.send(make_envelope(None)).unwrap();
    assert!(transport.recv().unwrap().is_none()); // inbox still empty
    assert_eq!(outbound.lock().unwrap().len(), 1); // outbound has the message
}

#[test]
fn transport_inbox_mut_access() {
    let outbound: OutboundQueue = Arc::new(Mutex::new(VecDeque::new()));
    let mut transport = MeshCapabilityTransport::new(node_id(1), outbound);
    let inbox_mut = transport.inbox_mut();
    assert!(inbox_mut.is_empty());
    inbox_mut.push(make_envelope(None));
    assert!(!inbox_mut.is_empty());
}

#[test]
fn transport_different_remote_ids() {
    let outbound: OutboundQueue = Arc::new(Mutex::new(VecDeque::new()));
    let t1 = MeshCapabilityTransport::new(node_id(1), Arc::clone(&outbound));
    let t2 = MeshCapabilityTransport::new(node_id(2), outbound);
    assert_ne!(t1.remote_id(), t2.remote_id());
}

// ===================================================================
// MeshEnvelopeDispatcher tests
// ===================================================================

#[test]
fn dispatcher_new_is_empty() {
    let mut dispatcher = MeshEnvelopeDispatcher::new();
    assert!(dispatcher.inboxes_mut().is_empty());
}

#[test]
fn dispatcher_register_inbox() {
    let mut dispatcher = MeshEnvelopeDispatcher::new();
    let remote = node_id(0xCC);
    let idx = dispatcher.register_inbox(remote);
    assert_eq!(idx, 0);
    assert!(dispatcher.inboxes_mut().contains_key(&remote));
}

#[test]
fn dispatcher_register_multiple_inboxes_same_remote() {
    let mut dispatcher = MeshEnvelopeDispatcher::new();
    let remote = node_id(0xCC);
    let idx0 = dispatcher.register_inbox(remote);
    let idx1 = dispatcher.register_inbox(remote);
    let idx2 = dispatcher.register_inbox(remote);
    assert_eq!(idx0, 0);
    assert_eq!(idx1, 1);
    assert_eq!(idx2, 2);

    let inboxes = dispatcher.inboxes_mut().get(&remote).unwrap();
    assert_eq!(inboxes.len(), 3);
}

#[test]
fn dispatcher_register_inboxes_different_remotes() {
    let mut dispatcher = MeshEnvelopeDispatcher::new();
    let r1 = node_id(1);
    let r2 = node_id(2);
    dispatcher.register_inbox(r1);
    dispatcher.register_inbox(r2);
    assert_eq!(dispatcher.inboxes_mut().len(), 2);
}

#[test]
fn dispatcher_unregister_inbox() {
    let mut dispatcher = MeshEnvelopeDispatcher::new();
    let remote = node_id(0xCC);
    dispatcher.register_inbox(remote);
    dispatcher.register_inbox(remote);

    dispatcher.unregister_inbox(remote, 0);
    let inboxes = dispatcher.inboxes_mut().get(&remote).unwrap();
    assert_eq!(inboxes.len(), 1);
}

#[test]
fn dispatcher_unregister_last_inbox_removes_key() {
    let mut dispatcher = MeshEnvelopeDispatcher::new();
    let remote = node_id(0xCC);
    dispatcher.register_inbox(remote);
    dispatcher.unregister_inbox(remote, 0);
    assert!(!dispatcher.inboxes_mut().contains_key(&remote));
}

#[test]
fn dispatcher_unregister_invalid_index_noop() {
    let mut dispatcher = MeshEnvelopeDispatcher::new();
    let remote = node_id(0xCC);
    dispatcher.register_inbox(remote);
    // index 1 is out of bounds
    dispatcher.unregister_inbox(remote, 1);
    assert!(dispatcher.inboxes_mut().contains_key(&remote));
}

#[test]
fn dispatcher_unregister_unknown_remote_noop() {
    let mut dispatcher = MeshEnvelopeDispatcher::new();
    dispatcher.unregister_inbox(node_id(0xFF), 0); // should not panic
}

#[test]
fn dispatcher_remove_peer() {
    let mut dispatcher = MeshEnvelopeDispatcher::new();
    let remote = node_id(0xDD);
    dispatcher.register_inbox(remote);
    dispatcher.register_inbox(remote);
    assert_eq!(dispatcher.inboxes_mut().get(&remote).unwrap().len(), 2);

    dispatcher.remove_peer(&remote);
    assert!(!dispatcher.inboxes_mut().contains_key(&remote));
}

#[test]
fn dispatcher_remove_peer_unknown_noop() {
    let mut dispatcher = MeshEnvelopeDispatcher::new();
    dispatcher.remove_peer(&node_id(0xFF)); // should not panic
}

#[test]
fn dispatcher_delivers_to_registered_inbox() {
    let mut dispatcher = MeshEnvelopeDispatcher::new();
    let remote = node_id(0xCC);
    dispatcher.register_inbox(remote);

    let payload = make_envelope(None).encode_to_vec();
    let frame = make_mesh_frame(remote, payload);

    assert!(dispatcher.deliver(&frame));

    let inboxes = dispatcher.inboxes_mut();
    let inbox = inboxes.get_mut(&remote).unwrap().first_mut().unwrap();
    assert!(!inbox.is_empty());
    assert!(inbox.pop().is_some());
}

#[test]
fn dispatcher_rejects_unknown_sender() {
    let mut dispatcher = MeshEnvelopeDispatcher::new();

    let payload = make_envelope(None).encode_to_vec();
    let frame = make_mesh_frame(node_id(0xFF), payload);

    assert!(!dispatcher.deliver(&frame));
}

#[test]
fn dispatcher_rejects_malformed_payload() {
    let mut dispatcher = MeshEnvelopeDispatcher::new();
    let remote = node_id(0xCC);
    dispatcher.register_inbox(remote);

    // Invalid protobuf payload
    let frame = make_mesh_frame(remote, vec![0xFF, 0xFE, 0xFD]);
    assert!(!dispatcher.deliver(&frame));
}

#[test]
fn dispatcher_delivers_to_first_inbox_only() {
    let mut dispatcher = MeshEnvelopeDispatcher::new();
    let remote = node_id(0xCC);
    dispatcher.register_inbox(remote);
    dispatcher.register_inbox(remote);

    let payload = make_envelope(None).encode_to_vec();
    let frame = make_mesh_frame(remote, payload);

    assert!(dispatcher.deliver(&frame));

    let inboxes = dispatcher.inboxes_mut();
    let inboxes_for_remote = inboxes.get(&remote).unwrap();
    // First inbox got the message
    assert!(!inboxes_for_remote[0].is_empty());
    // Second inbox is still empty
    assert!(inboxes_for_remote[1].is_empty());
}

#[test]
fn dispatcher_multiple_deliveries_same_sender() {
    let mut dispatcher = MeshEnvelopeDispatcher::new();
    let remote = node_id(0xCC);
    dispatcher.register_inbox(remote);

    for i in 0..3u8 {
        let payload = make_envelope(Some(capability_remote_envelope::Message::SessionOpen(
            make_session_open(i as u32, vec![i]),
        )))
        .encode_to_vec();
        let frame = make_mesh_frame(remote, payload);
        assert!(dispatcher.deliver(&frame));
    }

    let inboxes = dispatcher.inboxes_mut();
    let inbox = inboxes.get_mut(&remote).unwrap().first_mut().unwrap();
    for i in 0..3u8 {
        let env = inbox.pop().unwrap();
        if let Some(capability_remote_envelope::Message::SessionOpen(open)) = env.message {
            assert_eq!(open.version, i as u32);
        }
    }
    assert!(inbox.is_empty());
}

#[test]
fn dispatcher_multiple_remotes_independent() {
    let mut dispatcher = MeshEnvelopeDispatcher::new();
    let r1 = node_id(1);
    let r2 = node_id(2);
    dispatcher.register_inbox(r1);
    dispatcher.register_inbox(r2);

    let p1 = make_envelope(Some(capability_remote_envelope::Message::SessionOpen(
        make_session_open(1, b"r1".to_vec()),
    )))
    .encode_to_vec();
    let p2 = make_envelope(Some(capability_remote_envelope::Message::SessionOpen(
        make_session_open(2, b"r2".to_vec()),
    )))
    .encode_to_vec();

    assert!(dispatcher.deliver(&make_mesh_frame(r1, p1)));
    assert!(dispatcher.deliver(&make_mesh_frame(r2, p2)));

    // Check r1 inbox
    {
        let inboxes = dispatcher.inboxes_mut();
        let inbox = inboxes.get_mut(&r1).unwrap().first_mut().unwrap();
        assert!(!inbox.is_empty());
    }
    // Check r2 inbox
    {
        let inboxes = dispatcher.inboxes_mut();
        let inbox = inboxes.get_mut(&r2).unwrap().first_mut().unwrap();
        assert!(!inbox.is_empty());
    }
}

#[test]
fn dispatcher_inboxes_mut_returns_map() {
    let mut dispatcher = MeshEnvelopeDispatcher::new();
    let remote = node_id(0xCC);
    dispatcher.register_inbox(remote);

    let map = dispatcher.inboxes_mut();
    assert!(map.contains_key(&remote));
}

// ===================================================================
// MeshCapabilityClient tests
// ===================================================================

#[test]
fn client_creation_registers_inbox() {
    let outbound: OutboundQueue = Arc::new(Mutex::new(VecDeque::new()));
    let mut dispatcher = MeshEnvelopeDispatcher::new();
    let remote = node_id(0xDD);
    let _client = MeshCapabilityClient::new(remote, outbound, &mut dispatcher);

    assert!(dispatcher.inboxes_mut().contains_key(&remote));
}

#[test]
fn client_remote_id() {
    let outbound: OutboundQueue = Arc::new(Mutex::new(VecDeque::new()));
    let mut dispatcher = MeshEnvelopeDispatcher::new();
    let remote = node_id(0xDD);
    let client = MeshCapabilityClient::new(remote, outbound, &mut dispatcher);
    assert_eq!(*client.remote_id(), remote);
}

#[test]
fn client_transport_mut_access() {
    let outbound: OutboundQueue = Arc::new(Mutex::new(VecDeque::new()));
    let mut dispatcher = MeshEnvelopeDispatcher::new();
    let remote = node_id(0xDD);
    let mut client = MeshCapabilityClient::new(remote, outbound, &mut dispatcher);

    let transport = client.transport_mut();
    assert!(transport.inbox().is_empty());
    // Push via mutable access
    transport.inbox_mut().push(make_envelope(None));
    assert!(!transport.inbox().is_empty());
}

#[test]
fn client_can_send_via_transport() {
    let outbound: OutboundQueue = Arc::new(Mutex::new(VecDeque::new()));
    let mut dispatcher = MeshEnvelopeDispatcher::new();
    let remote = node_id(0xDD);
    let mut client = MeshCapabilityClient::new(remote, Arc::clone(&outbound), &mut dispatcher);

    let envelope = make_envelope(Some(capability_remote_envelope::Message::SessionOpen(
        make_session_open(1, b"client-session".to_vec()),
    )));
    client.transport_mut().send(envelope).unwrap();

    assert_eq!(outbound.lock().unwrap().len(), 1);
    assert_eq!(outbound.lock().unwrap()[0].0, remote);
}

#[test]
fn client_can_recv_via_transport() {
    let outbound: OutboundQueue = Arc::new(Mutex::new(VecDeque::new()));
    let mut dispatcher = MeshEnvelopeDispatcher::new();
    let remote = node_id(0xDD);
    let mut client = MeshCapabilityClient::new(remote, outbound, &mut dispatcher);

    // The client registers an inbox with the dispatcher, but the transport
    // has its own separate inbox. Deliver to dispatcher, then verify
    // the dispatcher's inbox got it (not the transport's inbox).
    let payload = make_envelope(None).encode_to_vec();
    let frame = make_mesh_frame(remote, payload);
    assert!(dispatcher.deliver(&frame));

    // Verify via dispatcher's inbox
    let inboxes = dispatcher.inboxes_mut();
    let inbox = inboxes.get_mut(&remote).unwrap().first_mut().unwrap();
    assert!(!inbox.is_empty());
    // The transport's inbox is separate and remains empty
    assert!(client.transport_mut().inbox().is_empty());
}

#[test]
fn multiple_clients_same_dispatcher() {
    let outbound: OutboundQueue = Arc::new(Mutex::new(VecDeque::new()));
    let mut dispatcher = MeshEnvelopeDispatcher::new();
    let r1 = node_id(10);
    let r2 = node_id(20);

    let _c1 = MeshCapabilityClient::new(r1, Arc::clone(&outbound), &mut dispatcher);
    let _c2 = MeshCapabilityClient::new(r2, outbound, &mut dispatcher);

    // Both inboxes should be registered
    assert!(dispatcher.inboxes_mut().contains_key(&r1));
    assert!(dispatcher.inboxes_mut().contains_key(&r2));

    // Deliver frames to each
    let p1 = make_envelope(Some(capability_remote_envelope::Message::SessionOpen(
        make_session_open(1, b"c1".to_vec()),
    )))
    .encode_to_vec();
    let p2 = make_envelope(Some(capability_remote_envelope::Message::SessionOpen(
        make_session_open(2, b"c2".to_vec()),
    )))
    .encode_to_vec();

    dispatcher.deliver(&make_mesh_frame(r1, p1));
    dispatcher.deliver(&make_mesh_frame(r2, p2));

    // Verify via dispatcher's inboxes (client transport inboxes are separate)
    {
        let inboxes = dispatcher.inboxes_mut();
        let inbox = inboxes.get_mut(&r1).unwrap().first_mut().unwrap();
        assert!(!inbox.is_empty());
    }
    {
        let inboxes = dispatcher.inboxes_mut();
        let inbox = inboxes.get_mut(&r2).unwrap().first_mut().unwrap();
        assert!(!inbox.is_empty());
    }
}

// ===================================================================
// MeshCapabilityServer tests
// ===================================================================

#[test]
fn server_new_has_empty_dispatcher() {
    let provider = MockProvider::default();
    let server = MeshCapabilityServer::new(provider);
    // Access via dispatcher is only through mutable self, so just confirm construction
    assert_eq!(server.provider().invoke_count, 0);
}

#[test]
fn server_provider_accessors() {
    let provider = MockProvider::default();
    let mut server = MeshCapabilityServer::new(provider);

    assert_eq!(server.provider().invoke_count, 0);
    server.provider_mut().invoke_count = 42;
    assert_eq!(server.provider().invoke_count, 42);
}

#[test]
fn server_serve_one_no_inboxes_returns_false() {
    let provider = MockProvider::default();
    let mut server = MeshCapabilityServer::new(provider);
    let mut link = MeshLink::new();

    // No inboxes registered, so serve_one should return Ok(false)
    let result = server.serve_one(&mut link);
    assert!(result.is_ok());
    assert!(!result.unwrap());
}

#[test]
fn server_serve_one_session_open() {
    let mut provider = MockProvider::default();
    provider.fail_on_open = false;
    let mut server = MeshCapabilityServer::new(provider);

    let remote = node_id(0xEE);
    server.dispatcher().register_inbox(remote);

    let open = make_session_open(1, b"srv-session".to_vec());
    let payload =
        make_envelope(Some(capability_remote_envelope::Message::SessionOpen(open))).encode_to_vec();
    let frame = make_mesh_frame(remote, payload);

    // Simulate mesh delivering the frame
    server.dispatcher().deliver(&frame);

    let mut link = MeshLink::new();
    let result = server.serve_one(&mut link);
    assert!(result.is_ok());
    assert!(result.unwrap());
    assert!(server.provider().session_opened); // open_session was called
}

#[test]
fn server_serve_one_session_open_error() {
    let mut provider = MockProvider::default();
    provider.fail_on_open = true;
    let mut server = MeshCapabilityServer::new(provider);

    let remote = node_id(0xEE);
    server.dispatcher().register_inbox(remote);

    let open = make_session_open(1, b"srv-session".to_vec());
    let payload =
        make_envelope(Some(capability_remote_envelope::Message::SessionOpen(open))).encode_to_vec();
    let frame = make_mesh_frame(remote, payload);
    server.dispatcher().deliver(&frame);

    let mut link = MeshLink::new();
    let result = server.serve_one(&mut link);
    assert!(result.is_err());
}

#[test]
fn server_serve_one_invocation() {
    let provider = MockProvider::default();
    let mut server = MeshCapabilityServer::new(provider);

    let remote = node_id(0xEE);
    server.dispatcher().register_inbox(remote);

    let invocation = make_invocation(b"inv-1".to_vec(), b"grant-1".to_vec());
    let payload = make_envelope(Some(capability_remote_envelope::Message::Invocation(
        invocation,
    )))
    .encode_to_vec();
    let frame = make_mesh_frame(remote, payload);
    server.dispatcher().deliver(&frame);

    let mut link = MeshLink::new();
    let result = server.serve_one(&mut link);
    assert!(result.is_ok());
    assert!(result.unwrap());
    assert_eq!(server.provider().invoke_count, 1);
}

#[test]
fn server_serve_one_invocation_error() {
    let mut provider = MockProvider::default();
    provider.fail_on_invoke = true;
    let mut server = MeshCapabilityServer::new(provider);

    let remote = node_id(0xEE);
    server.dispatcher().register_inbox(remote);

    let invocation = make_invocation(b"inv-err".to_vec(), b"grant-1".to_vec());
    let payload = make_envelope(Some(capability_remote_envelope::Message::Invocation(
        invocation,
    )))
    .encode_to_vec();
    let frame = make_mesh_frame(remote, payload);
    server.dispatcher().deliver(&frame);

    let mut link = MeshLink::new();
    let result = server.serve_one(&mut link);
    assert!(result.is_ok());
    assert!(result.unwrap());
    // The server should still return Ok(true) — it processes the error
    // and pushes an error result envelope back to the inbox
}

#[test]
fn server_serve_one_invocation_frame() {
    let provider = MockProvider::default();
    let mut server = MeshCapabilityServer::new(provider);

    let remote = node_id(0xEE);
    server.dispatcher().register_inbox(remote);

    let invocation = make_invocation(b"inv-frame".to_vec(), b"grant-1".to_vec());
    let frame_msg =
        capability_remote_envelope::Message::InvocationFrame(CapabilityInvocationFrame {
            invocation: Some(invocation),
            inline_parameters: vec![1, 2, 3],
        });
    let payload = make_envelope(Some(frame_msg)).encode_to_vec();
    let frame = make_mesh_frame(remote, payload);
    server.dispatcher().deliver(&frame);

    let mut link = MeshLink::new();
    let result = server.serve_one(&mut link);
    assert!(result.is_ok());
    assert!(result.unwrap());
    assert_eq!(server.provider().invoke_count, 1);
}

#[test]
fn server_serve_one_invocation_frame_missing_invocation() {
    let provider = MockProvider::default();
    let mut server = MeshCapabilityServer::new(provider);

    let remote = node_id(0xEE);
    server.dispatcher().register_inbox(remote);

    let frame_msg =
        capability_remote_envelope::Message::InvocationFrame(CapabilityInvocationFrame {
            invocation: None,
            inline_parameters: vec![],
        });
    let payload = make_envelope(Some(frame_msg)).encode_to_vec();
    let frame = make_mesh_frame(remote, payload);
    server.dispatcher().deliver(&frame);

    let mut link = MeshLink::new();
    let result = server.serve_one(&mut link);
    assert!(result.is_err());
}

#[test]
fn server_serve_one_session_close() {
    let provider = MockProvider::default();
    let mut server = MeshCapabilityServer::new(provider);

    let remote = node_id(0xEE);
    server.dispatcher().register_inbox(remote);

    let close = CapabilitySessionClose {
        session_id: b"sess-close".to_vec(),
        version: 1,
        reason: String::new(),
    };
    let payload = make_envelope(Some(capability_remote_envelope::Message::SessionClose(
        close,
    )))
    .encode_to_vec();
    let frame = make_mesh_frame(remote, payload);
    server.dispatcher().deliver(&frame);

    let mut link = MeshLink::new();
    let result = server.serve_one(&mut link);
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[test]
fn server_serve_one_request_no_grant() {
    let provider = MockProvider::default();
    let mut server = MeshCapabilityServer::new(provider);

    let remote = node_id(0xEE);
    server.dispatcher().register_inbox(remote);

    let request = CapabilityRequest {
        request_version: 1,
        request_id: b"req-1".to_vec(),
        requester: None,
        requester_node: None,
        selector: None,
        requested_operations: vec![],
        requested_constraints: vec![],
        purpose: String::new(),
        requested_duration: None,
        correlation_id: vec![],
        signature: None,
    };
    let payload =
        make_envelope(Some(capability_remote_envelope::Message::Request(request))).encode_to_vec();
    let frame = make_mesh_frame(remote, payload);
    server.dispatcher().deliver(&frame);

    let mut link = MeshLink::new();
    let result = server.serve_one(&mut link);
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[test]
fn server_serve_one_grant_message() {
    let provider = MockProvider::default();
    let mut server = MeshCapabilityServer::new(provider);

    let remote = node_id(0xEE);
    server.dispatcher().register_inbox(remote);

    let grant = edgerun_capabilities::CapabilityGrant {
        grant_version: 1,
        grant_id: b"grant-1".to_vec(),
        issuer: None,
        grantee: None,
        grantee_node: None,
        selector: None,
        granted_operations: vec![],
        enforced_constraints: vec![],
        access_class: 0,
        issued_at: None,
        expires_at: None,
        correlation_id: vec![],
        supersedes_revocation: None,
        signature: None,
    };
    let payload =
        make_envelope(Some(capability_remote_envelope::Message::Grant(grant))).encode_to_vec();
    let frame = make_mesh_frame(remote, payload);
    server.dispatcher().deliver(&frame);

    let mut link = MeshLink::new();
    let result = server.serve_one(&mut link);
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[test]
fn server_serve_one_revocation() {
    let provider = MockProvider::default();
    let mut server = MeshCapabilityServer::new(provider);

    let remote = node_id(0xEE);
    server.dispatcher().register_inbox(remote);

    let revocation = edgerun_proto::edgerun::v0::capability::CapabilityRevocation {
        revocation_version: 1,
        revocation_id: b"rev-1".to_vec(),
        grant_id: b"grant-1".to_vec(),
        issuer: None,
        effective_at: None,
        reason: String::new(),
        replacement_constraints: vec![],
        signature: None,
    };
    let payload = make_envelope(Some(capability_remote_envelope::Message::Revocation(
        revocation,
    )))
    .encode_to_vec();
    let frame = make_mesh_frame(remote, payload);
    server.dispatcher().deliver(&frame);

    let mut link = MeshLink::new();
    let result = server.serve_one(&mut link);
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[test]
fn server_serve_one_noop_messages() {
    // SessionAccept, SessionEvent, Result, ResultFrame, and None
    // should all return Ok(true) but not call any provider method.
    let noop_messages = vec![
        capability_remote_envelope::Message::SessionAccept(
            edgerun_proto::edgerun::v0::capability_runtime::CapabilitySessionAccept {
                version: 1,
                session_id: b"s".to_vec(),
                accepted: true,
                granted_operations: vec![],
                granted_access_class: 0,
                error_reason: String::new(),
                grant_id: vec![],
            },
        ),
        capability_remote_envelope::Message::SessionEvent(
            edgerun_proto::edgerun::v0::capability_runtime::CapabilitySessionEvent {
                version: 1,
                session_id: b"s".to_vec(),
                sequence_no: 0,
                event_kinds: vec![],
                payload_object: None,
                inline_payload: vec![],
            },
        ),
        capability_remote_envelope::Message::Result(CapabilityResult {
            result_version: 1,
            invocation_id: vec![],
            grant_id: vec![],
            success: true,
            result_access_class: 0,
            produced_event_kinds: vec![],
            payload_object: None,
            error_reason: String::new(),
            produced_at: None,
            signature: None,
        }),
        capability_remote_envelope::Message::ResultFrame(
            edgerun_proto::edgerun::v0::capability_runtime::CapabilityResultFrame {
                result: None,
                inline_payload: vec![],
            },
        ),
    ];

    for msg in noop_messages {
        let provider = MockProvider::default();
        let mut server = MeshCapabilityServer::new(provider);
        let remote = node_id(0xEE);
        server.dispatcher().register_inbox(remote);

        let payload = make_envelope(Some(msg)).encode_to_vec();
        let frame = make_mesh_frame(remote, payload);
        server.dispatcher().deliver(&frame);

        let mut link = MeshLink::new();
        let result = server.serve_one(&mut link);
        assert!(result.is_ok(), "noop message should succeed");
        assert!(result.unwrap(), "noop message should return true");
    }
}

#[test]
fn server_serve_one_null_envelope() {
    let provider = MockProvider::default();
    let mut server = MeshCapabilityServer::new(provider);

    let remote = node_id(0xEE);
    server.dispatcher().register_inbox(remote);

    let payload = make_envelope(None).encode_to_vec();
    let frame = make_mesh_frame(remote, payload);
    server.dispatcher().deliver(&frame);

    let mut link = MeshLink::new();
    let result = server.serve_one(&mut link);
    assert!(result.is_ok());
    assert!(result.unwrap()); // processes the None message, returns true
}

#[test]
fn server_serve_one_processes_in_order() {
    // Register two inboxes; serve_one should process the first sender's inbox
    let provider = MockProvider::default();
    let mut server = MeshCapabilityServer::new(provider);

    let r1 = node_id(1);
    let r2 = node_id(2);
    server.dispatcher().register_inbox(r1);
    server.dispatcher().register_inbox(r2);

    // Only deliver to r2
    let invocation = make_invocation(b"inv-2".to_vec(), b"grant-2".to_vec());
    let payload = make_envelope(Some(capability_remote_envelope::Message::Invocation(
        invocation,
    )))
    .encode_to_vec();
    let frame = make_mesh_frame(r2, payload);
    server.dispatcher().deliver(&frame);

    let mut link = MeshLink::new();
    let result = server.serve_one(&mut link);
    assert!(result.is_ok());
    assert!(result.unwrap());
    assert_eq!(server.provider().invoke_count, 1);
}

// ===================================================================
// Error type tests
// ===================================================================

#[test]
fn capability_error_display_invalid_request() {
    let err = CapabilityError::InvalidRequest("bad input");
    let msg = format!("{}", err);
    assert!(msg.contains("invalid capability request"));
    assert!(msg.contains("bad input"));
}

#[test]
fn capability_error_display_permission_denied() {
    let err = CapabilityError::PermissionDenied("not allowed");
    let msg = format!("{}", err);
    assert!(msg.contains("capability permission denied"));
    assert!(msg.contains("not allowed"));
}

#[test]
fn capability_error_display_unsupported() {
    let err = CapabilityError::Unsupported("operation X");
    let msg = format!("{}", err);
    assert!(msg.contains("unsupported capability operation"));
    assert!(msg.contains("operation X"));
}

#[test]
fn capability_error_display_provider() {
    let err = CapabilityError::Provider("provider error details".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("provider error details"));
}

#[test]
fn capability_error_is_std_error() {
    // Verify it implements std::error::Error
    let err: Box<dyn std::error::Error> = Box::new(CapabilityError::InvalidRequest("test"));
    assert!(err.source().is_none());
}

#[test]
fn capability_error_variants_are_distinct() {
    let e1 = CapabilityError::InvalidRequest("a");
    let e2 = CapabilityError::PermissionDenied("a");
    let e3 = CapabilityError::Unsupported("a");
    let e4 = CapabilityError::Provider("a".to_string());

    assert_ne!(e1, e2);
    assert_ne!(e1, e3);
    assert_ne!(e1, e4);
    assert_ne!(e2, e3);
    assert_ne!(e2, e4);
    assert_ne!(e3, e4);
}

// ===================================================================
// Integration: dispatcher -> transport -> server round-trip
// ===================================================================

#[test]
fn full_roundtrip_dispatcher_to_transport_recv() {
    let mut dispatcher = MeshEnvelopeDispatcher::new();
    let remote = node_id(0xBB);
    let idx = dispatcher.register_inbox(remote);

    // Dispatcher receives a mesh frame
    let payload = make_envelope(Some(capability_remote_envelope::Message::SessionOpen(
        make_session_open(1, b"rt-session".to_vec()),
    )))
    .encode_to_vec();
    let frame = make_mesh_frame(remote, payload);
    assert!(dispatcher.deliver(&frame));

    // Verify the inbox has it
    let inboxes = dispatcher.inboxes_mut();
    let inbox = &mut inboxes.get_mut(&remote).unwrap()[idx];
    assert!(!inbox.is_empty());

    let env = inbox.pop().unwrap();
    match &env.message {
        Some(capability_remote_envelope::Message::SessionOpen(open)) => {
            assert_eq!(open.session_id, b"rt-session");
        }
        _ => panic!("expected SessionOpen"),
    }
}

#[test]
fn multiple_clients_multiple_servers_shared_dispatcher() {
    // Simulate: two clients sending to one server
    let outbound: OutboundQueue = Arc::new(Mutex::new(VecDeque::new()));
    let mut dispatcher = MeshEnvelopeDispatcher::new();

    let r1 = node_id(100);
    let r2 = node_id(200);
    let mut c1 = MeshCapabilityClient::new(r1, Arc::clone(&outbound), &mut dispatcher);
    let mut c2 = MeshCapabilityClient::new(r2, Arc::clone(&outbound), &mut dispatcher);

    // Server listening on both remotes
    let provider = MockProvider::default();
    let mut server = MeshCapabilityServer::new(provider);
    server.dispatcher().register_inbox(r1);
    server.dispatcher().register_inbox(r2);

    // Client 1 sends a session open
    let open1 = make_session_open(1, b"c1-session".to_vec());
    c1.transport_mut()
        .send(make_envelope(Some(
            capability_remote_envelope::Message::SessionOpen(open1),
        )))
        .unwrap();

    // Client 2 sends a session open
    let open2 = make_session_open(2, b"c2-session".to_vec());
    c2.transport_mut()
        .send(make_envelope(Some(
            capability_remote_envelope::Message::SessionOpen(open2),
        )))
        .unwrap();

    // Verify outbound queue has both
    assert_eq!(outbound.lock().unwrap().len(), 2);
}

#[test]
fn dispatcher_unregister_breaks_delivery() {
    let mut dispatcher = MeshEnvelopeDispatcher::new();
    let remote = node_id(0xCC);
    dispatcher.register_inbox(remote);
    dispatcher.unregister_inbox(remote, 0);

    // After unregister, delivery should fail
    let payload = make_envelope(None).encode_to_vec();
    let frame = make_mesh_frame(remote, payload);
    assert!(!dispatcher.deliver(&frame));
}

// =======================================================================
// Integration Tests
// =======================================================================

/// Integration test: End-to-end capability session open via mesh transport.
///
/// Simulates a client on one node sending a SessionOpen to a server on
/// another node via the mesh capability transport, and receiving the
/// SessionAccept response.
#[test]
fn integration_capability_session_open_via_mesh() {
    let client_id = node_id(0xAA);

    // Server side: MockProvider that accepts all sessions
    let mut mock = MockProvider::default();
    mock.fail_on_open = false;
    let mut server = MeshCapabilityServer::new(mock);

    // Client side
    let outbound: OutboundQueue = Arc::new(Mutex::new(VecDeque::new()));
    let mut dispatcher = MeshEnvelopeDispatcher::new();
    let mut client = MeshCapabilityClient::new(client_id, outbound.clone(), &mut dispatcher);

    // Client sends SessionOpen to server
    let open = make_session_open(1, b"test-session".to_vec());
    client
        .transport_mut()
        .send(make_envelope(Some(
            capability_remote_envelope::Message::SessionOpen(open.clone()),
        )))
        .unwrap();

    // Verify outbound has the envelope
    assert_eq!(outbound.lock().unwrap().len(), 1);
    let (dest, payload) = outbound.lock().unwrap().pop_front().unwrap();
    assert_eq!(dest, client_id); // sent to the client's remote target

    // Simulate mesh delivery: server receives the envelope
    let envelope = CapabilityRemoteEnvelope::decode(&payload[..]).unwrap();
    if let Some(capability_remote_envelope::Message::SessionOpen(session_open)) = envelope.message {
        // Server processes the session open
        let accept = server.provider_mut().open_session(&session_open).unwrap();
        assert!(accept.accepted, "server should accept the session");
        assert_eq!(accept.session_id, session_open.session_id);
        assert_eq!(accept.granted_operations, session_open.requested_operations);
    } else {
        panic!("expected SessionOpen message");
    }
}

/// Integration test: Capability invocation round-trip via mesh transport.
///
/// Verifies the full lifecycle: SessionOpen → SessionAccept → Invocation → Result.
#[test]
fn integration_capability_invocation_roundtrip_via_mesh() {
    let client_id = node_id(0xAA);
    let session_id = b"invoke-test".to_vec();

    // Server: accepts sessions and allows invokes
    let mut mock = MockProvider::default();
    mock.fail_on_open = false;
    mock.fail_on_invoke = false;
    let mut server = MeshCapabilityServer::new(mock);

    // Step 1: Session open
    let open = make_session_open(1, session_id.clone());
    let accept = server.provider_mut().open_session(&open).unwrap();
    assert!(accept.accepted);

    // Step 2: Client sends invocation
    let invocation = make_invocation(b"inv-1".to_vec(), session_id.clone());
    let invoke_envelope = CapabilityRemoteEnvelope {
        message: Some(capability_remote_envelope::Message::Invocation(
            invocation.clone(),
        )),
    };

    // Step 3: Server processes invocation
    let response = server
        .process_envelope(&client_id, invoke_envelope)
        .unwrap();
    assert!(response.is_some());

    // Step 4: Verify result
    if let Some(resp_env) = response {
        if let Some(capability_remote_envelope::Message::Result(result)) = resp_env.message {
            assert!(result.success, "invocation should succeed");
            assert_eq!(result.invocation_id, invocation.invocation_id);
        } else {
            panic!("expected Result message");
        }
    } else {
        panic!("server should have responded to invocation");
    }
}
