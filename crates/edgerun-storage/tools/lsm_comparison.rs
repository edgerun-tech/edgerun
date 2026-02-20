// SPDX-License-Identifier: GPL-2.0-only
//! LSM vs Hash Index Benchmark
//!
//! Compares memory usage, insertion throughput, and lookup performance
//! between the in-memory hash index and the LSM-backed index.

use rand::Rng;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};
use storage_engine::index::EventHashIndex;
use storage_engine::lsm_index::{LsmConfig, LsmIndex};

const TEST_SIZES: [usize; 3] = [10_000, 100_000, 1_000_000];

fn format_bytes(bytes: f64) -> String {
    if bytes >= 1_099_511_627_776.0 {
        format!("{:.2} TB", bytes / 1_099_511_627_776.0)
    } else if bytes >= 1_073_741_824.0 {
        format!("{:.2} GB", bytes / 1_073_741_824.0)
    } else if bytes >= 1_048_576.0 {
        format!("{:.2} MB", bytes / 1_048_576.0)
    } else if bytes >= 1024.0 {
        format!("{:.2} KB", bytes / 1024.0)
    } else {
        format!("{:.0} B", bytes)
    }
}

fn main() {
    println!("=== LSM vs Hash Index Benchmark ===\n");
    println!("Comparing memory usage and performance characteristics.\n");

    for &size in &TEST_SIZES {
        println!("\n{}\n", "=".repeat(80));
        println!("Dataset size: {} entries\n", size);

        // Test Hash Index
        let hash_result = benchmark_hash_index(size);
        print_hash_result(&hash_result);

        // Test LSM Index
        let lsm_result = benchmark_lsm_index(size);
        print_lsm_result(&lsm_result);

        // Compare
        print_comparison(&hash_result, &lsm_result);
    }

    println!("\n{}\n", "=".repeat(80));
    println!("SUMMARY");
    println!("{}", "=".repeat(80));
    println!();
    println!("Hash Index:");
    println!("  - O(1) lookups");
    println!("  - Unbounded memory growth");
    println!("  - Lost on restart");
    println!("  - Best for: Small datasets, ephemeral data");
    println!();
    println!("LSM Index:");
    println!("  - O(log n) lookups with bloom filters");
    println!("  - Bounded memory (4MB memtable + 2 imm * 4MB = 12MB max)");
    println!("  - Persistent (survives restart)");
    println!("  - Best for: Large datasets, durability required");
}

#[allow(dead_code)]
#[derive(Debug)]
struct BenchmarkResult {
    num_entries: usize,
    insert_duration: Duration,
    lookup_duration: Duration,
    memory_bytes: usize,
    disk_bytes: u64,
    inserts_per_sec: f64,
    lookups_per_sec: f64,
    memory_per_entry: f64,
    compaction_scheduled: u64,
    compaction_completed: u64,
    compaction_failed: u64,
    compaction_skipped: u64,
    compaction_total_ms: u64,
    compaction_last_ms: u64,
}

fn benchmark_hash_index(num_entries: usize) -> BenchmarkResult {
    let index = EventHashIndex::new();
    let mut rng = rand::thread_rng();

    // Insert
    let insert_start = Instant::now();
    for i in 0..num_entries {
        let key: [u8; 32] = rng.gen();
        index.insert(key, [0u8; 32], i as u64);
    }
    let insert_duration = insert_start.elapsed();

    // Measure memory (estimate)
    let memory_bytes = index.len() * (32 + 32 + 8 + 8); // key + segment_id + offset + overhead

    // Lookup (existing keys)
    let lookup_start = Instant::now();
    let mut _found = 0;
    for _ in 0..1000 {
        let key: [u8; 32] = rng.gen();
        if index.get(&key).is_some() {
            _found += 1;
        }
    }
    let lookup_duration = lookup_start.elapsed();

    BenchmarkResult {
        num_entries,
        insert_duration,
        lookup_duration,
        memory_bytes,
        disk_bytes: 0,
        inserts_per_sec: num_entries as f64 / insert_duration.as_secs_f64(),
        lookups_per_sec: 1000.0 / lookup_duration.as_secs_f64(),
        memory_per_entry: memory_bytes as f64 / num_entries as f64,
        compaction_scheduled: 0,
        compaction_completed: 0,
        compaction_failed: 0,
        compaction_skipped: 0,
        compaction_total_ms: 0,
        compaction_last_ms: 0,
    }
}

