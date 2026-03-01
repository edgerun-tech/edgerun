// SPDX-License-Identifier: GPL-2.0-only
use crate::virtual_fs::LogIngestEntry;
use crate::StorageError;

#[derive(Debug, Clone)]
pub struct DockerLogRecord {
    pub container_id: String,
    pub container_name: String,
    pub stream: String,
    pub ts_unix_ms: u64,
    pub message: String,
}

pub trait DockerLogDecoder {
    fn decode_line(&mut self, line: &str) -> Result<Option<DockerLogRecord>, StorageError>;
}

pub trait DockerLogAdapter {
    fn partition_for(&self, record: &DockerLogRecord) -> String;
    fn to_log_ingest_entry(&self, record: &DockerLogRecord) -> LogIngestEntry;
}

/// Default decoder for non-JSON, pipe-delimited lines:
/// `container_id|container_name|stream|ts_unix_ms|message`
#[derive(Debug, Default)]
pub struct PipeDockerLogDecoder;

impl DockerLogDecoder for PipeDockerLogDecoder {
    fn decode_line(&mut self, line: &str) -> Result<Option<DockerLogRecord>, StorageError> {
        let trimmed = line.trim_end_matches(&['\n', '\r'][..]);
        if trimmed.trim().is_empty() {
            return Ok(None);
        }

        let mut parts = trimmed.splitn(5, '|');
        let container_id = parts.next().unwrap_or_default().trim().to_string();
        let container_name = parts.next().unwrap_or_default().trim().to_string();
        let stream = parts.next().unwrap_or_default().trim().to_string();
        let ts_raw = parts.next().unwrap_or_default().trim().to_string();
        let message = parts.next().unwrap_or_default().to_string();

        if container_id.is_empty() || stream.is_empty() || ts_raw.is_empty() {
            return Err(StorageError::InvalidData(
                "invalid docker log line: expected container_id|container_name|stream|ts_unix_ms|message".to_string(),
            ));
        }
        let ts_unix_ms = ts_raw.parse::<u64>().map_err(|e| {
            StorageError::InvalidData(format!("invalid docker log line timestamp '{ts_raw}': {e}"))
        })?;

        Ok(Some(DockerLogRecord {
            container_id,
            container_name,
            stream,
            ts_unix_ms,
            message,
        }))
    }
}

#[derive(Debug, Clone)]
pub struct DefaultDockerLogAdapter {
    partition_prefix: String,
}

impl Default for DefaultDockerLogAdapter {
    fn default() -> Self {
        Self {
            partition_prefix: "docker".to_string(),
        }
    }
}

impl DefaultDockerLogAdapter {
    pub fn new(partition_prefix: impl Into<String>) -> Self {
        Self {
            partition_prefix: partition_prefix.into(),
        }
    }
}

impl DockerLogAdapter for DefaultDockerLogAdapter {
    fn partition_for(&self, record: &DockerLogRecord) -> String {
        let name = if record.container_name.trim().is_empty() {
            &record.container_id
        } else {
            &record.container_name
        };
        format!(
            "{}.{}",
            sanitize_partition_component(&self.partition_prefix),
            sanitize_partition_component(name)
        )
    }

    fn to_log_ingest_entry(&self, record: &DockerLogRecord) -> LogIngestEntry {
        let entry_key = format!(
            "{}:{}:{}",
            record.container_id, record.stream, record.ts_unix_ms
        );
        let canonical = format!(
            "{}|{}|{}|{}|{}",
            record.container_id,
            record.container_name,
            record.stream,
            record.ts_unix_ms,
            record.message
        );
        let idempotency_key = hex::encode(blake3::hash(canonical.as_bytes()).as_bytes());
        let payload = format!(
            "container_id={}\ncontainer_name={}\nstream={}\nts_unix_ms={}\nmessage={}\n",
            record.container_id,
            record.container_name,
            record.stream,
            record.ts_unix_ms,
            record.message
        )
        .into_bytes();

        LogIngestEntry {
            entry_key,
            entry_payload: payload,
            idempotency_key,
            offset: None,
        }
    }
}

fn sanitize_partition_component(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return "unknown".to_string();
    }
    let mut out = String::with_capacity(trimmed.len());
    for ch in trimmed.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "unknown".to_string()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipe_decoder_parses_non_json_line() {
        let mut decoder = PipeDockerLogDecoder;
        let line = "abc123|web-api|stdout|1709251200123|hello world";
        let rec = decoder
            .decode_line(line)
            .expect("decode")
            .expect("record expected");

        assert_eq!(rec.container_id, "abc123");
        assert_eq!(rec.container_name, "web-api");
        assert_eq!(rec.stream, "stdout");
        assert_eq!(rec.ts_unix_ms, 1709251200123);
        assert_eq!(rec.message, "hello world");
    }

    #[test]
    fn pipe_decoder_preserves_delimiters_in_message() {
        let mut decoder = PipeDockerLogDecoder;
        let line = "abc123|web-api|stderr|1709251200123|hello|with|pipes";
        let rec = decoder
            .decode_line(line)
            .expect("decode")
            .expect("record expected");
        assert_eq!(rec.message, "hello|with|pipes");
    }

    #[test]
    fn default_adapter_builds_stable_idempotency_key() {
        let adapter = DefaultDockerLogAdapter::default();
        let record = DockerLogRecord {
            container_id: "cid".to_string(),
            container_name: "svc".to_string(),
            stream: "stdout".to_string(),
            ts_unix_ms: 42,
            message: "m".to_string(),
        };
        let a = adapter.to_log_ingest_entry(&record);
        let b = adapter.to_log_ingest_entry(&record);
        assert_eq!(a.idempotency_key, b.idempotency_key);
        assert!(a.idempotency_key.len() >= 32);
        assert_eq!(adapter.partition_for(&record), "docker.svc");
    }
}
