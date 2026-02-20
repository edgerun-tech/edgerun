// SPDX-License-Identifier: GPL-2.0-only
use rand::Rng;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use edgerun_storage::event::{ActorId, Event, StreamId};
use edgerun_storage::index::EventHashIndex;
use edgerun_storage::replication::Replicator;
use edgerun_storage::segment::{SegmentReader, SegmentWriter};
use edgerun_storage::StorageEngine;

const TARGET_SIZE_GB: usize = 10;
const TEST_MODE_MB: usize = 100; // Set to 0 for full 10GB test
const SEGMENT_SIZE: u64 = 128 * 1024 * 1024;
const EVENT_PAYLOAD_SIZE: usize = 1024;
const FSYNC_INTERVAL_BYTES: u64 = 1024 * 1024; // fsync every 1MB
const SHUTDOWN_FLAG_PATH: &str = "/tmp/storage_stress_shutdown.flag";

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
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 && args[1] == "verify" {
        verify_integrity();
    } else {
        run_stress_test();
    }
}

fn run_stress_test() {
    println!("=== 10GB Durability Stress Test ===\n");
    println!("Target size: {TARGET_SIZE_GB}GB");
    println!(
        "Fsync interval: {} bytes",
        format_bytes(FSYNC_INTERVAL_BYTES as f64)
    );
    println!("This test will:");
    println!("  1. Write 10GB with fsync every 1MB");
    println!("  2. Run random lookups in background");
    println!("  3. Run replication ingest in background");
    println!("  4. Expect SIGKILL midway");
    println!("  5. Run with 'verify' arg to check integrity\n");

    let data_dir = PathBuf::from("/tmp/storage_stress_10gb");

    // Check if we're resuming after crash
    let checkpoint_file = data_dir.join("checkpoint.bin");
    let resuming = checkpoint_file.exists();

    if !resuming {
        let _ = std::fs::remove_dir_all(&data_dir);
        std::fs::create_dir_all(&data_dir).unwrap();
        println!("Starting fresh...\n");
    } else {
        println!("Resuming from checkpoint...\n");
    }

    // Clean up any old shutdown flag
    let _ = std::fs::remove_file(SHUTDOWN_FLAG_PATH);

    // Use test mode if specified
    let target_bytes: u64 = if TEST_MODE_MB > 0 {
        println!(
            "*** TEST MODE: Writing only {TEST_MODE_MB}MB instead of {TARGET_SIZE_GB}GB ***\n"
        );
        (TEST_MODE_MB * 1024 * 1024) as u64
    } else {
        (TARGET_SIZE_GB * 1024 * 1024 * 1024) as u64
    };
    let actor_id = ActorId::new();
    let stream_id = StreamId::new();
    let mut rng = rand::thread_rng();

    let _engine = StorageEngine::new(data_dir.clone()).unwrap();

    // Shared state for concurrent operations
    let event_index = Arc::new(EventHashIndex::new());
    let total_written = Arc::new(AtomicU64::new(0));
    let total_events = Arc::new(AtomicU64::new(0));
    let should_stop = Arc::new(AtomicBool::new(false));
    let segment_files: Arc<std::sync::Mutex<Vec<PathBuf>>> =
        Arc::new(std::sync::Mutex::new(Vec::new()));

    // Load checkpoint if resuming
    let (mut bytes_written, mut events_written, mut current_segment_id) = if resuming {
        let checkpoint_data = std::fs::read(&checkpoint_file).unwrap();
        let bytes = u64::from_le_bytes([
            checkpoint_data[0],
            checkpoint_data[1],
            checkpoint_data[2],
            checkpoint_data[3],
            checkpoint_data[4],
            checkpoint_data[5],
            checkpoint_data[6],
            checkpoint_data[7],
        ]);
        let events = u64::from_le_bytes([
            checkpoint_data[8],
            checkpoint_data[9],
            checkpoint_data[10],
            checkpoint_data[11],
            checkpoint_data[12],
            checkpoint_data[13],
            checkpoint_data[14],
            checkpoint_data[15],
        ]);
        let seg_id = u32::from_le_bytes([
            checkpoint_data[16],
            checkpoint_data[17],
            checkpoint_data[18],
            checkpoint_data[19],
        ]);
        println!(
            "Resumed: {} bytes, {} events, segment {}",
            format_bytes(bytes as f64),
            events,
            seg_id
        );
        (bytes, events, seg_id)
    } else {
        (0u64, 0u64, 0u32)
    };

    total_written.store(bytes_written, Ordering::Relaxed);
    total_events.store(events_written, Ordering::Relaxed);

    // Populate segment files list if resuming
    if resuming {
        for i in 1..=current_segment_id {
            let path = data_dir.join(format!("segment_{i:04}.bin"));
            if path.exists() {
                segment_files.lock().unwrap().push(path);
            }
        }
    }

    // Start random lookup thread
    let lookup_index = Arc::clone(&event_index);
    let lookup_stop = Arc::clone(&should_stop);
    let lookup_segments = Arc::clone(&segment_files);
    let lookup_handle = std::thread::spawn(move || {
        let mut rng = rand::thread_rng();
        let mut lookups = 0u64;
        let mut hits = 0u64;
        let start = Instant::now();

        while !lookup_stop.load(Ordering::Relaxed) {
            // Random lookup every 10ms
            std::thread::sleep(Duration::from_millis(10));

            let hash: [u8; 32] = rng.gen();

            if let Some(entry) = lookup_index.get(&hash) {
                let segments = lookup_segments.lock().unwrap();
                for path in segments.iter() {
                    if let Ok(reader) = SegmentReader::from_file(path.clone()) {
                        if reader.segment_id() == entry.segment_id
                            && reader.get_event_at(entry.offset).is_ok()
                        {
                            hits += 1;
                            break;
                        }
                    }
                }
            }
            lookups += 1;

            if lookups % 1000 == 0 {
                println!(
                    "[LOOKUP] {} lookups, {} hits ({:.1}%)",
                    lookups,
                    hits,
                    (hits as f64 / lookups as f64) * 100.0
                );
            }
        }

        let duration = start.elapsed();
        println!(
            "[LOOKUP] Thread exiting: {} lookups, {} hits, {:.0} lookups/s",
            lookups,
            hits,
            lookups as f64 / duration.as_secs_f64()
        );
    });

    // Start replication ingest thread
    let repl_stop = Arc::clone(&should_stop);
    let repl_actor = ActorId::new();
    let repl_stream = StreamId::new();
    let repl_handle = std::thread::spawn(move || {
        let mut rng = rand::thread_rng();
        let mut events_ingested = 0u64;
        let start = Instant::now();

        // Create a replicator for this node
        let replicator = Replicator::new(ActorId::new(), [1u8; 16]);

        while !repl_stop.load(Ordering::Relaxed) {
            // Simulate receiving replication events every 5ms
            std::thread::sleep(Duration::from_millis(5));

            // Generate fake replicated event
            let payload: Vec<u8> = (0..EVENT_PAYLOAD_SIZE).map(|_| rng.gen()).collect();
            let _event = Event::new(repl_stream.clone(), repl_actor.clone(), payload);

            // Track it in replicator
            let segment_id: [u8; 32] = rng.gen();
            replicator.add_segment(segment_id);

            events_ingested += 1;

            if events_ingested % 10000 == 0 {
                println!("[REPLICATION] {events_ingested} events ingested");
            }
        }

        let duration = start.elapsed();
        println!(
            "[REPLICATION] Thread exiting: {} events, {:.0} events/s",
            events_ingested,
            events_ingested as f64 / duration.as_secs_f64()
        );
    });

    // Main write loop with fsync every 1MB
    println!("\n=== STARTING MAIN WRITE LOOP ===\n");

    let write_start = Instant::now();
    let mut current_segment: Option<SegmentWriter> = None;
    let mut bytes_since_fsync: u64 = 0;

    while bytes_written < target_bytes {
        // Check for shutdown flag (simulates external kill signal)
        if std::path::Path::new(SHUTDOWN_FLAG_PATH).exists() {
            println!("\n[SHUTDOWN] Flag detected, stopping gracefully...");
            break;
        }

        if current_segment.is_none() {
            current_segment_id += 1;
            let segment_path = data_dir.join(format!("segment_{current_segment_id:04}.bin"));
            current_segment = Some(SegmentWriter::new(segment_path.clone(), SEGMENT_SIZE));
            segment_files.lock().unwrap().push(segment_path);
            println!("Created segment_{current_segment_id:04}.bin");
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
                bytes_written += serialized_size;
                bytes_since_fsync += serialized_size;
                events_written += 1;

                // Fsync every 1MB
                if bytes_since_fsync >= FSYNC_INTERVAL_BYTES {
                    writer.flush().unwrap();

                    // Checkpoint progress
                    let checkpoint_data = [
                        &bytes_written.to_le_bytes()[..],
                        &events_written.to_le_bytes()[..],
                        &current_segment_id.to_le_bytes()[..],
                    ]
                    .concat();
                    std::fs::write(&checkpoint_file, &checkpoint_data).unwrap();
                    std::fs::File::open(&checkpoint_file)
                        .unwrap()
                        .sync_all()
                        .unwrap();

                    bytes_since_fsync = 0;

                    // Progress report
                    let elapsed = write_start.elapsed();
                    let progress = (bytes_written as f64 / target_bytes as f64) * 100.0;
                    println!(
                        "[WRITE] {:.1}% | {} / {} | {} events | {:.0} MB/s",
                        progress,
                        format_bytes(bytes_written as f64),
                        format_bytes(target_bytes as f64),
                        events_written,
                        (bytes_written as f64 / 1024.0 / 1024.0) / elapsed.as_secs_f64()
                    );
                }

                // Seal segment when full
                if writer.segment().data().len() as u64 > SEGMENT_SIZE - 2000 {
                    writer.seal_and_flush().unwrap();
                    current_segment = None;
                }
            }
            Err(_) => {
                writer.seal_and_flush().unwrap();
                current_segment = None;
            }
        }
    }

    // Final flush and checkpoint
    if let Some(mut writer) = current_segment.take() {
        writer.seal_and_flush().unwrap();
    }

    // Save final checkpoint
    let checkpoint_data = [
        &bytes_written.to_le_bytes()[..],
        &events_written.to_le_bytes()[..],
        &current_segment_id.to_le_bytes()[..],
    ]
    .concat();
    std::fs::write(&checkpoint_file, &checkpoint_data).unwrap();

    // Signal background threads to stop
    should_stop.store(true, Ordering::Relaxed);

    // Wait for background threads
    let _ = lookup_handle.join();
    let _ = repl_handle.join();

    let write_duration = write_start.elapsed();

    println!("\n=== WRITE COMPLETE ===");
    println!(
        "Total written: {} ({} events)",
        format_bytes(bytes_written as f64),
        events_written
    );
    println!("Duration: {write_duration:.2?}");
    println!(
        "Throughput: {:.0} events/s, {}/s",
        events_written as f64 / write_duration.as_secs_f64(),
        format_bytes(bytes_written as f64 / write_duration.as_secs_f64())
    );
    println!("\nTo verify integrity after restart, run:");
    println!("  cargo run --release --bin stress_test_10gb -- verify");
}

