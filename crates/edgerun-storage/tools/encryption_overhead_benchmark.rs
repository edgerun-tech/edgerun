// SPDX-License-Identifier: GPL-2.0-only
use std::path::PathBuf;
use std::time::Instant;

use storage_engine::encryption::{EncryptionMode, SegmentEncryptionConfig};
use storage_engine::event::{ActorId, Event, StreamId};
use storage_engine::segment::Segment;

#[derive(Debug, Clone)]
struct Config {
    events: usize,
    payload_size: usize,
    chunk_size: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            events: 100_000,
            payload_size: 512,
            chunk_size: 1024 * 1024,
        }
    }
}

fn parse_args() -> Config {
    let mut cfg = Config::default();
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let mut i = 0usize;
    while i < args.len() {
        let arg = &args[i];
        let next = args.get(i + 1);
        match (arg.as_str(), next) {
            ("--events", Some(v)) => {
                cfg.events = v.parse().unwrap_or(cfg.events);
                i += 2;
            }
            ("--payload-size", Some(v)) => {
                cfg.payload_size = v.parse().unwrap_or(cfg.payload_size);
                i += 2;
            }
            ("--chunk-size", Some(v)) => {
                cfg.chunk_size = v.parse().unwrap_or(cfg.chunk_size);
                i += 2;
            }
            ("--help", _) | ("-h", _) => {
                println!(
                    "Usage: encryption_overhead_benchmark [--events N] [--payload-size N] [--chunk-size N]"
                );
                std::process::exit(0);
            }
            _ => i += 1,
        }
    }
    cfg
}

fn build_segment(path: PathBuf, cfg: &Config) -> Segment {
    let mut seg = Segment::new(path, 1024 * 1024 * 1024);
    let stream = StreamId::new();
    let actor = ActorId::new();
    for i in 0..cfg.events {
        let payload = vec![(i % 251) as u8; cfg.payload_size];
        let event = Event::new(stream.clone(), actor.clone(), payload);
        seg.append_event(&event).expect("append event");
    }
    seg.seal().expect("seal");
    seg
}

fn main() {
    let cfg = parse_args();

    let plain_path = std::env::temp_dir().join("enc_overhead_plain.seg");
    let plain = build_segment(plain_path, &cfg);

    let enc_path = std::env::temp_dir().join("enc_overhead_encrypted.seg");
    let mut encrypted = build_segment(enc_path, &cfg);
    encrypted.enable_encryption(SegmentEncryptionConfig {
        store_uuid: [7u8; 16],
        key_epoch: 1,
        chunk_size: cfg.chunk_size,
        mode: EncryptionMode::PayloadOnly,
        store_key: [9u8; 32],
    });

    let start_plain = Instant::now();
    let plain_bytes = plain.serialize_result().expect("serialize plain");
    let plain_elapsed = start_plain.elapsed();

    let start_enc = Instant::now();
    let enc_bytes = encrypted.serialize_result().expect("serialize encrypted");
    let enc_elapsed = start_enc.elapsed();

    let plain_mib = plain_bytes.len() as f64 / (1024.0 * 1024.0);
    let enc_mib = enc_bytes.len() as f64 / (1024.0 * 1024.0);
    let plain_mib_s = plain_mib / plain_elapsed.as_secs_f64();
    let enc_mib_s = enc_mib / enc_elapsed.as_secs_f64();
    let overhead_pct = ((enc_elapsed.as_secs_f64() / plain_elapsed.as_secs_f64()) - 1.0) * 100.0;

    println!("events={}", cfg.events);
    println!("payload_size={}", cfg.payload_size);
    println!("chunk_size={}", cfg.chunk_size);
    println!("plain_bytes={}", plain_bytes.len());
    println!("encrypted_bytes={}", enc_bytes.len());
    println!("plain_serialize_s={:.6}", plain_elapsed.as_secs_f64());
    println!("encrypted_serialize_s={:.6}", enc_elapsed.as_secs_f64());
    println!("plain_mib_s={:.2}", plain_mib_s);
    println!("encrypted_mib_s={:.2}", enc_mib_s);
    println!("encryption_overhead_percent={:.2}", overhead_pct);
}
