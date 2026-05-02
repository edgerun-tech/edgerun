use std::path::{Path, PathBuf};
use crate::crate_model::BenchmarkArtifact;

pub fn collect_benchmarks(crate_dir: &Path, workspace_root: &Path) -> Vec<BenchmarkArtifact> {
    let mut artifacts = Vec::new();

    let bench_dir = crate_dir.join("benches");
    if bench_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&bench_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().map(|e| e == "rs").unwrap_or(false) {
                    artifacts.push(BenchmarkArtifact {
                        name: path.file_stem().unwrap().to_string_lossy().into(),
                        path: path.clone(),
                        scenario: None,
                        commit: get_commit_hash(crate_dir),
                    });
                }
            }
        }
    }

    let public_bench = workspace_root.join("benchmarks/public").join(crate_dir.file_name().unwrap_or_default());
    if public_bench.exists() {
        collect_bench_artifacts_recursive(&public_bench, &mut artifacts);
    }

    artifacts
}

fn collect_bench_artifacts_recursive(dir: &Path, artifacts: &mut Vec<BenchmarkArtifact>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                collect_bench_artifacts_recursive(&path, artifacts);
            } else if let Some(ext) = path.extension() {
                if ext == "json" || ext == "csv" || ext == "txt" || ext == "html" {
                    artifacts.push(BenchmarkArtifact {
                        name: path.file_name().unwrap().to_string_lossy().into(),
                        path: path.clone(),
                        scenario: None,
                        commit: None,
                    });
                }
            }
        }
    }
}

fn get_commit_hash(dir: &Path) -> Option<String> {
    std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(dir)
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}
