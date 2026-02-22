// SPDX-License-Identifier: Apache-2.0
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::{Duration, Instant};

use edgerun_storage::event::{ActorId, Event, StreamId};
use edgerun_storage::replication::{close_pooled_ack_connections, NodeInfo};
use edgerun_storage::StorageEngine;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Single,
    Batch,
}

#[derive(Clone, Copy, Debug)]
struct Config {
    events: usize,
    batch_size: usize,
    payload_size: usize,
    timeout_ms: u64,
    mode: Mode,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            events: 20_000,
            batch_size: 128,
            payload_size: 512,
            timeout_ms: 250,
            mode: Mode::Batch,
        }
    }
}

fn parse_args() -> Config {
    let mut cfg = Config::default();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--events" => {
                if let Some(v) = args.next() {
                    cfg.events = v.parse().unwrap_or(cfg.events);
                }
            }
            "--batch-size" => {
                if let Some(v) = args.next() {
                    cfg.batch_size = v.parse().unwrap_or(cfg.batch_size);
                }
            }
            "--payload-size" => {
                if let Some(v) = args.next() {
                    cfg.payload_size = v.parse().unwrap_or(cfg.payload_size);
                }
            }
            "--timeout-ms" => {
                if let Some(v) = args.next() {
                    cfg.timeout_ms = v.parse().unwrap_or(cfg.timeout_ms);
                }
            }
            "--mode" => {
                if let Some(v) = args.next() {
                    cfg.mode = match v.as_str() {
                        "single" => Mode::Single,
                        "batch" => Mode::Batch,
                        _ => cfg.mode,
                    };
                }
            }
            "--help" | "-h" => {
                println!(
                    "Usage: replication_group_commit_benchmark [--mode single|batch] [--events N] [--batch-size N] [--payload-size N] [--timeout-ms N]"
                );
                std::process::exit(0);
            }
            _ => {}
        }
    }
    cfg.events = cfg.events.max(1);
    cfg.batch_size = cfg.batch_size.max(1);
    cfg.payload_size = cfg.payload_size.max(1);
    cfg
}

struct AckServer {
    node: NodeInfo,
    handle: thread::JoinHandle<()>,
}

fn spawn_ack_server() -> AckServer {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ack server");
    let addr = listener.local_addr().expect("local addr");
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        loop {
            let mut magic = [0u8; 4];
            if stream.read_exact(&mut magic).is_err() {
                break;
            }
            match &magic {
                b"ACK?" => {
                    let mut op = [0u8; 32];
                    if stream.read_exact(&mut op).is_err() {
                        break;
                    }
                    if stream.write_all(b"ACK\n").is_err() {
                        break;
                    }
                }
                b"AKB?" => {
                    let mut count = [0u8; 2];
                    if stream.read_exact(&mut count).is_err() {
                        break;
                    }
                    let op_count = u16::from_be_bytes(count) as usize;
                    let mut body = vec![0u8; op_count * 32];
                    if stream.read_exact(&mut body).is_err() {
                        break;
                    }
                    let mut resp = Vec::with_capacity(6 + body.len());
                    resp.extend_from_slice(b"AKB!");
                    resp.extend_from_slice(&(op_count as u16).to_be_bytes());
                    resp.extend_from_slice(&body);
                    if stream.write_all(&resp).is_err() {
                        break;
                    }
                }
                _ => break,
            }
        }
    });

    AckServer {
        node: NodeInfo::new(ActorId::new(), addr.to_string(), [1u8; 16]),
        handle,
    }
}

fn main() {
    let cfg = parse_args();
    let data_dir = std::path::PathBuf::from(format!(
        "/tmp/replication_group_commit_bench_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&data_dir);
    std::fs::create_dir_all(&data_dir).expect("create benchmark dir");

    let engine = StorageEngine::new(data_dir).expect("create storage engine");
    let mut session = engine
        .create_append_session("replication-bench.seg", 256 * 1024 * 1024)
        .expect("create session");
    session.set_replication_timeout(Duration::from_millis(cfg.timeout_ms));
    session.set_replication_batch_size(cfg.batch_size);

    let s1 = spawn_ack_server();
    let s2 = spawn_ack_server();
    session.configure_replica_nodes(vec![s1.node.clone(), s2.node.clone()]);

    let stream = StreamId::new();
    let actor = ActorId::new();
    let start = Instant::now();
    let mut total_bytes = 0usize;

    match cfg.mode {
        Mode::Single => {
            session.set_replication_batch_size(1);
            let mut events = Vec::with_capacity(cfg.events);
            for i in 0..cfg.events {
                let payload = vec![(i % 251) as u8; cfg.payload_size];
                total_bytes += payload.len();
                events.push(Event::new(stream.clone(), actor.clone(), payload));
            }
            let _ = session
                .append_replicated_stream(events, 3)
                .expect("append stream replicated");
        }
        Mode::Batch => {
            let mut events = Vec::with_capacity(cfg.events);
            for i in 0..cfg.events {
                let payload = vec![(i % 251) as u8; cfg.payload_size];
                total_bytes += payload.len();
                events.push(Event::new(stream.clone(), actor.clone(), payload));
            }
            let _ = session
                .append_replicated_stream(events, 3)
                .expect("append stream replicated");
        }
    }

    close_pooled_ack_connections();
    drop(session);
    s1.handle.join().expect("join s1");
    s2.handle.join().expect("join s2");

    let elapsed = start.elapsed().as_secs_f64().max(1e-9);
    println!("=== Replication Group Commit Benchmark ===");
    println!("mode: {:?}", cfg.mode);
    println!("events: {}", cfg.events);
    println!("batch_size: {}", cfg.batch_size);
    println!("payload_size: {}", cfg.payload_size);
    println!("duration_s: {elapsed:.3}");
    println!("events_per_sec: {:.0}", cfg.events as f64 / elapsed);
    println!(
        "throughput_mb_s: {:.1}",
        (total_bytes as f64 / 1024.0 / 1024.0) / elapsed
    );
}
