// SPDX-License-Identifier: Apache-2.0
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};

use crate::process_helpers::{
    run_program_capture_sync_owned, run_program_sync, run_program_sync_owned,
    run_program_sync_with_env,
};
use crate::{ensure, StorageCommand};

#[derive(Clone, Copy)]
struct StorageSweepGate {
    min_top_score: f64,
    min_top_writes_ops: u64,
    min_top_reads_ops: u64,
    max_top_comp_failed: u64,
}

struct StorageSweepOptions {
    duration: u64,
    out_dir: PathBuf,
    max_cases: usize,
    gate: StorageSweepGate,
}

struct StorageSweepRecord {
    case_id: usize,
    writers: u64,
    readers: u64,
    write_batch: u64,
    read_batch: u64,
    key_space: u64,
    hot_key_space: u64,
    writes_ops: u64,
    reads_ops: u64,
    hit_rate_pct: f64,
    comp_sched: u64,
    comp_done: u64,
    comp_failed: u64,
    comp_skipped: u64,
    comp_total_ms: u64,
    score: f64,
    log_path: PathBuf,
}

pub(crate) fn run_storage_command(root: &Path, command: StorageCommand) -> Result<()> {
    let storage_root = root.join("crates/edgerun-storage");
    ensure(
        storage_root.exists(),
        &format!("missing storage crate: {}", storage_root.display()),
    )?;
    match command {
        StorageCommand::Check => run_program_sync_with_env(
            "Storage check",
            "cargo",
            &["check", "--all-targets"],
            &storage_root,
            false,
            &[(OsString::from("RUSTFLAGS"), OsString::from("-D warnings"))],
        ),
        StorageCommand::Test => run_program_sync(
            "Storage test",
            "cargo",
            &["test", "-q"],
            &storage_root,
            false,
        ),
        StorageCommand::PerfGate => run_storage_perf_gate(&storage_root),
        StorageCommand::Sweep {
            duration,
            out_dir,
            max_cases,
        } => {
            let opts = StorageSweepOptions {
                duration: duration.unwrap_or(8),
                out_dir: out_dir.unwrap_or_else(default_storage_sweep_out_dir),
                max_cases: max_cases.unwrap_or(0),
                gate: load_storage_sweep_gate_thresholds(),
            };
            run_storage_mixed_rw_tuning_sweep(&storage_root, opts).map(|_| ())
        }
        StorageCommand::Crash { args } => {
            let mut full = vec![
                "run".to_string(),
                "-q".to_string(),
                "--bin".to_string(),
                "crash_campaign".to_string(),
                "--".to_string(),
            ];
            full.extend(args);
            run_program_sync_owned(
                "Storage crash campaign",
                "cargo",
                &full,
                &storage_root,
                false,
            )
        }
        StorageCommand::Bench { args } => {
            let mut full = vec!["bench".to_string()];
            full.extend(args);
            run_program_sync_owned("Storage bench", "cargo", &full, &storage_root, false)
        }
        StorageCommand::RepBench { args } => {
            let mut full = vec![
                "run".to_string(),
                "-q".to_string(),
                "--bin".to_string(),
                "replication_group_commit_benchmark".to_string(),
                "--".to_string(),
            ];
            full.extend(args);
            run_program_sync_owned(
                "Storage replication benchmark",
                "cargo",
                &full,
                &storage_root,
                false,
            )
        }
        StorageCommand::EncDemo { args } => {
            let mut full = vec![
                "run".to_string(),
                "-q".to_string(),
                "--bin".to_string(),
                "encrypted_append_demo".to_string(),
                "--".to_string(),
            ];
            full.extend(args);
            run_program_sync_owned(
                "Storage encrypted append demo",
                "cargo",
                &full,
                &storage_root,
                false,
            )
        }
        StorageCommand::CiSmoke => run_storage_ci_smoke(&storage_root),
    }
}

