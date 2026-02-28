// SPDX-License-Identifier: GPL-2.0-only
use std::path::PathBuf;

use edgerun_storage::context_engine::{RustAnalyzerSnapshotV1, StorageBackedContextEngine};
use prost::Message;

fn usage() {
    eprintln!(
        "Usage: context_ingest_ra_proto --data-dir PATH --repo-id ID --branch ID --snapshot PATH"
    );
}

fn main() {
    let mut data_dir: Option<PathBuf> = None;
    let mut repo_id: Option<String> = None;
    let mut branch: Option<String> = None;
    let mut snapshot_path: Option<PathBuf> = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--data-dir" => data_dir = args.next().map(PathBuf::from),
            "--repo-id" => repo_id = args.next(),
            "--branch" => branch = args.next(),
            "--snapshot" => snapshot_path = args.next().map(PathBuf::from),
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
    let Some(branch) = branch else {
        usage();
        std::process::exit(2);
    };
    let Some(snapshot_path) = snapshot_path else {
        usage();
        std::process::exit(2);
    };

    let snapshot_bytes = match std::fs::read(&snapshot_path) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!(
                "failed to read snapshot file {}: {e}",
                snapshot_path.display()
            );
            std::process::exit(1);
        }
    };

    // Decode early for clear UX on invalid payloads.
    if let Err(e) = RustAnalyzerSnapshotV1::decode(snapshot_bytes.as_slice()) {
        eprintln!("invalid RustAnalyzerSnapshotV1 payload: {e}");
        std::process::exit(1);
    }

    let mut engine = match StorageBackedContextEngine::open_writer(data_dir, &repo_id) {
        Ok(engine) => engine,
        Err(e) => {
            eprintln!("failed to open context engine: {e}");
            std::process::exit(1);
        }
    };

    let outcome = match engine.ingest_rust_analyzer_snapshot_proto(&branch, &snapshot_bytes) {
        Ok(outcome) => outcome,
        Err(e) => {
            eprintln!("ingest failed: {e}");
            std::process::exit(1);
        }
    };

    println!(
        "ingested: symbols={} references={} diagnostics={}",
        outcome.symbols_upserted, outcome.references_recorded, outcome.diagnostics_recorded
    );
}
