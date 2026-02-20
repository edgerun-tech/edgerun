// SPDX-License-Identifier: GPL-2.0-only
use crossbeam_channel::bounded;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Instant;
use storage_engine::async_segment_writer::AsyncSegmentWriterFactory;
use storage_engine::event::{ActorId, Event, HlcTimestamp, StreamId};
use storage_engine::io_reactor::IoReactorStats;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    IoOnly,
    EndToEnd,
    Checkpoint,
    Both,
}

impl Mode {
    fn as_str(self) -> &'static str {
        match self {
            Mode::IoOnly => "io_only",
            Mode::EndToEnd => "end_to_end",
            Mode::Checkpoint => "checkpoint",
            Mode::Both => "both",
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Config {
    mode: Mode,
    events: usize,
    payload_size: usize,
    flush_interval: usize,
    producers: usize,
    producer_queue_depth: usize,
    direct_io: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mode: Mode::Both,
            events: 100_000,
            payload_size: 1024,
            flush_interval: 10_000,
            producers: 1,
            producer_queue_depth: 8192,
            direct_io: false,
        }
    }
}

fn parse_args() -> Config {
    let mut cfg = Config::default();
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--mode" => {
                if let Some(v) = args.next() {
                    cfg.mode = match v.as_str() {
                        "io_only" => Mode::IoOnly,
                        "end_to_end" => Mode::EndToEnd,
                        "checkpoint" => Mode::Checkpoint,
                        "both" => Mode::Both,
                        _ => {
                            eprintln!("Invalid --mode value: {v}");
                            print_usage_and_exit();
                        }
                    };
                } else {
                    print_usage_and_exit();
                }
            }
            "--events" => {
                cfg.events = parse_usize_arg(args.next(), "--events");
            }
            "--payload-size" => {
                cfg.payload_size = parse_usize_arg(args.next(), "--payload-size");
            }
            "--flush-interval" => {
                cfg.flush_interval = parse_usize_arg(args.next(), "--flush-interval");
            }
            "--producers" => {
                cfg.producers = parse_usize_arg(args.next(), "--producers");
            }
            "--producer-queue-depth" => {
                cfg.producer_queue_depth = parse_usize_arg(args.next(), "--producer-queue-depth");
            }
            "--direct-io" => {
                cfg.direct_io = true;
            }
            "--help" | "-h" => print_usage_and_exit(),
            _ => {
                eprintln!("Unknown argument: {arg}");
                print_usage_and_exit();
            }
        }
    }

    if cfg.events == 0
        || cfg.payload_size == 0
        || cfg.flush_interval == 0
        || cfg.producers == 0
        || cfg.producer_queue_depth == 0
    {
        eprintln!(
            "--events, --payload-size, --flush-interval, --producers, and --producer-queue-depth must be > 0"
        );
        std::process::exit(2);
    }

    cfg
}

fn parse_usize_arg(value: Option<String>, name: &str) -> usize {
    let Some(v) = value else {
        eprintln!("Missing value for {name}");
        print_usage_and_exit();
    };

    match v.parse::<usize>() {
        Ok(parsed) => parsed,
        Err(_) => {
            eprintln!("Invalid value for {name}: {v}");
            print_usage_and_exit();
        }
    }
}

fn print_usage_and_exit() -> ! {
    eprintln!(
        "Usage: async_writer_benchmark [--mode io_only|end_to_end|checkpoint|both] [--events N] [--payload-size N] [--flush-interval N] [--producers N] [--producer-queue-depth N] [--direct-io]"
    );
    std::process::exit(2);
}

struct WorkItem {
    serialized: Arc<[u8]>,
    hlc: HlcTimestamp,
}

fn stats_delta(before: &IoReactorStats, after: &IoReactorStats) -> IoReactorStats {
    IoReactorStats {
        ops_submitted: after.ops_submitted.saturating_sub(before.ops_submitted),
        ops_enqueued: after.ops_enqueued.saturating_sub(before.ops_enqueued),
        ops_completed: after.ops_completed.saturating_sub(before.ops_completed),
        writes_completed: after
            .writes_completed
            .saturating_sub(before.writes_completed),
        reads_completed: after.reads_completed.saturating_sub(before.reads_completed),
        fsyncs_completed: after
            .fsyncs_completed
            .saturating_sub(before.fsyncs_completed),
        bytes_written: after.bytes_written.saturating_sub(before.bytes_written),
        bytes_read: after.bytes_read.saturating_sub(before.bytes_read),
        errors: after.errors.saturating_sub(before.errors),
        avg_batch_size: after.avg_batch_size,
        current_inflight: after.current_inflight,
        max_inflight: after.max_inflight.saturating_sub(before.max_inflight),
        cqe_drain_calls: after.cqe_drain_calls.saturating_sub(before.cqe_drain_calls),
        cqe_drained_total: after
            .cqe_drained_total
            .saturating_sub(before.cqe_drained_total),
        queue_backpressure_events: after
            .queue_backpressure_events
            .saturating_sub(before.queue_backpressure_events),
    }
}

