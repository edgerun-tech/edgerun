use std::collections::BTreeSet;
use std::path::Path;

use crate::codealyzer::cargo_toml_projection::parse_cargo_toml_projection;
use crate::codealyzer::crate_model::*;
use crate::codealyzer::errors::Result;

pub fn parse_cargo_toml(path: &Path) -> Result<CrateIdentity> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| crate::codealyzer::errors::AnalyzerError::IoError(e.to_string()))?;

    let projection = parse_cargo_toml_projection(&content);
    if projection.package_name.is_none() {
        return Err(crate::codealyzer::errors::AnalyzerError::ParseError {
            file: path.to_path_buf(),
            message: "No [package] section".into(),
        });
    }

    let name = projection
        .package_name
        .unwrap_or_else(|| "unknown".to_string());

    let version = projection
        .package_version
        .unwrap_or_else(|| "0.0.0".to_string());

    let description = projection.package_description;
    let license = projection.package_license;
    let edition = projection
        .package_edition
        .unwrap_or_else(|| "2021".to_string());
    let rust_version = projection.package_rust_version;
    let features = projection.feature_keys;

    let crate_dir =
        path.parent()
            .ok_or_else(|| crate::codealyzer::errors::AnalyzerError::ParseError {
                file: path.to_path_buf(),
                message: "Cargo.toml has no parent directory".into(),
            })?;
    let lib_target = projection.lib_present || crate_dir.join("src/lib.rs").exists();

    let declared_bins = collect_declared_targets(&projection.bin_targets);
    let auto_bins = collect_bin_targets(crate_dir);
    let bin_targets = merge_target_lists(&declared_bins, &auto_bins);

    let test_targets = collect_declared_targets(&projection.test_targets);
    let bench_targets = collect_declared_targets(&projection.bench_targets);

    let crate_type = if lib_target && !bin_targets.is_empty() {
        CrateType::LibraryAndBinary
    } else if lib_target {
        CrateType::Library
    } else if !bin_targets.is_empty() {
        CrateType::Binary
    } else {
        CrateType::Unknown
    };

    Ok(CrateIdentity {
        name,
        version,
        path: crate_dir.to_path_buf(),
        description,
        license,
        edition,
        rust_version,
        features,
        lib_target,
        bin_targets,
        test_targets,
        bench_targets,
        crate_type,
        visible_files: Vec::new(),
        hidden_files_count: 0,
        commit_hash: get_commit_hash(crate_dir),
    })
}

fn collect_declared_targets(
    targets: &[crate::codealyzer::cargo_toml_projection::TargetProjection],
) -> Vec<String> {
    targets
        .iter()
        .filter_map(|target| target.name.clone())
        .collect()
}

fn collect_bin_targets(crate_dir: &Path) -> Vec<String> {
    let mut bins = Vec::new();
    let src_bin = crate_dir.join("src/bin");
    if src_bin.exists() {
        let discovered: Vec<String> = std::fs::read_dir(src_bin)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().map(|ext| ext == "rs").unwrap_or(false))
                    .filter_map(|e| {
                        e.file_name()
                            .to_str()
                            .map(|s| s.trim_end_matches(".rs").to_string())
                    })
                    .collect()
            })
            .unwrap_or_default();

        bins.extend(discovered);
    }

    if crate_dir.join("src/main.rs").exists() {
        bins.push("main".to_string());
    }

    dedupe_and_sort(bins)
}

fn dedupe_and_sort(values: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    values
        .into_iter()
        .filter(|name| !name.is_empty())
        .filter_map(|name| {
            if seen.insert(name.clone()) {
                Some(name)
            } else {
                None
            }
        })
        .collect()
}

fn merge_target_lists(declared: &[String], auto: &[String]) -> Vec<String> {
    let mut merged = Vec::new();
    merged.extend_from_slice(declared);
    merged.extend_from_slice(auto);
    dedupe_and_sort(merged)
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
