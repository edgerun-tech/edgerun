use std::collections::HashMap;
use std::env;
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use axum::{
    Json, Router,
    extract::State,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::{get, post},
};
use edgerun_hwvault_primitives::hardware::{
    DeviceSigner, HardwareSecurityMode, load_or_create_device_signer, random_token_b64url,
};
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, mpsc};
use tokio::time::{Duration, sleep};
use tower_http::services::ServeDir;

#[derive(Clone)]
struct AppState {
    device: DeviceSigner,
    challenges: std::sync::Arc<Mutex<HashMap<String, u64>>>,
}

#[derive(Debug, Serialize)]
struct DeviceIdentityResponse {
    backend: String,
    device_pubkey_b64url: String,
}

#[derive(Debug, Serialize)]
struct DeviceChallengeResponse {
    nonce_b64url: String,
    expires_at_unix_s: u64,
}

#[derive(Debug, Deserialize)]
struct DeviceHandshakeRequest {
    owner_pubkey: String,
    nonce_b64url: String,
}

#[derive(Debug, Serialize)]
struct RouteRegisterRequest {
    device_id: String,
    owner_pubkey: String,
    reachable_urls: Vec<String>,
    capabilities: Vec<String>,
    relay_session_id: Option<String>,
    ttl_secs: u64,
    challenge_nonce: String,
    signed_at_unix_s: u64,
    signature: String,
}

#[derive(Debug, Serialize)]
struct RouteChallengeRequest {
    device_id: String,
}

#[derive(Debug, Deserialize)]
struct RouteChallengeResponse {
    nonce: String,
    expires_at_unix_s: u64,
}

#[derive(Debug, Deserialize)]
struct RouteRegisterResponse {
    ok: bool,
    heartbeat_token: String,
}

#[derive(Debug, Serialize)]
struct RouteHeartbeatRequest {
    device_id: String,
    token: String,
    ttl_secs: u64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mode = match std::env::var("EDGERUN_HARDWARE_MODE")
        .unwrap_or_else(|_| "tpm-required".to_string())
        .trim()
    {
        "allow-software" => HardwareSecurityMode::AllowSoftwareFallback,
        _ => HardwareSecurityMode::TpmRequired,
    };
    let device = load_or_create_device_signer(mode).with_context(|| {
        "failed to initialize hardware-backed device signer (set EDGERUN_HARDWARE_MODE=allow-software for explicit non-TPM fallback)"
    })?;
    let boot_sig = device.sign_b64url(b"edgerun-term-server-boot");
    println!(
        "term-server device identity backend={:?} pubkey={} boot_sig={}",
        device.backend, device.public_key_b64url, boot_sig
    );

    let state = AppState {
        device,
        challenges: std::sync::Arc::new(Mutex::new(HashMap::new())),
    };

    let web_root = env::var("EDGERUN_TERM_WEB_ROOT")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("term-web"));
    let term_client_root = env::var("EDGERUN_TERM_CLIENT_ROOT")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("crates/edgerun-term-web"));
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/v1/device/identity", get(device_identity))
        .route("/v1/device/challenge", post(device_challenge))
        .route("/v1/device/handshake", post(device_handshake))
        .nest_service("/term", ServeDir::new(term_client_root.clone()))
        .with_state(state.clone())
        .fallback_service(ServeDir::new(web_root.clone()));

    let bind_addr = env::var("EDGERUN_TERM_SERVER_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    let addr: SocketAddr = match bind_addr.parse() {
        Ok(addr) => addr,
        Err(e) => {
            eprintln!("Failed to parse server address '{bind_addr}': {e}");
            std::process::exit(1);
        }
    };
    println!("term-server serving site root {}", web_root.display());
    println!("term-server serving terminal client root {} at /term/", term_client_root.display());
    println!("term-server listening on http://{addr}");
    maybe_start_route_announcer(&state.device, addr);

    axum::serve(tokio::net::TcpListener::bind(addr).await?, app)
        .await
        .context("server failed")?;

    Ok(())
}

async fn device_identity(State(state): State<AppState>) -> Json<DeviceIdentityResponse> {
    Json(DeviceIdentityResponse {
        backend: format!("{:?}", state.device.backend).to_lowercase(),
        device_pubkey_b64url: state.device.public_key_b64url.clone(),
    })
}

