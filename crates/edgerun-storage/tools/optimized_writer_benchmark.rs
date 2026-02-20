// SPDX-License-Identifier: GPL-2.0-only
use std::time::Instant;
use storage_engine::optimized_writer::{OptimizedSegmentWriter, OptimizedSegmentWriterConfig};

fn main() {
    println!("=== Optimized Segment Writer Benchmark ===\n");

    let config = OptimizedSegmentWriterConfig {
        num_cores: 8,
        buffer_size: 512 * 1024 * 1024,
        use_io_uring: false,
        batch_size: 512 * 1024 * 1024,
    };

    let path = std::env::temp_dir().join("benchmark_segment");

    println!(
        "Config: {} cores, {} buffer size\n",
        config.num_cores, config.buffer_size
    );

    let num_events = 1_000_000;
    let payload_size = 256;

    let mut writer = OptimizedSegmentWriter::new(path.clone(), config);
    let stream_id = storage_engine::event::StreamId::new();
    let actor_id = storage_engine::event::ActorId::new();

    println!(
        "Writing {num_events} events ({payload_size} bytes each)..."
    );

    let start = Instant::now();

    for _ in 0..num_events {
        let payload = vec![0u8; payload_size];
        writer.append(&stream_id, &actor_id, payload).unwrap();
    }

    let write_duration = start.elapsed();
    let write_throughput = num_events as f64 / write_duration.as_secs_f64();
    let bytes_written = writer.bytes_written();
    let mb_written = bytes_written as f64 / (1024.0 * 1024.0);
    let mb_per_sec = mb_written / write_duration.as_secs_f64();

    println!("Write phase:");
    println!("  Duration: {write_duration:.2?}");
    println!("  Events/s: {write_throughput:.0}");
    println!("  Throughput: {mb_per_sec:.1} MB/s");

    let seal_start = Instant::now();
    writer.seal().unwrap();
    let seal_duration = seal_start.elapsed();

    println!("\nSeal phase:");
    println!("  Duration: {seal_duration:.2?}");

    let flush_start = Instant::now();
    writer.flush().unwrap();
    let flush_duration = flush_start.elapsed();

    println!("\nFlush phase:");
    println!("  Duration: {flush_duration:.2?}");

    let total_duration = write_duration + seal_duration + flush_duration;
    let total_mb_per_sec = mb_written / total_duration.as_secs_f64();

    println!("\n=== Total ===");
    println!("  Total duration: {total_duration:.2?}");
    println!("  Total throughput: {total_mb_per_sec:.1} MB/s");
    println!("  Events written: {}", writer.events_written());
    println!("  Segments sealed: {}", writer.sealed_count());
}
