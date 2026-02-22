// SPDX-License-Identifier: Apache-2.0
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Result};
use edgerun_storage::durability::DurabilityLevel;
use edgerun_storage::event::{ActorId, Event as StorageEvent, StreamId};
use edgerun_storage::{EngineAppendSession, StorageEngine};
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter, Layer, Registry};

static LOGGING_INIT: OnceLock<()> = OnceLock::new();

#[derive(Debug, Clone, Copy)]
pub enum LogProviderKind {
    Pretty,
    Compact,
    Json,
    None,
}

impl FromStr for LogProviderKind {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "pretty" => Ok(Self::Pretty),
            "compact" => Ok(Self::Compact),
            "json" => Ok(Self::Json),
            "none" | "disabled" | "off" => Ok(Self::None),
            other => Err(anyhow!(
                "unsupported EDGERUN_LOG_PROVIDER='{other}', expected pretty|compact|json|none"
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub struct InitOptions {
    pub service_name: &'static str,
    pub default_level: &'static str,
    pub enable_log_bridge: bool,
}

impl InitOptions {
    pub const fn new(service_name: &'static str) -> Self {
        Self {
            service_name,
            default_level: "info",
            enable_log_bridge: false,
        }
    }
}

pub fn init_service(service_name: &'static str) -> Result<()> {
    init(InitOptions::new(service_name))
}

pub fn init(options: InitOptions) -> Result<()> {
    if LOGGING_INIT.get().is_some() {
        return Ok(());
    }
    if options.enable_log_bridge {
        let _ = tracing_log::LogTracer::init();
    }

    let provider = read_provider()?;
    let filter = read_filter(options.default_level)?;
    install_subscriber(provider, filter, options.service_name)?;
    LOGGING_INIT.set(()).ok();
    tracing::debug!(
        service = options.service_name,
        provider = ?provider,
        "logging initialized"
    );
    Ok(())
}

fn install_subscriber(
    provider: LogProviderKind,
    filter: EnvFilter,
    service_name: &'static str,
) -> Result<()> {
    let storage_layer = init_storage_log_layer(service_name);
    match (provider, storage_layer) {
        (LogProviderKind::Pretty, Some(storage)) => {
            let fmt_layer = fmt::layer()
                .with_target(true)
                .with_thread_names(true)
                .with_thread_ids(true)
                .with_ansi(read_env_bool("EDGERUN_LOG_ANSI", true))
                .pretty();
            Registry::default()
                .with(filter)
                .with(storage)
                .with(fmt_layer)
                .try_init()?;
        }
        (LogProviderKind::Pretty, None) => {
            let fmt_layer = fmt::layer()
                .with_target(true)
                .with_thread_names(true)
                .with_thread_ids(true)
                .with_ansi(read_env_bool("EDGERUN_LOG_ANSI", true))
                .pretty();
            Registry::default()
                .with(filter)
                .with(fmt_layer)
                .try_init()?;
        }
        (LogProviderKind::Compact, Some(storage)) => {
            let fmt_layer = fmt::layer()
                .with_target(true)
                .with_thread_names(true)
                .with_thread_ids(true)
                .with_ansi(read_env_bool("EDGERUN_LOG_ANSI", true))
                .compact();
            Registry::default()
                .with(filter)
                .with(storage)
                .with(fmt_layer)
                .try_init()?;
        }
        (LogProviderKind::Compact, None) => {
            let fmt_layer = fmt::layer()
                .with_target(true)
                .with_thread_names(true)
                .with_thread_ids(true)
                .with_ansi(read_env_bool("EDGERUN_LOG_ANSI", true))
                .compact();
            Registry::default()
                .with(filter)
                .with(fmt_layer)
                .try_init()?;
        }
        (LogProviderKind::Json, Some(storage)) => {
            let fmt_layer = fmt::layer()
                .with_target(true)
                .with_thread_names(true)
                .with_thread_ids(true)
                .json();
            Registry::default()
                .with(filter)
                .with(storage)
                .with(fmt_layer)
                .try_init()?;
        }
        (LogProviderKind::Json, None) => {
            let fmt_layer = fmt::layer()
                .with_target(true)
                .with_thread_names(true)
                .with_thread_ids(true)
                .json();
            Registry::default()
                .with(filter)
                .with(fmt_layer)
                .try_init()?;
        }
        (LogProviderKind::None, Some(storage)) => {
            Registry::default().with(filter).with(storage).try_init()?;
        }
        (LogProviderKind::None, None) => {
            Registry::default().with(filter).try_init()?;
        }
    }
    Ok(())
}

fn read_provider() -> Result<LogProviderKind> {
    let raw = std::env::var("EDGERUN_LOG_PROVIDER").unwrap_or_else(|_| "pretty".to_string());
    LogProviderKind::from_str(&raw)
}

fn read_filter(default_level: &str) -> Result<EnvFilter> {
    if let Ok(value) = std::env::var("RUST_LOG") {
        return EnvFilter::try_new(value).map_err(|e| anyhow!("invalid RUST_LOG value: {e}"));
    }
    let fallback = std::env::var("EDGERUN_LOG_LEVEL").unwrap_or_else(|_| default_level.to_string());
    EnvFilter::try_new(fallback).map_err(|e| anyhow!("invalid EDGERUN_LOG_LEVEL value: {e}"))
}

fn read_env_bool(name: &str, default: bool) -> bool {
    std::env::var(name)
        .ok()
        .map(|v| {
            matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(default)
}

fn init_storage_log_layer(service_name: &'static str) -> Option<StorageLogLayer> {
    if !read_env_bool("EDGERUN_LOG_ENGINE_ENABLED", true) {
        return None;
    }
    let data_dir = std::env::var("EDGERUN_LOG_ENGINE_DIR")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("out/log-engine"));
    let segment_name = std::env::var("EDGERUN_LOG_ENGINE_SEGMENT")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .map(|v| sanitize_segment_name(&v))
        .unwrap_or_else(|| format!("{}.seg", sanitize_segment_name(service_name)));
    let durability = read_log_engine_durability();
    match StorageLogSink::open(data_dir, segment_name, service_name, durability) {
        Ok(sink) => Some(StorageLogLayer {
            sink: Arc::new(sink),
        }),
        Err(err) => {
            eprintln!("edgerun-observability: storage log sink disabled: {err}");
            None
        }
    }
}

fn read_log_engine_durability() -> DurabilityLevel {
    let value =
        std::env::var("EDGERUN_LOG_ENGINE_DURABILITY").unwrap_or_else(|_| "ack_local".to_string());
    match value.trim().to_ascii_lowercase().as_str() {
        "ack_buffered" | "buffered" => DurabilityLevel::AckBuffered,
        "ack_local" | "local" => DurabilityLevel::AckLocal,
        "ack_durable" | "durable" => DurabilityLevel::AckDurable,
        "ack_checkpointed" | "checkpointed" => DurabilityLevel::AckCheckpointed,
        _ => DurabilityLevel::AckLocal,
    }
}

fn sanitize_segment_name(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "logs.seg".to_string()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_segment_name_replaces_unsafe_chars() {
        assert_eq!(
            sanitize_segment_name("worker/logs?.seg"),
            "worker_logs_.seg"
        );
    }

    #[test]
    fn sanitize_segment_name_falls_back_when_empty() {
        assert_eq!(sanitize_segment_name(""), "logs.seg");
    }
}

struct StorageLogLayer {
    sink: Arc<StorageLogSink>,
}

impl<S> Layer<S> for StorageLogLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        self.sink.append_event(event);
    }
}

struct StorageLogSink {
    session: Mutex<EngineAppendSession>,
    stream_id: StreamId,
    actor_id: ActorId,
    service_name: &'static str,
    durability: DurabilityLevel,
}

impl StorageLogSink {
    fn open(
        data_dir: PathBuf,
        segment_file: String,
        service_name: &'static str,
        durability: DurabilityLevel,
    ) -> Result<Self> {
        let engine = StorageEngine::new(data_dir)?;
        let session = engine.create_append_session(&segment_file, 128 * 1024 * 1024)?;
        let stream_id = stream_id_for_service(service_name);
        let actor_id = actor_id_for_service(service_name);
        Ok(Self {
            session: Mutex::new(session),
            stream_id,
            actor_id,
            service_name,
            durability,
        })
    }

    fn append_event(&self, event: &Event<'_>) {
        let meta = event.metadata();
        let mut fields = JsonFieldVisitor::default();
        event.record(&mut fields);

        let payload = serde_json::json!({
            "ts_unix_ms": now_unix_ms(),
            "service": self.service_name,
            "level": meta.level().as_str(),
            "target": meta.target(),
            "name": meta.name(),
            "fields": fields.values,
        });
        let payload_bytes = match serde_json::to_vec(&payload) {
            Ok(v) => v,
            Err(_) => return,
        };

        let storage_event =
            StorageEvent::new(self.stream_id.clone(), self.actor_id.clone(), payload_bytes);
        let Ok(mut guard) = self.session.lock() else {
            return;
        };
        if let Err(err) = guard.append_with_durability(&storage_event, self.durability) {
            eprintln!("edgerun-observability: storage append failed: {err}");
        }
    }
}

fn stream_id_for_service(service_name: &str) -> StreamId {
    let digest = blake3::hash(format!("edgerun-log-stream::{service_name}").as_bytes());
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest.as_bytes()[..16]);
    StreamId::from_bytes(bytes)
}

fn actor_id_for_service(service_name: &str) -> ActorId {
    let digest = blake3::hash(format!("edgerun-log-actor::{service_name}").as_bytes());
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest.as_bytes()[..16]);
    ActorId::from_bytes(bytes)
}

fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

#[derive(Default)]
struct JsonFieldVisitor {
    values: BTreeMap<String, String>,
}

impl Visit for JsonFieldVisitor {
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.values
            .insert(field.name().to_string(), value.to_string());
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.values
            .insert(field.name().to_string(), value.to_string());
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.values
            .insert(field.name().to_string(), value.to_string());
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.values
            .insert(field.name().to_string(), value.to_string());
    }

    fn record_error(&mut self, field: &Field, value: &(dyn std::error::Error + 'static)) {
        self.values
            .insert(field.name().to_string(), value.to_string());
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.values
            .insert(field.name().to_string(), format!("{value:?}"));
    }
}
