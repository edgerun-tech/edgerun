use std::net::SocketAddr;
use std::time::Duration;

use anyhow::{Context, Result};
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use clap::Parser;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug, Clone)]
#[command(name = "edgerun-rbi-gateway")]
#[command(about = "RBI gateway control-plane service")]
struct Args {
    #[arg(long, default_value = "127.0.0.1:7443")]
    bind: SocketAddr,
    #[arg(long, default_value = "http://127.0.0.1:9222")]
    cdp: String,
}

#[derive(Clone)]
struct AppState {
    cdp_base: String,
    http: Client,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    cdp_reachable: bool,
}

#[derive(Serialize)]
struct CdpVersionResponse {
    browser: Option<String>,
    protocol_version: Option<String>,
    web_socket_debugger_url: Option<String>,
}

#[derive(Deserialize)]
struct CdpVersionPayload {
    #[serde(rename = "Browser")]
    browser: Option<String>,
    #[serde(rename = "Protocol-Version")]
    protocol_version: Option<String>,
    #[serde(rename = "webSocketDebuggerUrl")]
    web_socket_debugger_url: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    let args = Args::parse();

    let http = Client::builder()
        .timeout(Duration::from_secs(4))
        .build()
        .context("failed to construct HTTP client")?;

    let state = AppState {
        cdp_base: args.cdp.trim_end_matches('/').to_string(),
        http,
    };

    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/cdp/version", get(cdp_version))
        .with_state(state);

    let listener = TcpListener::bind(args.bind)
        .await
        .with_context(|| format!("failed to bind {}", args.bind))?;
    info!(bind = %args.bind, cdp = %args.cdp, "starting edgerun-rbi-gateway");

    axum::serve(listener, app)
        .await
        .context("gateway server terminated unexpectedly")?;
    Ok(())
}

async fn healthz(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    let cdp_ok = fetch_cdp_version(&state).await.is_some();
    let status = if cdp_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (
        status,
        Json(HealthResponse {
            status: if cdp_ok { "ok" } else { "degraded" },
            cdp_reachable: cdp_ok,
        }),
    )
}

async fn cdp_version(
    State(state): State<AppState>,
) -> Result<Json<CdpVersionResponse>, (StatusCode, String)> {
    let value = fetch_cdp_version(&state).await.ok_or_else(|| {
        (
            StatusCode::BAD_GATEWAY,
            "failed to query CDP version endpoint".to_string(),
        )
    })?;

    let response = CdpVersionResponse {
        browser: value.browser,
        protocol_version: value.protocol_version,
        web_socket_debugger_url: value.web_socket_debugger_url,
    };
    Ok(Json(response))
}

async fn fetch_cdp_version(state: &AppState) -> Option<CdpVersionPayload> {
    let url = format!("{}/json/version", state.cdp_base);
    let response = state.http.get(url).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }
    let payload = response.bytes().await.ok()?;
    sonic_rs::from_slice::<CdpVersionPayload>(&payload).ok()
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
