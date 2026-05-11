use edgerun_work::{
    archived_packet_frame_from_bytes, blake3_hash, encode_work_packet_once, packet_bytes,
    packet_hash, SimNode, WorkPacket, DEPARTMENT_MESSAGE, NODE_ROLE_MESSAGE,
    WORK_TYPE_MESSAGE_DELIVER,
};

#[test]
fn encode_work_packet_once_matches_legacy_bytes_and_hash() {
    let mut sender = SimNode::from_seed(1, NODE_ROLE_MESSAGE);
    let receiver = SimNode::from_seed(2, NODE_ROLE_MESSAGE);
    let packet = sender.message_to(
        receiver.identity.node_id,
        receiver.identity.node_id,
        DEPARTMENT_MESSAGE,
        WORK_TYPE_MESSAGE_DELIVER,
        b"encode once regression".to_vec(),
    );

    let legacy_bytes = packet_bytes(&packet).expect("legacy packet bytes");
    let legacy_hash = blake3_hash(&legacy_bytes);
    let encoded = encode_work_packet_once(&packet).expect("encoded packet");

    assert_eq!(encoded.as_bytes(), legacy_bytes.as_slice());
    assert_eq!(encoded.hash, legacy_hash);
    assert_eq!(packet_hash(&packet).expect("packet hash"), legacy_hash);
}

#[test]
fn archived_packet_frame_validates_and_roundtrips_without_rehashing_owned_packet() {
    let mut sender = SimNode::from_seed(1, NODE_ROLE_MESSAGE);
    let receiver = SimNode::from_seed(2, NODE_ROLE_MESSAGE);
    let packet = sender.message_to(
        receiver.identity.node_id,
        receiver.identity.node_id,
        DEPARTMENT_MESSAGE,
        WORK_TYPE_MESSAGE_DELIVER,
        b"archived frame regression".to_vec(),
    );
    let encoded = encode_work_packet_once(&packet).expect("encoded packet");

    let frame = archived_packet_frame_from_bytes(encoded.as_bytes()).expect("archived frame");
    assert_eq!(frame.hash, encoded.hash);
    assert_eq!(frame.as_bytes(), encoded.as_bytes());
    frame.archived().expect("valid rkyv archive");

    let decoded = frame.into_packet().expect("owned packet only when requested");
    assert_eq!(decoded, packet);
    assert!(matches!(decoded, WorkPacket::NetworkMessage(_)));
}
