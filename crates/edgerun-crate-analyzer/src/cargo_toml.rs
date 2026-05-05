use std::collections::BTreeSet;
use std::path::Path;

use crate::crate_model::*;
use crate::errors::Result;

pub fn parse_cargo_toml(path: &Path) -> Result<CrateIdentity> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| crate::errors::AnalyzerError::IoError(e.to_string()))?;

    let toml = edgerun_json::from_toml_str(&content)
        .map_err(|e| crate::errors::AnalyzerError::ParseError {
            file: path.to_path_buf(),
            message: e.to_string(),
        })?;

    let package = toml.get("package").ok_or_else(|| crate::errors::AnalyzerError::ParseError {
            file: path.to_path_buf(),
            message: "No [package] section".into(),
        })?;

    let name = package
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let version = package
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("0.0.0")
        .to_string();

    let description = package
        .get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let license = package
        .get("license")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let edition = package
        .get("edition")
        .and_then(|v| v.as_str())
        .unwrap_or("2021")
        .to_string();
    let rust_version = package
        .get("rust-version")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let features = package
        .get("features")
        .and_then(|f| f.as_table())
        .map(|obj| obj.iter().map(|(name, _)| name.clone()).collect())
        .unwrap_or_default();

    let crate_dir = path.parent().unwrap();
    let lib_target = is_lib_target(package, crate_dir);

    let declared_bins = collect_declared_targets(&toml, "bin");
    let auto_bins = collect_bin_targets(crate_dir);
    let bin_targets = merge_target_lists(&declared_bins, &auto_bins);

    let test_targets = collect_declared_targets(&toml, "test");
    let bench_targets = collect_declared_targets(&toml, "bench");

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

fn is_lib_target(package: &edgerun_json::TomlValue, crate_dir: &Path) -> bool {
    if package
        .get("lib")
        .and_then(|v| v.as_table())
        .is_some()
    {
        return true;
    }

    crate_dir.join("src/lib.rs").exists()
}

fn collect_declared_targets(toml: &edgerun_json::TomlValue, section: &str) -> Vec<String> {
    let mut entries = Vec::new();
    let Some(value) = toml.get(section) else {
        return entries;
    };

    if let Some(array) = value.as_array() {
        for entry in array {
            if let Some(table) = entry.as_table() {
                let mut name = None;
                for (k, v) in table {
                    if k == "name" {
                        name = v.as_str().map(str::to_string);
                        break;
                    }
                }
                if let Some(name) = name {
                    entries.push(name);
                }
            }
        }
        return entries;
    }

    if let Some(table) = value.as_table() {
        let mut name = None;
        for (k, v) in table {
            if k == "name" {
                name = v.as_str().map(str::to_string);
                break;
            }
        }
        if let Some(name) = name {
            entries.push(name);
        }
    }

    entries
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
                    .filter_map(|e| e.file_name().to_str().map(|s| s.trim_end_matches(".rs").to_string()))
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
