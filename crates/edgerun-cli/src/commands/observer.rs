// SPDX-License-Identifier: Apache-2.0
use std::fs;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use edgerun_storage::durability::DurabilityLevel;
use edgerun_storage::event::{ActorId, Event as StorageEvent, StreamId};
use edgerun_storage::StorageEngine;
use serde_json::Value;

use crate::{ObserveCommand, ObserveDurability};

static EVENT_COUNTER: AtomicU64 = AtomicU64::new(1);

pub(crate) fn run_observer_command(_root: &Path, command: ObserveCommand) -> Result<()> {
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
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "ok": true,
                    "job_id": job_id,
                    "event_id": envelope["event_id"],
                    "event_hash": envelope["event_hash"],
                }))?
            );
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
                let payload = serde_json::json!({ "line": line });
                let envelope = build_event_envelope(
                    &job_id,
                    &run_id,
                    &actor,
                    &event_type,
                    payload,
                    prev_hash.clone(),
                )?;
                prev_hash = envelope
                    .get("event_hash")
                    .and_then(Value::as_str)
                    .map(ToString::to_string);
                sink.append(&job_id, &actor, &envelope, to_durability(durability))?;
                count = count.saturating_add(1);
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "ok": true,
                    "job_id": job_id,
                    "events_written": count
                }))?
            );
            Ok(())
        }
    }
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
        envelope: &Value,
        durability: DurabilityLevel,
    ) -> Result<()> {
        let payload = serde_json::to_vec(envelope)?;
        let event = StorageEvent::new(stream_id_for_job(job_id), actor_id_for_actor(actor), payload);
        self.session.append_with_durability(&event, durability)?;
        Ok(())
    }
}

fn load_payload(payload_json: Option<String>, payload_file: Option<PathBuf>) -> Result<Value> {
    match (payload_json, payload_file) {
        (Some(raw), None) => {
            let value: Value = serde_json::from_str(&raw).context("invalid --payload-json")?;
            Ok(value)
        }
        (None, Some(path)) => {
            let raw = fs::read_to_string(&path)
                .with_context(|| format!("failed to read payload file {}", path.display()))?;
            let value: Value = serde_json::from_str(&raw)
                .with_context(|| format!("invalid JSON in {}", path.display()))?;
            Ok(value)
        }
        (None, None) => Ok(serde_json::json!({})),
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
    payload: Value,
    prev_event_hash: Option<String>,
) -> Result<Value> {
    let ts_unix_ms = now_unix_ms();
    let event_id = format!(
        "{}-{}-{}",
        ts_unix_ms,
        std::process::id(),
        EVENT_COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    let mut base = serde_json::json!({
        "event_id": event_id,
        "job_id": job_id,
        "run_id": run_id,
        "ts_unix_ms": ts_unix_ms,
        "actor": actor,
        "event_type": event_type,
        "payload": payload,
        "prev_event_hash": prev_event_hash,
    });
    let canonical = serde_json::to_vec(&base)?;
    let hash = blake3::hash(&canonical);
    base["event_hash"] = Value::String(hash.to_hex().to_string());
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

fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}
