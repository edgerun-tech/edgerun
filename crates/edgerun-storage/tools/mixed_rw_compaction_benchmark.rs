// SPDX-License-Identifier: Apache-2.0
//! Mixed read/write benchmark with background compaction pressure.
//!
//! Usage:
//!   cargo run -q --bin mixed_rw_compaction_benchmark -- --duration 20 --writers 2 --readers 4

use edgerun_storage::lsm_index::{LsmConfig, LsmIndex};
use rand::Rng;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
struct Config {
    duration_secs: u64,
    writers: usize,
    readers: usize,
    write_batch: usize,
    read_batch: usize,
    key_space: u64,
    hot_key_space: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            duration_secs: 20,
            writers: 2,
            readers: 4,
            write_batch: 1024,
            read_batch: 4096,
            key_space: 2_000_000,
            hot_key_space: 200_000,
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
            ("--duration", Some(v)) => {
                cfg.duration_secs = v.parse().unwrap_or(cfg.duration_secs);
                i += 2;
            }
            ("--writers", Some(v)) => {
                cfg.writers = v.parse().unwrap_or(cfg.writers);
                i += 2;
            }
            ("--readers", Some(v)) => {
                cfg.readers = v.parse().unwrap_or(cfg.readers);
                i += 2;
            }
            ("--write-batch", Some(v)) => {
                cfg.write_batch = v.parse().unwrap_or(cfg.write_batch);
                i += 2;
            }
            ("--read-batch", Some(v)) => {
                cfg.read_batch = v.parse().unwrap_or(cfg.read_batch);
                i += 2;
            }
            ("--key-space", Some(v)) => {
                cfg.key_space = v.parse().unwrap_or(cfg.key_space);
                i += 2;
            }
            ("--hot-key-space", Some(v)) => {
                cfg.hot_key_space = v.parse().unwrap_or(cfg.hot_key_space);
                i += 2;
            }
            ("--help", _) | ("-h", _) => {
                println!(
                    "Usage: mixed_rw_compaction_benchmark [--duration N] [--writers N] [--readers N] [--write-batch N] [--read-batch N] [--key-space N] [--hot-key-space N]"
                );
                std::process::exit(0);
            }
            _ => {
                i += 1;
            }
        }
    }

    cfg
}

fn key_from_u64(x: u64) -> [u8; 32] {
    let mut key = [0u8; 32];
    key[0..8].copy_from_slice(&x.to_le_bytes());
    key[8..16].copy_from_slice(&x.rotate_left(13).to_le_bytes());
    key[16..24].copy_from_slice(&x.rotate_left(29).to_le_bytes());
    key[24..32].copy_from_slice(&x.rotate_left(47).to_le_bytes());
    key
}

