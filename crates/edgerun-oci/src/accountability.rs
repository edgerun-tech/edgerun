//! Accountable runtime event logging for edgerun-oci.
//!
//! This module is intentionally small and OCI-compatible:
//! - normal OCI bundles keep running when no EdgeRun accountability annotations exist;
//! - v0 writes an append-only JSONL event log under the existing container state dir;
//! - events are hash chained for tamper evidence before hardware-backed signing exists.
//!
//! Runtime events describe facts observed by the runtime. Application/domain
//! events should be emitted by a wrapper or SDK and joined by `run_id` later.

use crate::prelude::*;
use crate::spec::OciSpec;
use crate::state::container_state_dir;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub const ANNOTATION_VERSION: &str = "org.edgerun.accountability.version";
pub const ANNOTATION_ENABLED: &str = "org.edgerun.accountability.enabled";
pub const ANNOTATION_LOG: &str = "org.edgerun.accountability.log";
pub const ANNOTATION_SIGNER: &str = "org.edgerun.accountability.signer";
pub const SCHEMA: &str = "edgerun.runtime.event.v0";
pub const DEFAULT_LOG_NAME: &str = "events.log";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountabilitySigner {
    None,
    Software,
    OsBound,
    Hardware,
    MeasuredHardware,
}

impl AccountabilitySigner {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Software => "software",
            Self::OsBound => "os-bound",
            Self::Hardware => "hardware",
            Self::MeasuredHardware => "measured-hardware",
        }
    }

    pub fn trust_level(self) -> &'static str {
        match self {
            Self::None => "runtime-software-hash-chain",
            Self::Software => "runtime-software-key",
            Self::OsBound => "runtime-os-bound-key",
            Self::Hardware => "runtime-hardware-key",
            Self::MeasuredHardware => "runtime-measured-hardware-key",
        }
    }
}

impl Default for AccountabilitySigner {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone)]
pub struct AccountabilityConfig {
    pub enabled: bool,
    pub version: String,
    pub log_target: String,
    pub signer: AccountabilitySigner,
}

impl AccountabilityConfig {
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            version: String::new(),
            log_target: String::new(),
            signer: AccountabilitySigner::None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RuntimeEvent<'a> {
    pub event: &'a str,
    pub status: &'a str,
    pub pid: Option<u32>,
    pub bundle: Option<&'a str>,
    /// JSON object fragment for extra fields. Caller is responsible for passing valid JSON.
    pub payload_json: Option<&'a str>,
}

pub fn accountability_config(spec: &OciSpec) -> AccountabilityConfig {
    let Some(annotations) = spec.annotations.as_ref() else {
        return AccountabilityConfig::disabled();
    };

    let version = annotations
        .get(ANNOTATION_VERSION)
        .cloned()
        .unwrap_or_default();
    let enabled = annotations
        .get(ANNOTATION_ENABLED)
        .map(|value| matches!(value.as_str(), "true" | "1" | "yes" | "on"))
        .unwrap_or(false)
        || !version.is_empty();

    if !enabled {
        return AccountabilityConfig::disabled();
    }

    let log_target = annotations
        .get(ANNOTATION_LOG)
        .cloned()
        .unwrap_or_else(|| format!("state://{DEFAULT_LOG_NAME}"));
    let signer = annotations
        .get(ANNOTATION_SIGNER)
        .map(|value| parse_signer(value))
        .unwrap_or_default();

    AccountabilityConfig {
        enabled,
        version,
        log_target,
        signer,
    }
}

pub fn accountability_enabled(spec: &OciSpec) -> bool {
    accountability_config(spec).enabled
}

pub fn events_log_path(container_id: &str, config: &AccountabilityConfig) -> io::Result<PathBuf> {
    if config.log_target.is_empty() || config.log_target == "state://events.log" {
        return Ok(container_state_dir(container_id).join(DEFAULT_LOG_NAME));
    }

    if let Some(name) = config.log_target.strip_prefix("state://") {
        if name.is_empty() || name.contains('/') || name.contains("..") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid accountability log target: {}", config.log_target),
            ));
        }
        return Ok(container_state_dir(container_id).join(name));
    }

    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        format!(
            "unsupported accountability log target {}; only state:// targets are supported in v0",
            config.log_target
        ),
    ))
}

