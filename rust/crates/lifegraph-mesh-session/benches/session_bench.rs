//! Benchmarks for the Lifegraph mesh session encryption.
//!
//! Measures:
//! - ECDH handshake latency
//! - AES-256-GCM encrypt/decrypt throughput
//! - Handshake + full session setup latency

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use lifegraph_mesh_session::{HandshakeInit, SessionManager, SessionError, HANDSHAKE_MSG_SIZE};
use lifegraph_hardware_signing::NodeID;
use rand::rngs::OsRng;
use rand::RngCore;

fn node_id(v: u8) -> NodeID {
    let mut bytes = [0u8; 64];
    bytes[0] = v;
    NodeID(bytes)
}

fn make_session_pair() -> (SessionManager, SessionManager, lifegraph_mesh_session::EphemeralSecret) {
    let alice_id = node_id(0xAA);
    let bob_id = node_id(0xBB);

    let mut alice_mgr = SessionManager::new(alice_id);
    let mut bob_mgr = SessionManager::new(bob_id);

    let (init, alice_secret) = alice_mgr.initiate_handshake(bob_id);
    let (accept, _bob_secret) = bob_mgr.respond_to_handshake(&init).unwrap();
    alice_mgr.complete_handshake_initiator(&accept, &alice_secret).unwrap();

    (alice_mgr, bob_mgr, alice_secret)
}

// ── ECDH Handshake ──

fn bench_ecdh_handshake(c: &mut Criterion) {
    c.bench_function("ecdh_handshake_full_roundtrip", |b| {
        let alice_id = node_id(0xAA);
        let bob_id = node_id(0xBB);
        b.iter(|| {
            let mut alice_mgr = SessionManager::new(alice_id);
            let mut bob_mgr = SessionManager::new(bob_id);

            let (init, alice_secret) = alice_mgr.initiate_handshake(bob_id);
            let (accept, _bob_secret) = bob_mgr.respond_to_handshake(&init).unwrap();
            alice_mgr.complete_handshake_initiator(&accept, &alice_secret).unwrap();

            black_box((alice_mgr.session_count(), bob_mgr.session_count()))
        })
    });

    c.bench_function("ecdh_generate_keypair", |b| {
        b.iter(|| {
            black_box(lifegraph_mesh_session::EphemeralSecret::random(&mut OsRng))
        })
    });

    c.bench_function("ecdh_diffie_hellman", |b| {
        let secret_a = lifegraph_mesh_session::EphemeralSecret::random(&mut OsRng);
        let pub_b = lifegraph_mesh_session::EphemeralSecret::random(&mut OsRng).public_key();
        b.iter(|| {
            black_box(secret_a.diffie_hellman(&pub_b))
        })
    });

    c.bench_function("handshake_init_encode", |b| {
        let mut alice_mgr = SessionManager::new(node_id(0xAA));
        let (init, _secret) = alice_mgr.initiate_handshake(node_id(0xBB));
        b.iter(|| {
            black_box(init.encode())
        })
    });

    c.bench_function("handshake_init_decode", |b| {
        let mut alice_mgr = SessionManager::new(node_id(0xAA));
        let (init, _secret) = alice_mgr.initiate_handshake(node_id(0xBB));
        let encoded = init.encode();
        b.iter(|| {
            black_box(HandshakeInit::decode(&encoded))
        })
    });
}

// ── AES-256-GCM encrypt/decrypt ──

fn bench_encrypt_decrypt(c: &mut Criterion) {
    let alice_id = node_id(0xAA);
    let bob_id = node_id(0xBB);

    let sizes = [
        ("32_bytes", 32),
        ("128_bytes", 128),
        ("256_bytes", 256),
        ("512_bytes", 512),
        ("1024_bytes", 1024),
        ("1500_bytes", 1500),
        ("4096_bytes", 4096),
    ];

    for (name, size) in sizes {
        let plaintext: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();

        c.bench_function(&format!("encrypt_{}", name), |b| {
            b.iter_batched(
                || {
                    let mut alice_mgr = SessionManager::new(alice_id);
                    let mut bob_mgr = SessionManager::new(bob_id);
                    let (init, secret) = alice_mgr.initiate_handshake(bob_id);
                    let (accept, _) = bob_mgr.respond_to_handshake(&init).unwrap();
                    alice_mgr.complete_handshake_initiator(&accept, &secret).unwrap();
                    (alice_mgr, bob_id)
                },
                |(mut alice_mgr, peer)| {
                    black_box(alice_mgr.encrypt_for(peer, &plaintext).unwrap())
                },
                BatchSize::SmallInput,
            )
        });

        c.bench_function(&format!("decrypt_{}", name), |b| {
            b.iter_batched(
                || {
                    let mut alice_mgr = SessionManager::new(alice_id);
                    let mut bob_mgr = SessionManager::new(bob_id);
                    let (init, secret) = alice_mgr.initiate_handshake(bob_id);
                    let (accept, _) = bob_mgr.respond_to_handshake(&init).unwrap();
                    alice_mgr.complete_handshake_initiator(&accept, &secret).unwrap();
                    let ct = alice_mgr.encrypt_for(bob_id, &plaintext).unwrap();
                    (bob_mgr, alice_id, ct)
                },
                |(mut bob_mgr, peer, ct)| {
                    black_box(bob_mgr.decrypt_from(peer, &ct).unwrap())
                },
                BatchSize::SmallInput,
            )
        });
    }
}

