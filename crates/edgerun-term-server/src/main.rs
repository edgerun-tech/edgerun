// SPDX-License-Identifier: Apache-2.0
use std::collections::HashMap;
use std::env;
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
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
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::{Signer, SigningKey};
use edgerun_hwvault_primitives::hardware::{
    DeviceSigner, HardwareSecurityMode, load_or_create_device_signer, random_token_b64url,
};
use edgerun_transport_core::route_register_signing_message;
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, mpsc};
use tokio::time::{Duration, sleep};
use tower_http::services::ServeDir;
use tracing::{info, warn};

#[derive(Clone)]
struct AppState {
    device: DeviceSigner,
    challenges: std::sync::Arc<Mutex<HashMap<String, u64>>>,
    mux_token: Option<String>,
}

#[derive(Debug, Serialize)]
struct DeviceIdentityResponse {
    backend: String,
    device_pubkey_b64url: String,
}

type RouteSigner = Arc<dyn Fn(&[u8]) -> String + Send + Sync>;

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
struct DeviceHandshakeResponse {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    handshake: Option<edgerun_hwvault_primitives::hardware::DeviceHandshake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
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

#[derive(Debug)]
enum ShellRequest {
    Auth {
        token: String,
    },
    Spawn {
        id: Option<u32>,
        cmd: Option<String>,
        args: Option<Vec<String>>,
        cwd: Option<String>,
        env: Option<HashMap<String, String>>,
        cols: Option<u16>,
        rows: Option<u16>,
    },
    Resize {
        id: u32,
        cols: u16,
        rows: u16,
    },
    Close {
        id: u32,
    },
}

#[derive(Debug)]
enum ShellResponse {
    AuthOk,
    AuthError {
        error: String,
    },
    Spawned {
        id: u32,
        pid: Option<u32>,
    },
    Exit {
        id: u32,
        code: u32,
        signal: Option<String>,
    },
    Error {
        id: Option<u32>,
        error: String,
    },
}

struct PtySession {
    master: Box<dyn portable_pty::MasterPty + Send>,
    writer: std::sync::Mutex<Box<dyn std::io::Write + Send>>,
    killer: Box<dyn portable_pty::ChildKiller + Send + Sync>,
}

fn kill_all_sessions(sessions: &mut HashMap<u32, PtySession>) {
    for (_, session) in sessions.drain() {
        let _ = session.killer.clone_killer().kill();
    }
}

fn parse_exit_session_id(msg: &Message) -> Option<u32> {
    const PTY_FRAME_CONTROL_RESP: u8 = 0x7f;
    let Message::Binary(bytes) = msg else {
        return None;
    };
    if bytes.first().copied() != Some(PTY_FRAME_CONTROL_RESP) {
        return None;
    }
    let decoded = decode_shell_response(&bytes[1..]).ok()?;
    match decoded {
        ShellResponse::Exit { id, .. } => Some(id),
        _ => None,
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    edgerun_observability::init_service("edgerun-term-server")?;
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
    info!(
        backend = ?device.backend,
        pubkey = %device.public_key_b64url,
        boot_sig = %boot_sig,
        "term-server device identity"
    );

    let state = AppState {
        device,
        challenges: std::sync::Arc::new(Mutex::new(HashMap::new())),
        mux_token: env::var("EDGERUN_TERM_MUX_TOKEN")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty()),
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
        .route("/ws-mux", get(ws_mux_handler))
        .route("/v1/device/identity", get(device_identity))
        .route("/v1/device/challenge", post(device_challenge))
        .route("/v1/device/handshake", post(device_handshake))
        .nest_service("/term", ServeDir::new(term_client_root.clone()))
        .with_state(state.clone())
        .fallback_service(ServeDir::new(web_root.clone()));

    let bind_addr =
        env::var("EDGERUN_TERM_SERVER_ADDR").unwrap_or_else(|_| "0.0.0.0:5577".to_string());
    let addr: SocketAddr = bind_addr
        .parse()
        .with_context(|| format!("failed to parse EDGERUN_TERM_SERVER_ADDR '{bind_addr}'"))?;
    info!(path = %web_root.display(), "term-server serving site root");
    info!(
        path = %term_client_root.display(),
        "term-server serving terminal client root at /term/"
    );
    info!(%addr, "term-server listening");
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
        return Json(DeviceHandshakeResponse {
            ok: false,
            handshake: None,
            error: Some("unknown or expired nonce".to_string()),
        });
    };
    if exp <= now {
        return Json(DeviceHandshakeResponse {
            ok: false,
            handshake: None,
            error: Some("nonce expired".to_string()),
        });
    }
    drop(challenges);

    match state
        .device
        .build_handshake(req.owner_pubkey.trim(), req.nonce_b64url.trim(), now)
    {
        Ok(handshake) => Json(DeviceHandshakeResponse {
            ok: true,
            handshake: Some(handshake),
            error: None,
        }),
        Err(err) => Json(DeviceHandshakeResponse {
            ok: false,
            handshake: None,
            error: Some(err.to_string()),
        }),
    }
}

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn ws_mux_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_mux_socket(socket, state.mux_token.clone()))
}