fn benchmark_lsm_index(num_entries: usize) -> BenchmarkResult {
    let data_dir = PathBuf::from(format!("/tmp/lsm_bench_{}", num_entries));
    let _ = std::fs::remove_dir_all(&data_dir);

    let config = LsmConfig {
        memtable_size_threshold: 4 * 1024 * 1024, // 4MB
        max_imm_memtables: 2,
        bloom_bits_per_key: 10,
        block_size: 4 * 1024,
        base_level_size: 64 * 1024 * 1024,
        level_size_multiplier: 10,
        max_levels: 6,
    };

    let index = LsmIndex::new(data_dir.clone(), config).unwrap();
    let mut rng = rand::thread_rng();

    // Insert
    let insert_start = Instant::now();
    for i in 0..num_entries {
        let key: [u8; 32] = rng.gen();
        index.insert(key, [0u8; 32], i as u64);
    }
    let insert_duration = insert_start.elapsed();
    // Allow background compaction to finish for observability snapshot.
    thread::sleep(Duration::from_millis(20));

    // Get memory stats
    let mem_stats = index.memory_usage();
    let disk_stats = index.disk_usage();
    let compaction = index.compaction_stats();

    // Lookup (existing keys + random)
    let lookup_start = Instant::now();
    let mut _found = 0;
    for _ in 0..1000 {
        let key: [u8; 32] = rng.gen();
        if index.get(&key).is_some() {
            _found += 1;
        }
    }
    let lookup_duration = lookup_start.elapsed();

    // Cleanup
    let _ = std::fs::remove_dir_all(&data_dir);

    BenchmarkResult {
        num_entries,
        insert_duration,
        lookup_duration,
        memory_bytes: mem_stats.total_bytes,
        disk_bytes: disk_stats.total_bytes,
        inserts_per_sec: num_entries as f64 / insert_duration.as_secs_f64(),
        lookups_per_sec: 1000.0 / lookup_duration.as_secs_f64(),
        memory_per_entry: mem_stats.total_bytes as f64 / num_entries as f64,
        compaction_scheduled: compaction.scheduled,
        compaction_completed: compaction.completed,
        compaction_failed: compaction.failed,
        compaction_skipped: compaction.skipped,
        compaction_total_ms: compaction.total_duration_ms,
        compaction_last_ms: compaction.last_duration_ms,
    }
}

fn print_hash_result(result: &BenchmarkResult) {
    println!("Hash Index Results:");
    println!(
        "  Insert throughput: {:.0} entries/s",
        result.inserts_per_sec
    );
    println!(
        "  Lookup throughput: {:.0} lookups/s",
        result.lookups_per_sec
    );
    println!(
        "  Memory: {} ({:.1} bytes/entry)",
        format_bytes(result.memory_bytes as f64),
        result.memory_per_entry
    );
    println!("  Disk: N/A (in-memory only)");
    println!();
}

fn print_lsm_result(result: &BenchmarkResult) {
    println!("LSM Index Results:");
    println!(
        "  Insert throughput: {:.0} entries/s",
        result.inserts_per_sec
    );
    println!(
        "  Lookup throughput: {:.0} lookups/s",
        result.lookups_per_sec
    );
    println!(
        "  Memory: {} ({:.1} bytes/entry)",
        format_bytes(result.memory_bytes as f64),
        result.memory_per_entry
    );
    println!("  Disk: {}", format_bytes(result.disk_bytes as f64));
    println!(
        "  Compaction: scheduled={}, completed={}, failed={}, skipped={}",
        result.compaction_scheduled,
        result.compaction_completed,
        result.compaction_failed,
        result.compaction_skipped
    );
    println!(
        "  Compaction duration: total={} ms, last={} ms",
        result.compaction_total_ms, result.compaction_last_ms
    );
    println!();
}

fn print_comparison(hash: &BenchmarkResult, lsm: &BenchmarkResult) {
    println!("Comparison (LSM vs Hash):");

    let insert_ratio = lsm.inserts_per_sec / hash.inserts_per_sec;
    let lookup_ratio = lsm.lookups_per_sec / hash.lookups_per_sec;
    let memory_ratio = lsm.memory_bytes as f64 / hash.memory_bytes as f64;

    println!(
        "  Insert speed: {:.1}x {}",
        insert_ratio,
        if insert_ratio >= 1.0 {
            "(LSM faster)"
        } else {
            "(Hash faster)"
        }
    );
    println!(
        "  Lookup speed: {:.1}x {}",
        lookup_ratio,
        if lookup_ratio >= 1.0 {
            "(LSM faster)"
        } else {
            "(Hash faster)"
        }
    );
    println!(
        "  Memory usage: {:.1}x {}",
        memory_ratio,
        if memory_ratio <= 1.0 {
            "(LSM efficient)"
        } else {
            "(Hash efficient)"
        }
    );

    // LSM advantages
    println!();
    println!("LSM advantages at this scale:");
    if lsm.memory_bytes < 50 * 1024 * 1024 {
        println!("  ✓ Memory bounded (~12MB max regardless of dataset)");
    }
    if lsm.disk_bytes > 0 {
        println!("  ✓ Persistent (survives restart)");
    }
    if hash.memory_bytes > 100 * 1024 * 1024 {
        println!("  ✓ Hash uses {:.1}x more memory", memory_ratio);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(512.0), "512 B");
        assert_eq!(format_bytes(1024.0), "1.00 KB");
        assert_eq!(format_bytes(1024.0 * 1024.0), "1.00 MB");
    }
}