fn bench_throughput(c: &mut Criterion) {
    let alice_id = node_id(0xAA);
    let bob_id = node_id(0xBB);

    // Large payload to measure sustained throughput
    let plaintext: Vec<u8> = (0..65536).map(|i| (i % 256) as u8).collect();

    c.bench_function("encrypt_throughput_64KB", |b| {
        b.iter_batched(
            || {
                let mut alice_mgr = SessionManager::new(alice_id);
                let mut bob_mgr = SessionManager::new(bob_id);
                let (init, secret) = alice_mgr.initiate_handshake(bob_id);
                let (accept, _) = bob_mgr.respond_to_handshake(&init).unwrap();
                alice_mgr.complete_handshake_initiator(&accept, &secret).unwrap();
                (alice_mgr, bob_id)
            },
            |(mut alice_mgr, peer)| {
                let ct = alice_mgr.encrypt_for(peer, &plaintext).unwrap();
                black_box(ct.len())
            },
            BatchSize::SmallInput,
        )
    });

    c.bench_function("decrypt_throughput_64KB", |b| {
        b.iter_batched(
            || {
                let mut alice_mgr = SessionManager::new(alice_id);
                let mut bob_mgr = SessionManager::new(bob_id);
                let (init, secret) = alice_mgr.initiate_handshake(bob_id);
                let (accept, _) = bob_mgr.respond_to_handshake(&init).unwrap();
                alice_mgr.complete_handshake_initiator(&accept, &secret).unwrap();
                let ct = alice_mgr.encrypt_for(bob_id, &plaintext).unwrap();
                (bob_mgr, alice_id, ct)
            },
            |(mut bob_mgr, peer, ct)| {
                black_box(bob_mgr.decrypt_from(peer, &ct).unwrap().len())
            },
            BatchSize::SmallInput,
        )
    });
}

// ── Full session lifecycle ──

fn bench_session_lifecycle(c: &mut Criterion) {
    c.bench_function("session_lifecycle_100_frames", |b| {
        let alice_id = node_id(0xAA);
        let bob_id = node_id(0xBB);
        let payload = vec![0xABu8; 256];

        b.iter(|| {
            let mut alice_mgr = SessionManager::new(alice_id);
            let mut bob_mgr = SessionManager::new(bob_id);

            let (init, secret) = alice_mgr.initiate_handshake(bob_id);
            let (accept, _) = bob_mgr.respond_to_handshake(&init).unwrap();
            alice_mgr.complete_handshake_initiator(&accept, &secret).unwrap();

            for _ in 0..100 {
                let ct = alice_mgr.encrypt_for(bob_id, &payload).unwrap();
                let _pt = bob_mgr.decrypt_from(alice_id, &ct).unwrap();
            }

            black_box(())
        })
    });

    c.bench_function("session_lifecycle_10000_frames", |b| {
        let alice_id = node_id(0xAA);
        let bob_id = node_id(0xBB);
        let payload = vec![0xABu8; 64];

        b.iter(|| {
            let mut alice_mgr = SessionManager::new(alice_id);
            let mut bob_mgr = SessionManager::new(bob_id);

            let (init, secret) = alice_mgr.initiate_handshake(bob_id);
            let (accept, _) = bob_mgr.respond_to_handshake(&init).unwrap();
            alice_mgr.complete_handshake_initiator(&accept, &secret).unwrap();

            for _ in 0..10_000 {
                let ct = alice_mgr.encrypt_for(bob_id, &payload).unwrap();
                let _pt = bob_mgr.decrypt_from(alice_id, &ct).unwrap();
            }

            black_box(())
        })
    });
}

criterion_group!(
    benches,
    bench_ecdh_handshake,
    bench_encrypt_decrypt,
    bench_throughput,
    bench_session_lifecycle,
);
criterion_main!(benches);
