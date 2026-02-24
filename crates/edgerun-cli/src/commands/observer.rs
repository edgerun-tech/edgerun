// SPDX-License-Identifier: Apache-2.0
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use edgerun_storage::durability::DurabilityLevel;
use edgerun_storage::event::{ActorId, Event as StorageEvent, StreamId};
use edgerun_storage::StorageEngine;
use json::JsonValue;

use crate::{ObserveCommand, ObserveDurability};

static EVENT_COUNTER: AtomicU64 = AtomicU64::new(1);

pub(crate) fn run_observer_command(root: &Path, command: ObserveCommand) -> Result<()> {
    match command {
        ObserveCommand::Append {
            job_id,
            run_id,
            actor,
            event_type,
            payload_json,
            payload_file,
            prev_event_hash,
            data_dir,
            segment,
            durability,
        } => {
            let payload = load_payload(payload_json, payload_file)?;
            let mut sink = ObserverSink::open(data_dir, &segment)?;
            let envelope = build_event_envelope(
                &job_id,
                &run_id,
                &actor,
                &event_type,
                payload,
                prev_event_hash,
            )?;
            sink.append(&job_id, &actor, &envelope, to_durability(durability))?;
            let out = json::object! {
                ok: true,
                job_id: job_id,
                event_id: envelope["event_id"].clone(),
                event_hash: envelope["event_hash"].clone(),
            };
            println!("{}", out.pretty(2));
            Ok(())
        }
        ObserveCommand::IngestStdio {
            job_id,
            run_id,
            actor,
            event_type,
            data_dir,
            segment,
            durability,
        } => {
            let stdin = io::stdin();
            let mut sink = ObserverSink::open(data_dir, &segment)?;
            let mut prev_hash: Option<String> = None;
            let mut count = 0u64;
            for line in stdin.lock().lines() {
                let line = line.context("failed to read stdin line")?;
                let payload = json::object! { line: line };
                let envelope = build_event_envelope(
                    &job_id,
                    &run_id,
                    &actor,
                    &event_type,
                    payload,
                    prev_hash.clone(),
                )?;
                prev_hash = envelope["event_hash"].as_str().map(ToString::to_string);
                sink.append(&job_id, &actor, &envelope, to_durability(durability))?;
                count = count.saturating_add(1);
            }
            println!(
                "{}",
                json::object! {
                    ok: true,
                    job_id: job_id,
                    events_written: count
                }
                .pretty(2)
            );
            Ok(())
        }
        ObserveCommand::IngestGitChanges {
            job_id,
            run_id,
            actor,
            event_type,
            base_ref,
            repo_root,
            include_untracked,
            dry_run,
            data_dir,
            segment,
            durability,
        } => {
            let repo = repo_root.unwrap_or_else(|| root.to_path_buf());
            let changes = collect_git_file_changes(&repo, &base_ref, include_untracked)?;
            let mut prev_hash: Option<String> = None;

            if dry_run {
                let preview = changes
                    .iter()
                    .map(|change| {
                        json::object! {
                            path: change.path.clone(),
                            git_status: change.git_status.clone(),
                            change_scope: change.change_scope.clone(),
                            additions: change.additions.map(|v| v as u64),
                            deletions: change.deletions.map(|v| v as u64),
                        }
                    })
                    .collect::<Vec<_>>();
                println!(
                    "{}",
                    json::object! {
                        ok: true,
                        dry_run: true,
                        job_id: job_id,
                        run_id: run_id,
                        base_ref: base_ref,
                        include_untracked: include_untracked,
                        planned_events: preview.len(),
                        events: preview
                    }
                    .pretty(2)
                );
                return Ok(());
            }

            let mut sink = ObserverSink::open(data_dir, &segment)?;
            let mut written = 0u64;
            for change in &changes {
                let payload = json::object! {
                    path: change.path.clone(),
                    git_status: change.git_status.clone(),
                    change_scope: change.change_scope.clone(),
                    additions: change.additions.map(|v| v as u64),
                    deletions: change.deletions.map(|v| v as u64),
                    base_ref: base_ref.clone(),
                    include_untracked: include_untracked,
                };
                let envelope = build_event_envelope(
                    &job_id,
                    &run_id,
                    &actor,
                    &event_type,
                    payload,
                    prev_hash.clone(),
                )?;
                prev_hash = envelope["event_hash"].as_str().map(ToString::to_string);
                sink.append(&job_id, &actor, &envelope, to_durability(durability))?;
                written = written.saturating_add(1);
            }

            println!(
                "{}",
                json::object! {
                    ok: true,
                    job_id: job_id,
                    run_id: run_id,
                    base_ref: base_ref,
                    include_untracked: include_untracked,
                    events_written: written,
                    last_event_hash: prev_hash,
                }
                .pretty(2)
            );
            Ok(())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GitFileChange {
    path: String,
    git_status: String,
    change_scope: String,
    additions: Option<usize>,
    deletions: Option<usize>,
}

fn collect_git_file_changes(
    repo_root: &Path,
    base_ref: &str,
    include_untracked: bool,
) -> Result<Vec<GitFileChange>> {
    let name_status_output = run_git_output(
        repo_root,
        &[
            "diff",
            "--name-status",
            "--find-renames=90%",
            base_ref,
            "--",
        ],
    )?;
    let numstat_output = run_git_output(repo_root, &["diff", "--numstat", base_ref, "--"])?;

    let statuses = parse_name_status_output(&name_status_output);
    let numstats = parse_numstat_output(&numstat_output);
    let mut by_path: BTreeMap<String, GitFileChange> = BTreeMap::new();

    for (path, status) in statuses {
        let (additions, deletions) = numstats.get(&path).copied().unwrap_or((None, None));
        by_path.insert(
            path.clone(),
            GitFileChange {
                path: path.clone(),
                git_status: status,
                change_scope: classify_change_scope(&path).to_string(),
                additions,
                deletions,
            },
        );
    }

    if include_untracked {
        let untracked_output =
            run_git_output(repo_root, &["ls-files", "--others", "--exclude-standard"])?;
        for path in untracked_output
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
        {
            by_path
                .entry(path.to_string())
                .or_insert_with(|| GitFileChange {
                    path: path.to_string(),
                    git_status: "??".to_string(),
                    change_scope: classify_change_scope(path).to_string(),
                    additions: None,
                    deletions: None,
                });
        }
    }

    Ok(by_path.into_values().collect())
}

fn run_git_output(repo_root: &Path, args: &[&str]) -> Result<String> {
    let repo_arg = repo_root.to_string_lossy().to_string();
    let output = Command::new("git")
        .arg("-C")
        .arg(&repo_arg)
        .args(args)
        .output()
        .with_context(|| format!("failed to run git command: git {}", args.join(" ")))?;

    if !output.status.success() {
        return Err(anyhow!(
            "git command failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn parse_name_status_output(output: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let parts = line.split('\t').collect::<Vec<_>>();
        if parts.is_empty() {
            continue;
        }

        let raw_status = parts[0];
        let status = normalize_git_status(raw_status);
        let path = if raw_status.starts_with('R') || raw_status.starts_with('C') {
            parts
                .get(2)
                .copied()
                .unwrap_or(parts.get(1).copied().unwrap_or(""))
        } else {
            parts.get(1).copied().unwrap_or("")
        };
        if path.is_empty() {
            continue;
        }
        out.push((path.to_string(), status.to_string()));
    }
    out
}

fn parse_numstat_output(output: &str) -> BTreeMap<String, (Option<usize>, Option<usize>)> {
    let mut map = BTreeMap::new();
    for line in output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let parts = line.split('\t').collect::<Vec<_>>();
        if parts.len() < 3 {
            continue;
        }
        let additions = parse_numstat_field(parts[0]);
        let deletions = parse_numstat_field(parts[1]);
        let path = parts[2].trim();
        if path.is_empty() {
            continue;
        }
        map.insert(path.to_string(), (additions, deletions));
    }
    map
}

fn parse_numstat_field(raw: &str) -> Option<usize> {
    if raw == "-" {
        return None;
    }
    raw.parse::<usize>().ok()
}

fn normalize_git_status(raw: &str) -> &'static str {
    if raw.starts_with('A') {
        "A"
    } else if raw.starts_with('M') {
        "M"
    } else if raw.starts_with('D') {
        "D"
    } else if raw.starts_with('R') {
        "R"
    } else if raw.starts_with('C') {
        "C"
    } else if raw.starts_with('T') {
        "T"
    } else {
        "?"
    }
}

fn classify_change_scope(path: &str) -> &'static str {
    if let Some(ext) = Path::new(path).extension().and_then(|ext| ext.to_str()) {
        if matches!(
            ext,
            "rs" | "ts"
                | "tsx"
                | "js"
                | "jsx"
                | "mjs"
                | "cjs"
                | "py"
                | "go"
                | "java"
                | "kt"
                | "swift"
                | "c"
                | "cc"
                | "cpp"
                | "h"
                | "hpp"
                | "cs"
                | "sol"
                | "sh"
                | "bash"
                | "zsh"
                | "yaml"
                | "yml"
                | "toml"
                | "json"
                | "md"
                | "mdx"
        ) {
            return "code";
        }
    }
    "file"
}

struct ObserverSink {
    session: edgerun_storage::EngineAppendSession,
}

impl ObserverSink {
    fn open(data_dir: Option<PathBuf>, segment: &str) -> Result<Self> {
        let dir = data_dir.unwrap_or_else(|| PathBuf::from("out/observer-events"));
        let engine = StorageEngine::new(dir)?;
        let session = engine.create_append_session(segment, 128 * 1024 * 1024)?;
        Ok(Self { session })
    }

    fn append(
        &mut self,
        job_id: &str,
        actor: &str,
        envelope: &JsonValue,
        durability: DurabilityLevel,
    ) -> Result<()> {
        let payload = envelope.dump().into_bytes();
        let event = StorageEvent::new(
            stream_id_for_job(job_id),
            actor_id_for_actor(actor),
            payload,
        );
        self.session.append_with_durability(&event, durability)?;
        Ok(())
    }
}

fn load_payload(payload_json: Option<String>, payload_file: Option<PathBuf>) -> Result<JsonValue> {
    match (payload_json, payload_file) {
        (Some(raw), None) => {
            let value: JsonValue = json::parse(&raw).context("invalid --payload-json")?;
            Ok(value)
        }
        (None, Some(path)) => {
            let raw = fs::read_to_string(&path)
                .with_context(|| format!("failed to read payload file {}", path.display()))?;
            let value: JsonValue =
                json::parse(&raw).with_context(|| format!("invalid JSON in {}", path.display()))?;
            Ok(value)
        }
        (None, None) => Ok(json::object! {}),
        (Some(_), Some(_)) => Err(anyhow!(
            "use either --payload-json or --payload-file, not both"
        )),
    }
}

fn build_event_envelope(
    job_id: &str,
    run_id: &str,
    actor: &str,
    event_type: &str,
    payload: JsonValue,
    prev_event_hash: Option<String>,
) -> Result<JsonValue> {
    let ts_unix_ms = now_unix_ms();
    let event_id = format!(
        "{}-{}-{}",
        ts_unix_ms,
        std::process::id(),
        EVENT_COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    let mut base = json::object! {
        event_id: event_id,
        job_id: job_id,
        run_id: run_id,
        ts_unix_ms: ts_unix_ms,
        actor: actor,
        event_type: event_type,
        payload: payload,
        prev_event_hash: prev_event_hash,
    };
    let canonical = base.dump().into_bytes();
    let hash = blake3::hash(&canonical);
    base["event_hash"] = hash.to_hex().to_string().into();
    Ok(base)
}

fn stream_id_for_job(job_id: &str) -> StreamId {
    let digest = blake3::hash(format!("edgerun-observer-stream::{job_id}").as_bytes());
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest.as_bytes()[..16]);
    StreamId::from_bytes(bytes)
}

fn actor_id_for_actor(actor: &str) -> ActorId {
    let digest = blake3::hash(format!("edgerun-observer-actor::{actor}").as_bytes());
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest.as_bytes()[..16]);
    ActorId::from_bytes(bytes)
}

fn to_durability(value: ObserveDurability) -> DurabilityLevel {
    match value {
        ObserveDurability::Buffered => DurabilityLevel::AckBuffered,
        ObserveDurability::Local => DurabilityLevel::AckLocal,
        ObserveDurability::Durable => DurabilityLevel::AckDurable,
        ObserveDurability::Checkpointed => DurabilityLevel::AckCheckpointed,
    }
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{
        classify_change_scope, parse_name_status_output, parse_numstat_field, parse_numstat_output,
    };

    #[test]
    fn parses_name_status_with_rename_target() {
        let input = "M\tcrates/edgerun-cli/src/main.rs\nR100\told/name.rs\tnew/name.rs\n";
        let rows = parse_name_status_output(input);
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0],
            (
                "crates/edgerun-cli/src/main.rs".to_string(),
                "M".to_string()
            )
        );
        assert_eq!(rows[1], ("new/name.rs".to_string(), "R".to_string()));
    }

    #[test]
    fn parses_numstat_fields_and_binary_rows() {
        let input = "12\t7\tfrontend/app/devices/page.tsx\n-\t-\tassets/logo.png\n";
        let rows = parse_numstat_output(input);
        assert_eq!(
            rows.get("frontend/app/devices/page.tsx"),
            Some(&(Some(12), Some(7)))
        );
        assert_eq!(rows.get("assets/logo.png"), Some(&(None, None)));
        assert_eq!(parse_numstat_field("-"), None);
    }

    #[test]
    fn classifies_code_vs_file_scope() {
        assert_eq!(classify_change_scope("src/main.rs"), "code");
        assert_eq!(classify_change_scope("frontend/app/page.tsx"), "code");
        assert_eq!(classify_change_scope("public/logo.png"), "file");
    }
}