async fn handle_mux_socket(mut socket: WebSocket, token: Option<String>) {
    const PTY_FRAME_STDIN: u8 = 1;
    const PTY_FRAME_CONTROL_REQ: u8 = 0x7e;
    let mut authed = false;
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Message>();
    let mut sessions: HashMap<u32, PtySession> = HashMap::new();
    let mut next_session_id = 1u32;

    loop {
        tokio::select! {
            Some(msg) = out_rx.recv() => {
                if let Some(id) = parse_exit_session_id(&msg) {
                    sessions.remove(&id);
                }
                if socket.send(msg).await.is_err() {
                    break;
                }
            }
            incoming = socket.recv() => {
                let Some(Ok(msg)) = incoming else { break };
                if let Message::Binary(data) = msg {
                    if data.len() >= 5 && data[0] == PTY_FRAME_STDIN {
                        let id = u32::from_be_bytes([data[1], data[2], data[3], data[4]]);
                        if let Some(session) = sessions.get(&id)
                            && let Ok(mut writer) = session.writer.lock()
                        {
                            let _ = writer.write_all(&data[5..]);
                            let _ = writer.flush();
                        }
                        continue;
                    }
                    if data.first().copied() != Some(PTY_FRAME_CONTROL_REQ) {
                        continue;
                    }
                    let req: ShellRequest = match decode_shell_request(&data[1..]) {
                        Ok(value) => value,
                        Err(err) => {
                            send_mux_response(&out_tx, &ShellResponse::Error {
                                id: None,
                                error: format!("invalid request: {err}"),
                            });
                            continue;
                        }
                    };
                    match req {
                        ShellRequest::Auth { token: provided } => {
                            if let Some(expected) = token.as_deref() {
                                if expected == provided {
                                    authed = true;
                                    send_mux_response(&out_tx, &ShellResponse::AuthOk);
                                } else {
                                    send_mux_response(&out_tx, &ShellResponse::AuthError {
                                        error: "invalid token".to_string(),
                                    });
                                    break;
                                }
                            } else {
                                send_mux_response(&out_tx, &ShellResponse::AuthError {
                                    error: "mux token is not configured".to_string(),
                                });
                                break;
                            }
                        }
                        ShellRequest::Spawn { id, cmd, args, cwd, env, cols, rows } => {
                            if !authed {
                                send_mux_response(&out_tx, &ShellResponse::AuthError {
                                    error: "missing auth".to_string(),
                                });
                                continue;
                            }
                            let session_id = if let Some(id) = id {
                                if sessions.contains_key(&id) {
                                    send_mux_response(&out_tx, &ShellResponse::Error {
                                        id: Some(id),
                                        error: "session id already exists".to_string(),
                                    });
                                    continue;
                                }
                                id
                            } else {
                                let id = next_session_id;
                                next_session_id = next_session_id.wrapping_add(1);
                                id
                            };
                            match spawn_pty_session(
                                session_id,
                                cmd,
                                args,
                                cwd,
                                env,
                                cols.unwrap_or(80),
                                rows.unwrap_or(24),
                                out_tx.clone(),
                            ) {
                                Ok((session, pid)) => {
                                    sessions.insert(session_id, session);
                                    send_mux_response(
                                        &out_tx,
                                        &ShellResponse::Spawned {
                                            id: session_id,
                                            pid,
                                        },
                                    );
                                }
                                Err(err) => {
                                    send_mux_response(&out_tx, &ShellResponse::Error {
                                        id: Some(session_id),
                                        error: err.to_string(),
                                    });
                                }
                            }
                        }
                        ShellRequest::Resize { id, cols, rows } => {
                            if let Some(session) = sessions.get(&id) {
                                let _ = session.master.resize(PtySize {
                                    cols,
                                    rows,
                                    pixel_width: 0,
                                    pixel_height: 0,
                                });
                            }
                        }
                        ShellRequest::Close { id } => {
                            if let Some(session) = sessions.remove(&id) {
                                let _ = session.killer.clone_killer().kill();
                            }
                        }
                    }
                }
            }
        }
    }
    kill_all_sessions(&mut sessions);
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

    let mut child = match pair.slave.spawn_command(cmd) {
        Ok(child) => child,
        Err(err) => {
            let _ = socket
                .send(Message::Text(format!("spawn error: {err}").into()))
                .await;
            return;
        }
    };
    let killer = child.clone_killer();

    thread::spawn(move || {
        let _ = child.wait();
    });

    let mut closed = false;
    if closed {
        let _ = killer.clone_killer().kill();
    }

    if false {
        let _ = socket
            .send(Message::Text("spawn error".to_string().into()))
            .await;
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
                        closed = true;
                        break;
                    }
                    _ => {}
                }
            }
        }
    }
    if closed {
        let _ = killer.clone_killer().kill();
    }
}

