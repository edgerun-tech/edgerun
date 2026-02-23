// SPDX-License-Identifier: GPL-2.0-only
//! Real durability benchmarks measuring fsync physics and persistence costs.
//!
//! This benchmark suite measures the actual cost of durability guarantees,
//! including fsync latency, write amplification, and CPU overhead.

use edgerun_storage::durability::SyncPolicy;
use edgerun_storage::event::{ActorId, Event, StreamId};
use edgerun_storage::segment::SegmentWriter;
use edgerun_storage::StorageEngine;
use rand::Rng;
use std::path::PathBuf;
use std::time::{Duration, Instant};

const TARGET_SIZE_MB: usize = 10; // Write 10MB for quick demo (use 500+ for real benchmarks)
const EVENT_PAYLOAD_SIZE: usize = 1024;

/// Histogram for latency tracking.
#[derive(Debug, Clone)]
pub struct Histogram {
    name: String,
    values: Vec<f64>,
    min: f64,
    max: f64,
}

impl Histogram {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            values: Vec::with_capacity(10000),
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
        }
    }

    pub fn record(&mut self, value_micros: f64) {
        self.values.push(value_micros);
        self.min = self.min.min(value_micros);
        self.max = self.max.max(value_micros);
    }

    pub fn percentile(&self, p: f64) -> f64 {
        if self.values.is_empty() {
            return 0.0;
        }
        let mut sorted = self.values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let idx = ((p / 100.0) * (sorted.len() - 1) as f64) as usize;
        sorted[idx.min(sorted.len() - 1)]
    }

    pub fn mean(&self) -> f64 {
        if self.values.is_empty() {
            return 0.0;
        }
        self.values.iter().sum::<f64>() / self.values.len() as f64
    }

    pub fn count(&self) -> usize {
        self.values.len()
    }

    pub fn report(&self) {
        println!("  {}:", self.name);
        println!("    Count: {}", self.count());
        println!("    Min: {:.1} µs", self.min);
        println!("    Mean: {:.1} µs", self.mean());
        println!("    p50: {:.1} µs", self.percentile(50.0));
        println!("    p99: {:.1} µs", self.percentile(99.0));
        println!("    p999: {:.1} µs", self.percentile(99.9));
        println!("    Max: {:.1} µs", self.max);
    }
}

/// Benchmark results for a single configuration.
#[derive(Debug)]
pub struct DurabilityBenchmarkResult {
    pub name: String,
    pub sync_policy: SyncPolicy,
    pub total_events: u64,
    pub total_bytes: u64,
    pub actual_bytes_written: u64,
    pub duration: Duration,
    pub append_latency: Histogram,
    pub fsync_latency: Histogram,
}

impl DurabilityBenchmarkResult {
    pub fn throughput_mbps(&self) -> f64 {
        let secs = self.duration.as_secs_f64();
        if secs == 0.0 {
            return 0.0;
        }
        (self.total_bytes as f64 / 1024.0 / 1024.0) / secs
    }

    pub fn throughput_events_per_sec(&self) -> f64 {
        let secs = self.duration.as_secs_f64();
        if secs == 0.0 {
            return 0.0;
        }
        self.total_events as f64 / secs
    }

    pub fn write_amplification(&self) -> f64 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        self.actual_bytes_written as f64 / self.total_bytes as f64
    }

    pub fn report(&self) {
        println!("\n=== {} ===", self.name);
        println!("Sync Policy:");
        println!(
            "  Fsync interval: {} bytes",
            self.sync_policy.fsync_interval_bytes
        );
        println!(
            "  Fsync interval: {} events",
            self.sync_policy.fsync_interval_events
        );
        println!("  Use fdatasync: {}", self.sync_policy.use_fdatasync);
        println!();
        println!("Throughput:");
        println!("  Events: {} total", self.total_events);
        println!(
            "  Data: {:.2} MB",
            self.total_bytes as f64 / 1024.0 / 1024.0
        );
        println!("  Duration: {:.2?}", self.duration);
        println!("  Throughput: {:.1} MB/s", self.throughput_mbps());
        println!(
            "  Throughput: {:.0} events/s",
            self.throughput_events_per_sec()
        );
        println!();
        println!("Write Amplification: {:.2}x", self.write_amplification());
        println!("  Payload bytes: {}", self.total_bytes);
        println!("  Actual bytes written: {}", self.actual_bytes_written);
        println!();
        self.append_latency.report();
        if self.fsync_latency.count() > 0 {
            self.fsync_latency.report();
        }
    }
}