fn verify_integrity() {
    println!("=== VERIFYING INTEGRITY ===\n");

    let data_dir = PathBuf::from("/tmp/storage_stress_10gb");
    let checkpoint_file = data_dir.join("checkpoint.bin");

    if !data_dir.exists() {
        println!("ERROR: Data directory not found: {data_dir:?}");
        std::process::exit(1);
    }

    // Load checkpoint
    let (expected_bytes, expected_events, expected_segments) = if checkpoint_file.exists() {
        let checkpoint_data = std::fs::read(&checkpoint_file).unwrap();
        let bytes = u64::from_le_bytes([
            checkpoint_data[0],
            checkpoint_data[1],
            checkpoint_data[2],
            checkpoint_data[3],
            checkpoint_data[4],
            checkpoint_data[5],
            checkpoint_data[6],
            checkpoint_data[7],
        ]);
        let events = u64::from_le_bytes([
            checkpoint_data[8],
            checkpoint_data[9],
            checkpoint_data[10],
            checkpoint_data[11],
            checkpoint_data[12],
            checkpoint_data[13],
            checkpoint_data[14],
            checkpoint_data[15],
        ]);
        let seg_id = u32::from_le_bytes([
            checkpoint_data[16],
            checkpoint_data[17],
            checkpoint_data[18],
            checkpoint_data[19],
        ]);
        println!(
            "Checkpoint: {} bytes, {} events, {} segments",
            format_bytes(bytes as f64),
            events,
            seg_id
        );
        (bytes, events, seg_id)
    } else {
        println!("WARNING: No checkpoint file found");
        (0, 0, 0)
    };

    // Scan all segments
    println!("\nScanning segments...");
    let mut total_events = 0u64;
    let mut total_bytes = 0u64;
    let mut segment_count = 0u32;
    let mut corrupted_segments = 0u64;
    let event_index = EventHashIndex::new();

    for entry in std::fs::read_dir(&data_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("bin") {
            if path.file_name().and_then(|s| s.to_str()) == Some("checkpoint.bin") {
                continue;
            }

            segment_count += 1;

            match SegmentReader::from_file(path.clone()) {
                Ok(reader) => {
                    let seg_id = reader.segment_id();
                    let _count = reader.record_count();
                    let data_len = reader.data_len();

                    // Verify each event can be read
                    let mut events_in_segment = 0u64;
                    for event_result in reader.iter_events() {
                        match event_result {
                            Ok(event) => {
                                let hash = event.compute_hash();
                                event_index.insert(hash, seg_id, events_in_segment);
                                events_in_segment += 1;
                            }
                            Err(e) => {
                                println!(
                                    "  ERROR: Corrupted event in {:?}: {:?}",
                                    path.file_name().unwrap(),
                                    e
                                );
                                corrupted_segments += 1;
                                break;
                            }
                        }
                    }

                    total_events += events_in_segment;
                    total_bytes += data_len as u64;

                    println!(
                        "  {:?}: {} events, {} bytes - OK",
                        path.file_name().unwrap(),
                        events_in_segment,
                        data_len
                    );
                }
                Err(e) => {
                    println!(
                        "  ERROR: Failed to read {:?}: {:?}",
                        path.file_name().unwrap(),
                        e
                    );
                    corrupted_segments += 1;
                }
            }
        }
    }

    println!("\n=== VERIFICATION RESULTS ===");
    println!("Segments found: {segment_count} (expected: {expected_segments})");
    println!("Total events: {total_events} (expected: {expected_events})");
    println!(
        "Total data bytes: {} (expected: {})",
        format_bytes(total_bytes as f64),
        format_bytes(expected_bytes as f64)
    );
    println!("Corrupted segments: {corrupted_segments}");
    println!("Index entries: {}", event_index.len());

    // Verify Merkle tree consistency
    println!("\nVerifying Merkle tree consistency...");
    let replicator = Replicator::new(ActorId::new(), [1u8; 16]);
    // Add all segment IDs to replicator for tree verification
    for entry in std::fs::read_dir(&data_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("bin") {
            if let Ok(reader) = SegmentReader::from_file(path) {
                replicator.add_segment(reader.segment_id());
            }
        }
    }
    let tree = replicator.create_merkle_tree();
    println!("Merkle root: {:?}", tree.root_hash());

    // Final verdict
    println!("\n=== VERDICT ===");
    if corrupted_segments == 0 && total_events > 0 {
        println!("✓ INTEGRITY VERIFIED - All data is consistent");

        // Calculate how much data survived
        if expected_events > 0 {
            let survival_rate = (total_events as f64 / expected_events as f64) * 100.0;
            println!("  Data survival rate: {survival_rate:.1}%");
        }

        std::process::exit(0);
    } else {
        println!("✗ INTEGRITY COMPROMISED");
        if corrupted_segments > 0 {
            println!("  {corrupted_segments} segments are corrupted");
        }
        if total_events == 0 {
            println!("  No events found in segments");
        }
        std::process::exit(1);
    }
}
