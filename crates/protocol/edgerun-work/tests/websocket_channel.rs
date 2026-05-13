#![cfg(feature = "std")]

use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

use edgerun_work::*;

fn masked_client_binary(payload: &[u8]) -> Vec<u8> {
    let mask = [1u8, 2, 3, 4];
    let mut frame = Vec::new();
    frame.push(0x82);
    if payload.len() < 126 {
        frame.push(0x80 | payload.len() as u8);
    } else {
        frame.push(0x80 | 126);
        frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    }
    frame.extend_from_slice(&mask);
    frame.extend(
        payload
            .iter()
            .enumerate()
            .map(|(index, byte)| byte ^ mask[index % mask.len()]),
    );
    frame
}

fn wait_for_envelopes(hub: &WebSocketWorkHub) -> Vec<ChannelEnvelope> {
    for _ in 0..50 {
        let envelopes = hub.drain_envelopes();
        if !envelopes.is_empty() {
            return envelopes;
        }
        thread::sleep(Duration::from_millis(10));
    }
    Vec::new()
}

#[test]
fn websocket_hub_accepts_masked_browser_envelope() {
    let hub = WebSocketWorkHub::bind("127.0.0.1:0").expect("bind websocket hub");
    let mut stream = TcpStream::connect(hub.listen_addr()).expect("connect websocket hub");
    stream
        .write_all(
            b"GET /work HTTP/1.1\r\n\
              Host: 127.0.0.1\r\n\
              Upgrade: websocket\r\n\
              Connection: Upgrade\r\n\
              Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
              Sec-WebSocket-Version: 13\r\n\
              Sec-WebSocket-Protocol: edgerun-work-v1\r\n\
              \r\n",
        )
        .expect("write handshake");
    let mut response = [0u8; 256];
    let read = stream.read(&mut response).expect("read handshake");
    let response = String::from_utf8_lossy(&response[..read]);
    assert!(response.contains("101 Switching Protocols"));
    assert!(response.contains("Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo="));

    let sender = SimNode::from_seed(1, NODE_ROLE_MESSAGE);
    let recipient = SimNode::from_seed(2, NODE_ROLE_MESSAGE);
    let packet = WorkPacket::Ack(WorkAck {
        ok: true,
        code: 200,
        text: "hello over websocket".to_owned(),
    });
    let envelope = ChannelEnvelope {
        abi_version: WORK_WIRE_ABI_VERSION,
        channel_id: [3u8; 32],
        from: sender.identity.node_id,
        to: recipient.identity.node_id,
        route_hash: [4u8; 32],
        packet_hash: encode_work_packet_once(&packet)
            .expect("encoded packet")
            .hash,
        packet,
    };
    let envelope_bytes = channel_envelope_bytes(&envelope).expect("envelope bytes");
    stream
        .write_all(&masked_client_binary(&envelope_bytes))
        .expect("write masked frame");

    let envelopes = wait_for_envelopes(&hub);
    assert_eq!(envelopes, vec![envelope]);
    assert_eq!(hub.connected_peer_count(), 1);
}

#[test]
fn sealed_chat_message_roundtrips_over_websocket_into_thread() {
    let hub = WebSocketWorkHub::bind("127.0.0.1:0").expect("bind websocket hub");
    let mut stream = TcpStream::connect(hub.listen_addr()).expect("connect websocket hub");
    stream
        .write_all(
            b"GET /work HTTP/1.1\r\n\
              Host: 127.0.0.1\r\n\
              Upgrade: websocket\r\n\
              Connection: Upgrade\r\n\
              Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
              Sec-WebSocket-Version: 13\r\n\
              Sec-WebSocket-Protocol: edgerun-work-v1\r\n\
              \r\n",
        )
        .expect("write handshake");
    let mut response = [0u8; 256];
    let _ = stream.read(&mut response).expect("read handshake");

    let mut sender = SimNode::from_seed(11, NODE_ROLE_MESSAGE);
    let recipient = SimNode::from_seed(12, NODE_ROLE_MESSAGE);
    let plaintext = b"this is the first reliable chat message";
    let sealed_payload = seal_message_for_recipient_with_ephemeral(
        &sender.identity,
        &recipient.identity,
        plaintext,
        [77u8; 32],
        [88u8; 12],
    )
    .expect("seal payload");
    let packet = sender.message_to(
        recipient.identity.node_id,
        [9u8; 32],
        DEPARTMENT_MESSAGE,
        WORK_TYPE_MESSAGE_DELIVER,
        sealed_payload.clone(),
    );
    let envelope = ChannelEnvelope {
        abi_version: WORK_WIRE_ABI_VERSION,
        channel_id: [3u8; 32],
        from: sender.identity.node_id,
        to: recipient.identity.node_id,
        route_hash: [4u8; 32],
        packet_hash: encode_work_packet_once(&packet)
            .expect("encoded packet")
            .hash,
        packet,
    };
    let envelope_bytes = channel_envelope_bytes(&envelope).expect("envelope bytes");
    stream
        .write_all(&masked_client_binary(&envelope_bytes))
        .expect("write masked frame");

    let envelopes = wait_for_envelopes(&hub);
    let WorkPacket::NetworkMessage(message) = &envelopes[0].packet else {
        panic!("expected network message");
    };
    assert_eq!(message.payload_hash, chat_payload_hash(&sealed_payload));
    let opened = unseal_message_from_recipient_payload(
        &recipient.key,
        &recipient.identity,
        &message.payload,
    )
    .expect("recipient decrypts");
    assert_eq!(opened, plaintext);

    let mut thread = empty_thread(sender.identity.node_id, recipient.identity.node_id, 1_000);
    let message_object = finalize_message_object(MessageObject {
        abi_version: WORK_WIRE_ABI_VERSION,
        message_id: [0u8; 32],
        thread_id: thread.thread_id,
        from: sender.identity.node_id,
        to: recipient.identity.node_id,
        sequence: 1,
        created_unix_ms: 1_001,
        message_kind: CHAT_MESSAGE_KIND_TEXT,
        payload_hash: chat_payload_hash(plaintext),
        payload_len: plaintext.len() as u64,
        sealed_payload_hash: chat_payload_hash(&message.payload),
        storage_ref: Vec::new(),
        previous_message_hash: thread.head_message_hash,
    });
    append_thread_message(&mut thread, &message_object).expect("append message");

    assert_eq!(thread.message_count, 1);
    assert_eq!(thread.head_message_hash, message_object.message_id);
}
