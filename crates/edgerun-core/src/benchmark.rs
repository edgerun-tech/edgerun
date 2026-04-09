/// Fast deterministic benchmarks for producing PerformanceCertificates.
///
/// Each benchmark runs for a fixed wall-clock duration (~50ms) and
/// counts how many iterations complete. This gives proportional scores
/// regardless of absolute CPU speed — fast CPUs get more iterations done.
///
/// Total suite completes in ~1-2 seconds.

use std::time::{Duration, Instant};

use crate::accounting::PerformanceCertificate;
use crate::crypto::sha256;

// ===========================================================================
// CPU Integer Math Benchmark
//
// Mixed ALU: integer hash (simple multiply-add) + branch-heavy sieve.
// ===========================================================================

pub fn benchmark_cpu_int() -> u64 {
    let start = Instant::now();
    let target = Duration::from_millis(50);
    let mut iters: u64 = 0;

    while start.elapsed() < target {
        // Integer hash — tight loop, mostly multiply + add
        let mut h: u64 = 0x9e37_79b9_7f4a_7c15;
        for i in 0..1000 {
            h = h.wrapping_mul(6364136223846793005).wrapping_add(i);
            h ^= h.rotate_left(13);
        }
        iters += 1;
        std::hint::black_box(h);
    }

    // Score = iterations per microsecond (gives 100K-10M range)
    let elapsed_us = start.elapsed().as_micros() as u64;
    if elapsed_us > 0 {
        iters.saturating_mul(1_000_000) / elapsed_us
    } else {
        0
    }
}

// ===========================================================================
// CPU Crypto Benchmark
//
// SHA-256 throughput (we have our own implementation).
// ===========================================================================

pub fn benchmark_cpu_crypto() -> u64 {
    let data: [u8; 256] = [0xAB; 256];
    let start = Instant::now();
    let target = Duration::from_millis(50);
    let mut count: u64 = 0;

    while start.elapsed() < target {
        let _ = sha256(&data);
        count += 1;
    }

    let elapsed_us = start.elapsed().as_micros() as u64;
    if elapsed_us > 0 {
        count.saturating_mul(1_000_000) / elapsed_us
    } else {
        0
    }
}

// ===========================================================================
// Memory Bandwidth Benchmark
//
// Sequential read through 64 MB with checksum.
// ===========================================================================

pub fn benchmark_memory_bandwidth() -> u64 {
    const SIZE: usize = 64 * 1024 * 1024;
    let mut buf = vec![0u8; SIZE];
    for (i, b) in buf.iter_mut().enumerate() {
        *b = (i & 0xFF) as u8;
    }

    let start = Instant::now();
    let mut checksum: u64 = 0;

    // Single pass
    for chunk in buf.chunks(64) {
        for &byte in chunk {
            checksum = checksum.wrapping_add(byte as u64);
        }
    }

    let elapsed = start.elapsed();
    std::hint::black_box(checksum);

    let mbps = (SIZE as u64)
        .saturating_mul(1_000_000)
        .checked_div(elapsed.as_micros() as u64)
        .unwrap_or(0)
        / (1024 * 1024);

    mbps
}

// ===========================================================================
// Memory Latency Benchmark
//
// Random pointer-chasing through 512 MB — sized to exceed even the largest
// known CPU caches (e.g., AMD EPYC 9654 has 384 MB L3) to ensure we measure
// real DRAM latency, not cache hits.
// ===========================================================================

pub fn benchmark_memory_latency() -> u64 {
    const SIZE_MB: usize = 512;
    let size = (SIZE_MB * 1024 * 1024) / 8; // u64 entries
    let mut pointers: Vec<usize> = (0..size).collect();

    // Deterministic shuffle (Fisher-Yates with LCG)
    let mut state: u64 = 0xDEAD_BEEF_CAFE_BABE;
    for i in (1..size).rev() {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let j = (state as usize) % (i + 1);
        pointers.swap(i, j);
    }

    let start = Instant::now();
    let target = Duration::from_millis(100);
    let mut idx = 0usize;
    let mut visits: u64 = 0;

    while start.elapsed() < target {
        idx = pointers[idx];
        visits += 1;
    }

    std::hint::black_box(idx);

    let elapsed_ns = start.elapsed().as_nanos() as u64;
    if visits > 0 {
        elapsed_ns / visits
    } else {
        0
    }
}

