use edgerun_crypto::{Ed25519SigningKey, Signer};
use edgerun_relay::{MAX_PAYLOAD_LEN, encode_packet, sha256_array, submit_preimage, verify_submit};
use edgerun_wire::{
    RELAY_WIRE_ABI_VERSION, RelayIdentity, RelayMessage, RelaySignature, RelaySubmit,
    SIGNATURE_ALGORITHM_ECDSA_P256_SHA256, SIGNATURE_ALGORITHM_ED25519,
};
use std::hint::black_box;
use std::time::{Duration, Instant};

fn main() {
    let iterations = std::env::args()
        .nth(1)
        .and_then(|arg| arg.parse::<usize>().ok())
        .unwrap_or(100_000);
    let payload_len = std::env::args()
        .nth(2)
        .and_then(|arg| arg.parse::<usize>().ok())
        .unwrap_or(256)
        .min(MAX_PAYLOAD_LEN);

    let payload = vec![0xA5; payload_len];
    let (_, to) = ed25519_identity(2);
    let ed25519_submit = submit_ed25519(3, to.clone(), [0x11; 32], &payload);
    let p256_submit = submit_p256(4, to, [0x22; 32], &payload);
    let encoded = encode_packet(&RelayMessage::Submit(ed25519_submit.clone())).expect("encode");

    println!(
        "iterations={iterations} payload_len={payload_len} encoded_submit_len={}",
        encoded.len()
    );
    print_result(
        "encode submit",
        iterations,
        time_loop(iterations, || {
            black_box(
                encode_packet(black_box(&RelayMessage::Submit(ed25519_submit.clone()))).unwrap(),
            );
        }),
    );
    print_result(
        "decode submit",
        iterations,
        time_loop(iterations, || {
            black_box(edgerun_relay::decode_packet(black_box(&encoded)).unwrap());
        }),
    );
    print_result(
        "verify ed25519 submit",
        iterations,
        time_loop(iterations, || {
            black_box(verify_submit(black_box(&ed25519_submit)));
        }),
    );
    print_result(
        "verify p256 submit",
        iterations,
        time_loop(iterations, || {
            black_box(verify_submit(black_box(&p256_submit)));
        }),
    );
}

fn time_loop(iterations: usize, mut f: impl FnMut()) -> Duration {
    let start = Instant::now();
    for _ in 0..iterations {
        f();
    }
    start.elapsed()
}

fn print_result(label: &str, iterations: usize, elapsed: Duration) {
    let seconds = elapsed.as_secs_f64();
    let ops_per_second = iterations as f64 / seconds;
    let ns_per_op = elapsed.as_nanos() as f64 / iterations as f64;
    println!("{label}: {ops_per_second:.0} ops/s, {ns_per_op:.1} ns/op, {elapsed:?}");
}

fn ed25519_identity(seed: u8) -> (Ed25519SigningKey, RelayIdentity) {
    let key = Ed25519SigningKey::from_bytes(&[seed; 32]);
    let identity = RelayIdentity {
        algorithm: SIGNATURE_ALGORITHM_ED25519,
        public_key: key.verifying_key().as_bytes().to_vec(),
    };
    (key, identity)
}

fn sign_ed25519(
    key: &Ed25519SigningKey,
    identity: &RelayIdentity,
    preimage: &[u8],
) -> RelaySignature {
    RelaySignature {
        algorithm: identity.algorithm,
        public_key: identity.public_key.clone(),
        signature: key.sign(preimage).to_bytes().to_vec(),
    }
}

fn submit_ed25519(
    sender_seed: u8,
    to: RelayIdentity,
    message_id: [u8; 32],
    payload: &[u8],
) -> RelaySubmit {
    let (key, from) = ed25519_identity(sender_seed);
    let mut submit = RelaySubmit {
        abi_version: RELAY_WIRE_ABI_VERSION,
        flags: 1,
        message_id,
        from,
        to,
        sequence: 7,
        payload_sha256: sha256_array(payload),
        payload: payload.to_vec(),
        signature: RelaySignature {
            algorithm: 0,
            public_key: Vec::new(),
            signature: Vec::new(),
        },
    };
    submit.signature = sign_ed25519(&key, &submit.from, &submit_preimage(&submit));
    submit
}

fn submit_p256(
    sender_seed: u8,
    to: RelayIdentity,
    message_id: [u8; 32],
    payload: &[u8],
) -> RelaySubmit {
    let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes((&[sender_seed; 32]).into())
        .expect("p256 key");
    let from = RelayIdentity {
        algorithm: SIGNATURE_ALGORITHM_ECDSA_P256_SHA256,
        public_key: edgerun_protocols::keygen::node_id_from_signing_key(&key).to_vec(),
    };
    let mut submit = RelaySubmit {
        abi_version: RELAY_WIRE_ABI_VERSION,
        flags: 1,
        message_id,
        from,
        to,
        sequence: 7,
        payload_sha256: sha256_array(payload),
        payload: payload.to_vec(),
        signature: RelaySignature {
            algorithm: 0,
            public_key: Vec::new(),
            signature: Vec::new(),
        },
    };
    let digest = edgerun_crypto::sha256(&submit_preimage(&submit));
    use edgerun_crypto::p256::ecdsa::signature::hazmat::PrehashSigner as _;
    let signature: edgerun_crypto::p256::ecdsa::Signature =
        key.sign_prehash(&digest).expect("p256 sign");
    submit.signature = RelaySignature {
        algorithm: submit.from.algorithm,
        public_key: submit.from.public_key.clone(),
        signature: signature.to_bytes().to_vec(),
    };
    submit
}
