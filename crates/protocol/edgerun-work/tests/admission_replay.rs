use std::net::{TcpListener, TcpStream};
use std::thread::{self, JoinHandle};

use edgerun_crypto::Ed25519SigningKey;
use edgerun_work::*;

fn signed_request(key: &Ed25519SigningKey, user_sequence: u64, request_id: Hash) -> WorkRequest {
    let mut user = [0u8; 32];
    user.copy_from_slice(key.verifying_key().as_bytes());
    sign_work_request(
        key,
        WorkRequest {
            abi_version: WORK_WIRE_ABI_VERSION,
            request_id,
            user,
            user_sequence,
            recipient: [9u8; 32],
            work_type: WORK_TYPE_COMPUTE_RUN,
            department: DEPARTMENT_COMPUTE,
            payload_hash: [1u8; 32],
            input_root: [2u8; 32],
            max_total_cost: 7,
            valid_until_unix_ms: u64::MAX,
            signature: empty_signature(),
        },
    )
}

fn controller() -> InMemoryAdmissionController {
    InMemoryAdmissionController::new(
        Ed25519SigningKey::from_bytes(&[40u8; 32]),
        AdmissionConfig {
            dao_id: [41u8; 32],
            policy_hash: [42u8; 32],
            heartbeat_grace_secs: 1,
        },
    )
}

fn user_public_key(key: &Ed25519SigningKey) -> PublicKey {
    let mut user = [0u8; 32];
    user.copy_from_slice(key.verifying_key().as_bytes());
    user
}

fn signed_relay_available(key: &Ed25519SigningKey) -> NodeAvailable {
    sign_node_available(
        key,
        NodeAvailable {
            abi_version: WORK_WIRE_ABI_VERSION,
            node: node_identity_from_key(key, NODE_ROLE_RELAY),
            sequence: 1,
            unix_ms: unix_ms(),
            listen_host: "127.0.0.1".to_owned(),
            listen_port: 37001,
            heartbeat_secs: DEFAULT_HEARTBEAT_SECS,
            log_head: [0u8; 32],
            signature: empty_signature(),
        },
    )
}

fn admit_relay(controller: &InMemoryAdmissionController, seed: u8) -> (TcpStream, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind admission test listener");
    let addr = listener.local_addr().expect("admission test listener addr");
    let controller = controller.clone();
    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept admission test stream");
        controller.handle_connection(stream).expect("handle relay admission connection");
    });

    let mut client = TcpStream::connect(addr).expect("connect relay admission client");
    let relay_key = Ed25519SigningKey::from_bytes(&[seed; 32]);
    write_work_packet(
        &mut client,
        &WorkPacket::NodeAvailable(signed_relay_available(&relay_key)),
    )
    .expect("write relay availability");
    match read_work_packet(&mut client).expect("read relay admission response") {
        WorkPacket::RelayPeerList(list) => assert_eq!(list.relays.len(), 1),
        other => panic!("expected relay peer list, got {other:?}"),
    }
    (client, server)
}

fn assert_ack(packet: WorkPacket, code: u16, text: &str) {
    let WorkPacket::Ack(ack) = packet else {
        panic!("expected ack, got {packet:?}");
    };
    assert!(!ack.ok);
    assert_eq!(ack.code, code);
    assert_eq!(ack.text, text);
}

fn assert_admission(packet: WorkPacket) {
    let WorkPacket::WorkAdmission(admission) = packet else {
        panic!("expected work admission, got {packet:?}");
    };
    assert!(verify_work_admission(&admission));
}

#[test]
fn admission_replay_state_rejects_duplicate_request_id() {
    let key = Ed25519SigningKey::from_bytes(&[31u8; 32]);
    let first = signed_request(&key, 1, [3u8; 32]);
    let duplicate = signed_request(&key, 2, first.request_id);
    let mut replay = AdmissionReplayState::new();

    replay.record_checked(&first).expect("first request accepted");

    assert_eq!(
        replay.record_checked(&duplicate),
        Err(AdmissionReplayError::DuplicateRequestId)
    );
    assert_eq!(replay.seen_request_count(), 1);
    assert_eq!(replay.highest_sequence_for(&first.user), Some(1));
}

#[test]
fn admission_replay_state_rejects_non_increasing_user_sequence() {
    let key = Ed25519SigningKey::from_bytes(&[32u8; 32]);
    let first = signed_request(&key, 10, [10u8; 32]);
    let stale = signed_request(&key, 10, [11u8; 32]);
    let older = signed_request(&key, 9, [12u8; 32]);
    let fresh = signed_request(&key, 11, [13u8; 32]);
    let mut replay = AdmissionReplayState::new();

    replay.record_checked(&first).expect("first sequence accepted");

    assert_eq!(
        replay.record_checked(&stale),
        Err(AdmissionReplayError::NonIncreasingUserSequence {
            last_seen: 10,
            received: 10,
        })
    );
    assert_eq!(
        replay.record_checked(&older),
        Err(AdmissionReplayError::NonIncreasingUserSequence {
            last_seen: 10,
            received: 9,
        })
    );
    replay.record_checked(&fresh).expect("fresh sequence accepted");

    assert_eq!(replay.seen_request_count(), 2);
    assert_eq!(replay.highest_sequence_for(&first.user), Some(11));
}