// ===========================================================================
// Storage I/O Benchmark
//
// Write + read + delete small files. Measures random IOPS.
//
// IMPORTANT: Uses a path within the node's data directory (not /tmp) to
// ensure we benchmark actual workload storage, not a RAM disk.
// ===========================================================================

pub fn benchmark_storage_iops() -> u64 {
    // Use the node's data dir if available, fall back to temp_dir.
    // IMPORTANT: Prefer real disk over RAM disk when possible.
    let base_dir = std::env::var("EDGERUN_DATA_DIR")
        .ok()
        .map(std::path::PathBuf::from)
        .or_else(|| {
            // Try /var/lib/edgerun first (real disk on most systems)
            let p = std::path::PathBuf::from("/var/lib/edgerun");
            std::fs::create_dir_all(&p).ok().map(|_| p)
        })
        .unwrap_or_else(|| std::env::temp_dir());
    let _ = std::fs::create_dir_all(&base_dir);

    let tmp_dir = base_dir.join(format!(
        "eg_b_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros()
    ));
    let _ = std::fs::create_dir_all(&tmp_dir);

    let data = [0xCDu8; 4096];
    let start = Instant::now();
    let target = Duration::from_millis(200);
    let mut ops: u64 = 0;
    let mut i: u32 = 0;

    while start.elapsed() < target {
        let path = tmp_dir.join(format!("f{}", i));
        if std::fs::write(&path, &data).is_ok() {
            ops += 1;
            if std::fs::read(&path).is_ok() {
                ops += 1;
            }
            let _ = std::fs::remove_file(&path);
        } else {
            break;
        }
        i += 1;
    }

    let _ = std::fs::remove_dir_all(&tmp_dir);

    let elapsed_us = start.elapsed().as_micros() as u64;
    if elapsed_us > 0 {
        ops.saturating_mul(1_000_000) / elapsed_us
    } else {
        0
    }
}

// ===========================================================================
// Storage Sequential Throughput
//
// Write + read a single 16 MB file. Uses the same real-disk path as IOPS.
// ===========================================================================

pub fn benchmark_storage_sequential() -> u64 {
    const SIZE: usize = 16 * 1024 * 1024;

    let base_dir = std::env::var("EDGERUN_DATA_DIR")
        .ok()
        .map(std::path::PathBuf::from)
        .or_else(|| {
            let p = std::path::PathBuf::from("/var/lib/edgerun");
            std::fs::create_dir_all(&p).ok().map(|_| p)
        })
        .unwrap_or_else(|| std::env::temp_dir());
    let _ = std::fs::create_dir_all(&base_dir);

    let tmp_dir = base_dir.join(format!(
        "eg_bs_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros()
    ));
    let _ = std::fs::create_dir_all(&tmp_dir);
    let path = tmp_dir.join("seq.bin");

    let data: Vec<u8> = (0..SIZE).map(|i| (i & 0xFF) as u8).collect();

    let w_start = Instant::now();
    if std::fs::write(&path, &data).is_err() {
        let _ = std::fs::remove_dir_all(&tmp_dir);
        return 0;
    }
    let w_us = w_start.elapsed().as_micros() as u64;

    let r_start = Instant::now();
    let rd = match std::fs::read(&path) {
        Ok(d) => d,
        Err(_) => { let _ = std::fs::remove_dir_all(&tmp_dir); return 0; }
    };
    let r_us = r_start.elapsed().as_micros() as u64;

    let _ = std::fs::remove_dir_all(&tmp_dir);
    if rd.len() != SIZE { return 0; }
    std::hint::black_box(&rd);

    let total_us = w_us.saturating_add(r_us);
    if total_us > 0 {
        ((SIZE as u64) * 2).saturating_mul(1_000_000) / total_us / (1024 * 1024)
    } else {
        0
    }
}

// ===========================================================================
// Full Suite
// ===========================================================================