fn parse_resize(text: &str) -> Option<(u16, u16)> {
    let rest = text.strip_prefix("resize:")?;
    let mut parts = rest.split('x');
    let cols = parts.next()?.parse::<u16>().ok()?;
    let rows = parts.next()?.parse::<u16>().ok()?;
    Some((cols, rows))
}

#[allow(clippy::too_many_arguments)]
fn spawn_pty_session(
    id: u32,
    cmd: Option<String>,
    args: Option<Vec<String>>,
    cwd: Option<String>,
    env_vars: Option<HashMap<String, String>>,
    cols: u16,
    rows: u16,
    out_tx: mpsc::UnboundedSender<Message>,
) -> anyhow::Result<(PtySession, Option<u32>)> {
    const PTY_FRAME_STDOUT: u8 = 2;
    let pty_system = NativePtySystem::default();
    let pair = pty_system.openpty(PtySize {
        cols,
        rows,
        pixel_width: 0,
        pixel_height: 0,
    })?;
    let mut builder = if let Some(cmd) = cmd {
        CommandBuilder::new(cmd)
    } else {
        let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
        CommandBuilder::new(shell)
    };
    if let Some(args) = args {
        builder.args(args);
    }
    if let Some(cwd) = cwd {
        builder.cwd(cwd);
    }
    if let Some(env_vars) = env_vars {
        for (key, value) in env_vars {
            builder.env(key, value);
        }
    }
    builder.env("TERM", "xterm-256color");

    let mut child = pair.slave.spawn_command(builder)?;
    let pid = child.process_id();
    let killer = child.clone_killer();

    let mut reader = pair.master.try_clone_reader()?;
    let stdout_tx = out_tx.clone();
    thread::spawn(move || {
        let mut buf = [0u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let frame = encode_pty_frame(PTY_FRAME_STDOUT, id, &buf[..n]);
                    let _ = stdout_tx.send(Message::Binary(frame.into()));
                }
                Err(_) => break,
            }
        }
    });

    let exit_tx = out_tx.clone();
    thread::spawn(move || {
        if let Ok(status) = child.wait() {
            let resp = ShellResponse::Exit {
                id,
                code: status.exit_code(),
                signal: None,
            };
            send_mux_response(&exit_tx, &resp);
        }
    });

    let writer = pair.master.take_writer()?;
    Ok((
        PtySession {
            master: pair.master,
            writer: std::sync::Mutex::new(writer),
            killer,
        },
        pid,
    ))
}

fn encode_pty_frame(kind: u8, id: u32, payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(5 + payload.len());
    frame.push(kind);
    frame.extend_from_slice(&id.to_be_bytes());
    frame.extend_from_slice(payload);
    frame
}