async fn device_challenge(State(state): State<AppState>) -> Json<DeviceChallengeResponse> {
    let now = now_unix_s();
    let expires_at = now + 120;
    let nonce = random_token_b64url(24);

    let mut guard = state.challenges.lock().await;
    guard.retain(|_, exp| *exp > now);
    guard.insert(nonce.clone(), expires_at);

    Json(DeviceChallengeResponse {
        nonce_b64url: nonce,
        expires_at_unix_s: expires_at,
    })
}

async fn device_handshake(
    State(state): State<AppState>,
    Json(req): Json<DeviceHandshakeRequest>,
) -> impl IntoResponse {
    let now = now_unix_s();
    let mut challenges = state.challenges.lock().await;
    challenges.retain(|_, exp| *exp > now);

    let Some(exp) = challenges.remove(req.nonce_b64url.trim()) else {
        return Json(serde_json::json!({
            "ok": false,
            "error": "unknown or expired nonce"
        }));
    };
    if exp <= now {
        return Json(serde_json::json!({
            "ok": false,
            "error": "nonce expired"
        }));
    }
    drop(challenges);

    match state
        .device
        .build_handshake(req.owner_pubkey.trim(), req.nonce_b64url.trim(), now)
    {
        Ok(handshake) => Json(serde_json::json!({
            "ok": true,
            "handshake": handshake
        })),
        Err(err) => Json(serde_json::json!({
            "ok": false,
            "error": err.to_string()
        })),
    }
}

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    let pty_system = NativePtySystem::default();
    let pair = match pty_system.openpty(PtySize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    }) {
        Ok(pair) => pair,
        Err(err) => {
            let _ = socket
                .send(Message::Text(format!("pty error: {err}").into()))
                .await;
            return;
        }
    };

    let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
    let mut cmd = CommandBuilder::new(shell);
    cmd.env("TERM", "xterm-256color");

    if let Err(err) = pair.slave.spawn_command(cmd) {
        let _ = socket
            .send(Message::Text(format!("spawn error: {err}").into()))
            .await;
        return;
    }

    let master = pair.master;
    let mut reader = match master.try_clone_reader() {
        Ok(reader) => reader,
        Err(err) => {
            let _ = socket
                .send(Message::Text(format!("pty reader error: {err}").into()))
                .await;
            return;
        }
    };
    let mut writer = match master.take_writer() {
        Ok(w) => w,
        Err(err) => {
            let _ = socket
                .send(Message::Text(format!("pty writer error: {err}").into()))
                .await;
            return;
        }
    };

    let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();
    thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let _ = tx.send(buf[..n].to_vec());
                }
                Err(_) => break,
            }
        }
    });

    loop {
        tokio::select! {
            Some(bytes) = rx.recv() => {
                if socket.send(Message::Binary(bytes.into())).await.is_err() {
                    break;
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Binary(data))) => {
                        let _ = writer.write_all(&data);
                        let _ = writer.flush();
                    }
                    Some(Ok(Message::Text(text))) => {
                        if let Some((cols, rows)) = parse_resize(&text) {
                            let _ = master.resize(PtySize {
                                cols,
                                rows,
                                pixel_width: 0,
                                pixel_height: 0,
                            });
                        } else {
                            let _ = writer.write_all(text.as_bytes());
                            let _ = writer.flush();
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        break;
                    }
                    _ => {}
                }
            }
        }
    }
}

fn parse_resize(text: &str) -> Option<(u16, u16)> {
    let rest = text.strip_prefix("resize:")?;
    let mut parts = rest.split('x');
    let cols = parts.next()?.parse::<u16>().ok()?;
    let rows = parts.next()?.parse::<u16>().ok()?;
    Some((cols, rows))
}

