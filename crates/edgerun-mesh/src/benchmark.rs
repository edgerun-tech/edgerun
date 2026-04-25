/// Network benchmarks for the edgerun mesh.
///
/// These complement the CPU/memory/storage benchmarks in `edgerun-core::benchmark`.
///
/// ## Benchmarks
/// - `benchmark_frame_encode_decode`: MeshFrame serialization throughput (ops/s)
/// - `benchmark_frame_sign_verify`: ECDSA P-256 sign + verify rate (ops/s)
/// - `benchmark_udp_throughput`: Raw UDP send/recv loopback (ops/s)
/// - `benchmark_router_lookup`: Mesh router next-hop lookup rate (ops/s)
///
/// Total runtime: ~1-2 seconds.
use std::time::{Duration, Instant};

use crate::{FrameType, MeshFrame, MeshFrameHeader, NodeID};

// ===========================================================================
// Mesh Frame Encode/Decode Benchmark
// ===========================================================================

pub fn benchmark_frame_encode_decode() -> u64 {
    let src_id = node_id_with_byte(0xAA);
    let dst_id = node_id_with_byte(0xBB);

    let header = MeshFrameHeader {
        dest: dst_id,
        src: src_id,
        ttl: 16,
        frame_type: FrameType::Data,
    };

    let payload = vec![0xCCu8; 256]; // realistic payload size

    let start = Instant::now();
    let target = Duration::from_millis(100);
    let mut ops: u64 = 0;

    while start.elapsed() < target {
        let frame = MeshFrame {
            header,
            payload: payload.clone(),
            signature: [0u8; 64],
        };

        // Encode
        let wire = frame.to_wire();
        std::hint::black_box(&wire);

        // Decode
        if MeshFrame::from_wire(&wire).is_some() {
            ops += 1; // One encode+decode roundtrip
        } else {
            break;
        }
    }

    let elapsed_us = start.elapsed().as_micros() as u64;
    if elapsed_us > 0 {
        ops.saturating_mul(1_000_000) / elapsed_us
    } else {
        0
    }
}

// ===========================================================================
// Mesh Frame Sign/Verify Benchmark
// ===========================================================================

pub fn benchmark_frame_sign_verify() -> u64 {
    use edgerun_crypto::p256::ecdsa::SigningKey;

    // Generate a real signing key using getrandom
    let mut key_bytes = [0u8; 32];
    edgerun_crypto::getrandom::fill(&mut key_bytes).expect("getrandom failed");
    let signing_key = SigningKey::from_bytes((&key_bytes).into()).expect("invalid key bytes");
    let src_id = node_id_from_signing_key(&signing_key);

    let dst_id = node_id_with_byte(0xBB);
    let header = MeshFrameHeader {
        dest: dst_id,
        src: src_id,
        ttl: 16,
        frame_type: FrameType::Data,
    };

    let payload = vec![0xCCu8; 256];

    let start = Instant::now();
    let target = Duration::from_millis(200);
    let mut ops: u64 = 0;

    while start.elapsed() < target {
        let mut frame = MeshFrame {
            header,
            payload: payload.clone(),
            signature: [0u8; 64],
        };

        // Sign
        crate::sign_frame(&mut frame, &signing_key);

        // Verify
        if frame.verify_signature() {
            ops += 1;
        } else {
            break;
        }
    }

    let elapsed_us = start.elapsed().as_micros() as u64;
    if elapsed_us > 0 {
        ops.saturating_mul(1_000_000) / elapsed_us
    } else {
        0
    }
}

// ===========================================================================
// UDP Throughput Benchmark
//
// Measures raw UDP send/recv on loopback (127.0.0.1).
// ===========================================================================