fn send_mux_response(out_tx: &mpsc::UnboundedSender<Message>, response: &ShellResponse) {
    const PTY_FRAME_CONTROL_RESP: u8 = 0x7f;
    if let Ok(payload) = encode_shell_response(response) {
        let mut frame = Vec::with_capacity(1 + payload.len());
        frame.push(PTY_FRAME_CONTROL_RESP);
        frame.extend_from_slice(&payload);
        let _ = out_tx.send(Message::Binary(frame.into()));
    }
}

fn encode_shell_response(response: &ShellResponse) -> anyhow::Result<Vec<u8>> {
    let mut out = Vec::new();
    match response {
        ShellResponse::AuthOk => out.push(0),
        ShellResponse::AuthError { error } => {
            out.push(1);
            put_str(&mut out, error)?;
        }
        ShellResponse::Spawned { id, pid } => {
            out.push(2);
            out.extend_from_slice(&id.to_be_bytes());
            match pid {
                Some(pid) => {
                    out.push(1);
                    out.extend_from_slice(&pid.to_be_bytes());
                }
                None => out.push(0),
            }
        }
        ShellResponse::Exit { id, code, signal } => {
            out.push(3);
            out.extend_from_slice(&id.to_be_bytes());
            out.extend_from_slice(&code.to_be_bytes());
            match signal {
                Some(signal) => {
                    out.push(1);
                    put_str(&mut out, signal)?;
                }
                None => out.push(0),
            }
        }
        ShellResponse::Error { id, error } => {
            out.push(4);
            match id {
                Some(id) => {
                    out.push(1);
                    out.extend_from_slice(&id.to_be_bytes());
                }
                None => out.push(0),
            }
            put_str(&mut out, error)?;
        }
    }
    Ok(out)
}

fn decode_shell_response(bytes: &[u8]) -> anyhow::Result<ShellResponse> {
    let mut cur = 0usize;
    let tag = take_u8(bytes, &mut cur)?;
    let value = match tag {
        0 => ShellResponse::AuthOk,
        1 => ShellResponse::AuthError {
            error: take_str(bytes, &mut cur)?,
        },
        2 => {
            let id = take_u32(bytes, &mut cur)?;
            let has_pid = take_u8(bytes, &mut cur)? != 0;
            let pid = if has_pid {
                Some(take_u32(bytes, &mut cur)?)
            } else {
                None
            };
            ShellResponse::Spawned { id, pid }
        }
        3 => {
            let id = take_u32(bytes, &mut cur)?;
            let code = take_u32(bytes, &mut cur)?;
            let has_signal = take_u8(bytes, &mut cur)? != 0;
            let signal = if has_signal {
                Some(take_str(bytes, &mut cur)?)
            } else {
                None
            };
            ShellResponse::Exit { id, code, signal }
        }
        4 => {
            let has_id = take_u8(bytes, &mut cur)? != 0;
            let id = if has_id {
                Some(take_u32(bytes, &mut cur)?)
            } else {
                None
            };
            let error = take_str(bytes, &mut cur)?;
            ShellResponse::Error { id, error }
        }
        _ => anyhow::bail!("unknown response tag"),
    };
    Ok(value)
}

