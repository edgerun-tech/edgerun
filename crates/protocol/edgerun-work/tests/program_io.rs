use alloc::collections::BTreeSet;

use edgerun_crypto::Ed25519SigningKey;
use edgerun_work::*;

extern crate alloc;

#[derive(Default)]
struct EchoProgramAdapter {
    sessions: BTreeSet<Hash>,
}

impl ProgramIoAdapter for EchoProgramAdapter {
    fn open(&mut self, request: ProgramOpen) -> Result<Vec<ProgramIoEvent>, ProgramIoError> {
        if !self.sessions.insert(request.session_id) {
            return Err(ProgramIoError::SessionExists);
        }
        Ok(vec![ProgramIoEvent::Output(ProgramOutput {
            session_id: request.session_id,
            stream: PROGRAM_STREAM_STDOUT,
            bytes: request.program.into_bytes(),
            eof: false,
        })])
    }

    fn stdin(&mut self, input: ProgramStdin) -> Result<Vec<ProgramIoEvent>, ProgramIoError> {
        if !self.sessions.contains(&input.session_id) {
            return Err(ProgramIoError::SessionMissing);
        }
        Ok(vec![ProgramIoEvent::Output(ProgramOutput {
            session_id: input.session_id,
            stream: PROGRAM_STREAM_STDOUT,
            bytes: input.bytes,
            eof: input.eof,
        })])
    }

    fn close(&mut self, close: ProgramClose) -> Result<Vec<ProgramIoEvent>, ProgramIoError> {
        if !self.sessions.remove(&close.session_id) {
            return Err(ProgramIoError::SessionMissing);
        }
        Ok(vec![ProgramIoEvent::Exit(ProgramExit {
            session_id: close.session_id,
            code: 0,
        })])
    }

    fn poll(&mut self, poll: ProgramPoll) -> Result<Vec<ProgramIoEvent>, ProgramIoError> {
        if !self.sessions.contains(&poll.session_id) {
            return Err(ProgramIoError::SessionMissing);
        }
        Ok(Vec::new())
    }
}

fn signed_program_message(
    key: &Ed25519SigningKey,
    from: NodeId,
    to: NodeId,
    via_relay: NodeId,
    work_type: u16,
    payload: ProgramIoPayload,
    sequence: u64,
) -> NetworkMessage {
    let payload = program_io_payload_bytes(&payload).expect("program payload bytes");
    let payload_hash = blake3_hash(&payload);
    let message_id = HashBuilder::domain(b"edgerun:test:program-io")
        .node_id(&from)
        .node_id(&to)
        .node_id(&via_relay)
        .u64(sequence)
        .hash(&payload_hash)
        .finish();
    sign_network_message_payload(
        key,
        message_id,
        [0u8; 32],
        from,
        to,
        via_relay,
        DEPARTMENT_COMPUTE,
        work_type,
        sequence,
        payload,
    )
}

fn decode_events(bytes: &[u8]) -> Vec<ProgramIoEvent> {
    program_io_events_from_bytes(bytes)
        .expect("program events")
        .events
}

fn output_bytes(response: &WorkServiceResponse) -> Vec<u8> {
    let events = decode_events(&response.bytes);
    let ProgramIoEvent::Output(output) = &events[0] else {
        panic!("expected output event");
    };
    output.bytes.clone()
}

#[cfg(feature = "std")]
fn poll_message(
    key: &Ed25519SigningKey,
    from: NodeId,
    to: NodeId,
    via_relay: NodeId,
    session_id: Hash,
    sequence: u64,
) -> NetworkMessage {
    signed_program_message(
        key,
        from,
        to,
        via_relay,
        WORK_TYPE_PROGRAM_POLL,
        ProgramIoPayload::Poll(ProgramPoll { session_id }),
        sequence,
    )
}

