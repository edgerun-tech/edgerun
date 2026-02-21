// SPDX-License-Identifier: GPL-2.0-only
use edgerun_storage::crash_test::{CrashTestConfig, CrashTestHarness, KillPoint};
use std::path::PathBuf;
use std::time::Duration;

fn parse_args() -> CrashTestConfig {
    let mut cfg = CrashTestConfig::default();
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let mut i = 0usize;

    while i < args.len() {
        let arg = &args[i];
        let next = args.get(i + 1);
        match (arg.as_str(), next) {
            ("--iterations", Some(v)) => {
                cfg.iterations = v.parse().unwrap_or(cfg.iterations);
                i += 2;
            }
            ("--data-dir", Some(v)) => {
                cfg.data_dir = PathBuf::from(v);
                i += 2;
            }
            ("--target-mb", Some(v)) => {
                let mb: u64 = v.parse().unwrap_or(10);
                cfg.target_size = mb * 1024 * 1024;
                i += 2;
            }
            ("--timeout-secs", Some(v)) => {
                let s: u64 = v.parse().unwrap_or(60);
                cfg.iteration_timeout = Duration::from_secs(s);
                i += 2;
            }
            ("--keep-failed", Some(v)) => {
                cfg.keep_failed_data = matches!(v.as_str(), "1" | "true" | "yes");
                i += 2;
            }
            ("--random", Some(v)) => {
                cfg.random_kill_points = matches!(v.as_str(), "1" | "true" | "yes");
                i += 2;
            }
            ("--kill-point", Some(v)) => {
                cfg.random_kill_points = false;
                cfg.kill_points = vec![match v.as_str() {
                    "after_append" => KillPoint::AfterAppend,
                    "mid_write" => KillPoint::MidWrite,
                    "before_fsync" => KillPoint::BeforeFsync,
                    "after_fsync_before_manifest" => KillPoint::AfterFsyncBeforeManifest,
                    "after_manifest_before_msync" => KillPoint::AfterManifestBeforeMsync,
                    "during_index_flush" => KillPoint::DuringIndexFlush,
                    "during_compaction" => KillPoint::DuringCompaction,
                    _ => KillPoint::Random,
                }];
                i += 2;
            }
            ("--help", _) | ("-h", _) => {
                println!("Usage: crash_campaign [--iterations N] [--data-dir PATH] [--target-mb N] [--timeout-secs N] [--keep-failed true|false] [--random true|false] [--kill-point after_append|mid_write|before_fsync|after_fsync_before_manifest|after_manifest_before_msync|during_index_flush|during_compaction]");
                std::process::exit(0);
            }
            _ => {
                i += 1;
            }
        }
    }

    cfg
}

fn main() {
    let cfg = parse_args();
    println!("=== Crash Campaign ===");
    println!("iterations: {}", cfg.iterations);
    println!("data_dir: {}", cfg.data_dir.display());
    println!("target_size_mb: {}", cfg.target_size / 1024 / 1024);
    println!("random_kill_points: {}", cfg.random_kill_points);
    println!();

    let harness = CrashTestHarness::new(cfg.clone());
    let results = harness.run();
    results.print_report();

    let report_path = cfg.data_dir.join("campaign_report.json");
    if let Err(e) = std::fs::create_dir_all(&cfg.data_dir) {
        eprintln!("WARN: could not create report dir: {e}");
    } else {
        let summary = format!(
            "{{\"iterations\":{},\"passed\":{},\"failed\":{},\"avg_survival\":{},\"min_survival\":{},\"max_survival\":{},\"total_duration_ms\":{}}}",
            results.iterations.len(),
            results.passed,
            results.failed,
            results.avg_survival_rate,
            results.min_survival_rate,
            results.max_survival_rate,
            results.total_duration.as_millis()
        );
        if let Err(e) = std::fs::write(&report_path, summary) {
            eprintln!(
                "WARN: could not write report {}: {}",
                report_path.display(),
                e
            );
        } else {
            println!("Report: {}", report_path.display());
        }
    }

    if !results.all_passed() {
        std::process::exit(1);
    }
}
