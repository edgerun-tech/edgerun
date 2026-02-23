// SPDX-License-Identifier: Apache-2.0
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyAuditEvent {
    pub ts: u64,
    pub action: String,
    pub target: String,
    pub details: String,
}

pub fn append_event_jsonl(path: &Path, event: &PolicyAuditEvent) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut f = OpenOptions::new().create(true).append(true).open(path)?;
    let line = json::stringify(json::object! {
        ts: event.ts,
        action: event.action.as_str(),
        target: event.target.as_str(),
        details: event.details.as_str(),
    });
    f.write_all(line.as_bytes())?;
    f.write_all(b"\n")?;
    f.flush()?;
    Ok(())
}

pub fn list_recent_events_jsonl(
    path: &Path,
    limit: usize,
) -> std::io::Result<Vec<PolicyAuditEvent>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let f = OpenOptions::new().read(true).open(path)?;
    let reader = BufReader::new(f);
    let mut events = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let Ok(value) = json::parse(&line) else {
            continue;
        };
        let Some(ts) = value["ts"].as_u64() else {
            continue;
        };
        let Some(action) = value["action"].as_str() else {
            continue;
        };
        let Some(target) = value["target"].as_str() else {
            continue;
        };
        let Some(details) = value["details"].as_str() else {
            continue;
        };
        events.push(PolicyAuditEvent {
            ts,
            action: action.to_string(),
            target: target.to_string(),
            details: details.to_string(),
        });
    }
    events.reverse();
    if events.len() > limit {
        events.truncate(limit);
    }
    Ok(events)
}