fn run_storage_ci_smoke(storage_root: &Path) -> Result<()> {
    run_program_sync(
        "Storage fmt check",
        "cargo",
        &["fmt", "--check"],
        storage_root,
        false,
    )?;
    run_program_sync_with_env(
        "Storage check",
        "cargo",
        &["check", "--all-targets"],
        storage_root,
        false,
        &[(OsString::from("RUSTFLAGS"), OsString::from("-D warnings"))],
    )?;
    run_program_sync(
        "Storage test",
        "cargo",
        &["test", "-q"],
        storage_root,
        false,
    )?;
    let opts = StorageSweepOptions {
        duration: 1,
        out_dir: default_storage_sweep_out_dir(),
        max_cases: 1,
        gate: StorageSweepGate {
            min_top_score: 1.0,
            min_top_writes_ops: 1,
            min_top_reads_ops: 1,
            max_top_comp_failed: 999,
        },
    };
    run_storage_perf_gate_with_options(storage_root, opts)
}

fn run_storage_perf_gate(storage_root: &Path) -> Result<()> {
    let min_end_to_end_p1 = env_f64("MIN_END_TO_END_P1_MBPS", 120.0);
    let min_io_only_p1 = env_f64("MIN_IO_ONLY_P1_MBPS", 1800.0);
    let min_end_to_end_p8 = env_f64("MIN_END_TO_END_P8_MBPS", 450.0);
    let sweep_duration = env_u64("MIXED_RW_SWEEP_DURATION", 4);
    let sweep_max_cases = env_usize("MIXED_RW_SWEEP_MAX_CASES", 4);
    let opts = StorageSweepOptions {
        duration: sweep_duration,
        out_dir: default_storage_sweep_out_dir(),
        max_cases: sweep_max_cases,
        gate: load_storage_sweep_gate_thresholds(),
    };
    run_storage_perf_gate_with_options_and_thresholds(
        storage_root,
        opts,
        min_end_to_end_p1,
        min_io_only_p1,
        min_end_to_end_p8,
    )
}

fn run_storage_perf_gate_with_options(
    storage_root: &Path,
    opts: StorageSweepOptions,
) -> Result<()> {
    run_storage_perf_gate_with_options_and_thresholds(storage_root, opts, 120.0, 1800.0, 450.0)
}

fn run_storage_perf_gate_with_options_and_thresholds(
    storage_root: &Path,
    opts: StorageSweepOptions,
    min_end_to_end_p1: f64,
    min_io_only_p1: f64,
    min_end_to_end_p8: f64,
) -> Result<()> {
    println!("Running Phase A perf gate...");
    let out_p1 = run_program_capture_sync_owned(
        "Storage async writer benchmark (both, producers=1)",
        "cargo",
        &[
            "run".to_string(),
            "-q".to_string(),
            "--bin".to_string(),
            "async_writer_benchmark".to_string(),
            "--".to_string(),
            "--mode".to_string(),
            "both".to_string(),
            "--producers".to_string(),
            "1".to_string(),
        ],
        storage_root,
        &[],
    )?;
    let e2e_p1 = extract_mode_throughput_mbps(&out_p1, "end_to_end")
        .ok_or_else(|| anyhow!("unable to parse end_to_end throughput for producers=1"))?;
    let io_p1 = extract_mode_throughput_mbps(&out_p1, "io_only")
        .ok_or_else(|| anyhow!("unable to parse io_only throughput for producers=1"))?;

    let out_p8 = run_program_capture_sync_owned(
        "Storage async writer benchmark (end_to_end, producers=8)",
        "cargo",
        &[
            "run".to_string(),
            "-q".to_string(),
            "--bin".to_string(),
            "async_writer_benchmark".to_string(),
            "--".to_string(),
            "--mode".to_string(),
            "end_to_end".to_string(),
            "--producers".to_string(),
            "8".to_string(),
        ],
        storage_root,
        &[],
    )?;
    let e2e_p8 = extract_mode_throughput_mbps(&out_p8, "end_to_end")
        .ok_or_else(|| anyhow!("unable to parse end_to_end throughput for producers=8"))?;

    assert_mbps_ge(e2e_p1, min_end_to_end_p1, "end_to_end producers=1")?;
    assert_mbps_ge(io_p1, min_io_only_p1, "io_only producers=1")?;
    assert_mbps_ge(e2e_p8, min_end_to_end_p8, "end_to_end producers=8")?;
    println!("Phase A perf gate passed.\n");
    println!("Running mixed RW tuning sweep gate...");
    let result = run_storage_mixed_rw_tuning_sweep(storage_root, opts)?;
    println!("Mixed RW tuning sweep gate passed.");
    println!("CSV: {}", result.csv.display());
    println!("Summary: {}", result.summary.display());
    Ok(())
}

