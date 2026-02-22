use std::collections::HashMap;
use std::time::Duration;

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use serde::{Deserialize, Serialize};

const PTY_FRAME_STDIN: u8 = 1;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ShellRequest {
    Auth {
        token: String,
    },
    Spawn {
        id: Option<u32>,
        cmd: Option<String>,
        args: Option<Vec<String>>,
        cwd: Option<String>,
        env: Option<HashMap<String, String>>,
        cols: Option<u16>,
        rows: Option<u16>,
    },
    Resize {
        id: u32,
        cols: u16,
        rows: u16,
    },
    Close {
        id: u32,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ShellResponse {
    Spawned { id: u32, pid: Option<u32> },
    Error { id: Option<u32>, error: String },
}

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

fn bench_shell_request_json(c: &mut Criterion) {
    let mut group = c.benchmark_group("shell_request_json");
    let auth = serde_json::to_string(&ShellRequest::Auth {
        token: "tok_sample_123".to_string(),
    })
    .expect("serialize auth");
    let resize = serde_json::to_string(&ShellRequest::Resize {
        id: 7,
        cols: 160,
        rows: 52,
    })
    .expect("serialize resize");
    let mut env = HashMap::new();
    env.insert("TERM".to_string(), "xterm-256color".to_string());
    env.insert("COLORTERM".to_string(), "truecolor".to_string());
    let spawn = serde_json::to_string(&ShellRequest::Spawn {
        id: Some(7),
        cmd: Some("/bin/bash".to_string()),
        args: Some(vec!["-lc".to_string(), "echo benchmark".to_string()]),
        cwd: Some("/tmp".to_string()),
        env: Some(env),
        cols: Some(160),
        rows: Some(52),
    })
    .expect("serialize spawn");

    for (name, payload) in [("auth", auth), ("resize", resize), ("spawn", spawn)] {
        group.throughput(Throughput::Bytes(payload.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("deserialize", name),
            &payload,
            |b, payload| {
                b.iter(|| {
                    let req: ShellRequest =
                        serde_json::from_str(black_box(payload)).expect("valid json");
                    black_box(req);
                });
            },
        );
    }
    group.finish();
}

fn bench_shell_response_json(c: &mut Criterion) {
    let mut group = c.benchmark_group("shell_response_json");
    let spawned = ShellResponse::Spawned {
        id: 9,
        pid: Some(4242),
    };
    let error = ShellResponse::Error {
        id: Some(9),
        error: "session id already exists".to_string(),
    };

    for (name, response) in [("spawned", spawned), ("error", error)] {
        group.bench_with_input(
            BenchmarkId::new("serialize", name),
            &response,
            |b, response| {
                b.iter(|| {
                    let text = serde_json::to_string(black_box(response)).expect("serialize");
                    black_box(text);
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
        bench_mux_frame_decode_with_payload_touch,
        bench_shell_request_json,
        bench_shell_response_json
}
criterion_main!(benches);
