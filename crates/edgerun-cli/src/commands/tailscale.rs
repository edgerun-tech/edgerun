// SPDX-License-Identifier: Apache-2.0
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use axum::http::{HeaderValue, Method};
use axum::{extract::State, response::IntoResponse, routing::get, Json, Router};
use serde::Serialize;
use serde_json::{json, Value};
use tokio::process::Command;
use tower_http::cors::{Any, CorsLayer};

use crate::integration_helpers::kill_child;
use crate::{command_exists, ensure, TailscaleCommand};

#[derive(Clone, Debug)]
struct TailscaleBridgeState {
    term_port: u16,
    include_offline: bool,
}

#[derive(Debug, Serialize)]
struct TailscaleBridgeDevice {
    name: String,
    base_url: String,
    online: bool,
    source: &'static str,
}

pub(crate) async fn run_tailscale_command(root: &Path, command: TailscaleCommand) -> Result<()> {
    match command {
        TailscaleCommand::Bridge {
            listen,
            term_port,
            include_offline,
        } => run_tailscale_bridge(listen, term_port, include_offline).await,
        TailscaleCommand::Dev {
            bridge_listen,
            port,
            web_root,
            hardware_mode,
            include_offline,
            tailscale_serve,
        } => {
            run_tailscale_dev(
                root,
                bridge_listen,
                port,
                web_root,
                &hardware_mode,
                include_offline,
                tailscale_serve,
            )
            .await
        }
    }
}

async fn run_tailscale_bridge(
    listen: SocketAddr,
    term_port: u16,
    include_offline: bool,
) -> Result<()> {
    if !command_exists("tailscale") {
        return Err(anyhow!(
            "tailscale CLI not found in PATH; install Tailscale first"
        ));
    }

    let state = TailscaleBridgeState {
        term_port,
        include_offline,
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::OPTIONS])
        .allow_headers(Any)
        .max_age(Duration::from_secs(600));

    let app = Router::new()
        .route("/v1/tailscale/devices", get(tailscale_devices_handler))
        .with_state(state)
        .layer(cors);

    println!(
        "tailscale bridge listening on http://{listen} (term_port={term_port}, include_offline={include_offline})"
    );
    println!("endpoint: GET /v1/tailscale/devices");

    let listener = tokio::net::TcpListener::bind(listen)
        .await
        .with_context(|| format!("failed to bind tailscale bridge on {listen}"))?;
    axum::serve(listener, app)
        .await
        .context("tailscale bridge server failed")?;
    Ok(())
}

async fn run_tailscale_dev(
    root: &Path,
    bridge_listen: SocketAddr,
    port: u16,
    web_root: Option<PathBuf>,
    hardware_mode: &str,
    include_offline: bool,
    tailscale_serve: bool,
) -> Result<()> {
    if hardware_mode != "allow-software" && hardware_mode != "tpm-required" {
        return Err(anyhow!(
            "invalid --hardware-mode '{}'; expected 'allow-software' or 'tpm-required'",
            hardware_mode
        ));
    }
    let web_root = web_root.unwrap_or_else(|| root.join("out/frontend/site"));
    let term_client_root = root.join("crates/edgerun-term-web");
    ensure(
        web_root.exists(),
        &format!(
            "web root not found: {} (build frontend first: 'cd frontend && bun run build')",
            web_root.display()
        ),
    )?;
    ensure(
        term_client_root.exists(),
        &format!(
            "terminal client root not found: {}",
            term_client_root.display()
        ),
    )?;

    println!("starting local terminal dev stack");
    println!("- local gateway: http://127.0.0.1:{port} (EDGERUN_HARDWARE_MODE={hardware_mode})");
    println!("- web root: {}", web_root.display());
    println!("- tailscale bridge: http://{bridge_listen}");

    let mut term_server = Command::new("cargo");
    term_server
        .arg("run")
        .arg("-p")
        .arg("edgerun-term-server")
        .current_dir(root)
        .env("EDGERUN_HARDWARE_MODE", hardware_mode)
        .env("EDGERUN_TERM_SERVER_ADDR", format!("0.0.0.0:{port}"))
        .env("EDGERUN_TERM_WEB_ROOT", web_root.display().to_string())
        .env(
            "EDGERUN_TERM_CLIENT_ROOT",
            term_client_root.display().to_string(),
        )
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    let mut term_server = term_server
        .spawn()
        .context("failed to start edgerun-term-server")?;

    let exe = std::env::current_exe().context("failed to resolve current executable path")?;
    let mut bridge = Command::new(exe);
    bridge
        .arg("--root")
        .arg(root.display().to_string())
        .arg("tailscale")
        .arg("bridge")
        .arg("--listen")
        .arg(bridge_listen.to_string())
        .arg("--term-port")
        .arg(port.to_string())
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    if include_offline {
        bridge.arg("--include-offline");
    }
    let mut bridge = bridge.spawn().context("failed to start tailscale bridge")?;

    if tailscale_serve {
        let target = format!("http://127.0.0.1:{port}");
        println!("- enabling tailscale serve for {target}");
        let status = Command::new("tailscale")
            .arg("serve")
            .arg("--bg")
            .arg(&target)
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await
            .context("failed to execute tailscale serve")?;
        if !status.success() {
            eprintln!(
                "warning: tailscale serve exited with status {:?}",
                status.code()
            );
        }
    }

    println!("dev stack running. Press Ctrl+C to stop.");

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            println!("received Ctrl+C, shutting down...");
        }
        status = term_server.wait() => {
            let code = status.ok().and_then(|s| s.code()).unwrap_or_default();
            kill_child(&mut bridge).await;
            return Err(anyhow!("edgerun-term-server exited unexpectedly (code={code})"));
        }
        status = bridge.wait() => {
            let code = status.ok().and_then(|s| s.code()).unwrap_or_default();
            kill_child(&mut term_server).await;
            return Err(anyhow!("tailscale bridge exited unexpectedly (code={code})"));
        }
    }

    kill_child(&mut term_server).await;
    kill_child(&mut bridge).await;
    Ok(())
}

