use std::collections::HashMap;
use std::env;
use std::io::{Read, Write};
use std::net::SocketAddr;
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

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/v1/device/identity", get(device_identity))
        .route("/v1/device/challenge", post(device_challenge))
        .route("/v1/device/handshake", post(device_handshake))
        .with_state(state)
        .fallback_service(ServeDir::new("term-web"));

    let addr: SocketAddr = match "0.0.0.0:8080".parse() {
        Ok(addr) => addr,
        Err(e) => {
            eprintln!("Failed to parse server address: {e}");
            std::process::exit(1);
        }
    };
    println!("term-server listening on http://{addr}");

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
