// SPDX-License-Identifier: GPL-2.0-only
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use storage_engine::durability::DurabilityLevel;
use storage_engine::event::{ActorId, Event, StreamId};
use storage_engine::replication::{
    build_authenticated_ack_response, close_pooled_ack_connections, derive_auth_key,
    verify_authenticated_ack_request, NodeInfo,
};
use storage_engine::{StorageEngine, StorageError};
use tempfile::TempDir;

const ACK_RESPONSE_OK: &[u8; 4] = b"ACK\n";

struct AckServer {
    node: NodeInfo,
    observed_op_ids: Arc<Mutex<Vec<[u8; 32]>>>,
    observed_batch_sizes: Arc<Mutex<Vec<usize>>>,
    handle: thread::JoinHandle<()>,
}

fn spawn_ack_server(response_delay: Duration) -> AckServer {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ack server");
    let addr = listener.local_addr().expect("local addr");
    let observed_op_ids = Arc::new(Mutex::new(Vec::new()));
    let observed_clone = Arc::clone(&observed_op_ids);
    let observed_batch_sizes = Arc::new(Mutex::new(Vec::new()));
    let batch_clone = Arc::clone(&observed_batch_sizes);

    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        loop {
            let mut magic = [0u8; 4];
            if stream.read_exact(&mut magic).is_err() {
                break;
            }
            match &magic {
                b"ACK?" => {
                    let mut op = [0u8; 32];
                    stream.read_exact(&mut op).expect("read op");
                    observed_clone.lock().unwrap().push(op);
                    batch_clone.lock().unwrap().push(1);
                    if !response_delay.is_zero() {
                        thread::sleep(response_delay);
                    }
                    stream.write_all(ACK_RESPONSE_OK).expect("write ack");
                }
                b"AKB?" => {
                    let mut count = [0u8; 2];
                    stream.read_exact(&mut count).expect("read count");
                    let op_count = u16::from_be_bytes(count) as usize;
                    let mut body = vec![0u8; op_count * 32];
                    stream.read_exact(&mut body).expect("read body");
                    let mut guard = observed_clone.lock().unwrap();
                    for chunk in body.chunks_exact(32) {
                        let mut op = [0u8; 32];
                        op.copy_from_slice(chunk);
                        guard.push(op);
                    }
                    drop(guard);
                    batch_clone.lock().unwrap().push(op_count);
                    if !response_delay.is_zero() {
                        thread::sleep(response_delay);
                    }
                    let mut resp = Vec::with_capacity(6 + body.len());
                    resp.extend_from_slice(b"AKB!");
                    resp.extend_from_slice(&(op_count as u16).to_be_bytes());
                    resp.extend_from_slice(&body);
                    stream.write_all(&resp).expect("write batch ack");
                }
                _ => break,
            }
        }
    });

    AckServer {
        node: NodeInfo::new(ActorId::new(), addr.to_string(), [1u8; 16]),
        observed_op_ids,
        observed_batch_sizes,
        handle,
    }
}

fn spawn_authenticated_ack_server(
    response_delay: Duration,
    store_uuid: [u8; 16],
    auth_key: [u8; 32],
) -> AckServer {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ack server");
    let addr = listener.local_addr().expect("local addr");
    let observed_op_ids = Arc::new(Mutex::new(Vec::new()));
    let observed_clone = Arc::clone(&observed_op_ids);
    let observed_batch_sizes = Arc::new(Mutex::new(Vec::new()));
    let batch_clone = Arc::clone(&observed_batch_sizes);

    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        loop {
            let mut req = [0u8; 84];
            if stream.read_exact(&mut req).is_err() {
                break;
            }
            let op =
                verify_authenticated_ack_request(&req, store_uuid, auth_key).expect("verify req");
            observed_clone.lock().unwrap().push(op);
            batch_clone.lock().unwrap().push(1);
            if !response_delay.is_zero() {
                thread::sleep(response_delay);
            }
            let response = build_authenticated_ack_response(store_uuid, op, auth_key);
            stream.write_all(&response).expect("write ack");
        }
    });

    AckServer {
        node: NodeInfo::new(ActorId::new(), addr.to_string(), store_uuid).with_auth_key(auth_key),
        observed_op_ids,
        observed_batch_sizes,
        handle,
    }
}

#[test]
fn test_replicated_quorum_succeeds_with_partial_network() -> Result<(), StorageError> {
    let temp = TempDir::new().unwrap();
    let engine = StorageEngine::new(temp.path().to_path_buf())?;
    let mut session = engine.create_append_session("replicated-q1.seg", 1024 * 1024)?;
    session.set_replication_timeout(Duration::from_millis(400));

    let s1 = spawn_ack_server(Duration::ZERO);
    let s2 = spawn_ack_server(Duration::ZERO);
    let dead = NodeInfo::new(ActorId::new(), "127.0.0.1:1".to_string(), [2u8; 16]);
    session.configure_replica_nodes(vec![s1.node.clone(), s2.node.clone(), dead]);

    let event = Event::new(
        StreamId::new(),
        ActorId::new(),
        b"phase-d-q-success".to_vec(),
    );
    // Need local + 2 remote ACKs.
    let _ = session.append_with_durability(&event, DurabilityLevel::AckReplicatedN(3))?;
    close_pooled_ack_connections();
    s1.handle.join().unwrap();
    s2.handle.join().unwrap();
    Ok(())
}

