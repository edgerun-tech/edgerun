// SPDX-License-Identifier: GPL-2.0-only
use std::io::{self, BufRead};
use std::path::PathBuf;

use edgerun_storage::docker_logger::{DefaultDockerLogAdapter, PipeDockerLogDecoder};
use edgerun_storage::virtual_fs::{
    SourceImportRequestV1, StorageBackedVirtualFs, VfsModeV1, VfsSourceKindV1,
};

fn usage() {
    eprintln!(
        "Usage: docker_log_ingest --data-dir PATH --repo-id ID --branch ID [--declared-by ID] [--partition-prefix PREFIX] [--batch-lines N] [--ensure-log-source]"
    );
}

fn main() {
    let mut data_dir: Option<PathBuf> = None;
    let mut repo_id: Option<String> = None;
    let mut branch_id: Option<String> = None;
    let mut declared_by = "docker_log_ingest".to_string();
    let mut partition_prefix = "docker".to_string();
    let mut batch_lines: usize = 1000;
    let mut ensure_log_source = false;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--data-dir" => data_dir = args.next().map(PathBuf::from),
            "--repo-id" => repo_id = args.next(),
            "--branch" => branch_id = args.next(),
            "--declared-by" => declared_by = args.next().unwrap_or_else(|| declared_by.clone()),
            "--partition-prefix" => {
                partition_prefix = args.next().unwrap_or_else(|| partition_prefix.clone())
            }
            "--batch-lines" => {
                let raw = args.next().unwrap_or_else(|| "1000".to_string());
                batch_lines = raw.parse::<usize>().unwrap_or(1000).max(1);
            }
            "--ensure-log-source" => ensure_log_source = true,
            "--help" | "-h" => {
                usage();
                return;
            }
            _ => {
                eprintln!("unknown arg: {arg}");
                usage();
                std::process::exit(2);
            }
        }
    }

    let Some(data_dir) = data_dir else {
        usage();
        std::process::exit(2);
    };
    let Some(repo_id) = repo_id else {
        usage();
        std::process::exit(2);
    };
    let Some(branch_id) = branch_id else {
        usage();
        std::process::exit(2);
    };

    let mut vfs = match StorageBackedVirtualFs::open_writer(data_dir, &repo_id) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("failed to open vfs: {e}");
            std::process::exit(1);
        }
    };

    if ensure_log_source {
        let req = SourceImportRequestV1 {
            schema_version: 1,
            repo_id: repo_id.clone(),
            source_kind: VfsSourceKindV1::VfsSourceKindLogStream as i32,
            mode: VfsModeV1::VfsModeLog as i32,
            source_locator: "docker://stdin".to_string(),
            source_ref: "stream".to_string(),
            initiated_by: declared_by.clone(),
        };
        let _ = vfs.import_source(&req, 0, 0, vec![0u8; 32], "{}".to_string());
    }

    let mut decoder = PipeDockerLogDecoder;
    let adapter = DefaultDockerLogAdapter::new(partition_prefix);
    let mut outcome = edgerun_storage::virtual_fs::LogIngestOutcome::default();
    let mut lines_total: usize = 0;
    let mut batch = Vec::<String>::with_capacity(batch_lines);
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(v) => v,
            Err(e) => {
                eprintln!("failed reading stdin: {e}");
                std::process::exit(1);
            }
        };
        lines_total = lines_total.saturating_add(1);
        batch.push(line);
        if batch.len() >= batch_lines {
            let step = match vfs.ingest_docker_log_lines_batched(
                &branch_id,
                &batch,
                &declared_by,
                &mut decoder,
                &adapter,
                batch_lines,
            ) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("failed to ingest docker log lines: {e}");
                    std::process::exit(1);
                }
            };
            merge_outcome(&mut outcome, step);
            batch.clear();
        }
    }

    if !batch.is_empty() {
        let step = match vfs.ingest_docker_log_lines_batched(
            &branch_id,
            &batch,
            &declared_by,
            &mut decoder,
            &adapter,
            batch_lines,
        ) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("failed to ingest docker log lines: {e}");
                std::process::exit(1);
            }
        };
        merge_outcome(&mut outcome, step);
    }

    if lines_total == 0 {
        println!("ingest_appended=0");
        println!("ingest_skipped_idempotent=0");
        println!("ingest_declared_partition=false");
        println!("ingest_lines_total=0");
        return;
    }

    println!("ingest_appended={}", outcome.appended);
    println!("ingest_skipped_idempotent={}", outcome.skipped_idempotent);
    println!("ingest_declared_partition={}", outcome.declared_partition);
    println!("ingest_lines_total={lines_total}");
    if let Some(first) = outcome.first_event_offset {
        println!("ingest_first_event_offset={first}");
    }
    if let Some(last) = outcome.last_event_offset {
        println!("ingest_last_event_offset={last}");
    }
}

fn merge_outcome(
    total: &mut edgerun_storage::virtual_fs::LogIngestOutcome,
    out: edgerun_storage::virtual_fs::LogIngestOutcome,
) {
    total.appended = total.appended.saturating_add(out.appended);
    total.skipped_idempotent = total
        .skipped_idempotent
        .saturating_add(out.skipped_idempotent);
    total.declared_partition = total.declared_partition || out.declared_partition;
    if total.first_event_offset.is_none() {
        total.first_event_offset = out.first_event_offset;
    }
    total.last_event_offset = out.last_event_offset.or(total.last_event_offset);
}
