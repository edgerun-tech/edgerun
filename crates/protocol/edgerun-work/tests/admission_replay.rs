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
