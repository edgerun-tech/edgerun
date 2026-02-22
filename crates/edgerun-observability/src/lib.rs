// SPDX-License-Identifier: Apache-2.0
use std::str::FromStr;
use std::sync::OnceLock;

use anyhow::{anyhow, Result};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter, Registry};

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

pub trait LogProvider {
    fn install(&self, filter: EnvFilter) -> Result<()>;
}

struct PrettyProvider;
struct CompactProvider;
struct JsonProvider;
struct NoneProvider;

impl LogProvider for PrettyProvider {
    fn install(&self, filter: EnvFilter) -> Result<()> {
        let layer = fmt::layer()
            .with_target(true)
            .with_thread_names(true)
            .with_thread_ids(true)
            .with_ansi(read_env_bool("EDGERUN_LOG_ANSI", true))
            .pretty();
        Registry::default().with(filter).with(layer).try_init()?;
        Ok(())
    }
}

impl LogProvider for CompactProvider {
    fn install(&self, filter: EnvFilter) -> Result<()> {
        let layer = fmt::layer()
            .with_target(true)
            .with_thread_names(true)
            .with_thread_ids(true)
            .with_ansi(read_env_bool("EDGERUN_LOG_ANSI", true))
            .compact();
        Registry::default().with(filter).with(layer).try_init()?;
        Ok(())
    }
}

impl LogProvider for JsonProvider {
    fn install(&self, filter: EnvFilter) -> Result<()> {
        let layer = fmt::layer()
            .with_target(true)
            .with_thread_names(true)
            .with_thread_ids(true)
            .json();
        Registry::default().with(filter).with(layer).try_init()?;
        Ok(())
    }
}

impl LogProvider for NoneProvider {
    fn install(&self, filter: EnvFilter) -> Result<()> {
        Registry::default().with(filter).try_init()?;
        Ok(())
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
    let provider_impl: Box<dyn LogProvider + Send + Sync> = match provider {
        LogProviderKind::Pretty => Box::new(PrettyProvider),
        LogProviderKind::Compact => Box::new(CompactProvider),
        LogProviderKind::Json => Box::new(JsonProvider),
        LogProviderKind::None => Box::new(NoneProvider),
    };

    let filter = read_filter(options.default_level)?;
    provider_impl.install(filter)?;
    LOGGING_INIT.set(()).ok();
    tracing::debug!(
        service = options.service_name,
        provider = ?provider,
        "logging initialized"
    );
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
