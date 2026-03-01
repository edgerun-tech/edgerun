// SPDX-License-Identifier: GPL-2.0-only
use std::path::PathBuf;
use std::time::Instant;

use edgerun_storage::docker_logger::{DefaultDockerLogAdapter, PipeDockerLogDecoder};
use edgerun_storage::virtual_fs::{
    SourceImportRequestV1, StorageBackedVirtualFs, VfsModeV1, VfsSourceKindV1,
};

fn percentile_ms(samples: &[u128], pct: f64) -> u128 {
    if samples.is_empty() {
        return 0;
    }
    let mut v = samples.to_vec();
    v.sort_unstable();
    let idx = ((v.len() as f64 - 1.0) * pct).round() as usize;
    v[idx]
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let total_lines: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(50_000);
    let batch_sizes: Vec<usize> = if let Some(csv) = args.get(2) {
        csv.split(',')
            .filter_map(|s| s.trim().parse::<usize>().ok())
            .filter(|n| *n > 0)
            .collect()
    } else {
        vec![100, 500, 1000, 5000]
    };

    let repo_root = PathBuf::from("/home/ken/src/edgerun");
    let out_root = repo_root.join("out").join("docker-log-ingest-benchmark");
    let _ = std::fs::remove_dir_all(&out_root);
    std::fs::create_dir_all(&out_root).expect("create out dir");

    let lines = build_lines(total_lines);
    println!("benchmark_total_lines={}", lines.len());
    println!(
        "benchmark_batch_sizes={}",
        batch_sizes
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );

    for batch in batch_sizes {
        let data_dir = out_root.join(format!("batch-{batch}"));
        std::fs::create_dir_all(&data_dir).expect("create data dir");
        let repo_id = format!("docker-bench-{batch}");
        let mut vfs = StorageBackedVirtualFs::open_writer(data_dir, &repo_id).expect("open writer");
        let req = SourceImportRequestV1 {
            schema_version: 1,
            repo_id: repo_id.clone(),
            source_kind: VfsSourceKindV1::VfsSourceKindLogStream as i32,
            mode: VfsModeV1::VfsModeLog as i32,
            source_locator: "docker://bench".to_string(),
            source_ref: "stream".to_string(),
            initiated_by: "bench".to_string(),
        };
        let _ = vfs
            .import_source(&req, 0, 0, vec![0u8; 32], "{}".to_string())
            .expect("import source");

        let mut decoder = PipeDockerLogDecoder;
        let adapter = DefaultDockerLogAdapter::default();
        let mut batch_latencies = Vec::<u128>::new();
        let mut total_appended = 0u64;
        let mut total_skipped = 0u64;

        let started = Instant::now();
        let mut i = 0usize;
        while i < lines.len() {
            let end = (i + batch).min(lines.len());
            let chunk = &lines[i..end];
            let t0 = Instant::now();
            let out = vfs
                .ingest_docker_log_lines_batched(
                    "main",
                    chunk,
                    "bench",
                    &mut decoder,
                    &adapter,
                    batch,
                )
                .expect("ingest batch");
            batch_latencies.push(t0.elapsed().as_millis());
            total_appended = total_appended.saturating_add(out.appended);
            total_skipped = total_skipped.saturating_add(out.skipped_idempotent);
            i = end;
        }
        let elapsed = started.elapsed();
        let lines_per_sec = if elapsed.as_secs_f64() > 0.0 {
            lines.len() as f64 / elapsed.as_secs_f64()
        } else {
            0.0
        };

        println!("batch_size={batch}");
        println!("elapsed_ms={}", elapsed.as_millis());
        println!("lines_per_sec={:.2}", lines_per_sec);
        println!("batches={}", batch_latencies.len());
        println!(
            "batch_latency_p50_ms={}",
            percentile_ms(&batch_latencies, 0.50)
        );
        println!(
            "batch_latency_p95_ms={}",
            percentile_ms(&batch_latencies, 0.95)
        );
        println!("appended={total_appended}");
        println!("skipped_idempotent={total_skipped}");
    }
}

fn build_lines(total: usize) -> Vec<String> {
    let mut out = Vec::with_capacity(total);
    for i in 0..total {
        let container = if i % 3 == 0 { "api" } else { "worker" };
        let stream = if i % 5 == 0 { "stderr" } else { "stdout" };
        let ts = 1_709_251_200_000u64.saturating_add(i as u64);
        let msg = format!("message-{i}");
        out.push(format!("cid-{container}|{container}|{stream}|{ts}|{msg}"));
    }
    out
}
