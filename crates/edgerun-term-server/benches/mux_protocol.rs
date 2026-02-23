// SPDX-License-Identifier: Apache-2.0
use std::time::Duration;

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};

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

fn bench_mux_frame_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("mux_frame_encode");
    for size in [64usize, 512, 4096, 16384] {
        let payload = vec![b'x'; size];
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::new("encode", size), &payload, |b, payload| {
            b.iter(|| {
                let frame = encode_pty_frame(PTY_FRAME_STDIN, 42, black_box(payload));
                black_box(frame);
            });
        });
    }
    group.finish();
}

fn bench_mux_frame_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("mux_frame_decode");
    for size in [64usize, 512, 4096, 16384] {
        let frame = encode_pty_frame(PTY_FRAME_STDIN, 42, &vec![b'x'; size]);
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::new("decode", size), &frame, |b, frame| {
            b.iter(|| {
                let parsed = decode_frame_header(black_box(frame)).expect("valid frame");
                black_box(parsed);
            });
        });
    }
    group.finish();
}

fn bench_mux_frame_decode_with_payload_touch(c: &mut Criterion) {
    let mut group = c.benchmark_group("mux_frame_decode_payload_touch");
    for size in [64usize, 512, 4096, 16384] {
        let frame = encode_pty_frame(PTY_FRAME_STDIN, 42, &vec![b'x'; size]);
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(
            BenchmarkId::new("decode_checksum", size),
            &frame,
            |b, frame| {
                b.iter(|| {
                    let (_, id, payload) =
                        decode_frame_header(black_box(frame)).expect("valid frame");
                    let checksum = payload
                        .iter()
                        .fold(id as u64, |acc, byte| acc.wrapping_add(*byte as u64));
                    black_box(checksum);
                });
            },
        );
    }
    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(20)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_millis(800));
    targets =
        bench_mux_frame_encode,
        bench_mux_frame_decode,
        bench_mux_frame_decode_with_payload_touch
}
criterion_main!(benches);