#[test]
fn test_replicated_quorum_times_out_under_partition() -> Result<(), StorageError> {
    let temp = TempDir::new().unwrap();
    let engine = StorageEngine::new(temp.path().to_path_buf())?;
    let mut session = engine.create_append_session("replicated-q2.seg", 1024 * 1024)?;
    session.set_replication_timeout(Duration::from_millis(80));

    let slow = spawn_ack_server(Duration::from_millis(250));
    let dead = NodeInfo::new(ActorId::new(), "127.0.0.1:1".to_string(), [3u8; 16]);
    session.configure_replica_nodes(vec![slow.node.clone(), dead]);

    let event = Event::new(
        StreamId::new(),
        ActorId::new(),
        b"phase-d-q-timeout".to_vec(),
    );
    // Need local + 2 remotes, but one is dead and one misses timeout.
    let err = session
        .append_with_durability(&event, DurabilityLevel::AckReplicatedN(3))
        .unwrap_err();
    assert!(matches!(
        err,
        StorageError::Replication(
            storage_engine::replication::ReplicationError::QuorumTimeout { .. }
        )
    ));
    close_pooled_ack_connections();
    slow.handle.join().unwrap();
    Ok(())
}

#[test]
fn test_replicated_quorum_consistent_operation_id_across_replicas() -> Result<(), StorageError> {
    let temp = TempDir::new().unwrap();
    let engine = StorageEngine::new(temp.path().to_path_buf())?;
    let mut session = engine.create_append_session("replicated-q3.seg", 1024 * 1024)?;
    session.set_replication_timeout(Duration::from_millis(400));

    let s1 = spawn_ack_server(Duration::ZERO);
    let s2 = spawn_ack_server(Duration::ZERO);
    session.configure_replica_nodes(vec![s1.node.clone(), s2.node.clone()]);

    let event = Event::new(StreamId::new(), ActorId::new(), b"phase-d-q-opid".to_vec());
    let expected = event.compute_hash();
    let _ = session.append_with_durability(&event, DurabilityLevel::AckReplicatedN(3))?;
    close_pooled_ack_connections();
    s1.handle.join().unwrap();
    s2.handle.join().unwrap();

    let ids1 = s1.observed_op_ids.lock().unwrap().clone();
    let ids2 = s2.observed_op_ids.lock().unwrap().clone();
    assert_eq!(ids1.len(), 1);
    assert_eq!(ids2.len(), 1);
    assert_eq!(ids1[0], expected);
    assert_eq!(ids2[0], expected);
    assert_eq!(ids1[0], ids2[0]);
    Ok(())
}

#[test]
fn test_replicated_quorum_authenticated_peer_identity() -> Result<(), StorageError> {
    let temp = TempDir::new().unwrap();
    let engine = StorageEngine::new(temp.path().to_path_buf())?;
    let mut session = engine.create_append_session("replicated-q4.seg", 1024 * 1024)?;
    session.set_replication_timeout(Duration::from_millis(400));

    let auth_key = derive_auth_key(b"phase-d-integration-shared-key");
    let store_uuid_1 = [0xA1; 16];
    let store_uuid_2 = [0xB2; 16];
    let s1 = spawn_authenticated_ack_server(Duration::ZERO, store_uuid_1, auth_key);
    let s2 = spawn_authenticated_ack_server(Duration::ZERO, store_uuid_2, auth_key);
    session.configure_replica_nodes(vec![s1.node.clone(), s2.node.clone()]);

    let event = Event::new(
        StreamId::new(),
        ActorId::new(),
        b"phase-d-authenticated-ack".to_vec(),
    );
    let _ = session.append_with_durability(&event, DurabilityLevel::AckReplicatedN(3))?;
    close_pooled_ack_connections();
    s1.handle.join().unwrap();
    s2.handle.join().unwrap();
    Ok(())
}

#[test]
fn test_replicated_quorum_group_commit_uses_batch_transport() -> Result<(), StorageError> {
    let temp = TempDir::new().unwrap();
    let engine = StorageEngine::new(temp.path().to_path_buf())?;
    let mut session = engine.create_append_session("replicated-q5.seg", 1024 * 1024)?;
    session.set_replication_timeout(Duration::from_millis(400));

    let s1 = spawn_ack_server(Duration::ZERO);
    let s2 = spawn_ack_server(Duration::ZERO);
    session.configure_replica_nodes(vec![s1.node.clone(), s2.node.clone()]);

    let e1 = Event::new(
        StreamId::new(),
        ActorId::new(),
        b"phase-d-group-commit-1".to_vec(),
    );
    let e2 = Event::new(
        StreamId::new(),
        ActorId::new(),
        b"phase-d-group-commit-2".to_vec(),
    );

    let _ = session.append_batch_with_durability(&[e1, e2], DurabilityLevel::AckReplicatedN(3))?;
    close_pooled_ack_connections();
    s1.handle.join().unwrap();
    s2.handle.join().unwrap();

    let s1_batch_sizes = s1.observed_batch_sizes.lock().unwrap().clone();
    let s2_batch_sizes = s2.observed_batch_sizes.lock().unwrap().clone();
    assert!(s1_batch_sizes.iter().any(|&n| n >= 2));
    assert!(s2_batch_sizes.iter().any(|&n| n >= 2));
    Ok(())
}
