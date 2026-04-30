//! First-party HTTP analytics for Edgerun-hosted surfaces.
//!
//! The module is intentionally small and host-only. It records coarse pageview
//! events to local TSV files, does not set cookies, does not beacon to a third
//! party, and avoids query strings by default.

#![cfg_attr(target_os = "none", no_std)]

extern crate alloc;
#[cfg(not(target_os = "none"))]
extern crate std;

#[cfg(target_os = "none")]
compile_error!("edgerun-analytics is host-only because it writes local files");

use alloc::boxed::Box;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use core::future::Future;
use core::pin::Pin;
use edgerun_http::{Handler, Request, Response};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug)]
pub struct AnalyticsConfig {
    pub log_dir: PathBuf,
    pub include_static_assets: bool,
    pub max_path_len: usize,
}

impl AnalyticsConfig {
    pub fn new(log_dir: impl Into<PathBuf>) -> Self {
        Self {
            log_dir: log_dir.into(),
            include_static_assets: false,
            max_path_len: 180,
        }
    }
}

#[derive(Debug)]
pub struct Analytics {
    config: AnalyticsConfig,
    lock: Mutex<()>,
}

impl Analytics {
    pub fn new(config: AnalyticsConfig) -> Self {
        Self {
            config,
            lock: Mutex::new(()),
        }
    }

    pub fn record(&self, request: &Request, response: &Response) -> io::Result<()> {
        let path = normalized_path(&request.uri().request_target(), self.config.max_path_len);
        if !self.config.include_static_assets && is_static_asset(&path) {
            return Ok(());
        }

        let now = unix_seconds();
        let host = header(request, "host");
        let referer = referer_origin(header(request, "referer").as_deref().unwrap_or_default());
        let ua = user_agent_family(header(request, "user-agent").as_deref().unwrap_or_default());
        let line = format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            now,
            sanitize_field(request.method().as_str()),
            sanitize_field(&host.unwrap_or_default()),
            sanitize_field(&path),
            response.status().as_u16(),
            sanitize_field(&referer),
            sanitize_field(&ua)
        );

        let _guard = self
            .lock
            .lock()
            .map_err(|_| io::Error::other("analytics lock poisoned"))?;
        fs::create_dir_all(&self.config.log_dir)?;
        let path = self
            .config
            .log_dir
            .join(format!("events-{}.tsv", now / 86_400));
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        file.write_all(line.as_bytes())
    }

    pub fn log_dir(&self) -> &Path {
        &self.config.log_dir
    }
}

#[derive(Clone)]
pub struct AnalyticsHandler {
    inner: Arc<dyn Handler>,
    analytics: Arc<Analytics>,
}

impl AnalyticsHandler {
    pub fn new<H: Handler>(inner: H, config: AnalyticsConfig) -> Self {
        Self {
            inner: Arc::new(inner),
            analytics: Arc::new(Analytics::new(config)),
        }
    }

    pub fn with_shared(inner: Arc<dyn Handler>, analytics: Arc<Analytics>) -> Self {
        Self { inner, analytics }
    }

    pub fn analytics(&self) -> Arc<Analytics> {
        Arc::clone(&self.analytics)
    }
}

impl Handler for AnalyticsHandler {
    fn handle(&self, request: Request) -> Pin<Box<dyn Future<Output = Response> + Send + '_>> {
        Box::pin(async move {
            let snapshot = request.clone();
            let response = self.inner.handle(request).await;
            if let Err(error) = self.analytics.record(&snapshot, &response) {
                edgerun_log::warn!("analytics record failed: {}", error);
            }
            response
        })
    }
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn header(request: &Request, name: &str) -> Option<String> {
    request
        .headers()
        .get(name)
        .map(|value| value.as_str().to_string())
}

fn normalized_path(target: &str, max_len: usize) -> String {
    let raw = target.split('?').next().unwrap_or(target);
    let path = if raw.is_empty() { "/" } else { raw };
    let mut out = String::new();
    for ch in path.chars().take(max_len) {
        if ch.is_ascii_graphic() || ch == ' ' {
            out.push(ch);
        }
    }
    if out.is_empty() {
        "/".to_string()
    } else {
        out
    }
}

fn is_static_asset(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    matches!(
        lower.rsplit('.').next(),
        Some("css" | "js" | "svg" | "png" | "jpg" | "jpeg" | "gif" | "webp" | "ico" | "wasm")
    ) || lower == "/favicon.ico"
        || lower == "/favicon.svg"
        || lower == "/robots.txt"
        || lower == "/site.webmanifest"
        || lower == "/opensearch.xml"
}

fn referer_origin(value: &str) -> String {
    let Some(rest) = value
        .strip_prefix("https://")
        .or_else(|| value.strip_prefix("http://"))
    else {
        return String::new();
    };
    let scheme = if value.starts_with("https://") {
        "https://"
    } else {
        "http://"
    };
    let host = rest.split('/').next().unwrap_or_default();
    if host.is_empty() {
        String::new()
    } else {
        format!("{scheme}{host}")
    }
}

fn user_agent_family(value: &str) -> String {
    let lower = value.to_ascii_lowercase();
    if lower.contains("bot") || lower.contains("crawler") || lower.contains("spider") {
        "bot".to_string()
    } else if lower.contains("firefox") {
        "firefox".to_string()
    } else if lower.contains("edg/") {
        "edge".to_string()
    } else if lower.contains("chrome") || lower.contains("chromium") {
        "chromium".to_string()
    } else if lower.contains("safari") {
        "safari".to_string()
    } else if lower.is_empty() {
        String::new()
    } else {
        "other".to_string()
    }
}

fn sanitize_field(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\t' | '\r' | '\n' => out.push(' '),
            ch if ch.is_control() => {}
            ch => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_http::{Method, Request, Response, StatusCode};
    use std::time::SystemTime;

    #[test]
    fn normalizes_path_without_query() {
        assert_eq!(normalized_path("/surface/blog?a=b", 100), "/surface/blog");
        assert_eq!(normalized_path("", 100), "/");
    }

    #[test]
    fn keeps_only_referer_origin() {
        assert_eq!(
            referer_origin("https://example.test/path?q=secret"),
            "https://example.test"
        );
    }

    #[test]
    fn records_pageview_tsv_without_query_or_full_user_agent() {
        let dir = std::env::temp_dir().join(format!(
            "edgerun-analytics-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let analytics = Analytics::new(AnalyticsConfig::new(&dir));
        let mut request = Request::builder()
            .method(Method::GET)
            .uri("https://dash.edgerun.tech/surface/blog/posts/email.html?token=secret")
            .header("User-Agent", "Mozilla/5.0 Firefox/123.0")
            .build()
            .unwrap();
        request
            .headers_mut()
            .insert("Host", "dash.edgerun.tech")
            .unwrap();
        request
            .headers_mut()
            .insert("Referer", "https://example.test/private/path")
            .unwrap();
        let response = Response::text(StatusCode::OK, "ok");

        analytics.record(&request, &response).unwrap();

        let entries = fs::read_dir(&dir)
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(entries.len(), 1);
        let text = fs::read_to_string(entries[0].path()).unwrap();
        assert!(text.contains("/surface/blog/posts/email.html"));
        assert!(!text.contains("token=secret"));
        assert!(text.contains("https://example.test"));
        assert!(text.contains("\tfirefox\n"));
        let _ = fs::remove_dir_all(&dir);
    }
}