fn decode_shell_request(bytes: &[u8]) -> anyhow::Result<ShellRequest> {
    let mut cur = 0usize;
    let tag = take_u8(bytes, &mut cur)?;
    let value = match tag {
        0 => ShellRequest::Auth {
            token: take_str(bytes, &mut cur)?,
        },
        1 => {
            let flags = take_u8(bytes, &mut cur)?;
            let id = if flags & (1 << 0) != 0 {
                Some(take_u32(bytes, &mut cur)?)
            } else {
                None
            };
            let cmd = if flags & (1 << 1) != 0 {
                Some(take_str(bytes, &mut cur)?)
            } else {
                None
            };
            let args = if flags & (1 << 2) != 0 {
                let count = take_u16(bytes, &mut cur)? as usize;
                let mut values = Vec::with_capacity(count);
                for _ in 0..count {
                    values.push(take_str(bytes, &mut cur)?);
                }
                Some(values)
            } else {
                None
            };
            let cwd = if flags & (1 << 3) != 0 {
                Some(take_str(bytes, &mut cur)?)
            } else {
                None
            };
            let env = if flags & (1 << 4) != 0 {
                let count = take_u16(bytes, &mut cur)? as usize;
                let mut values = HashMap::with_capacity(count);
                for _ in 0..count {
                    let key = take_str(bytes, &mut cur)?;
                    let value = take_str(bytes, &mut cur)?;
                    values.insert(key, value);
                }
                Some(values)
            } else {
                None
            };
            let cols = if flags & (1 << 5) != 0 {
                Some(take_u16(bytes, &mut cur)?)
            } else {
                None
            };
            let rows = if flags & (1 << 6) != 0 {
                Some(take_u16(bytes, &mut cur)?)
            } else {
                None
            };
            ShellRequest::Spawn {
                id,
                cmd,
                args,
                cwd,
                env,
                cols,
                rows,
            }
        }
        2 => ShellRequest::Resize {
            id: take_u32(bytes, &mut cur)?,
            cols: take_u16(bytes, &mut cur)?,
            rows: take_u16(bytes, &mut cur)?,
        },
        3 => ShellRequest::Close {
            id: take_u32(bytes, &mut cur)?,
        },
        _ => anyhow::bail!("unknown request tag"),
    };
    Ok(value)
}

fn put_str(out: &mut Vec<u8>, value: &str) -> anyhow::Result<()> {
    let len = u16::try_from(value.len()).context("string too long")?;
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(value.as_bytes());
    Ok(())
}

fn take_u8(bytes: &[u8], cur: &mut usize) -> anyhow::Result<u8> {
    let Some(value) = bytes.get(*cur).copied() else {
        anyhow::bail!("unexpected eof")
    };
    *cur += 1;
    Ok(value)
}

fn take_u16(bytes: &[u8], cur: &mut usize) -> anyhow::Result<u16> {
    if bytes.len().saturating_sub(*cur) < 2 {
        anyhow::bail!("unexpected eof");
    }
    let value = u16::from_be_bytes([bytes[*cur], bytes[*cur + 1]]);
    *cur += 2;
    Ok(value)
}

fn take_u32(bytes: &[u8], cur: &mut usize) -> anyhow::Result<u32> {
    if bytes.len().saturating_sub(*cur) < 4 {
        anyhow::bail!("unexpected eof");
    }
    let value = u32::from_be_bytes([bytes[*cur], bytes[*cur + 1], bytes[*cur + 2], bytes[*cur + 3]]);
    *cur += 4;
    Ok(value)
}

fn take_str(bytes: &[u8], cur: &mut usize) -> anyhow::Result<String> {
    let len = take_u16(bytes, cur)? as usize;
    if bytes.len().saturating_sub(*cur) < len {
        anyhow::bail!("unexpected eof");
    }
    let slice = &bytes[*cur..*cur + len];
    *cur += len;
    String::from_utf8(slice.to_vec()).context("invalid utf8")
}