fn main() {
    let cfg = parse_args();

    let data_dir = PathBuf::from(format!(
        "/tmp/mixed_rw_compaction_{}_{}_{}",
        std::process::id(),
        cfg.writers,
        cfg.readers
    ));
    let _ = std::fs::remove_dir_all(&data_dir);

    let lsm_cfg = LsmConfig {
        memtable_size_threshold: 512 * 1024,
        max_imm_memtables: 2,
        bloom_bits_per_key: 10,
        block_size: 4096,
        base_level_size: 8 * 1024 * 1024,
        level_size_multiplier: 8,
        max_levels: 6,
    };

    let index = Arc::new(LsmIndex::new(data_dir.clone(), lsm_cfg).expect("create LSM index"));
    let stop = Arc::new(AtomicBool::new(false));

    let write_ops = Arc::new(AtomicU64::new(0));
    let read_ops = Arc::new(AtomicU64::new(0));
    let read_hits = Arc::new(AtomicU64::new(0));
    let read_misses = Arc::new(AtomicU64::new(0));

    println!("=== Mixed Read/Write + Compaction Benchmark ===");
    println!("data_dir: {}", data_dir.display());
    println!("duration: {}s", cfg.duration_secs);
    println!(
        "threads: {} writer(s), {} reader(s)",
        cfg.writers, cfg.readers
    );
    println!(
        "batches: write={}, read={}, key_space={}, hot_key_space={}",
        cfg.write_batch, cfg.read_batch, cfg.key_space, cfg.hot_key_space
    );
    println!();

    let baseline_compaction = index.compaction_stats();
    let started = Instant::now();

    let mut handles = Vec::new();

    for w in 0..cfg.writers {
        let index = Arc::clone(&index);
        let stop = Arc::clone(&stop);
        let write_ops = Arc::clone(&write_ops);
        let write_batch = cfg.write_batch;
        let key_space = cfg.key_space.max(1);
        handles.push(thread::spawn(move || {
            let mut seq = w as u64;
            let stride = cfg.writers.max(1) as u64;
            while !stop.load(Ordering::Relaxed) {
                for _ in 0..write_batch {
                    seq = seq.wrapping_add(stride);
                    let key_num = seq % key_space;
                    let key = key_from_u64(key_num);
                    index.insert(key, key_from_u64(seq), seq);
                }
                write_ops.fetch_add(write_batch as u64, Ordering::Relaxed);
            }
        }));
    }

    for _ in 0..cfg.readers {
        let index = Arc::clone(&index);
        let stop = Arc::clone(&stop);
        let read_ops = Arc::clone(&read_ops);
        let read_hits = Arc::clone(&read_hits);
        let read_misses = Arc::clone(&read_misses);
        let read_batch = cfg.read_batch;
        let key_space = cfg.key_space.max(1);
        let hot_key_space = cfg.hot_key_space.max(1).min(key_space);
        handles.push(thread::spawn(move || {
            let mut rng = rand::thread_rng();
            while !stop.load(Ordering::Relaxed) {
                let mut hits = 0u64;
                let mut misses = 0u64;
                for _ in 0..read_batch {
                    // 80% reads target active key-space, 20% target non-existent key-space.
                    let key_num = if rng.gen_bool(0.8) {
                        rng.gen_range(0..hot_key_space)
                    } else {
                        key_space + rng.gen_range(1..=hot_key_space)
                    };
                    let key = key_from_u64(key_num);
                    if index.get(&key).is_some() {
                        hits += 1;
                    } else {
                        misses += 1;
                    }
                }
                read_ops.fetch_add(read_batch as u64, Ordering::Relaxed);
                read_hits.fetch_add(hits, Ordering::Relaxed);
                read_misses.fetch_add(misses, Ordering::Relaxed);
            }
        }));
    }

    let mut last_w = 0u64;
    let mut last_r = 0u64;
    for sec in 1..=cfg.duration_secs {
        thread::sleep(Duration::from_secs(1));
        let w = write_ops.load(Ordering::Relaxed);
        let r = read_ops.load(Ordering::Relaxed);
        let wps = w.saturating_sub(last_w);
        let rps = r.saturating_sub(last_r);
        last_w = w;
        last_r = r;
        println!(
            "t+{:02}s  write/s={:>10}  read/s={:>10}  compaction={:?}",
            sec,
            wps,
            rps,
            index.compaction_stats()
        );
    }

    stop.store(true, Ordering::Relaxed);
    for h in handles {
        let _ = h.join();
    }

    let elapsed = started.elapsed().as_secs_f64().max(1e-9);
    let total_writes = write_ops.load(Ordering::Relaxed);
    let total_reads = read_ops.load(Ordering::Relaxed);
    let hits = read_hits.load(Ordering::Relaxed);
    let misses = read_misses.load(Ordering::Relaxed);
    let hit_rate = if total_reads > 0 {
        hits as f64 / total_reads as f64
    } else {
        0.0
    };

    let compaction = index.compaction_stats();
    let compaction_scheduled = compaction
        .scheduled
        .saturating_sub(baseline_compaction.scheduled);
    let compaction_completed = compaction
        .completed
        .saturating_sub(baseline_compaction.completed);
    let compaction_failed = compaction.failed.saturating_sub(baseline_compaction.failed);
    let compaction_skipped = compaction
        .skipped
        .saturating_sub(baseline_compaction.skipped);
    let compaction_ms = compaction
        .total_duration_ms
        .saturating_sub(baseline_compaction.total_duration_ms);
    let mem = index.memory_usage();
    let disk = index.disk_usage();

    println!("\n=== Summary ===");
    println!(
        "writes: {} total ({:.0} ops/s)",
        total_writes,
        total_writes as f64 / elapsed
    );
    println!(
        "reads:  {} total ({:.0} ops/s), hit_rate={:.1}%",
        total_reads,
        total_reads as f64 / elapsed,
        hit_rate * 100.0
    );
    println!("hits={hits}, misses={misses}");
    println!(
        "compaction: scheduled={compaction_scheduled}, completed={compaction_completed}, failed={compaction_failed}, skipped={compaction_skipped}, total_ms={compaction_ms}"
    );
    println!(
        "memory_bytes={} (memtable={}, imm={})",
        mem.total_bytes, mem.memtable_bytes, mem.imm_memtables_bytes
    );
    println!(
        "disk_bytes={}, sstables={}, levels={}",
        disk.total_bytes, disk.sstable_count, disk.level_count
    );

    let _ = std::fs::remove_dir_all(&data_dir);
}
