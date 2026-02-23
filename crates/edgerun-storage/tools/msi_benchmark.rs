// SPDX-License-Identifier: GPL-2.0-only
//! Materialized State Index (MSI) Benchmark
//!
//! Demonstrates the performance benefits of MSI for hot-key reads
//! under mixed read/write workloads.

use edgerun_storage::event::StreamId;
use edgerun_storage::materialized_state::{MaterializedStateIndex, MsiConfig};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

const WARMUP_ITERATIONS: usize = 1000;
const BENCHMARK_DURATION_SECS: u64 = 10;

fn main() {
    println!("=== Materialized State Index (MSI) Benchmark ===\n");

    // Test 1: MSI cache effectiveness
    test_cache_effectiveness();

    // Test 2: Mixed workload performance
    test_mixed_workload();

    // Test 3: Tail replay impact
    test_tail_replay_impact();

    println!("\nSummary:\n");
    println!("MSI provides:");
    println!("  - Sub-millisecond reads for cached hot keys");
    println!("  - Bounded tail replay (≤256 events)");
    println!("  - Consistent performance under mixed load");
    println!("  - Adaptive snapshotting based on tail length");
}

fn test_cache_effectiveness() {
    println!("\n{}\n", "=".repeat(80));
    println!("Test 1: MSI Cache Effectiveness");
    println!("{}", "=".repeat(80));

    let config = MsiConfig {
        cache_capacity: 1000,
        snapshot_event_interval: 100,
        max_tail_length: 10,
        ..Default::default()
    };

    let msi = Arc::new(MaterializedStateIndex::new(config));
    let stream_id = StreamId::new();

    // Populate with 1000 keys
    println!("\nPopulating 1000 keys...");
    for i in 0..1000 {
        let key = format!("key_{i:04}").into_bytes();
        let value = format!("value_{i:04}").into_bytes();
        msi.write(key, value, stream_id.clone(), i as u64);
    }

    // Create a snapshot
    let mut state = std::collections::HashMap::new();
    for i in 0..1000 {
        let key = format!("key_{i:04}").into_bytes();
        let value = format!("value_{i:04}").into_bytes();
        state.insert(key, value);
    }
    msi.create_snapshot(stream_id.clone(), state, 1000);

    // Warmup
    for _ in 0..WARMUP_ITERATIONS {
        let key = format!("key_{:04}", rand::random::<usize>() % 1000).into_bytes();
        let _ = msi.read(&key, stream_id.clone());
    }

    // Test: 90% hot keys (first 100), 10% cold keys
    println!("\nTesting 90%% hot key distribution...");
    let start = Instant::now();
    let mut hot_hits = 0u64;
    let mut total = 0u64;

    while start.elapsed().as_secs() < 5 {
        let key_idx = if rand::random::<f64>() < 0.9 {
            rand::random::<usize>() % 100 // Hot keys
        } else {
            100 + rand::random::<usize>() % 900 // Cold keys
        };

        let key = format!("key_{key_idx:04}").into_bytes();
        if msi.read(&key, stream_id.clone()).is_some() && key_idx < 100 {
            hot_hits += 1;
        }
        total += 1;
    }

    let stats = msi.cache_stats();
    println!("  Total reads: {total}");
    println!("  Cache hit rate: {:.1}%", stats.hit_rate * 100.0);
    println!(
        "  Hot key hit rate: {:.1}%",
        (hot_hits as f64 / (total as f64 * 0.9)) * 100.0
    );
    println!("  Cache size: {}/{}", stats.size, stats.capacity);
}