#[test]
fn admission_replay_state_does_not_mutate_on_failed_validate() {
    let key = Ed25519SigningKey::from_bytes(&[33u8; 32]);
    let first = signed_request(&key, 5, [20u8; 32]);
    let stale = signed_request(&key, 4, [21u8; 32]);
    let mut replay = AdmissionReplayState::new();

    replay.record_checked(&first).expect("first sequence accepted");
    assert!(replay.validate(&stale).is_err());

    assert_eq!(replay.seen_request_count(), 1);
    assert_eq!(replay.highest_sequence_for(&first.user), Some(5));
}

#[test]
fn admission_controller_rejects_duplicate_request_id_after_admission() {
    let controller = controller();
    let (relay_client, relay_thread) = admit_relay(&controller, 43);
    let user_key = Ed25519SigningKey::from_bytes(&[44u8; 32]);
    controller.set_user_balance(user_public_key(&user_key), 100);

    let first = signed_request(&user_key, 1, [30u8; 32]);
    let duplicate = signed_request(&user_key, 2, first.request_id);

    assert_admission(
        controller
            .handle_packet(WorkPacket::WorkRequest(first), None)
            .expect("first request handled"),
    );
    assert_ack(
        controller
            .handle_packet(WorkPacket::WorkRequest(duplicate), None)
            .expect("duplicate request handled"),
        409,
        "duplicate request id",
    );

    drop(relay_client);
    relay_thread.join().expect("relay admission thread joined");
}

#[test]
fn admission_controller_rejects_stale_user_sequence_after_admission() {
    let controller = controller();
    let (relay_client, relay_thread) = admit_relay(&controller, 45);
    let user_key = Ed25519SigningKey::from_bytes(&[46u8; 32]);
    controller.set_user_balance(user_public_key(&user_key), 100);

    assert_admission(
        controller
            .handle_packet(WorkPacket::WorkRequest(signed_request(&user_key, 10, [31u8; 32])), None)
            .expect("first request handled"),
    );
    assert_ack(
        controller
            .handle_packet(WorkPacket::WorkRequest(signed_request(&user_key, 10, [32u8; 32])), None)
            .expect("equal sequence request handled"),
        409,
        "stale user request sequence",
    );
    assert_ack(
        controller
            .handle_packet(WorkPacket::WorkRequest(signed_request(&user_key, 9, [33u8; 32])), None)
            .expect("older sequence request handled"),
        409,
        "stale user request sequence",
    );
    assert_admission(
        controller
            .handle_packet(WorkPacket::WorkRequest(signed_request(&user_key, 11, [34u8; 32])), None)
            .expect("fresh request handled"),
    );

    drop(relay_client);
    relay_thread.join().expect("relay admission thread joined");
}

#[test]
fn admission_controller_does_not_consume_replay_state_on_insufficient_balance() {
    let controller = controller();
    let (relay_client, relay_thread) = admit_relay(&controller, 47);
    let user_key = Ed25519SigningKey::from_bytes(&[48u8; 32]);
    let user = user_public_key(&user_key);
    let request = signed_request(&user_key, 1, [35u8; 32]);

    assert_ack(
        controller
            .handle_packet(WorkPacket::WorkRequest(request.clone()), None)
            .expect("underfunded request handled"),
        402,
        "insufficient work credit",
    );
    controller.set_user_balance(user, 100);
    assert_admission(
        controller
            .handle_packet(WorkPacket::WorkRequest(request), None)
            .expect("funded retry handled"),
    );

    drop(relay_client);
    relay_thread.join().expect("relay admission thread joined");
}

#[test]
fn admission_controller_does_not_consume_replay_state_when_no_relay_available() {
    let controller = controller();
    let user_key = Ed25519SigningKey::from_bytes(&[49u8; 32]);
    let user = user_public_key(&user_key);
    let request = signed_request(&user_key, 1, [36u8; 32]);
    controller.set_user_balance(user, 100);

    assert_ack(
        controller
            .handle_packet(WorkPacket::WorkRequest(request.clone()), None)
            .expect("no relay request handled"),
        503,
        "no relay available",
    );

    let (relay_client, relay_thread) = admit_relay(&controller, 50);
    assert_admission(
        controller
            .handle_packet(WorkPacket::WorkRequest(request), None)
            .expect("relay retry handled"),
    );

    drop(relay_client);
    relay_thread.join().expect("relay admission thread joined");
}