#[test]
fn program_io_service_binds_adapter_to_compute_identity() {
    let compute_key = Ed25519SigningKey::from_bytes(&[101u8; 32]);
    let client_key = Ed25519SigningKey::from_bytes(&[102u8; 32]);
    let relay = SimNode::from_seed(103, NODE_ROLE_RELAY);
    let client = node_identity_from_key(&client_key, NODE_ROLE_MESSAGE);
    let mut service = ProgramIoService::new(compute_key.clone(), EchoProgramAdapter::default());
    let compute = node_identity_from_key(&compute_key, NODE_ROLE_COMPUTE);
    let session_id = blake3_hash(b"program session");

    assert_eq!(service.identity(), &compute);
    let route = service.route_binding(
        memory_endpoint("program-worker", &compute.node_id),
        relay.identity.node_id,
        u64::MAX,
    );
    assert!(verify_route_binding(&route));
    assert_eq!(route.node, compute);
    assert_eq!(route.departments, vec![DEPARTMENT_COMPUTE]);

    let opened = service.handle_packet(
        WorkPacket::NetworkMessage(signed_program_message(
            &client_key,
            client.node_id,
            compute.node_id,
            relay.identity.node_id,
            WORK_TYPE_PROGRAM_OPEN,
            ProgramIoPayload::Open(ProgramOpen {
                session_id,
                program: "echo".into(),
                args: vec!["hello".into()],
                env: Vec::new(),
                cwd: String::new(),
            }),
            1,
        )),
        1_000,
    );
    assert_eq!(opened.status, ROLE_STATUS_ACCEPTED);
    let events = decode_events(&opened.bytes);
    assert!(matches!(
        &events[0],
        ProgramIoEvent::Output(output)
            if output.session_id == session_id && output.bytes == b"echo"
    ));
    let Some(WorkPacket::NetworkMessage(event_message)) = opened.packet else {
        panic!("expected signed program event packet");
    };
    assert_eq!(event_message.from, compute.node_id);
    assert_eq!(event_message.to, client.node_id);
    assert_eq!(event_message.via_relay, relay.identity.node_id);
    assert_eq!(event_message.work_type, WORK_TYPE_PROGRAM_EVENT);
    assert!(verify_network_message(&event_message, &compute));
    assert_eq!(decode_events(&event_message.payload), events);

    let wrote = service.handle_packet(
        WorkPacket::NetworkMessage(signed_program_message(
            &client_key,
            client.node_id,
            compute.node_id,
            relay.identity.node_id,
            WORK_TYPE_PROGRAM_STDIN,
            ProgramIoPayload::Stdin(ProgramStdin {
                session_id,
                bytes: b"through relay".to_vec(),
                eof: false,
            }),
            2,
        )),
        1_001,
    );
    assert_eq!(wrote.status, ROLE_STATUS_ACCEPTED);
    let events = decode_events(&wrote.bytes);
    assert!(matches!(
        &events[0],
        ProgramIoEvent::Output(output)
            if output.stream == PROGRAM_STREAM_STDOUT && output.bytes == b"through relay"
    ));

    let closed = service.handle_packet(
        WorkPacket::NetworkMessage(signed_program_message(
            &client_key,
            client.node_id,
            compute.node_id,
            relay.identity.node_id,
            WORK_TYPE_PROGRAM_CLOSE,
            ProgramIoPayload::Close(ProgramClose {
                session_id,
                signal: 0,
            }),
            3,
        )),
        1_002,
    );
    assert_eq!(closed.status, ROLE_STATUS_ACCEPTED);
    let events = decode_events(&closed.bytes);
    assert!(matches!(
        &events[0],
        ProgramIoEvent::Exit(exit) if exit.session_id == session_id && exit.code == 0
    ));
}

#[test]
fn program_io_messages_can_be_forwarded_through_relay() {
    let compute_key = Ed25519SigningKey::from_bytes(&[111u8; 32]);
    let client_key = Ed25519SigningKey::from_bytes(&[112u8; 32]);
    let client = node_identity_from_key(&client_key, NODE_ROLE_MESSAGE);
    let relay_node = SimNode::from_seed(113, NODE_ROLE_RELAY);
    let mut relay = RelayRole::from_seed(113, 0);
    let mut service = ProgramIoService::new(compute_key.clone(), EchoProgramAdapter::default());
    let compute = node_identity_from_key(&compute_key, NODE_ROLE_COMPUTE);
    let session_id = blake3_hash(b"relay program session");

    let mut channel = MemoryChannelEngine::new();
    channel
        .add_route(
            relay_node.bind_memory_route(relay_node.identity.node_id, vec![DEPARTMENT_RELAY]),
        )
        .expect("relay route");
    channel
        .add_route(service.route_binding(
            memory_endpoint("compute-worker", &compute.node_id),
            relay_node.identity.node_id,
            u64::MAX,
        ))
        .expect("compute route");
    channel
        .add_route(
            SimNode::from_seed(112, NODE_ROLE_MESSAGE)
                .bind_memory_route(relay_node.identity.node_id, vec![DEPARTMENT_COMPUTE]),
        )
        .expect("client return route");

    let payload = program_io_payload_bytes(&ProgramIoPayload::Open(ProgramOpen {
        session_id,
        program: "shell".into(),
        args: vec!["-lc".into(), "cat".into()],
        env: Vec::new(),
        cwd: String::new(),
    }))
    .expect("open payload");
    let message = signed_program_message(
        &client_key,
        client.node_id,
        compute.node_id,
        relay_node.identity.node_id,
        WORK_TYPE_PROGRAM_OPEN,
        program_io_payload_from_bytes(&payload).expect("open payload decode"),
        1,
    );
    let packet = WorkPacket::NetworkMessage(message);
    let ordered = deliver_ordered(
        &mut channel,
        &mut ChannelOrderBook::new(),
        client.node_id,
        relay_node.identity.node_id,
        packet,
    )
    .expect("client to relay");
    relay
        .forward_ordered_on(&mut channel, &ordered, [0u8; 32], [0u8; 32])
        .expect("relay forwards program open");

    let mut inbox = channel.drain_inbox(compute.node_id);
    assert_eq!(inbox.len(), 1);
    let response = service.handle_packet(inbox.remove(0).packet, 1_000);
    assert_eq!(response.status, ROLE_STATUS_ACCEPTED);
    assert_eq!(output_bytes(&response), b"shell");
    let Some(WorkPacket::NetworkMessage(event_message)) = response.packet else {
        panic!("expected signed event response");
    };
    let ordered = deliver_ordered(
        &mut channel,
        &mut ChannelOrderBook::new(),
        compute.node_id,
        relay_node.identity.node_id,
        WorkPacket::NetworkMessage(event_message),
    )
    .expect("compute event to relay");
    relay
        .forward_ordered_on(&mut channel, &ordered, [0u8; 32], [0u8; 32])
        .expect("relay forwards program event");
    let mut client_inbox = channel.drain_inbox(client.node_id);
    assert_eq!(client_inbox.len(), 1);
    let WorkPacket::NetworkMessage(returned) = client_inbox.remove(0).packet else {
        panic!("expected network message");
    };
    assert_eq!(returned.from, compute.node_id);
    assert_eq!(returned.to, client.node_id);
    assert_eq!(returned.work_type, WORK_TYPE_PROGRAM_EVENT);
    assert!(verify_network_message(&returned, &compute));
    assert_eq!(
        decode_events(&returned.payload)[0],
        ProgramIoEvent::Output(ProgramOutput {
            session_id,
            stream: PROGRAM_STREAM_STDOUT,
            bytes: b"shell".to_vec(),
            eof: false,
        })
    );
}