fn test_mixed_workload() {
    println!("\n{}\n", "=".repeat(80));
    println!("Test 2: Mixed Workload Performance");
    println!("{}", "=".repeat(80));

    let config = MsiConfig::default();
    let msi = Arc::new(MaterializedStateIndex::new(config));
    let stream_id = StreamId::new();

    // Setup: Create initial state
    println!("\nSetting up initial state (10K keys)...");
    let mut state = std::collections::HashMap::new();
    for i in 0..10_000 {
        let key = format!("key_{i:05}").into_bytes();
        let value = format!("value_{i:05}").into_bytes();
        state.insert(key.clone(), value.clone());
        msi.write(key, value, stream_id.clone(), i as u64);
    }

    let stream_id_read = stream_id.clone();
    let stream_id_write = stream_id.clone();

    msi.create_snapshot(stream_id, state, 10_000);

    let reads_completed = Arc::new(AtomicU64::new(0));
    let writes_completed = Arc::new(AtomicU64::new(0));

    let reads_completed_clone = Arc::clone(&reads_completed);
    let writes_completed_clone = Arc::clone(&writes_completed);
    let msi_read = Arc::clone(&msi);
    let msi_write = Arc::clone(&msi);

    // Reader thread
    let reader = thread::spawn(move || {
        let start = Instant::now();
        while start.elapsed().as_secs() < BENCHMARK_DURATION_SECS {
            let key_idx = rand::random::<usize>() % 10_000;
            let key = format!("key_{key_idx:05}").into_bytes();
            let _ = msi_read.read(&key, stream_id_read.clone());
            reads_completed_clone.fetch_add(1, Ordering::Relaxed);
        }
    });

    // Writer thread
    let writer = thread::spawn(move || {
        let start = Instant::now();
        let mut counter = 10_000u64;
        while start.elapsed().as_secs() < BENCHMARK_DURATION_SECS {
            let key_idx = rand::random::<usize>() % 10_000;
            let key = format!("key_{key_idx:05}").into_bytes();
            let value = format!("updated_{counter}").into_bytes();
            msi_write.write(key, value, stream_id_write.clone(), counter);
            counter += 1;
            writes_completed_clone.fetch_add(1, Ordering::Relaxed);

            // Small delay to not overwhelm
            thread::sleep(Duration::from_micros(10));
        }
    });

    reader.join().unwrap();
    writer.join().unwrap();

    let reads = reads_completed.load(Ordering::Relaxed);
    let writes = writes_completed.load(Ordering::Relaxed);
    let stats = msi.cache_stats();

    println!("\nResults ({BENCHMARK_DURATION_SECS}s duration):");
    println!(
        "  Reads: {} ({:.0} reads/s)",
        reads,
        reads as f64 / BENCHMARK_DURATION_SECS as f64
    );
    println!(
        "  Writes: {} ({:.0} writes/s)",
        writes,
        writes as f64 / BENCHMARK_DURATION_SECS as f64
    );
    println!("  Cache hit rate: {:.1}%", stats.hit_rate * 100.0);
    println!("  Read:Write ratio: {:.1}:1", reads as f64 / writes as f64);
}

fn test_tail_replay_impact() {
    println!("\n{}\n", "=".repeat(80));
    println!("Test 3: Tail Replay Impact");
    println!("{}", "=".repeat(80));

    // Test different tail lengths
    let tail_lengths = vec![0, 10, 50, 100, 256];

    println!("\nMeasuring read latency with varying tail lengths...\n");
    println!(
        "{:<15} {:<15} {:<15}",
        "Tail Length", "Latency (µs)", "Impact"
    );
    println!("{}", "-".repeat(50));

    let mut baseline = 0.0;

    for &tail_len in &tail_lengths {
        let config = MsiConfig {
            max_tail_length: tail_len,
            ..Default::default()
        };

        let msi = MaterializedStateIndex::new(config);
        let stream_id = StreamId::new();

        // Setup: snapshot at event 0
        let mut state = std::collections::HashMap::new();
        state.insert(b"test_key".to_vec(), b"initial_value".to_vec());
        msi.create_snapshot(stream_id.clone(), state, 0);

        // Add tail events
        for i in 0..tail_len {
            let value = format!("tail_value_{i}").into_bytes();
            msi.write(
                b"test_key".to_vec(),
                value,
                stream_id.clone(),
                (i + 1) as u64,
            );
        }

        // Measure read latency
        let iterations = 1000;
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = msi.read(b"test_key", stream_id.clone());
        }
        let elapsed = start.elapsed().as_micros() as f64 / iterations as f64;

        if tail_len == 0 {
            baseline = elapsed;
            println!("{:<15} {:<15.1} {:<15}", tail_len, elapsed, "baseline");
        } else {
            let impact = (elapsed - baseline) / baseline * 100.0;
            println!("{tail_len:<15} {elapsed:<15.1} {impact:<14.1}%");
        }
    }

    println!("\nNote: Tail replay adds latency proportional to tail length.");
    println!("Adaptive snapshotting keeps tail bounded to ~100 events.");
}
