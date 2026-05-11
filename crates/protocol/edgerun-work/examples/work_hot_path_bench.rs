use std::hint::black_box;
use std::time::Instant;

use edgerun_work::{
    blake3_hash, empty_signature, encode_work_packet_once, packet_bytes, packet_hash,
    sign_network_message, verify_network_message, NetworkMessage, SimNode, WorkPacket,
    DEPARTMENT_MESSAGE, NODE_ROLE_MESSAGE, WORK_TYPE_MESSAGE_DELIVER, WORK_WIRE_ABI_VERSION,
};

fn bench(name: &str, payload_bytes: usize, iters: usize, mut f: impl FnMut()) {
    let start = Instant::now();
    for _ in 0..iters {
        f();
    }
    let elapsed = start.elapsed();
    let ns_per_op = elapsed.as_nanos() / iters as u128;
    let ops_per_sec = if ns_per_op == 0 {
        0
    } else {
        1_000_000_000u128 / ns_per_op
    };
    println!("{name},{payload_bytes},{iters},{ns_per_op},{ops_per_sec}");
}

fn unsigned_message_template(sender: &SimNode, receiver: &SimNode, payload: Vec<u8>) -> NetworkMessage {
    let payload_hash = blake3_hash(&payload);
    let mut id_input = Vec::new();
    id_input.extend_from_slice(&sender.identity.node_id);
    id_input.extend_from_slice(&receiver.identity.node_id);
    id_input.extend_from_slice(&1u64.to_be_bytes());
    id_input.extend_from_slice(&payload_hash);

    NetworkMessage {
        abi_version: WORK_WIRE_ABI_VERSION,
        message_id: blake3_hash(&id_input),
        prev_hash: [0u8; 32],
        from: sender.identity.node_id,
        to: receiver.identity.node_id,
        via_relay: receiver.identity.node_id,
        department: DEPARTMENT_MESSAGE,
        work_type: WORK_TYPE_MESSAGE_DELIVER,
        sequence: 1,
        payload_hash,
        payload,
        signature: empty_signature(),
    }
}

fn main() {
    let payload_1k = vec![7u8; 1024];
    let payload_16k = vec![9u8; 16 * 1024];

    let mut sender = SimNode::from_seed(1, NODE_ROLE_MESSAGE);
    let receiver = SimNode::from_seed(2, NODE_ROLE_MESSAGE);
    let packet_1k = sender.message_to(
        receiver.identity.node_id,
        receiver.identity.node_id,
        DEPARTMENT_MESSAGE,
        WORK_TYPE_MESSAGE_DELIVER,
        payload_1k.clone(),
    );
    let signed_1k = match &packet_1k {
        WorkPacket::NetworkMessage(message) => message.clone(),
        _ => unreachable!("message_to always returns WorkPacket::NetworkMessage"),
    };
    let unsigned_1k = unsigned_message_template(&sender, &receiver, payload_1k.clone());

    println!("metric,payload_bytes,iters,ns_per_op,ops_per_sec");

    bench("hash_1k", payload_1k.len(), 200_000, || {
        black_box(blake3_hash(black_box(&payload_1k)));
    });

    bench("hash_16k", payload_16k.len(), 50_000, || {
        black_box(blake3_hash(black_box(&payload_16k)));
    });

    bench("packet_bytes_1k", payload_1k.len(), 50_000, || {
        black_box(packet_bytes(black_box(&packet_1k)).expect("packet encodes"));
    });

    bench("encode_work_packet_once_1k", payload_1k.len(), 50_000, || {
        black_box(encode_work_packet_once(black_box(&packet_1k)).expect("packet encodes"));
    });

    bench("packet_hash_1k", payload_1k.len(), 50_000, || {
        black_box(packet_hash(black_box(&packet_1k)).expect("packet hashes"));
    });

    bench("sign_network_message_1k", payload_1k.len(), 10_000, || {
        black_box(sign_network_message(
            black_box(&sender.key),
            black_box(unsigned_1k.clone()),
        ));
    });

    bench("verify_network_message_1k", payload_1k.len(), 10_000, || {
        black_box(verify_network_message(
            black_box(&signed_1k),
            black_box(&sender.identity),
        ));
    });
}