pub fn append_runtime_event(
    spec: &OciSpec,
    container_id: &str,
    runtime_event: RuntimeEvent<'_>,
) -> io::Result<Option<String>> {
    let config = accountability_config(spec);
    if !config.enabled {
        return Ok(None);
    }

    let log_path = events_log_path(container_id, &config)?;
    append_runtime_event_with_config(container_id, &config, &log_path, runtime_event).map(Some)
}

pub fn append_runtime_event_with_config(
    container_id: &str,
    config: &AccountabilityConfig,
    log_path: &PathBuf,
    runtime_event: RuntimeEvent<'_>,
) -> io::Result<String> {
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let seq = next_sequence(log_path)?;
    let prev_hash = previous_line_hash(log_path)?;
    let timestamp_unix_ms = now_unix_ms();
    let pid_json = runtime_event
        .pid
        .map(|pid| pid.to_string())
        .unwrap_or_else(|| "null".into());
    let bundle_json = runtime_event
        .bundle
        .map(|bundle| quote_json(bundle))
        .unwrap_or_else(|| "null".into());
    let payload_json = runtime_event.payload_json.unwrap_or("{}");

    let body = format!(
        "{{\"schema\":\"{}\",\"seq\":{},\"prev_hash\":\"{}\",\"timestamp_unix_ms\":{},\"runtime\":\"edgerun-oci\",\"container_id\":{},\"event\":{},\"status\":{},\"pid\":{},\"bundle\":{},\"payload\":{},\"trust\":{{\"level\":\"{}\",\"signer\":\"{}\",\"hash\":\"sha256\"}}}}",
        SCHEMA,
        seq,
        prev_hash,
        timestamp_unix_ms,
        quote_json(container_id),
        quote_json(runtime_event.event),
        quote_json(runtime_event.status),
        pid_json,
        bundle_json,
        payload_json,
        config.signer.trust_level(),
        config.signer.as_str(),
    );
    let event_hash = digest_ref(body.as_bytes());
    let line = insert_event_hash(&body, &event_hash);

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    file.write_all(line.as_bytes())?;
    file.write_all(b"\n")?;
    file.sync_data()?;

    Ok(event_hash)
}

fn parse_signer(value: &str) -> AccountabilitySigner {
    match value {
        "software" | "software-key" => AccountabilitySigner::Software,
        "os-bound" | "keyring" | "kms" | "vault" => AccountabilitySigner::OsBound,
        "hardware" | "tpm" | "hsm" | "tee" => AccountabilitySigner::Hardware,
        "measured-hardware" | "measured" | "tpm-pcr" => AccountabilitySigner::MeasuredHardware,
        _ => AccountabilitySigner::None,
    }
}

fn next_sequence(path: &PathBuf) -> io::Result<u64> {
    let data = match fs::read_to_string(path) {
        Ok(data) => data,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error),
    };
    Ok(data.lines().filter(|line| !line.trim().is_empty()).count() as u64)
}

fn previous_line_hash(path: &PathBuf) -> io::Result<String> {
    let data = match fs::read(path) {
        Ok(data) => data,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok("sha256:genesis".into()),
        Err(error) => return Err(error),
    };
    let Some(last_line) = data
        .split(|byte| *byte == b'\n')
        .rev()
        .find(|line| !line.is_empty())
    else {
        return Ok("sha256:genesis".into());
    };
    Ok(digest_ref(last_line))
}

fn insert_event_hash(body: &str, event_hash: &str) -> String {
    let without_opening_brace = body.strip_prefix('{').unwrap_or(body);
    format!("{{\"event_hash\":\"{}\",{}", event_hash, without_opening_brace)
}

fn digest_ref(bytes: &[u8]) -> String {
    format!(
        "sha256:{}",
        edgerun_encoding::hex::bytes_to_hex(&edgerun_crypto::sha256(bytes))
    )
}

fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn quote_json(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(all(test, not(target_os = "none")))]
mod tests {
    use super::*;

    #[test]
    fn quote_json_escapes_control_characters() {
        assert_eq!(quote_json("a\"b\\c\n"), "\"a\\\"b\\\\c\\n\"");
    }

    #[test]
    fn parse_signer_defaults_unknown_to_none() {
        assert_eq!(parse_signer("tpm"), AccountabilitySigner::Hardware);
        assert_eq!(parse_signer("wat"), AccountabilitySigner::None);
    }
}