fn run_mode(
    mode: Mode,
    cfg: Config,
    factory: &AsyncSegmentWriterFactory,
    stream_id: &StreamId,
    actor_id: &ActorId,
) {
    let data_dir = PathBuf::from(format!("/tmp/async_bench_{}", mode.as_str()));
    let _ = std::fs::remove_dir_all(&data_dir);
    std::fs::create_dir_all(&data_dir).expect("create benchmark dir");

    println!("--- Mode: {} ---", mode.as_str());
    println!(
        "Writing {} events (payload {} bytes), flush every {} events, producers {}",
        cfg.events, cfg.payload_size, cfg.flush_interval, cfg.producers
    );

    let segment_path = data_dir.join("segment.bin");
    let mut writer = factory
        .create_writer(segment_path, 512 * 1024 * 1024)
        .expect("create writer");
    if mode == Mode::Checkpoint {
        let manifest_path = data_dir.join("manifest.json");
        writer
            .attach_manifest(manifest_path)
            .expect("attach manifest for checkpoint mode");
    }

    let template_event = Event::new(
        stream_id.clone(),
        actor_id.clone(),
        vec![0x5A; cfg.payload_size],
    );
    let serialized_template = template_event.serialize().expect("serialize template");
    let bytes_per_event = serialized_template.len();

    let stats_before = factory.stats();
    let start = Instant::now();

    let mut events_written = 0usize;
    if cfg.producers == 1 {
        for i in 0..cfg.events {
            let _offset = match mode {
                Mode::IoOnly => writer
                    .append_serialized(&serialized_template, template_event.hlc_timestamp)
                    .expect("append serialized"),
                Mode::Checkpoint => writer
                    .append_serialized(&serialized_template, template_event.hlc_timestamp)
                    .expect("append serialized"),
                Mode::EndToEnd => {
                    let payload = vec![(i % 251) as u8; cfg.payload_size];
                    let event = Event::new(stream_id.clone(), actor_id.clone(), payload);
                    writer.append(&event).expect("append event")
                }
                Mode::Both => unreachable!("run_mode should not be called with Mode::Both"),
            };

            events_written += 1;
            if events_written % cfg.flush_interval == 0 {
                match mode {
                    Mode::Checkpoint => {
                        let epoch = events_written / cfg.flush_interval;
                        let manifest = format!(
                            r#"{{"epoch":{},"events":{},"mode":"checkpoint"}}"#,
                            epoch, events_written
                        )
                        .into_bytes();
                        writer
                            .flush_checkpointed(manifest)
                            .expect("flush checkpointed");
                    }
                    _ => writer.flush_async().expect("flush async"),
                }
            }
        }
    } else {
        let (tx, rx) = bounded::<WorkItem>(cfg.producer_queue_depth);
        let mut producer_handles = Vec::with_capacity(cfg.producers);
        let events_per = cfg.events / cfg.producers;
        let remainder = cfg.events % cfg.producers;
        let io_template = Arc::<[u8]>::from(serialized_template.clone());
        let io_hlc = template_event.hlc_timestamp;

        for p in 0..cfg.producers {
            let tx = tx.clone();
            let stream_id = stream_id.clone();
            let actor_id = actor_id.clone();
            let to_send = events_per + usize::from(p < remainder);
            let payload_size = cfg.payload_size;
            let mode_local = mode;
            let template = io_template.clone();

            producer_handles.push(thread::spawn(move || {
                for i in 0..to_send {
                    let item = match mode_local {
                        Mode::IoOnly => WorkItem {
                            serialized: template.clone(),
                            hlc: io_hlc,
                        },
                        Mode::Checkpoint => WorkItem {
                            serialized: template.clone(),
                            hlc: io_hlc,
                        },
                        Mode::EndToEnd => {
                            let payload = vec![((i + p) % 251) as u8; payload_size];
                            let event = Event::new(stream_id.clone(), actor_id.clone(), payload);
                            let serialized = event.serialize().expect("producer serialize");
                            WorkItem {
                                serialized: Arc::from(serialized),
                                hlc: event.hlc_timestamp,
                            }
                        }
                        Mode::Both => unreachable!("run_mode should not be called with Mode::Both"),
                    };
                    tx.send(item).expect("send work item");
                }
            }));
        }
        drop(tx);

        while events_written < cfg.events {
            let item = rx.recv().expect("receive work item");
            writer
                .append_serialized(item.serialized.as_ref(), item.hlc)
                .expect("append serialized");
            events_written += 1;
            if events_written % cfg.flush_interval == 0 {
                match mode {
                    Mode::Checkpoint => {
                        let epoch = events_written / cfg.flush_interval;
                        let manifest = format!(
                            r#"{{"epoch":{},"events":{},"mode":"checkpoint"}}"#,
                            epoch, events_written
                        )
                        .into_bytes();
                        writer
                            .flush_checkpointed(manifest)
                            .expect("flush checkpointed");
                    }
                    _ => writer.flush_async().expect("flush async"),
                }
            }
        }

        for handle in producer_handles {
            handle.join().expect("producer thread join");
        }
    }

    match mode {
        Mode::Checkpoint => {
            let final_manifest = format!(
                r#"{{"epoch":{},"events":{},"mode":"checkpoint","final":true}}"#,
                cfg.events.div_ceil(cfg.flush_interval),
                events_written
            )
            .into_bytes();
            writer
                .flush_checkpointed(final_manifest)
                .expect("final flush checkpointed");
        }
        _ => writer.flush_blocking().expect("flush blocking"),
    }
    let elapsed = start.elapsed();

    let stats_after = factory.stats();
    let stats = stats_delta(&stats_before, &stats_after);

    let throughput_eps = cfg.events as f64 / elapsed.as_secs_f64();
    let mb_written = (cfg.events * bytes_per_event) as f64 / (1024.0 * 1024.0);
    let throughput_mb = mb_written / elapsed.as_secs_f64();

    println!("Duration: {:.2}s", elapsed.as_secs_f64());
    println!("Throughput: {:.0} events/s", throughput_eps);
    println!("Throughput: {:.2} MB/s", throughput_mb);
    println!(
        "Ops submitted/completed: {}/{}",
        stats.ops_submitted, stats.ops_completed
    );
    println!("Fsyncs completed: {}", stats.fsyncs_completed);
    println!("Max inflight ops: {}", stats.max_inflight);
    println!(
        "CQ drain calls/CQEs drained: {}/{}",
        stats.cqe_drain_calls, stats.cqe_drained_total
    );
    println!("Backpressure events: {}", stats.queue_backpressure_events);
    println!("Errors: {}", stats.errors);
    println!();

    let _ = std::fs::remove_dir_all(&data_dir);
}

