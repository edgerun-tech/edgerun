// SPDX-License-Identifier: GPL-2.0-only
use std::path::PathBuf;

use edgerun_storage::context_engine::StorageBackedContextEngine;

fn usage() {
    eprintln!(
        "Usage: context_bundle_dump --data-dir PATH --repo-id ID --branch ID --files path1,path2 [--symbol-limit N] [--diagnostic-limit N]"
    );
}

fn main() {
    let mut data_dir: Option<PathBuf> = None;
    let mut repo_id: Option<String> = None;
    let mut branch: Option<String> = None;
    let mut files_csv: Option<String> = None;
    let mut symbol_limit: usize = 100;
    let mut diagnostic_limit: usize = 100;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--data-dir" => data_dir = args.next().map(PathBuf::from),
            "--repo-id" => repo_id = args.next(),
            "--branch" => branch = args.next(),
            "--files" => files_csv = args.next(),
            "--symbol-limit" => {
                let Some(raw) = args.next() else {
                    usage();
                    std::process::exit(2);
                };
                symbol_limit = raw.parse::<usize>().unwrap_or(100);
            }
            "--diagnostic-limit" => {
                let Some(raw) = args.next() else {
                    usage();
                    std::process::exit(2);
                };
                diagnostic_limit = raw.parse::<usize>().unwrap_or(100);
            }
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
    let Some(files_csv) = files_csv else {
        usage();
        std::process::exit(2);
    };

    let files: Vec<String> = files_csv
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect();

    if files.is_empty() {
        eprintln!("--files must contain at least one path");
        std::process::exit(2);
    }

    let engine = match StorageBackedContextEngine::open_reader(data_dir, &repo_id) {
        Ok(engine) => engine,
        Err(e) => {
            eprintln!("failed to open context engine: {e}");
            std::process::exit(1);
        }
    };

    let bundle = match engine.build_bundle(&branch, &files, symbol_limit, diagnostic_limit) {
        Ok(bundle) => bundle,
        Err(e) => {
            eprintln!("build bundle failed: {e}");
            std::process::exit(1);
        }
    };

    println!("branch={}", bundle.branch_id);
    println!("files={}", bundle.file_paths.join(","));
    println!("symbols={}", bundle.symbols.len());
    for s in &bundle.symbols {
        println!(
            "  symbol {} {} {}:{}:{}-{}:{} {}",
            s.symbol_id,
            s.symbol_kind,
            s.file_path,
            s.line_start,
            s.col_start,
            s.line_end,
            s.col_end,
            s.symbol_name
        );
    }
    println!("references={}", bundle.references.len());
    for r in &bundle.references {
        println!(
            "  ref {} {}:{}:{} {}",
            r.symbol_id, r.file_path, r.line, r.col, r.context_snippet
        );
    }
    println!("diagnostics={}", bundle.diagnostics.len());
    for d in &bundle.diagnostics {
        println!(
            "  diag {} {} {}:{}:{} {}",
            d.severity, d.diagnostic_id, d.file_path, d.line, d.col, d.message
        );
    }
    println!("touched_files={}", bundle.touched_files.join(","));
}
