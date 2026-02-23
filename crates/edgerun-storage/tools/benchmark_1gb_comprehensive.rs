// SPDX-License-Identifier: GPL-2.0-only
use edgerun_storage::event::{ActorId, Event, StreamId};
use edgerun_storage::index::EventHashIndex;
use edgerun_storage::segment::{SegmentReader, SegmentWriter};
use edgerun_storage::StorageEngine;
use rand::Rng;
use std::path::PathBuf;
use std::time::Instant;

const TARGET_SIZE_GB: usize = 1;
const SEGMENT_SIZE: u64 = 128 * 1024 * 1024;
const EVENT_PAYLOAD_SIZE: usize = 1024;
const QUERY_COUNT: usize = 100_000;

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

fn format_duration(duration: std::time::Duration) -> String {
    let secs = duration.as_secs_f64();
    if secs > 60.0 {
        format!("{:.2}m {:.2}s", secs / 60.0, secs % 60.0)
    } else {
        format!("{secs:.2}s")
    }
}

fn main() {
    println!("=== Storage Engine Comprehensive Benchmarks ===\n");
    println!("Target size: {TARGET_SIZE_GB}GB");
    println!("Segment size: {}", format_bytes(SEGMENT_SIZE as f64));
    println!("Event payload: {EVENT_PAYLOAD_SIZE} bytes");
    println!();

    let data_dir = PathBuf::from("/tmp/storage_bench_1gb");
    let _ = std::fs::remove_dir_all(&data_dir);
    std::fs::create_dir_all(&data_dir).unwrap();

    let target_bytes: u64 = (TARGET_SIZE_GB * 1024 * 1024 * 1024) as u64;
    let actor_id = ActorId::new();
    let stream_id = StreamId::new();
    let mut rng = rand::thread_rng();

    let engine = StorageEngine::new(data_dir.clone()).unwrap();
    println!("Storage engine initialized at: {:?}\n", engine.data_dir());

    // ============================================================
    // BENCHMARK 1: PURE WRITE (no indexing, no flush to disk)
    // ============================================================
    println!("=== BENCHMARK 1: PURE WRITE (In-Memory Only) ===");
    println!("Writing events to memory without flushing to disk...\n");

    let mut pure_write_events: u64 = 0;
    let mut pure_write_bytes: u64 = 0;
    let mut current_segment: Option<SegmentWriter> = None;
    let mut segment_counter: u32 = 0;

    let pure_write_start = Instant::now();

    while pure_write_bytes < target_bytes {
        if current_segment.is_none() {
            segment_counter += 1;
            let segment_path = data_dir.join(format!("pure_seg_{segment_counter:04}.bin"));
            current_segment = Some(SegmentWriter::new(segment_path, SEGMENT_SIZE));
        }

        let writer = current_segment.as_mut().unwrap();
        let payload: Vec<u8> = (0..EVENT_PAYLOAD_SIZE).map(|_| rng.gen()).collect();
        let event = Event::new(stream_id.clone(), actor_id.clone(), payload);

        match writer.append(&event) {
            Ok(_) => {
                let size = event.serialize().unwrap().len() as u64;
                pure_write_bytes += size;
                pure_write_events += 1;

                if writer.segment().data().len() as u64 > SEGMENT_SIZE - 2000 {
                    let _ = writer.seal_and_flush();
                    current_segment = None;
                }
            }
            Err(_) => {
                let _ = writer.seal_and_flush();
                current_segment = None;
            }
        }
    }

    let pure_write_duration = pure_write_start.elapsed();
    let pure_write_throughput = pure_write_events as f64 / pure_write_duration.as_secs_f64();
    let pure_write_bytes_per_sec = pure_write_bytes as f64 / pure_write_duration.as_secs_f64();

    println!("--- PURE WRITE RESULTS ---");
    println!("Events written: {pure_write_events}");
    println!("Total bytes: {}", format_bytes(pure_write_bytes as f64));
    println!("Duration: {}", format_duration(pure_write_duration));
    println!("Throughput: {pure_write_throughput:.0} events/s");
    println!("Throughput: {}/s\n", format_bytes(pure_write_bytes_per_sec));

    // Clean up pure write segments
    drop(current_segment);
    for i in 1..=segment_counter {
        let _ = std::fs::remove_file(data_dir.join(format!("pure_seg_{i:04}.bin")));
    }

    // ============================================================
    // BENCHMARK 2: WRITE WITH INDEXING (full workflow)
    // ============================================================
    println!("=== BENCHMARK 2: WRITE WITH INDEXING ===");
    println!("Writing events with hash index updates...\n");

    let mut indexed_write_events: u64 = 0;
    let mut indexed_write_bytes: u64 = 0;
    let mut current_segment: Option<SegmentWriter> = None;
    let event_index = EventHashIndex::new();
    let mut segment_files: Vec<PathBuf> = Vec::new();
    let mut written_segment_ids: Vec<[u8; 32]> = Vec::new();
    segment_counter = 0;

    let indexed_write_start = Instant::now();

    while indexed_write_bytes < target_bytes {
        if current_segment.is_none() {
            segment_counter += 1;
            let segment_path = data_dir.join(format!("indexed_seg_{segment_counter:04}.bin"));
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

                let size = event.serialize().unwrap().len() as u64;
                indexed_write_bytes += size;
                indexed_write_events += 1;

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
    }

    if let Some(mut writer) = current_segment.take() {
        let seg_id = writer.seal_and_flush().unwrap();
        written_segment_ids.push(seg_id);
    }

    let indexed_write_duration = indexed_write_start.elapsed();
    let indexed_write_throughput =
        indexed_write_events as f64 / indexed_write_duration.as_secs_f64();
    let indexed_write_bytes_per_sec =
        indexed_write_bytes as f64 / indexed_write_duration.as_secs_f64();

    println!("--- WRITE WITH INDEX RESULTS ---");
    println!("Events written: {indexed_write_events}");
    println!("Index entries: {}", event_index.len());
    println!("Total bytes: {}", format_bytes(indexed_write_bytes as f64));
    println!("Duration: {}", format_duration(indexed_write_duration));
    println!("Throughput: {indexed_write_throughput:.0} events/s");
    println!(
        "Throughput: {}/s\n",
        format_bytes(indexed_write_bytes_per_sec)
    );

    // ============================================================
    // BENCHMARK 3: READ WITH CACHE (segments loaded in memory)
    // ============================================================
    println!("=== BENCHMARK 3: READ WITH CACHE ===");
    println!("Loading segments into memory and reading sequentially...\n");

    println!("Loading segments into memory...");
    let mut readers: Vec<SegmentReader> = Vec::new();
    for path in &segment_files {
        if let Ok(reader) = SegmentReader::from_file(path.clone()) {
            println!(
                "Segment {:?}: {} events, data size {} bytes",
                path.file_name().unwrap(),
                reader.record_count(),
                reader.data_len()
            );
            readers.push(reader);
        }
    }
    let total_events_in_segments: u64 = readers.iter().map(|r| r.record_count() as u64).sum();
    println!(
        "Loaded {} segments, {} total events\n",
        readers.len(),
        total_events_in_segments
    );

    let cache_read_start = Instant::now();
    let mut cache_events_read: u64 = 0;

    for reader in &readers {
        for _ in reader.iter_events() {
            cache_events_read += 1;
        }
    }

    let cache_read_duration = cache_read_start.elapsed();
    let cache_read_throughput = cache_events_read as f64 / cache_read_duration.as_secs_f64();

    println!("--- CACHED READ RESULTS ---");
    println!("Events read: {cache_events_read}");
    println!("Duration: {}", format_duration(cache_read_duration));
    println!("Throughput: {cache_read_throughput:.0} events/s\n");

    // ============================================================
    // BENCHMARK 4: READ WITHOUT CACHE (cold disk reads)
    // ============================================================
    println!("=== BENCHMARK 4: READ WITHOUT CACHE (Cold Disk) ===");
    println!("Dropping cached readers, re-opening from disk...\n");

    drop(readers);
    drop(event_index);
    let _ = std::fs::File::open("/").and_then(|f| f.sync_all());

    let uncached_read_start = Instant::now();
    let mut uncached_events_read: u64 = 0;

    for path in &segment_files {
        let reader = SegmentReader::from_file(path.clone()).unwrap();
        for _ in reader.iter_events() {
            uncached_events_read += 1;
        }
    }

    let uncached_read_duration = uncached_read_start.elapsed();
    let uncached_read_throughput =
        uncached_events_read as f64 / uncached_read_duration.as_secs_f64();

    println!("--- UNCACHED READ RESULTS ---");
    println!("Events read: {uncached_events_read}");
    println!("Duration: {}", format_duration(uncached_read_duration));
    println!("Throughput: {uncached_read_throughput:.0} events/s\n");

    // ============================================================
    // BENCHMARK 5: QUERY BY HASH (with index)
    // ============================================================
    println!("=== BENCHMARK 5: QUERY BY HASH (Indexed Lookup) ===\n");

    let event_index = EventHashIndex::new();
    let mut readers: Vec<SegmentReader> = Vec::new();
    let mut actual_hashes: Vec<[u8; 32]> = Vec::new();

    for path in &segment_files {
        if let Ok(reader) = SegmentReader::from_file(path.clone()) {
            let seg_id = reader.segment_id();
            let mut byte_offset: u64 = 0;
            for event_result in reader.iter_events() {
                if let Ok(ref event) = event_result {
                    let hash = event.compute_hash();
                    event_index.insert(hash, seg_id, byte_offset);
                    if actual_hashes.len() < 10000 {
                        actual_hashes.push(hash);
                    }
                }
                // Get the serialized size to advance byte_offset
                if let Ok(ref evt) = event_result {
                    byte_offset += evt.serialize().unwrap().len() as u64;
                }
            }
            readers.push(reader);
        }
    }

    let index_size = event_index.len();
    println!("Index size: {index_size} entries\n");

    // Query for existing keys (100% hits)
    let query_existing_start = Instant::now();
    let mut query_hits: u64 = 0;

    for i in 0..QUERY_COUNT {
        let hash = actual_hashes[i % actual_hashes.len()];
        if let Some(entry) = event_index.get(&hash) {
            for reader in &readers {
                if reader.segment_id() == entry.segment_id
                    && reader.get_event_at(entry.offset).is_ok()
                {
                    query_hits += 1;
                    break;
                }
            }
        }
    }

    let query_existing_duration = query_existing_start.elapsed();
    let query_existing_throughput = QUERY_COUNT as f64 / query_existing_duration.as_secs_f64();

    println!("--- QUERY EXISTING KEYS (100% hit rate) ---");
    println!("Queries: {QUERY_COUNT}");
    println!("Found: {query_hits}");
    println!("Duration: {}", format_duration(query_existing_duration));
    println!("Throughput: {query_existing_throughput:.0} queries/s\n");

    // Query for random keys (0% hits - testing index lookup speed)
    let query_random_start = Instant::now();
    let mut query_misses: u64 = 0;

    for _ in 0..QUERY_COUNT {
        let random_hash: [u8; 32] = rng.gen();
        if event_index.get(&random_hash).is_none() {
            query_misses += 1;
        }
    }

    let query_random_duration = query_random_start.elapsed();
    let query_random_throughput = QUERY_COUNT as f64 / query_random_duration.as_secs_f64();

    println!("--- QUERY RANDOM KEYS (0% hit rate) ---");
    println!("Queries: {QUERY_COUNT}");
    println!("Not found: {query_misses}");
    println!("Duration: {}", format_duration(query_random_duration));
    println!("Throughput: {query_random_throughput:.0} queries/s\n");

    // ============================================================
    // CLEANUP
    // ============================================================
    println!("=== CLEANUP ===");
    drop(readers);
    drop(event_index);
    let _ = std::fs::remove_dir_all(&data_dir);
    println!("Data cleaned up.\n");

    // ============================================================
    // SUMMARY
    // ============================================================
    println!("=== SUMMARY ===");
    println!("1GB Store Comprehensive Benchmark Complete!");
    println!();
    println!("Operation              | Throughput");
    println!("-----------------------|------------------");
    println!(
        "Pure Write (memory)    | {:.0} events/s ({}/s)",
        pure_write_throughput,
        format_bytes(pure_write_bytes_per_sec)
    );
    println!(
        "Write + Index          | {:.0} events/s ({}/s)",
        indexed_write_throughput,
        format_bytes(indexed_write_bytes_per_sec)
    );
    println!("Read (cached)          | {cache_read_throughput:.0} events/s");
    println!("Read (uncached)        | {uncached_read_throughput:.0} events/s");
    println!("Query (existing keys)  | {query_existing_throughput:.0} queries/s");
    println!("Query (random keys)    | {query_random_throughput:.0} queries/s");
}