fn main() {
    let cfg = parse_args();

    println!("=== Async Segment Writer Benchmark (io_uring) ===\n");
    println!("Configuration:");
    println!("  - io_uring queue depth: 1024");
    println!("  - Batch size: 128");
    println!("  - Mode: {}", cfg.mode.as_str());
    println!("  - Producers: {}", cfg.producers);
    println!("  - Producer queue depth: {}", cfg.producer_queue_depth);
    println!("  - Direct I/O: {}", cfg.direct_io);
    println!();

    let factory = if cfg.direct_io {
        AsyncSegmentWriterFactory::new_direct_io().expect("create direct-io factory")
    } else {
        AsyncSegmentWriterFactory::new().expect("create factory")
    };
    let stream_id = StreamId::new();
    let actor_id = ActorId::new();

    match cfg.mode {
        Mode::IoOnly => run_mode(Mode::IoOnly, cfg, &factory, &stream_id, &actor_id),
        Mode::EndToEnd => run_mode(Mode::EndToEnd, cfg, &factory, &stream_id, &actor_id),
        Mode::Checkpoint => run_mode(Mode::Checkpoint, cfg, &factory, &stream_id, &actor_id),
        Mode::Both => {
            run_mode(Mode::EndToEnd, cfg, &factory, &stream_id, &actor_id);
            run_mode(Mode::IoOnly, cfg, &factory, &stream_id, &actor_id);
            run_mode(Mode::Checkpoint, cfg, &factory, &stream_id, &actor_id);
        }
    }
}