fn extract_mode_throughput_mbps(text: &str, mode: &str) -> Option<f64> {
    let mut in_mode = false;
    for line in text.lines() {
        if line.starts_with("--- Mode: ") {
            in_mode = line.contains(mode);
            continue;
        }
        if !in_mode {
            continue;
        }
        if line.contains("Throughput:") && line.contains("MB/s") {
            for token in line.split_whitespace() {
                if let Ok(v) = token.replace("MB/s", "").parse::<f64>() {
                    return Some(v);
                }
            }
        }
    }
    None
}

fn assert_mbps_ge(value: f64, min: f64, label: &str) -> Result<()> {
    if value < min {
        return Err(anyhow!("FAIL: {label} {value:.2} MB/s < {min:.2} MB/s"));
    }
    println!("PASS: {label} {value:.2} MB/s >= {min:.2} MB/s");
    Ok(())
}

struct StorageSweepResult {
    csv: PathBuf,
    summary: PathBuf,
}

fn run_storage_mixed_rw_tuning_sweep(
    storage_root: &Path,
    opts: StorageSweepOptions,
) -> Result<StorageSweepResult> {
    let logs_dir = opts.out_dir.join("logs");
    std::fs::create_dir_all(&logs_dir)
        .with_context(|| format!("failed to create {}", logs_dir.display()))?;
    let csv = opts.out_dir.join("results.csv");
    let summary = opts.out_dir.join("summary.mdx");
    std::fs::write(
        &csv,
        "case_id,writers,readers,write_batch,read_batch,key_space,hot_key_space,writes_ops,reads_ops,hit_rate_pct,comp_sched,comp_done,comp_failed,comp_skipped,comp_total_ms,score,log\n",
    )?;

    let cases: &[(u64, u64, u64, u64, u64, u64)] = &[
        (2, 4, 512, 2048, 2_000_000, 200_000),
        (2, 4, 1024, 2048, 2_000_000, 200_000),
        (2, 6, 512, 4096, 2_500_000, 250_000),
        (3, 6, 512, 4096, 2_500_000, 250_000),
        (3, 8, 512, 4096, 3_000_000, 300_000),
        (4, 8, 512, 4096, 3_000_000, 300_000),
        (4, 8, 1024, 4096, 3_000_000, 300_000),
        (4, 10, 1024, 4096, 3_500_000, 350_000),
    ];

    let mut records = Vec::new();
    for (idx, (writers, readers, write_batch, read_batch, key_space, hot_key_space)) in
        cases.iter().copied().enumerate()
    {
        if opts.max_cases > 0 && records.len() >= opts.max_cases {
            break;
        }
        let case_id = idx + 1;
        let log_path = logs_dir.join(format!("case_{case_id}.log"));
        println!(
            "[case {case_id}] writers={writers} readers={readers} write_batch={write_batch} read_batch={read_batch}"
        );
        let output = run_program_capture_sync_owned(
            &format!("Storage mixed RW benchmark case {case_id}"),
            "cargo",
            &[
                "run".to_string(),
                "-q".to_string(),
                "--bin".to_string(),
                "mixed_rw_compaction_benchmark".to_string(),
                "--".to_string(),
                "--duration".to_string(),
                opts.duration.to_string(),
                "--writers".to_string(),
                writers.to_string(),
                "--readers".to_string(),
                readers.to_string(),
                "--write-batch".to_string(),
                write_batch.to_string(),
                "--read-batch".to_string(),
                read_batch.to_string(),
                "--key-space".to_string(),
                key_space.to_string(),
                "--hot-key-space".to_string(),
                hot_key_space.to_string(),
            ],
            storage_root,
            &[],
        )?;
        std::fs::write(&log_path, &output)
            .with_context(|| format!("failed to write {}", log_path.display()))?;

        let writes_line = output
            .lines()
            .rev()
            .find(|l| l.starts_with("writes:"))
            .ok_or_else(|| anyhow!("missing writes line in case {case_id}"))?;
        let reads_line = output
            .lines()
            .rev()
            .find(|l| l.starts_with("reads:"))
            .ok_or_else(|| anyhow!("missing reads line in case {case_id}"))?;
        let comp_line = output
            .lines()
            .rev()
            .find(|l| l.starts_with("compaction:"))
            .ok_or_else(|| anyhow!("missing compaction line in case {case_id}"))?;

        let writes_ops = parse_ops_per_second(writes_line)
            .ok_or_else(|| anyhow!("missing writes ops/s in case {case_id}"))?;
        let reads_ops = parse_ops_per_second(reads_line)
            .ok_or_else(|| anyhow!("missing reads ops/s in case {case_id}"))?;
        let hit_rate_pct = parse_hit_rate_pct(reads_line)
            .ok_or_else(|| anyhow!("missing hit_rate in case {case_id}"))?;
        let comp_sched = parse_line_u64(comp_line, "scheduled")
            .ok_or_else(|| anyhow!("missing compaction scheduled in case {case_id}"))?;
        let comp_done = parse_line_u64(comp_line, "completed")
            .ok_or_else(|| anyhow!("missing compaction completed in case {case_id}"))?;
        let comp_failed = parse_line_u64(comp_line, "failed")
            .ok_or_else(|| anyhow!("missing compaction failed in case {case_id}"))?;
        let comp_skipped = parse_line_u64(comp_line, "skipped")
            .ok_or_else(|| anyhow!("missing compaction skipped in case {case_id}"))?;
        let comp_total_ms = parse_line_u64(comp_line, "total_ms")
            .ok_or_else(|| anyhow!("missing compaction total_ms in case {case_id}"))?;
        let score =
            writes_ops as f64 + (reads_ops as f64 * 4.0) - (comp_failed as f64 * 1_000_000.0);

        let record = StorageSweepRecord {
            case_id,
            writers,
            readers,
            write_batch,
            read_batch,
            key_space,
            hot_key_space,
            writes_ops,
            reads_ops,
            hit_rate_pct,
            comp_sched,
            comp_done,
            comp_failed,
            comp_skipped,
            comp_total_ms,
            score,
            log_path: log_path.clone(),
        };
        append_storage_csv(&csv, &record)?;
        records.push(record);
    }

    ensure(!records.is_empty(), "no sweep results found")?;
    let mut ranked = records;
    ranked.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let top = ranked.first().expect("ranked is non-empty");

    let mut gate_failed = false;
    if top.score < opts.gate.min_top_score {
        eprintln!(
            "FAIL: top score {:.2} < min {:.2}",
            top.score, opts.gate.min_top_score
        );
        gate_failed = true;
    }
    if top.writes_ops < opts.gate.min_top_writes_ops {
        eprintln!(
            "FAIL: top writes/s {} < min {}",
            top.writes_ops, opts.gate.min_top_writes_ops
        );
        gate_failed = true;
    }
    if top.reads_ops < opts.gate.min_top_reads_ops {
        eprintln!(
            "FAIL: top reads/s {} < min {}",
            top.reads_ops, opts.gate.min_top_reads_ops
        );
        gate_failed = true;
    }
    if top.comp_failed > opts.gate.max_top_comp_failed {
        eprintln!(
            "FAIL: top comp_failed {} > max {}",
            top.comp_failed, opts.gate.max_top_comp_failed
        );
        gate_failed = true;
    }

    write_storage_sweep_summary(&summary, opts.duration, &csv, &ranked, opts.gate)?;
    println!("Sweep complete.");
    println!("CSV: {}", csv.display());
    println!("Summary: {}", summary.display());
    if gate_failed {
        return Err(anyhow!("mixed RW tuning sweep gate failed"));
    }
    println!("Mixed RW tuning sweep gate passed.");
    Ok(StorageSweepResult { csv, summary })
}

