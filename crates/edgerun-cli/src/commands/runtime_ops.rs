// SPDX-License-Identifier: Apache-2.0
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use serde_json::Value;

use crate::process_helpers::run_program_sync_owned;
use crate::{command_exists, ensure, AppConfig};

pub(crate) async fn run_replay_corpus(root: &Path, config: &AppConfig) -> Result<()> {
    let out_dir = std::env::var("REPLAY_OUT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| root.join(".edgerun-replay-corpus"));
    std::fs::create_dir_all(&out_dir)?;

    let profile_debug = std::env::var("REPLAY_PROFILE_DEBUG")
        .ok()
        .or_else(|| config.runtime.replay_profile_debug.clone())
        .unwrap_or_else(|| "local-debug".to_string());
    let profile_release = std::env::var("REPLAY_PROFILE_RELEASE")
        .ok()
        .or_else(|| config.runtime.replay_profile_release.clone())
        .unwrap_or_else(|| "local-release".to_string());
    let runs = std::env::var("REPLAY_CORPUS_RUNS")
        .ok()
        .or_else(|| config.runtime.replay_runs.map(|v| v.to_string()))
        .unwrap_or_else(|| "3".to_string());

    let debug_artifact = out_dir.join(format!("{profile_debug}.json"));
    let release_artifact = out_dir.join(format!("{profile_release}.json"));
    run_program_sync_owned(
        "Replay debug",
        "cargo",
        &[
            "run".to_string(),
            "-p".to_string(),
            "edgerun-runtime".to_string(),
            "--".to_string(),
            "replay-corpus".to_string(),
            "--profile".to_string(),
            profile_debug.clone(),
            "--artifact".to_string(),
            debug_artifact.display().to_string(),
            "--runs".to_string(),
            runs.clone(),
        ],
        root,
        false,
    )?;
    run_program_sync_owned(
        "Replay release",
        "cargo",
        &[
            "run".to_string(),
            "--release".to_string(),
            "-p".to_string(),
            "edgerun-runtime".to_string(),
            "--".to_string(),
            "replay-corpus".to_string(),
            "--profile".to_string(),
            profile_release.clone(),
            "--artifact".to_string(),
            release_artifact.display().to_string(),
            "--runs".to_string(),
            runs.clone(),
        ],
        root,
        false,
    )?;

    compare_replay_profiles(&debug_artifact, &release_artifact)
}

pub(crate) async fn run_weekly_fuzz(root: &Path, config: &AppConfig) -> Result<()> {
    ensure(command_exists("cargo-fuzz"), "cargo-fuzz not installed")?;

    let artifact_dir = std::env::var("FUZZ_ARTIFACT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| root.join("out/fuzz-weekly"));
    std::fs::create_dir_all(&artifact_dir)?;
    let secs = std::env::var("FUZZ_SECONDS_PER_TARGET")
        .ok()
        .or_else(|| {
            config
                .runtime
                .fuzz_seconds_per_target
                .map(|v| v.to_string())
        })
        .unwrap_or_else(|| "300".to_string());
    let fuzz_dir = root.join("crates/edgerun-runtime/fuzz");
    let fuzz_crash_dir = fuzz_dir.join("artifacts");
    if fuzz_crash_dir.exists() {
        std::fs::remove_dir_all(&fuzz_crash_dir)
            .with_context(|| format!("failed to clear {}", fuzz_crash_dir.display()))?;
    }
    std::fs::create_dir_all(&fuzz_crash_dir)?;

    for target in [
        "fuzz_bundle_decode",
        "fuzz_validate_wasm",
        "fuzz_hostcall_boundary",
    ] {
        run_program_sync_owned(
            "Run fuzz target",
            "cargo",
            &[
                "fuzz".to_string(),
                "run".to_string(),
                target.to_string(),
                "--".to_string(),
                format!("-max_total_time={secs}"),
            ],
            &fuzz_dir,
            false,
        )?;
    }

    let crash_count = count_files_recursive(&fuzz_crash_dir)?;
    if crash_count > 0 {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let out_dir = artifact_dir.join(format!("run-{stamp}"));
        copy_dir_recursive(&fuzz_crash_dir, &out_dir)?;
        return Err(anyhow!(
            "fuzz crashes detected: {crash_count} files (copied to {})",
            out_dir.display()
        ));
    }
    Ok(())
}

pub(crate) fn compare_replay_profiles(a_path: &Path, b_path: &Path) -> Result<()> {
    let a: Value = serde_json::from_slice(&std::fs::read(a_path)?)
        .with_context(|| format!("failed parsing {}", a_path.display()))?;
    let b: Value = serde_json::from_slice(&std::fs::read(b_path)?)
        .with_context(|| format!("failed parsing {}", b_path.display()))?;

    fn normalize(doc: &Value) -> BTreeMap<String, (Value, bool, bool)> {
        let mut out = BTreeMap::new();
        if let Some(cases) = doc["cases"].as_array() {
            for case in cases {
                if let Some(name) = case["case"].as_str() {
                    out.insert(
                        name.to_string(),
                        (
                            case.get("actual").cloned().unwrap_or(Value::Null),
                            case["passed"].as_bool().unwrap_or(false),
                            case["stable"].as_bool().unwrap_or(false),
                        ),
                    );
                }
            }
        }
        out
    }

    let a_cases = normalize(&a);
    let b_cases = normalize(&b);
    ensure(a_cases == b_cases, "replay profile mismatch detected")?;
    ensure(
        a_cases
            .values()
            .all(|(_, passed, stable)| *passed && *stable),
        "replay cases are not fully passed/stable in first profile",
    )?;
    Ok(())
}

pub(crate) fn validate_external_security_review(path: &Path) -> Result<()> {
    let doc: Value = serde_json::from_slice(&std::fs::read(path)?)
        .with_context(|| format!("failed parsing {}", path.display()))?;

    for key in [
        "review_cycle_id",
        "status",
        "provider",
        "scope_version",
        "sign_off",
        "findings",
    ] {
        ensure(
            doc.get(key).is_some(),
            &format!("missing top-level key: {key}"),
        )?;
    }

    let status = doc["status"].as_str().unwrap_or_default();
    ensure(
        matches!(status, "planned" | "in_progress" | "completed"),
        "invalid status",
    )?;

    let provider = &doc["provider"];
    ensure(provider.is_object(), "provider must be an object")?;
    ensure(
        provider.get("organization").is_some(),
        "missing provider.organization",
    )?;
    ensure(
        provider.get("reviewer").is_some(),
        "missing provider.reviewer",
    )?;

    let sign_off = &doc["sign_off"];
    ensure(sign_off.is_object(), "sign_off must be an object")?;
    for key in ["date", "approved", "notes"] {
        ensure(
            sign_off.get(key).is_some(),
            &format!("missing sign_off.{key}"),
        )?;
    }

    let findings = doc["findings"]
        .as_array()
        .ok_or_else(|| anyhow!("findings must be a list"))?;
    let mut unresolved_high_or_critical = Vec::new();
    for (i, finding) in findings.iter().enumerate() {
        ensure(
            finding.is_object(),
            &format!("finding[{i}] must be an object"),
        )?;
        for key in ["id", "title", "severity", "status", "owner", "notes"] {
            ensure(
                finding.get(key).is_some(),
                &format!("finding[{i}] missing key: {key}"),
            )?;
        }
        let severity = finding["severity"].as_str().unwrap_or_default();
        let finding_status = finding["status"].as_str().unwrap_or_default();
        ensure(
            matches!(severity, "low" | "medium" | "high" | "critical"),
            &format!("finding[{i}] invalid severity"),
        )?;
        ensure(
            matches!(finding_status, "open" | "closed" | "accepted_risk"),
            &format!("finding[{i}] invalid status"),
        )?;
        if (severity == "high" || severity == "critical") && finding_status != "closed" {
            unresolved_high_or_critical.push(
                finding["id"]
                    .as_str()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "<unknown>".to_string()),
            );
        }
    }

    if status == "completed" {
        ensure(
            sign_off["approved"].as_bool().unwrap_or(false),
            "completed review requires sign_off.approved=true",
        )?;
        ensure(
            !sign_off["date"]
                .as_str()
                .unwrap_or_default()
                .trim()
                .is_empty(),
            "completed review requires non-empty sign_off.date",
        )?;
        let org = provider["organization"].as_str().unwrap_or_default().trim();
        let reviewer = provider["reviewer"].as_str().unwrap_or_default().trim();
        ensure(
            !org.is_empty() && org != "TBD",
            "completed review requires provider.organization",
        )?;
        ensure(
            !reviewer.is_empty() && reviewer != "TBD",
            "completed review requires provider.reviewer",
        )?;
        ensure(
            unresolved_high_or_critical.is_empty(),
            "completed review cannot have unresolved high/critical findings",
        )?;
    }

    Ok(())
}

fn count_files_recursive(path: &Path) -> Result<usize> {
    if !path.exists() {
        return Ok(0);
    }
    let mut count = 0usize;
    let mut stack = vec![path.to_path_buf()];
    while let Some(next) = stack.pop() {
        for entry in std::fs::read_dir(&next)? {
            let entry = entry?;
            let entry_path = entry.path();
            if entry_path.is_dir() {
                stack.push(entry_path);
            } else if entry_path.is_file() {
                count += 1;
            }
        }
    }
    Ok(count)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    if !src.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path).with_context(|| {
                format!(
                    "failed copying {} to {}",
                    src_path.display(),
                    dst_path.display()
                )
            })?;
        }
    }
    Ok(())
}