fn now_unix_s() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn maybe_start_route_announcer(device: &DeviceSigner, addr: SocketAddr) {
    let control_base = env::var("EDGERUN_ROUTE_CONTROL_BASE")
        .ok()
        .map(|v| v.trim().trim_end_matches('/').to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "https://api.edgerun.tech".to_string());
    let control_base_is_local =
        is_non_public_route_url(&control_base, "EDGERUN_ROUTE_CONTROL_BASE");

    let device_id = env::var("EDGERUN_ROUTE_DEVICE_ID")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| device.public_key_b64url.clone());
    let owner_signing_key = parse_owner_signing_key_from_env();
    let owner_pubkey_from_signing_key = owner_signing_key
        .as_ref()
        .map(|sk| bs58::encode(sk.verifying_key().to_bytes()).into_string());
    let configured_owner_pubkey = env::var("EDGERUN_ROUTE_OWNER_PUBKEY")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .or(owner_pubkey_from_signing_key)
        .unwrap_or_else(|| device.public_key_b64url.clone());
    let mut owner_pubkey = configured_owner_pubkey.clone();
    if owner_signing_key.is_none() && owner_pubkey != device.public_key_b64url {
        warn!(
            configured_owner_pubkey = %configured_owner_pubkey,
            owner_pubkey = %owner_pubkey,
            "owner pubkey configured without a matching EDGERUN_ROUTE_OWNER_SECRET_KEY_B58; route registration will fail signature validation. Falling back to device pubkey to keep terminal announcements alive."
        );
        owner_pubkey = device.public_key_b64url.clone();
    }
    let (public_base_url, public_base_was_set) = env::var("EDGERUN_TERM_PUBLIC_BASE_URL")
        .map(|v| {
            let value = v.trim().trim_end_matches('/').to_string();
            let was_set = !value.is_empty();
            (value, was_set)
        })
        .ok()
        .filter(|(_, is_set)| *is_set)
        .unwrap_or_else(|| (format!("http://{}", routable_addr(addr)), false));
    let mut reachable_urls = match parse_reachable_urls("EDGERUN_ROUTE_REACHABLE_URLS") {
        Ok(value) => value,
        Err(err) => {
            warn!(env = "EDGERUN_ROUTE_REACHABLE_URLS", error = %err, "ignoring configured reachable URLs");
            vec![]
        }
    };
    let public_base_is_non_public =
        is_non_public_route_url(&public_base_url, "EDGERUN_TERM_PUBLIC_BASE_URL");

    if public_base_is_non_public && !control_base_is_local {
        if public_base_was_set {
            warn!(
                control_base = %control_base,
                public_base_url = %public_base_url,
                "EDGERUN_TERM_PUBLIC_BASE_URL must be publicly routable when control base is remote"
            );
        } else {
            warn!(
                control_base = %control_base,
                public_base_url = %public_base_url,
                "EDGERUN_TERM_PUBLIC_BASE_URL was not provided; defaulted URL may not be externally reachable"
            );
        }
        warn!("route announcer disabled to avoid publishing unreachable terminal endpoint");
        return;
    }

    if !reachable_urls.iter().any(|value| value == &public_base_url) {
        reachable_urls.push(public_base_url.clone());
    }

    info!(
        control = %control_base,
        %device_id,
        %owner_pubkey,
        %public_base_url,
        "route announcer enabled"
    );
    let sign_route_payload: RouteSigner = if let Some(owner_signing_key) = owner_signing_key {
        let signer = Arc::new(owner_signing_key);
        Arc::new(move |message: &[u8]| {
            let sig = signer.sign(message);
            URL_SAFE_NO_PAD.encode(sig.to_bytes())
        })
    } else {
        let signer = device.clone();
        Arc::new(move |message: &[u8]| signer.sign_b64url(message))
    };

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
                        warn!(%status, %body, "route announcer heartbeat failed");
                        heartbeat_token = None;
                    }
                    Err(err) => {
                        warn!(error = %err, "route announcer heartbeat error");
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
                    Ok(resp) if resp.status().is_success() => {
                        match resp.json::<RouteChallengeResponse>().await {
                            Ok(value) => value,
                            Err(err) => {
                                warn!(error = %err, "route announcer challenge parse error");
                                sleep(Duration::from_secs(10)).await;
                                continue;
                            }
                        }
                    }
                    Ok(resp) => {
                        let status = resp.status();
                        let body = resp.text().await.unwrap_or_default();
                        warn!(%status, %body, "route announcer challenge failed");
                        sleep(Duration::from_secs(10)).await;
                        continue;
                    }
                    Err(err) => {
                        warn!(error = %err, "route announcer challenge error");
                        sleep(Duration::from_secs(10)).await;
                        continue;
                    }
                };
                if challenge.expires_at_unix_s <= now_unix_s() {
                    sleep(Duration::from_secs(2)).await;
                    continue;
                }

                let signed_at = now_unix_s();
                let reachable_urls = reachable_urls.clone();
                let signing_message = route_register_signing_message(
                    &owner_pubkey,
                    &device_id,
                    &reachable_urls,
                    &challenge.nonce,
                    signed_at,
                );
                let signature = sign_route_payload(signing_message.as_bytes());

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
                    Ok(resp) if resp.status().is_success() => {
                        match resp.json::<RouteRegisterResponse>().await {
                            Ok(value) => {
                                if value.ok {
                                    heartbeat_token = Some(value.heartbeat_token);
                                } else {
                                    heartbeat_token = None;
                                }
                            }
                            Err(err) => {
                                warn!(error = %err, "route announcer register parse error");
                            }
                        }
                    }
                    Ok(resp) => {
                        let status = resp.status();
                        let body = resp.text().await.unwrap_or_default();
                        warn!(%status, %body, "route announcer register failed");
                    }
                    Err(err) => {
                        warn!(error = %err, "route announcer register error");
                    }
                }
            }
            sleep(Duration::from_secs(30)).await;
        }
    });
}

