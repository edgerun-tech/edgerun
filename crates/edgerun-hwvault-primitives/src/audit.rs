// SPDX-License-Identifier: Apache-2.0
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
    let line = serde_json::to_string(event)?;
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
        if let Ok(evt) = serde_json::from_str::<PolicyAuditEvent>(&line) {
            events.push(evt);
        }
    }
    events.reverse();
    if events.len() > limit {
        events.truncate(limit);
    }
    Ok(events)
}
