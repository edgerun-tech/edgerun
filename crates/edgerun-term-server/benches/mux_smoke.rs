// SPDX-License-Identifier: Apache-2.0
use std::time::Duration;

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};

const PTY_FRAME_STDIN: u8 = 1;

fn encode_pty_frame(kind: u8, id: u32, payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(5 + payload.len());
    frame.push(kind);
    frame.extend_from_slice(&id.to_be_bytes());
    frame.extend_from_slice(payload);
    frame
}

fn decode_frame_header(frame: &[u8]) -> Option<(u8, u32, &[u8])> {
    if frame.len() < 5 {
        return None;
    }
    let kind = frame[0];
    let id = u32::from_be_bytes([frame[1], frame[2], frame[3], frame[4]]);
    Some((kind, id, &frame[5..]))
}

fn bench_mux_4k_roundtrip(c: &mut Criterion) {
    let payload = vec![b'x'; 4096];
    let frame = encode_pty_frame(PTY_FRAME_STDIN, 42, &payload);
    let mut group = c.benchmark_group("mux_smoke");
    group.throughput(Throughput::Bytes(payload.len() as u64));

    group.bench_function("frame_encode_4k", |b| {
        b.iter(|| {
            let encoded = encode_pty_frame(PTY_FRAME_STDIN, 42, black_box(&payload));
            black_box(encoded);
        })
    });

    group.bench_function("frame_decode_touch_4k", |b| {
        b.iter(|| {
            let (_, id, bytes) = decode_frame_header(black_box(&frame)).expect("valid frame");
            let checksum = bytes
                .iter()
                .fold(id as u64, |acc, byte| acc.wrapping_add(*byte as u64));
            black_box(checksum);
        })
    });
    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(10)
        .warm_up_time(Duration::from_millis(150))
        .measurement_time(Duration::from_millis(250));
    targets = bench_mux_4k_roundtrip
}
criterion_main!(benches);
