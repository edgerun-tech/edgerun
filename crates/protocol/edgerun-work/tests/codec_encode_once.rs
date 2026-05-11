use edgerun_work::{
    blake3_hash, encode_work_packet_once, packet_bytes, packet_hash, SimNode, DEPARTMENT_MESSAGE,
    NODE_ROLE_MESSAGE, WORK_TYPE_MESSAGE_DELIVER,
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

    assert_eq!(encoded.bytes, legacy_bytes);
    assert_eq!(encoded.hash, legacy_hash);
    assert_eq!(packet_hash(&packet).expect("packet hash"), legacy_hash);
}
