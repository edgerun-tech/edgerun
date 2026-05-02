use std::path::Path;
use crate::errors::Result;
use crate::crate_model::*;

pub fn parse_cargo_toml(path: &Path) -> Result<CrateIdentity> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| crate::errors::AnalyzerError::IoError(e.to_string()))?;

    let toml = edgerun_json::toml_parse(&content)
        .map_err(|e| crate::errors::AnalyzerError::ParseError {
            file: path.to_path_buf(),
            message: e.to_string(),
        })?;

    let package = toml.get("package")
        .ok_or_else(|| crate::errors::AnalyzerError::ParseError {
            file: path.to_path_buf(),
            message: "No [package] section".into(),
        })?;

    let name = package.get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let version = package.get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("0.0.0")
        .to_string();

    let description = package.get("description").and_then(|v| v.as_str()).map(|s| s.to_string());
    let license = package.get("license").and_then(|v| v.as_str()).map(|s| s.to_string());
    let edition = package.get("edition").and_then(|v| v.as_str()).unwrap_or("2021").to_string();
    let rust_version = package.get("rust-version").and_then(|v| v.as_str()).map(|s| s.to_string());

    let features = package.get("features")
        .and_then(|f| f.as_object())
        .map(|obj| obj.keys().cloned().collect())
        .unwrap_or_default();

    let lib_target = path.parent().unwrap().join("src/lib.rs").exists();
    let bin_targets = collect_bin_targets(path.parent().unwrap());
    let test_targets = Vec::new();
    let bench_targets = Vec::new();

    Ok(CrateIdentity {
        name,
        version,
        path: path.parent().unwrap().to_path_buf(),
        description,
        license,
        edition,
        rust_version,
        features,
        lib_target,
        bin_targets,
        test_targets,
        bench_targets,
        visible_files: Vec::new(),
        hidden_files_count: 0,
        commit_hash: get_commit_hash(path.parent().unwrap()),
    })
}

fn collect_bin_targets(crate_dir: &Path) -> Vec<String> {
    let src_bin = crate_dir.join("src/bin");
    if !src_bin.exists() { return Vec::new(); }
    std::fs::read_dir(src_bin)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().map(|ext| ext == "rs").unwrap_or(false))
                .filter_map(|e| e.file_name().to_str().map(|s| s.trim_end_matches(".rs").to_string()))
                .collect()
        })
        .unwrap_or_default()
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