fn now_unix_s() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn maybe_start_route_announcer(device: &DeviceSigner, addr: SocketAddr) {
    let Some(control_base) = env::var("EDGERUN_ROUTE_CONTROL_BASE")
        .ok()
        .map(|v| v.trim().trim_end_matches('/').to_string())
        .filter(|v| !v.is_empty())
    else {
        return;
    };

    let device_id = env::var("EDGERUN_ROUTE_DEVICE_ID")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| device.public_key_b64url.clone());
    let owner_pubkey = env::var("EDGERUN_ROUTE_OWNER_PUBKEY")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| device.public_key_b64url.clone());
    let public_base_url = env::var("EDGERUN_TERM_PUBLIC_BASE_URL")
        .ok()
        .map(|v| v.trim().trim_end_matches('/').to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| format!("http://{}", routable_addr(addr)));

    println!(
        "route announcer enabled: control={} device_id={} public_base_url={}",
        control_base, device_id, public_base_url
    );
    let signer = device.clone();

    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let mut heartbeat_token: Option<String> = None;
        loop {
            if let Some(token) = heartbeat_token.as_ref() {
                let hb = RouteHeartbeatRequest {
                    device_id: device_id.clone(),
                    token: token.clone(),
                    ttl_secs: 90,
                };
                let hb_url = format!("{control_base}/v1/route/heartbeat");
                match client.post(hb_url).json(&hb).send().await {
                    Ok(resp) if resp.status().is_success() => {}
                    Ok(resp) => {
                        let status = resp.status();
                        let body = resp.text().await.unwrap_or_default();
                        eprintln!("route announcer heartbeat failed: status={} body={}", status, body);
                        heartbeat_token = None;
                    }
                    Err(err) => {
                        eprintln!("route announcer heartbeat error: {err}");
                        heartbeat_token = None;
                    }
                }
            }

            if heartbeat_token.is_none() {
                let challenge_url = format!("{control_base}/v1/route/challenge");
                let challenge_req = RouteChallengeRequest {
                    device_id: device_id.clone(),
                };
                let challenge = match client.post(challenge_url).json(&challenge_req).send().await {
                    Ok(resp) if resp.status().is_success() => match resp.json::<RouteChallengeResponse>().await {
                        Ok(value) => value,
                        Err(err) => {
                            eprintln!("route announcer challenge parse error: {err}");
                            sleep(Duration::from_secs(10)).await;
                            continue;
                        }
                    },
                    Ok(resp) => {
                        let status = resp.status();
                        let body = resp.text().await.unwrap_or_default();
                        eprintln!("route announcer challenge failed: status={} body={}", status, body);
                        sleep(Duration::from_secs(10)).await;
                        continue;
                    }
                    Err(err) => {
                        eprintln!("route announcer challenge error: {err}");
                        sleep(Duration::from_secs(10)).await;
                        continue;
                    }
                };
                if challenge.expires_at_unix_s <= now_unix_s() {
                    sleep(Duration::from_secs(2)).await;
                    continue;
                }

                let signed_at = now_unix_s();
                let reachable_urls = vec![public_base_url.clone()];
                let signing_message = route_register_signing_message(
                    &owner_pubkey,
                    &device_id,
                    &reachable_urls,
                    &challenge.nonce,
                    signed_at,
                );
                let signature = signer.sign_b64url(signing_message.as_bytes());

                let payload = RouteRegisterRequest {
                    device_id: device_id.clone(),
                    owner_pubkey: owner_pubkey.clone(),
                    reachable_urls,
                    capabilities: vec!["terminal-ws".to_string(), "webrtc-datachannel".to_string()],
                    relay_session_id: None,
                    ttl_secs: 90,
                    challenge_nonce: challenge.nonce,
                    signed_at_unix_s: signed_at,
                    signature,
                };
                let url = format!("{control_base}/v1/route/register");
                match client.post(url).json(&payload).send().await {
                    Ok(resp) if resp.status().is_success() => match resp.json::<RouteRegisterResponse>().await {
                        Ok(value) => {
                            if value.ok {
                                heartbeat_token = Some(value.heartbeat_token);
                            } else {
                                heartbeat_token = None;
                            }
                        }
                        Err(err) => {
                            eprintln!("route announcer register parse error: {err}");
                        }
                    },
                    Ok(resp) => {
                        let status = resp.status();
                        let body = resp.text().await.unwrap_or_default();
                        eprintln!("route announcer register failed: status={} body={}", status, body);
                    }
                    Err(err) => {
                        eprintln!("route announcer register error: {err}");
                    }
                }
            }
            sleep(Duration::from_secs(30)).await;
        }
    });
}

fn routable_addr(addr: SocketAddr) -> String {
    if addr.ip().is_unspecified() {
        format!("127.0.0.1:{}", addr.port())
    } else {
        addr.to_string()
    }
}

fn route_register_signing_message(
    owner_pubkey: &str,
    device_id: &str,
    reachable_urls: &[String],
    challenge_nonce: &str,
    signed_at_unix_s: u64,
) -> String {
    let urls = reachable_urls.join(",");
    format!(
        "edgerun:route_register:v1|{}|{}|{}|{}|{}",
        owner_pubkey, device_id, urls, challenge_nonce, signed_at_unix_s
    )
}