pub fn benchmark_udp_throughput() -> u64 {
    use std::net::UdpSocket;

    let sender = match UdpSocket::bind("127.0.0.1:0") {
        Ok(s) => s,
        Err(_) => return 0,
    };
    let receiver = match UdpSocket::bind("127.0.0.1:0") {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let sender_addr = sender.local_addr().unwrap();
    let receiver_addr = receiver.local_addr().unwrap();

    let _ = sender.connect(receiver_addr);
    let _ = receiver.connect(sender_addr);

    let msg = [0xDDu8; 512]; // typical mesh frame size
    let mut recv_buf = [0u8; 8192];

    let _ = sender.set_nonblocking(true);
    let _ = receiver.set_nonblocking(true);

    let start = Instant::now();
    let target = Duration::from_millis(200);
    let mut ops: u64 = 0;
    let mut pending = 0u64;

    while start.elapsed() < target {
        // Send burst
        while pending < 64 {
            match sender.send(&msg) {
                Ok(_) => pending += 1,
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }

        // Drain
        while pending > 0 {
            match receiver.recv(&mut recv_buf) {
                Ok(_) => {
                    ops += 1;
                    pending -= 1;
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }

        if pending == 0 {
            std::thread::yield_now();
        }
    }

    // Drain remaining
    receiver.set_nonblocking(false).ok();
    receiver
        .set_read_timeout(Some(Duration::from_millis(10)))
        .ok();
    while pending > 0 {
        match receiver.recv(&mut recv_buf) {
            Ok(_) => {
                ops += 1;
                pending -= 1;
            }
            Err(_) => break,
        }
    }

    let elapsed_us = start.elapsed().as_micros() as u64;
    if elapsed_us > 0 {
        ops.saturating_mul(1_000_000) / elapsed_us
    } else {
        0
    }
}

// ===========================================================================
// Helpers
// ===========================================================================

fn node_id_with_byte(b: u8) -> NodeID {
    let mut bytes = [0u8; 64];
    bytes[0] = b;
    NodeID(bytes)
}

#[allow(dead_code)]
fn node_id_with_pattern(v: u8) -> NodeID {
    let mut bytes = [0u8; 64];
    for b in bytes.iter_mut() {
        *b = v;
    }
    NodeID(bytes)
}

fn node_id_from_signing_key(key: &edgerun_crypto::p256::ecdsa::SigningKey) -> NodeID {
    let encoded = key.verifying_key().to_encoded_point(false);
    let bytes = encoded.as_bytes();
    let mut node_bytes = [0u8; 64];
    node_bytes.copy_from_slice(&bytes[1..65]);
    NodeID(node_bytes)
}

// ===========================================================================
// Run mesh benchmarks (no router — that's in edgerun-mesh-router)
// ===========================================================================

/// Run mesh network benchmarks (encode/decode, sign/verify, UDP).
/// Format: (frame_encode_decode, frame_sign_verify, udp_throughput)
pub fn run_mesh_benchmarks() -> (u64, u64, u64) {
    let encode_decode = benchmark_frame_encode_decode();
    let sign_verify = benchmark_frame_sign_verify();
    let udp = benchmark_udp_throughput();
    (encode_decode, sign_verify, udp)
}

/// Human-readable output for mesh benchmarks.
pub fn print_mesh_results(encode_decode: u64, sign_verify: u64, udp: u64) {
    println!("=== edgerun Mesh Benchmarks ===");
    println!("  Frame Encode/Decode: {} ops/s", encode_decode);
    println!("  Frame Sign/Verify:   {} ops/s", sign_verify);
    println!("  UDP Throughput:      {} ops/s", udp);
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_encode_decode_fast() {
        let ops = benchmark_frame_encode_decode();
        assert!(ops > 0, "frame encode/decode was 0");
    }

    #[test]
    fn frame_sign_verify_fast() {
        let ops = benchmark_frame_sign_verify();
        assert!(ops > 0, "frame sign/verify was 0");
    }

    #[test]
    fn udp_throughput_fast() {
        let ops = benchmark_udp_throughput();
        assert!(ops > 0, "UDP throughput was 0");
    }

    #[test]
    fn run_mesh_produces_results() {
        let (enc, sig, udp) = run_mesh_benchmarks();
        assert!(enc > 0);
        assert!(sig > 0);
        assert!(udp > 0);
    }
}