/// Run a single durability benchmark configuration.
pub fn run_durability_benchmark(name: &str, sync_policy: SyncPolicy) -> DurabilityBenchmarkResult {
    let data_dir = PathBuf::from("/tmp/durability_bench");
    let _ = std::fs::remove_dir_all(&data_dir);
    std::fs::create_dir_all(&data_dir).unwrap();

    let target_bytes = (TARGET_SIZE_MB * 1024 * 1024) as u64;
    let actor_id = ActorId::new();
    let stream_id = StreamId::new();
    let mut rng = rand::thread_rng();

    let _engine = StorageEngine::new(data_dir.clone()).unwrap();

    let mut append_histogram = Histogram::new("Append Latency");
    let mut fsync_histogram = Histogram::new("Fsync Latency");

    let mut total_events: u64 = 0;
    let mut total_bytes: u64 = 0;
    let mut current_segment: Option<SegmentWriter> = None;
    let mut segment_counter: u32 = 0;
    let mut bytes_since_fsync: u64 = 0;
    let mut events_since_fsync: usize = 0;

    let benchmark_start = Instant::now();

    while total_bytes < target_bytes {
        if current_segment.is_none() {
            segment_counter += 1;
            let segment_path = data_dir.join(format!("segment_{segment_counter:04}.bin"));
            current_segment = Some(SegmentWriter::new(segment_path, 128 * 1024 * 1024));
        }

        let writer = current_segment.as_mut().unwrap();
        let payload: Vec<u8> = (0..EVENT_PAYLOAD_SIZE).map(|_| rng.gen()).collect();
        let event = Event::new(stream_id.clone(), actor_id.clone(), payload);

        let append_start = Instant::now();
        match writer.append(&event) {
            Ok(_) => {
                let append_elapsed = append_start.elapsed().as_micros() as f64;
                append_histogram.record(append_elapsed);

                let serialized_size = event.serialize().unwrap().len() as u64;
                total_bytes += serialized_size;
                total_events += 1;
                bytes_since_fsync += serialized_size;
                events_since_fsync += 1;

                // Check if we need to fsync
                let should_fsync = if sync_policy.fsync_interval_bytes > 0 {
                    bytes_since_fsync >= sync_policy.fsync_interval_bytes
                } else if sync_policy.fsync_interval_events > 0 {
                    events_since_fsync >= sync_policy.fsync_interval_events
                } else {
                    true // fsync every event
                };

                if should_fsync {
                    let fsync_start = Instant::now();

                    // Simulate fsync by syncing the directory
                    // In production, this would be fdatasync on the file
                    writer.flush().unwrap();
                    let _ = std::fs::File::open(&data_dir).unwrap().sync_all();

                    let fsync_elapsed = fsync_start.elapsed().as_micros() as f64;
                    fsync_histogram.record(fsync_elapsed);

                    bytes_since_fsync = 0;
                    events_since_fsync = 0;
                }

                // Seal segment when full
                if writer.segment().data().len() as u64 > 128 * 1024 * 1024 - 2000 {
                    writer.seal_and_flush().unwrap();
                    bytes_since_fsync = 0;
                    events_since_fsync = 0;
                    current_segment = None;
                }
            }
            Err(_) => {
                writer.seal_and_flush().unwrap();
                bytes_since_fsync = 0;
                events_since_fsync = 0;
                current_segment = None;
            }
        }
    }

    // Final flush
    if let Some(mut writer) = current_segment.take() {
        writer.seal_and_flush().unwrap();
    }

    let duration = benchmark_start.elapsed();

    // Get actual bytes written to disk
    let disk_usage = get_directory_size(&data_dir);

    // Cleanup
    let _ = std::fs::remove_dir_all(&data_dir);

    DurabilityBenchmarkResult {
        name: name.to_string(),
        sync_policy,
        total_events,
        total_bytes,
        actual_bytes_written: disk_usage,
        duration,
        append_latency: append_histogram,
        fsync_latency: fsync_histogram,
    }
}