#[cfg(feature = "std")]
#[test]
fn native_process_adapter_pipes_stdio() {
    use std::thread;
    use std::time::Duration;

    let compute_key = Ed25519SigningKey::from_bytes(&[121u8; 32]);
    let client_key = Ed25519SigningKey::from_bytes(&[122u8; 32]);
    let client = node_identity_from_key(&client_key, NODE_ROLE_MESSAGE);
    let relay = SimNode::from_seed(123, NODE_ROLE_RELAY);
    let mut service = ProgramIoService::new(compute_key.clone(), NativeProcessAdapter::new());
    let compute = node_identity_from_key(&compute_key, NODE_ROLE_COMPUTE);
    let session_id = blake3_hash(b"native cat session");

    let opened = service.handle_packet(
        WorkPacket::NetworkMessage(signed_program_message(
            &client_key,
            client.node_id,
            compute.node_id,
            relay.identity.node_id,
            WORK_TYPE_PROGRAM_OPEN,
            ProgramIoPayload::Open(ProgramOpen {
                session_id,
                program: "cat".into(),
                args: Vec::new(),
                env: Vec::new(),
                cwd: String::new(),
            }),
            1,
        )),
        1_000,
    );
    assert_eq!(opened.status, ROLE_STATUS_ACCEPTED);

    let wrote = service.handle_packet(
        WorkPacket::NetworkMessage(signed_program_message(
            &client_key,
            client.node_id,
            compute.node_id,
            relay.identity.node_id,
            WORK_TYPE_PROGRAM_STDIN,
            ProgramIoPayload::Stdin(ProgramStdin {
                session_id,
                bytes: b"native process io\n".to_vec(),
                eof: true,
            }),
            2,
        )),
        1_001,
    );
    assert_eq!(wrote.status, ROLE_STATUS_ACCEPTED);

    let mut saw_stdout = false;
    let mut saw_exit = false;
    for sequence in 3..50 {
        let polled = service.handle_packet(
            WorkPacket::NetworkMessage(poll_message(
                &client_key,
                client.node_id,
                compute.node_id,
                relay.identity.node_id,
                session_id,
                sequence,
            )),
            1_000 + sequence,
        );
        assert_eq!(polled.status, ROLE_STATUS_ACCEPTED);
        if !polled.bytes.is_empty() {
            for event in decode_events(&polled.bytes) {
                match event {
                    ProgramIoEvent::Output(output)
                        if output.stream == PROGRAM_STREAM_STDOUT
                            && output.bytes == b"native process io\n" =>
                    {
                        saw_stdout = true;
                    }
                    ProgramIoEvent::Exit(exit)
                        if exit.session_id == session_id && exit.code == 0 =>
                    {
                        saw_exit = true;
                    }
                    _ => {}
                }
            }
        }
        if saw_stdout && saw_exit {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }

    assert!(saw_stdout, "native process stdout was not forwarded");
    assert!(saw_exit, "native process exit was not forwarded");
}
