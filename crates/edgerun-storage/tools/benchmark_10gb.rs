// SPDX-License-Identifier: GPL-2.0-only
use rand::Rng;
use std::path::PathBuf;
use std::time::Instant;
use storage_engine::event::{ActorId, Event, StreamId};
use storage_engine::index::EventHashIndex;
use storage_engine::segment::{SegmentReader, SegmentWriter};
use storage_engine::StorageEngine;

const TARGET_SIZE_GB: usize = 10;
const SEGMENT_SIZE: u64 = 256 * 1024 * 1024;
const EVENT_PAYLOAD_SIZE: usize = 1024;

fn format_bytes(bytes: f64) -> String {
    if bytes > 1_099_511_627_776.0 {
        format!("{:.2} TB", bytes / 1_099_511_627_776.0)
    } else if bytes > 1_073_741_824.0 {
        format!("{:.2} GB", bytes / 1_073_741_824.0)
    } else if bytes > 1_048_576.0 {
        format!("{:.2} MB", bytes / 1_048_576.0)
    } else if bytes > 1024.0 {
        format!("{:.2} KB", bytes / 1024.0)
    } else {
        format!("{bytes:.0} B")
    }
}

fn main() {
    println!("=== Storage Engine Benchmark: 10GB Store ===\n");

    let data_dir = PathBuf::from("/tmp/storage_bench_10gb");
    let _ = std::fs::remove_dir_all(&data_dir);
    std::fs::create_dir_all(&data_dir).unwrap();

    let target_bytes = TARGET_SIZE_GB * 1024 * 1024 * 1024;

    let engine = StorageEngine::new(data_dir.clone()).unwrap();
    println!("Storage engine initialized at: {:?}", engine.data_dir());

    let actor_id = ActorId::new();
    let stream_id = StreamId::new();

    let event_index = EventHashIndex::new();

    println!("\n=== WRITE BENCHMARK ===");
    println!("Target size: {TARGET_SIZE_GB}GB");
    println!("Segment size: {}", format_bytes(SEGMENT_SIZE as f64));
    println!("Event payload: {EVENT_PAYLOAD_SIZE} bytes\n");

    let mut total_events: u64 = 0;
    let mut total_bytes: u64 = 0;
    let mut current_segment: Option<SegmentWriter> = None;
    let mut segment_files: Vec<PathBuf> = Vec::new();
    let mut written_segment_ids: Vec<[u8; 32]> = Vec::new();

    let write_start = Instant::now();
    let mut last_report = Instant::now();
    let report_interval = std::time::Duration::from_secs(5);

    let mut rng = rand::thread_rng();
    let mut segment_counter: u32 = 0;

    while total_bytes < target_bytes as u64 {
        if current_segment.is_none() {
            segment_counter += 1;
            let segment_path = data_dir.join(format!("segment_{segment_counter:04}.bin"));
            current_segment = Some(SegmentWriter::new(segment_path.clone(), SEGMENT_SIZE));
            segment_files.push(segment_path);
        }

        let writer = current_segment.as_mut().unwrap();

        let payload: Vec<u8> = (0..EVENT_PAYLOAD_SIZE).map(|_| rng.gen()).collect();

        let event = Event::new(stream_id.clone(), actor_id.clone(), payload);
        let event_hash = event.compute_hash();

        match writer.append(&event) {
            Ok(offset) => {
                let segment_id = writer.segment_id();
                event_index.insert(event_hash, segment_id, offset);

                let serialized_size = event.serialize().unwrap().len() as u64;
                total_bytes += serialized_size;
                total_events += 1;

                if writer.segment().data().len() as u64 > SEGMENT_SIZE - 2000 {
                    let seg_id = writer.seal_and_flush().unwrap();
                    written_segment_ids.push(seg_id);
                    current_segment = None;
                }
            }
            Err(_) => {
                let seg_id = writer.seal_and_flush().unwrap();
                written_segment_ids.push(seg_id);
                current_segment = None;
            }
        }

        if last_report.elapsed() >= report_interval {
            let elapsed = write_start.elapsed();
            let events_per_sec = total_events as f64 / elapsed.as_secs_f64();
            let bytes_per_sec = total_bytes as f64 / elapsed.as_secs_f64();
            let progress = (total_bytes as f64 / target_bytes as f64) * 100.0;

            println!(
                "Progress: {:.1}% | Events: {} | {} written | {:.0} events/s | {}/s",
                progress,
                total_events,
                format_bytes(total_bytes as f64),
                events_per_sec,
                format_bytes(bytes_per_sec)
            );

            last_report = Instant::now();
        }
    }

    if let Some(mut writer) = current_segment.take() {
        let seg_id = writer.seal_and_flush().unwrap();
        written_segment_ids.push(seg_id);
    }

    let write_duration = write_start.elapsed();
    let write_throughput_events = total_events as f64 / write_duration.as_secs_f64();
    let write_throughput_bytes = total_bytes as f64 / write_duration.as_secs_f64();

    println!("\n--- WRITE RESULTS ---");
    println!("Total events: {total_events}");
    println!("Total bytes: {}", format_bytes(total_bytes as f64));
    println!("Segments created: {}", segment_files.len());
    println!("Duration: {write_duration:.2?}");
    println!("Throughput: {write_throughput_events:.0} events/s");
    println!("Throughput: {}/s", format_bytes(write_throughput_bytes));

    println!("\n=== QUERY BENCHMARK (Random Lookup) ===\n");

    let index_size = event_index.len();
    println!("Index size: {index_size} entries\n");

    println!("Warming up: reading segment metadata...");
    let mut readers: Vec<SegmentReader> = Vec::new();
    for path in &segment_files {
        if let Ok(reader) = SegmentReader::from_file(path.clone()) {
            readers.push(reader);
        }
    }
    println!("Loaded {} segments into memory\n", readers.len());

    let query_count = 100_000;
    println!("Running {query_count} random queries...");

    let query_start = Instant::now();
    let mut found_count = 0;

    for i in 0..query_count {
        let random_hash: [u8; 32] = rng.gen();

        if let Some(entry) = event_index.get(&random_hash) {
            for reader in &readers {
                if reader.segment_id() == entry.segment_id
                    && reader.get_event_at(entry.offset).is_ok() {
                        found_count += 1;
                        break;
                    }
            }
        }

        if (i + 1) % 25000 == 0 {
            println!("Queries: {}/{}", i + 1, query_count);
        }
    }

    let query_duration = query_start.elapsed();
    let queries_per_sec = query_count as f64 / query_duration.as_secs_f64();

    println!("\n--- QUERY RESULTS ---");
    println!("Total queries: {query_count}");
    println!("Found (cache hits): {found_count}");
    println!("Duration: {query_duration:.2?}");
    println!("Throughput: {queries_per_sec:.0} queries/s");

    println!("\n=== SEQUENTIAL READ BENCHMARK ===\n");

    let mut seq_events_read: u64 = 0;
    let seq_read_start = Instant::now();

    for reader in &readers {
        let count = reader.record_count() as u64;
        for i in 0..count {
            if reader.get_event_at(i).is_ok() {
                seq_events_read += 1;
            }
        }
    }

    let seq_read_duration = seq_read_start.elapsed();
    let seq_read_throughput = seq_events_read as f64 / seq_read_duration.as_secs_f64();

    println!("--- SEQUENTIAL READ RESULTS ---");
    println!("Events read: {seq_events_read}");
    println!("Duration: {seq_read_duration:.2?}");
    println!("Throughput: {seq_read_throughput:.0} events/s");

    println!("\n=== CLEANUP ===");
    println!("Removing test data...");
    drop(readers);
    drop(event_index);
    let _ = std::fs::remove_dir_all(&data_dir);
    println!("Done.\n");

    println!("=== SUMMARY ===");
    println!("10GB Store Benchmark Complete!");
    println!(
        "  Write: {:.0} events/s, {}/s",
        write_throughput_events,
        format_bytes(write_throughput_bytes)
    );
    println!("  Random Query: {queries_per_sec:.0} queries/s");
    println!("  Sequential Read: {seq_read_throughput:.0} events/s");
}