async fn tailscale_devices_handler(State(state): State<TailscaleBridgeState>) -> impl IntoResponse {
    match discover_tailscale_devices(state.term_port, state.include_offline).await {
        Ok(devices) => (
            [(
                axum::http::header::CACHE_CONTROL,
                HeaderValue::from_static("no-store"),
            )],
            Json(json!({
                "ok": true,
                "count": devices.len(),
                "devices": devices
            })),
        )
            .into_response(),
        Err(err) => (
            axum::http::StatusCode::BAD_GATEWAY,
            Json(json!({
                "ok": false,
                "error": err.to_string()
            })),
        )
            .into_response(),
    }
}

async fn discover_tailscale_devices(
    term_port: u16,
    include_offline: bool,
) -> Result<Vec<TailscaleBridgeDevice>> {
    let output = Command::new("tailscale")
        .arg("status")
        .arg("--json")
        .stdin(Stdio::null())
        .output()
        .await
        .context("failed to execute 'tailscale status --json'")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let msg = stderr.trim();
        if msg.is_empty() {
            return Err(anyhow!("tailscale status returned non-zero exit code"));
        }
        return Err(anyhow!(msg.to_string()));
    }

    let doc: Value =
        serde_json::from_slice(&output.stdout).context("failed to parse tailscale status JSON")?;
    let mut devices = Vec::new();
    append_tailscale_entry(
        &mut devices,
        doc.get("Self"),
        true,
        term_port,
        include_offline,
    );
    if let Some(peers) = doc.get("Peer").and_then(|v| v.as_object()) {
        for peer in peers.values() {
            append_tailscale_entry(&mut devices, Some(peer), false, term_port, include_offline);
        }
    }
    devices.sort_by(|a, b| a.name.cmp(&b.name));
    devices.dedup_by(|a, b| a.base_url == b.base_url);
    Ok(devices)
}

fn append_tailscale_entry(
    out: &mut Vec<TailscaleBridgeDevice>,
    entry: Option<&Value>,
    is_self: bool,
    term_port: u16,
    include_offline: bool,
) {
    let Some(entry) = entry else {
        return;
    };
    let online = entry
        .get("Online")
        .and_then(|v| v.as_bool())
        .unwrap_or(is_self);
    if !include_offline && !online {
        return;
    }

    let dns_name = entry
        .get("DNSName")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim_end_matches('.')
        .to_string();
    let host_name = entry
        .get("HostName")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let tailnet_name = if !host_name.is_empty() {
        host_name
    } else if !dns_name.is_empty() {
        dns_name.clone()
    } else {
        "Tailscale Device".to_string()
    };
    let base_host = if !dns_name.is_empty() {
        dns_name
    } else {
        let maybe_ip = entry
            .get("TailscaleIPs")
            .and_then(|v| v.as_array())
            .and_then(|ips| ips.first())
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if maybe_ip.is_empty() {
            return;
        }
        maybe_ip.to_string()
    };
    let source = if is_self {
        "tailscale-self"
    } else {
        "tailscale-peer"
    };
    out.push(TailscaleBridgeDevice {
        name: tailnet_name,
        base_url: format!("http://{base_host}:{term_port}"),
        online,
        source,
    });
}