fn parse_owner_signing_key_from_env() -> Option<SigningKey> {
    let raw = env::var("EDGERUN_ROUTE_OWNER_SECRET_KEY_B58").ok()?;
    let encoded = raw.trim();
    if encoded.is_empty() {
        return None;
    }
    let bytes = match bs58::decode(encoded).into_vec() {
        Ok(value) => value,
        Err(err) => {
            warn!(error = %err, "invalid EDGERUN_ROUTE_OWNER_SECRET_KEY_B58");
            return None;
        }
    };

    let seed: [u8; 32] = match bytes.len() {
        32 => bytes.as_slice().try_into().expect("length checked"),
        64 => {
            let mut seed = [0_u8; 32];
            seed.copy_from_slice(&bytes[..32]);
            seed
        }
        _ => {
            warn!(
                len = bytes.len(),
                "EDGERUN_ROUTE_OWNER_SECRET_KEY_B58 must decode to 32 or 64 bytes"
            );
            return None;
        }
    };

    Some(SigningKey::from_bytes(&seed))
}

fn parse_reachable_urls(env_name: &str) -> Result<Vec<String>, String> {
    let raw = std::env::var(env_name).map_err(|_| "missing value".to_string())?;
    let mut normalized = Vec::<String>::new();
    let mut seen = std::collections::HashSet::<String>::new();
    for item in raw.split(',') {
        let value = item.trim().trim_end_matches('/').to_string();
        if value.is_empty() {
            continue;
        }
        if seen.insert(value.clone()) {
            normalized.push(value);
        }
    }
    if normalized.is_empty() {
        return Err(format!("{env_name} has no valid URLs"));
    }
    Ok(normalized)
}

fn routable_addr(addr: SocketAddr) -> String {
    if addr.ip().is_unspecified() {
        format!("127.0.0.1:{}", addr.port())
    } else {
        addr.to_string()
    }
}

fn is_non_public_route_url(url: &str, env_name: &str) -> bool {
    let parsed = match reqwest::Url::parse(url) {
        Ok(value) => value,
        Err(err) => {
            warn!(
                env = env_name,
                url = %url,
                error = %err,
                "route URL is invalid for route announcement"
            );
            return true;
        }
    };

    let Some(host) = parsed.host_str() else {
        warn!(
            env = env_name,
            url = %url,
            "route URL has no host for route announcement"
        );
        return true;
    };

    let normalized_host = host.trim_start_matches('[').trim_end_matches(']');
    if normalized_host.eq_ignore_ascii_case("localhost") || normalized_host == "::1" {
        return true;
    }

    match normalized_host.parse::<IpAddr>() {
        Ok(IpAddr::V4(ip)) => {
            ip.is_loopback()
                || ip.is_private()
                || ip.is_link_local()
                || ip.is_multicast()
                || ip.is_broadcast()
                || ip.is_unspecified()
        }
        Ok(IpAddr::V6(ip)) => {
            ip.is_loopback() || ip.is_unspecified() || ip.is_multicast() || ip.is_unique_local()
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::is_non_public_route_url;

    #[test]
    fn localhost_route_urls_are_rejected() {
        assert!(is_non_public_route_url("http://127.0.0.1:5577", "TEST"));
        assert!(is_non_public_route_url("http://localhost:5577", "TEST"));
    }

    #[test]
    fn private_ipv6_addresses_are_rejected() {
        assert!(is_non_public_route_url("https://[::1]:443", "TEST"));
        assert!(is_non_public_route_url("http://10.0.0.7", "TEST"));
    }

    #[test]
    fn public_dns_names_are_allowed() {
        assert!(!is_non_public_route_url(
            "https://term.edgerun.tech",
            "TEST"
        ));
    }
}