fn append_storage_csv(csv_path: &Path, record: &StorageSweepRecord) -> Result<()> {
    let row = format!(
        "{},{},{},{},{},{},{},{},{},{:.2},{},{},{},{},{},{:.2},{}\n",
        record.case_id,
        record.writers,
        record.readers,
        record.write_batch,
        record.read_batch,
        record.key_space,
        record.hot_key_space,
        record.writes_ops,
        record.reads_ops,
        record.hit_rate_pct,
        record.comp_sched,
        record.comp_done,
        record.comp_failed,
        record.comp_skipped,
        record.comp_total_ms,
        record.score,
        record.log_path.display()
    );
    let mut existing = std::fs::read_to_string(csv_path)
        .with_context(|| format!("failed to read {}", csv_path.display()))?;
    existing.push_str(&row);
    std::fs::write(csv_path, existing)
        .with_context(|| format!("failed to write {}", csv_path.display()))?;
    Ok(())
}

fn write_storage_sweep_summary(
    summary_path: &Path,
    duration: u64,
    csv: &Path,
    ranked: &[StorageSweepRecord],
    gate: StorageSweepGate,
) -> Result<()> {
    let top = ranked.first().expect("ranked non-empty");
    let mut text = String::new();
    text.push_str("# Mixed RW Tuning Sweep\n\n");
    text.push_str(&format!("- Duration per case: {duration}s\n"));
    text.push_str(&format!("- Cases run: {}\n", ranked.len()));
    text.push_str(&format!("- CSV: `{}`\n", csv.display()));
    text.push_str("- Gate thresholds:\n");
    text.push_str(&format!("  - min_top_score: {:.2}\n", gate.min_top_score));
    text.push_str(&format!(
        "  - min_top_writes_ops: {}\n",
        gate.min_top_writes_ops
    ));
    text.push_str(&format!(
        "  - min_top_reads_ops: {}\n",
        gate.min_top_reads_ops
    ));
    text.push_str(&format!(
        "  - max_top_comp_failed: {}\n",
        gate.max_top_comp_failed
    ));
    text.push_str(&format!(
        "- Top case: {} (score={:.2}, writes/s={}, reads/s={}, comp_failed={})\n\n",
        top.case_id, top.score, top.writes_ops, top.reads_ops, top.comp_failed
    ));
    text.push_str("## Ranked Results\n\n");
    text.push_str(
        "| Rank | Case | W | R | WB | RB | writes/s | reads/s | hit% | comp_failed | score |\n",
    );
    text.push_str("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n");
    for (rank, rec) in ranked.iter().enumerate() {
        text.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {:.2} | {} | {:.2} |\n",
            rank + 1,
            rec.case_id,
            rec.writers,
            rec.readers,
            rec.write_batch,
            rec.read_batch,
            rec.writes_ops,
            rec.reads_ops,
            rec.hit_rate_pct,
            rec.comp_failed,
            rec.score
        ));
    }
    std::fs::write(summary_path, text)
        .with_context(|| format!("failed to write {}", summary_path.display()))
}

