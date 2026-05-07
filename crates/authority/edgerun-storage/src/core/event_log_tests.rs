use super::*;
use edgerun_protocols::core_protocol::protocol::{EventType, Signature};

fn event_with_signature(marker: u8) -> EventEnvelope {
    EventEnvelope {
        envelope_version: 1,
        stream_id: b"stream-1".to_vec(),
        seq: 0,
        prev_event_hash: None,
        event_type: EventType::NodeGenesis as i32,
        event_version: 1,
        recorded_at: None,
        effective_at: None,
        payload_object: None,
        related_events: vec![],
        related_commands: vec![],
        related_objects: vec![],
        related_delegations: vec![],
        related_revocations: vec![],
        event_metadata: None,
        signature: Some(Signature {
            algorithm: 1,
            value: vec![marker; 64],
        }),
    }
}

#[test]
fn canonical_event_hash_matches_stream_hash() {
    let event = event_with_signature(0xAB);

    assert_eq!(
        canonical_event_hash(&event),
        edgerun_stream::compute_event_hash(&event)
    );
}

#[test]
fn canonical_event_hash_uses_signable_form() {
    let first = event_with_signature(0xAB);
    let second = event_with_signature(0xCD);

    assert_eq!(canonical_event_hash(&first), canonical_event_hash(&second));
}