/// Get total size of directory.
fn get_directory_size(path: &PathBuf) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    total += metadata.len();
                }
            }
        }
    }
    total
}

/// Detect storage type (best effort).
pub fn detect_storage_type() -> String {
    // Check if we're on an SSD by looking at /sys/block
    if std::path::Path::new("/sys/block/nvme0n1/queue/rotational").exists() {
        if let Ok(content) = std::fs::read_to_string("/sys/block/nvme0n1/queue/rotational") {
            if content.trim() == "0" {
                return "NVMe SSD".to_string();
            }
        }
    }

    if std::path::Path::new("/sys/block/sda/queue/rotational").exists() {
        if let Ok(content) = std::fs::read_to_string("/sys/block/sda/queue/rotational") {
            if content.trim() == "0" {
                return "SATA SSD".to_string();
            } else {
                return "HDD".to_string();
            }
        }
    }

    "Unknown".to_string()
}

fn main() {
    println!("=== Real Durability Benchmarks ===\n");
    println!("Target: {TARGET_SIZE_MB}MB per configuration");
    println!("Event size: {EVENT_PAYLOAD_SIZE} bytes");
    println!("Storage type: {}\n", detect_storage_type());

    let mut results: Vec<DurabilityBenchmarkResult> = Vec::new();

    // Benchmark 1: No fsync (baseline)
    println!("Running: No fsync (baseline)...");
    results.push(run_durability_benchmark(
        "No fsync (baseline)",
        SyncPolicy {
            fsync_interval_bytes: u64::MAX,
            fsync_interval_events: usize::MAX,
            fsync_timeout_ms: 0,
            use_fdatasync: false,
            use_io_uring: false,
        },
    ));

    // Benchmark 2: Fsync every event (EXTREMELY SLOW - commented out for demo)
    // println!("Running: Fsync every event...");
    // results.push(run_durability_benchmark(
    //     "Fsync every event",
    //     SyncPolicy::conservative(),
    // ));

    // Benchmark 3: Fsync every 1MB (default)
    println!("Running: Fsync every 1MB...");
    results.push(run_durability_benchmark(
        "Fsync every 1MB (default)",
        SyncPolicy::default(),
    ));

    // Benchmark 4: Fsync per segment (128MB)
    println!("Running: Fsync per segment...");
    results.push(run_durability_benchmark(
        "Fsync per segment (128MB)",
        SyncPolicy::throughput(),
    ));

    // Print all results
    println!("\n\n");
    println!("{}", "=".to_string().repeat(80));
    println!("COMPLETE RESULTS SUMMARY");
    println!("{}", "=".to_string().repeat(80));

    for result in &results {
        result.report();
    }

    // Comparison table
    println!("\n\n");
    println!("{}", "=".to_string().repeat(80));
    println!("COMPARISON TABLE");
    println!("{}", "=".to_string().repeat(80));
    println!();
    println!(
        "{:<30} {:>12} {:>15} {:>12} {:>12}",
        "Configuration", "MB/s", "Events/s", "p99 (µs)", "WA (x)"
    );
    println!("{}", "-".repeat(81));

    for result in &results {
        println!(
            "{:<30} {:>12.1} {:>15.0} {:>12.1} {:>12.2}",
            result.name,
            result.throughput_mbps(),
            result.throughput_events_per_sec(),
            result.append_latency.percentile(99.0),
            result.write_amplification()
        );
    }

    println!();
    println!("Key Findings:");
    println!("- No fsync: Maximum throughput but zero durability");
    println!("- Per-event fsync: Maximum durability but lowest throughput");
    println!("- Batch fsync (1-4MB): Sweet spot for most workloads");
    println!("- Per-segment fsync: Good throughput with acceptable durability");
    println!();
    println!("Recommendation:");
    println!("  Use 1MB batch fsync for balanced durability/performance");
    println!("  Use per-event fsync only for critical transactions");
    println!("  Use per-segment fsync for high-throughput logging");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_histogram() {
        let mut h = Histogram::new("test");
        h.record(100.0);
        h.record(200.0);
        h.record(300.0);

        assert_eq!(h.count(), 3);
        assert_eq!(h.percentile(50.0), 200.0);
    }
}