/// Run all benchmarks and return a PerformanceCertificate.
/// Digest is computed; signature is zeros (caller signs with node key).
///
/// Total runtime: ~0.5-1.5 seconds on typical hardware.
pub fn run_full_benchmark(node_id: [u8; 64]) -> PerformanceCertificate {
    let started = now_us();

    let cpu_int_score = benchmark_cpu_int();
    let cpu_crypto_score = benchmark_cpu_crypto();
    let mem_bw = benchmark_memory_bandwidth();
    let mem_lat = benchmark_memory_latency();
    let storage_iops = benchmark_storage_iops();
    let storage_seq = benchmark_storage_sequential();

    let completed = now_us();

    let mut cert = PerformanceCertificate {
        node_id,
        cpu_int_score,
        cpu_crypto_score,
        mem_bandwidth_mbps: mem_bw,
        mem_latency_ns: mem_lat,
        storage_random_iops: storage_iops,
        storage_seq_mbps: storage_seq,
        gpu_score: None,
        npu_score: None,
        benchmark_started_us: started,
        benchmark_completed_us: completed,
        digest: [0u8; 32],
        signature: [0u8; 64],
    };

    cert.digest = cert.compute_digest();
    cert
}

fn now_us() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64
}

/// Human-readable output.
pub fn print_benchmark_results(cert: &PerformanceCertificate) {
    println!("=== edgerun Performance Certificate ===");
    println!("  Node ID:      {}...", hex8(&cert.node_id));
    println!("  CPU Int:      {} ops/s", cert.cpu_int_score);
    println!("  CPU Crypto:   {} hashes/s", cert.cpu_crypto_score);
    println!("  Memory BW:    {} MB/s", cert.mem_bandwidth_mbps);
    println!("  Memory Lat:   {} ns", cert.mem_latency_ns);
    println!("  Storage IOPS: {} ops/s", cert.storage_random_iops);
    println!("  Storage Seq:  {} MB/s", cert.storage_seq_mbps);
    println!("  Duration:     {} ms",
        (cert.benchmark_completed_us - cert.benchmark_started_us) / 1000);
    println!("  Digest:       {}", hex32(&cert.digest));
}

fn hex8(b: &[u8; 64]) -> String {
    b[..4].iter().map(|x| format!("{:02x}", x)).collect()
}
fn hex32(b: &[u8; 32]) -> String {
    b.iter().map(|x| format!("{:02x}", x)).collect::<Vec<_>>().join("")
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_int_fast() {
        let s = benchmark_cpu_int();
        assert!(s > 0, "cpu_int score was 0");
        assert!(s < 100_000_000_000u64, "cpu_int score {} impossibly high", s);
    }

    #[test]
    fn cpu_crypto_fast() {
        let s = benchmark_cpu_crypto();
        assert!(s > 0, "cpu_crypto score was 0");
        assert!(s < 100_000_000_000u64, "cpu_crypto score {} impossibly high", s);
    }

    #[test]
    fn mem_bw_fast() {
        let mbps = benchmark_memory_bandwidth();
        assert!(mbps > 100, "memory bandwidth {} MB/s impossibly low", mbps);
        assert!(mbps < 1_000_000, "memory bandwidth {} MB/s impossibly high", mbps);
    }

    #[test]
    fn mem_lat_fast() {
        let ns = benchmark_memory_latency();
        assert!(ns > 0 && ns < 100_000, "memory latency {} ns out of range", ns);
    }

    #[test]
    fn storage_iops_fast() {
        let iops = benchmark_storage_iops();
        assert!(iops > 0, "storage iops was 0");
    }

    #[test]
    fn storage_seq_fast() {
        let mbps = benchmark_storage_sequential();
        assert!(mbps > 0, "storage seq was 0");
    }

    #[test]
    fn full_suite_produces_certificate() {
        let nid = [0x42u8; 64];
        let cert = run_full_benchmark(nid);
        assert_eq!(cert.node_id, nid);
        assert!(cert.cpu_int_score > 0);
        assert!(cert.mem_bandwidth_mbps > 0);
        assert!(cert.storage_random_iops > 0);
        assert!(cert.benchmark_completed_us > cert.benchmark_started_us);
        assert_ne!(cert.digest, [0u8; 32]);
    }

    #[test]
    fn cpu_multiplier_reasonable() {
        let cert = run_full_benchmark([1u8; 64]);
        let m = cert.cpu_core_multiplier();
        assert!(!m.is_zero());
        assert!(m.to_raw() > 1_000);  // > 0.015x
        assert!(m.to_raw() < 100_000_000); // < 1500x
    }
}