fn parse_ops_per_second(line: &str) -> Option<u64> {
    let start = line.find('(')?;
    let end = line.find(" ops/s")?;
    line[start + 1..end].trim().parse::<u64>().ok()
}

fn parse_hit_rate_pct(line: &str) -> Option<f64> {
    let marker = "hit_rate=";
    let idx = line.find(marker)?;
    let rest = &line[idx + marker.len()..];
    let end = rest.find('%')?;
    rest[..end].trim().parse::<f64>().ok()
}

fn parse_line_u64(line: &str, key: &str) -> Option<u64> {
    let marker = format!("{key}=");
    let idx = line.find(&marker)?;
    let rest = &line[idx + marker.len()..];
    let token = rest.split_whitespace().next()?;
    token.parse::<u64>().ok()
}

fn default_storage_sweep_out_dir() -> PathBuf {
    std::env::temp_dir().join(format!(
        "mixed_rw_tuning_sweep_{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    ))
}

fn load_storage_sweep_gate_thresholds() -> StorageSweepGate {
    StorageSweepGate {
        min_top_score: env_f64("MIN_TOP_SCORE", 700_000.0),
        min_top_writes_ops: env_u64("MIN_TOP_WRITES_OPS", 250_000),
        min_top_reads_ops: env_u64("MIN_TOP_READS_OPS", 80_000),
        max_top_comp_failed: env_u64("MAX_TOP_COMP_FAILED", 0),
    }
}

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(default)
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(default)
}

fn env_f64(name: &str, default: f64) -> f64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(default)
}
