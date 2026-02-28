// SPDX-License-Identifier: Apache-2.0
use std::collections::HashMap;
use std::env;
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use axum::{
    Router,
    body::Bytes,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{StatusCode, header},
    response::IntoResponse,
    routing::{get, post},
};
use edgerun_event_bus::edge_internal::EdgeInternalClient;
use edgerun_event_bus::{EventTopic, OverlayNetwork};
use edgerun_hwvault_primitives::hardware::{
    DeviceSigner, HardwareSecurityMode, load_or_create_device_signer, random_token_b64url,
};
use edgerun_types::intent_pipeline::{
    EVENT_PAYLOAD_TYPE_TERM_SERVER_LIFECYCLE_START, EVENT_SCHEMA_VERSION_V1,
    EVENT_TOPIC_TERM_SERVER_LIFECYCLE_START, TermServerLifecycleEvent,
};
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use tokio::sync::{Mutex, mpsc};
use tower_http::services::ServeDir;
use tracing::{info, warn};

#[derive(Clone)]
struct AppState {
    device: DeviceSigner,
    challenges: std::sync::Arc<Mutex<HashMap<String, u64>>>,
    mux_token: Option<String>,
}

#[derive(Debug)]
struct DeviceIdentityResponse {
    backend: String,
    device_pubkey_b64url: String,
}

#[derive(Debug)]
struct DeviceChallengeResponse {
    nonce_b64url: String,
    expires_at_unix_s: u64,
}

#[derive(Debug)]
struct DeviceHandshakeRequest {
    owner_pubkey: String,
    nonce_b64url: String,
}

#[derive(Debug)]
struct DeviceHandshakeResponse {
    ok: bool,
    handshake: Option<edgerun_hwvault_primitives::hardware::DeviceHandshake>,
    error: Option<String>,
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
    let edge_internal_client = connect_edge_internal_event_bus().await;
    publish_term_server_lifecycle_event(&state, addr, edge_internal_client.as_ref()).await;

    axum::serve(tokio::net::TcpListener::bind(addr).await?, app)
        .await
        .context("server failed")?;

    Ok(())
}

async fn connect_edge_internal_event_bus() -> Option<EdgeInternalClient> {
    let enabled = env::var("EDGERUN_TERM_EDGE_INTERNAL_EVENT_BUS_ENABLED")
        .ok()
        .map(|v| {
            let normalized = v.trim().to_ascii_lowercase();
            matches!(normalized.as_str(), "1" | "true" | "yes" | "on")
        })
        .unwrap_or(false);
    if !enabled {
        return None;
    }
    let socket = env::var("EDGERUN_TERM_EDGE_INTERNAL_EVENT_BUS_SOCK")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(".edgerun-scheduler-data/event-bus/edge-internal.sock"));
    match EdgeInternalClient::connect(&socket).await {
        Ok(client) => Some(client),
        Err(err) => {
            warn!(
                socket = %socket.display(),
                error = %err,
                "edge-internal event bus connect failed; continuing without term event publish"
            );
            None
        }
    }
}

async fn publish_term_server_lifecycle_event(
    state: &AppState,
    bind_addr: SocketAddr,
    client: Option<&EdgeInternalClient>,
) {
    let Some(client) = client else {
        return;
    };
    let payload = TermServerLifecycleEvent {
        schema_version: EVENT_SCHEMA_VERSION_V1,
        backend: format!("{:?}", state.device.backend).to_lowercase(),
        device_pubkey_b64url: state.device.public_key_b64url.clone(),
        bind_addr: bind_addr.to_string(),
    };
    publish_term_server_lifecycle_payload(client, payload).await;
}

async fn publish_term_server_lifecycle_payload(
    client: &EdgeInternalClient,
    payload: TermServerLifecycleEvent,
) {
    let topic = match EventTopic::new(
        OverlayNetwork::EdgeInternal,
        EVENT_TOPIC_TERM_SERVER_LIFECYCLE_START,
    ) {
        Ok(topic) => topic,
        Err(err) => {
            warn!(error = %err, "invalid term-server lifecycle topic");
            return;
        }
    };
    let encoded = match bincode::serialize(&payload) {
        Ok(encoded) => encoded,
        Err(err) => {
            warn!(error = %err, "failed to encode term-server lifecycle event");
            return;
        }
    };
    if let Err(err) = client
        .publish(
            &topic,
            "term-server",
            EVENT_PAYLOAD_TYPE_TERM_SERVER_LIFECYCLE_START,
            encoded,
        )
        .await
    {
        warn!(error = %err, "failed to publish term-server lifecycle event");
    }
}

async fn device_identity(State(state): State<AppState>) -> impl IntoResponse {
    json_response(
        StatusCode::OK,
        device_identity_json(&DeviceIdentityResponse {
            backend: format!("{:?}", state.device.backend).to_lowercase(),
            device_pubkey_b64url: state.device.public_key_b64url.clone(),
        }),
    )
}

async fn device_challenge(State(state): State<AppState>) -> impl IntoResponse {
    let now = now_unix_s();
    let expires_at = now + 120;
    let nonce = random_token_b64url(24);

    let mut guard = state.challenges.lock().await;
    guard.retain(|_, exp| *exp > now);
    guard.insert(nonce.clone(), expires_at);

    json_response(
        StatusCode::OK,
        device_challenge_json(&DeviceChallengeResponse {
            nonce_b64url: nonce,
            expires_at_unix_s: expires_at,
        }),
    )
}

async fn device_handshake(State(state): State<AppState>, body: Bytes) -> impl IntoResponse {
    let req = match parse_device_handshake_request(&body) {
        Ok(value) => value,
        Err(err) => {
            return json_response(
                StatusCode::BAD_REQUEST,
                device_handshake_json(&DeviceHandshakeResponse {
                    ok: false,
                    handshake: None,
                    error: Some(err),
                }),
            );
        }
    };
    let now = now_unix_s();
    let mut challenges = state.challenges.lock().await;
    challenges.retain(|_, exp| *exp > now);

    let Some(exp) = challenges.remove(req.nonce_b64url.trim()) else {
        return json_response(
            StatusCode::OK,
            device_handshake_json(&DeviceHandshakeResponse {
                ok: false,
                handshake: None,
                error: Some("unknown or expired nonce".to_string()),
            }),
        );
    };
    if exp <= now {
        return json_response(
            StatusCode::OK,
            device_handshake_json(&DeviceHandshakeResponse {
                ok: false,
                handshake: None,
                error: Some("nonce expired".to_string()),
            }),
        );
    }
    drop(challenges);

    match state
        .device
        .build_handshake(req.owner_pubkey.trim(), req.nonce_b64url.trim(), now)
    {
        Ok(handshake) => json_response(
            StatusCode::OK,
            device_handshake_json(&DeviceHandshakeResponse {
                ok: true,
                handshake: Some(handshake),
                error: None,
            }),
        ),
        Err(err) => json_response(
            StatusCode::OK,
            device_handshake_json(&DeviceHandshakeResponse {
                ok: false,
                handshake: None,
                error: Some(err.to_string()),
            }),
        ),
    }
}

fn json_response(status: StatusCode, body: String) -> axum::response::Response {
    (
        status,
        [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
        body,
    )
        .into_response()
}

fn json_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 8);
    for ch in input.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                let code = c as u32;
                out.push_str(&format!("\\u{code:04x}"));
            }
            c => out.push(c),
        }
    }
    out
}

fn json_string(input: &str) -> String {
    format!("\"{}\"", json_escape(input))
}

fn device_identity_json(value: &DeviceIdentityResponse) -> String {
    format!(
        "{{\"backend\":{},\"device_pubkey_b64url\":{}}}",
        json_string(&value.backend),
        json_string(&value.device_pubkey_b64url)
    )
}

fn device_challenge_json(value: &DeviceChallengeResponse) -> String {
    format!(
        "{{\"nonce_b64url\":{},\"expires_at_unix_s\":{}}}",
        json_string(&value.nonce_b64url),
        value.expires_at_unix_s
    )
}

fn device_handshake_json(value: &DeviceHandshakeResponse) -> String {
    let mut body = String::from("{\"ok\":");
    body.push_str(if value.ok { "true" } else { "false" });
    if let Some(handshake) = value.handshake.as_ref() {
        body.push_str(",\"handshake\":");
        body.push_str(&device_handshake_payload_json(handshake));
    }
    if let Some(error) = value.error.as_ref() {
        body.push_str(",\"error\":");
        body.push_str(&json_string(error));
    }
    body.push('}');
    body
}

fn device_handshake_payload_json(
    handshake: &edgerun_hwvault_primitives::hardware::DeviceHandshake,
) -> String {
    format!(
        "{{\"payload\":{{\"version\":{},\"owner_pubkey\":{},\"device_pubkey_b64url\":{},\"nonce_b64url\":{},\"issued_at_unix_s\":{}}},\"canonical\":{},\"signature_b64url\":{},\"backend\":{}}}",
        handshake.payload.version,
        json_string(&handshake.payload.owner_pubkey),
        json_string(&handshake.payload.device_pubkey_b64url),
        json_string(&handshake.payload.nonce_b64url),
        handshake.payload.issued_at_unix_s,
        json_string(&handshake.canonical),
        json_string(&handshake.signature_b64url),
        json_string(&format!("{:?}", handshake.backend).to_lowercase())
    )
}

fn parse_device_handshake_request(body: &[u8]) -> Result<DeviceHandshakeRequest, String> {
    let text = std::str::from_utf8(body).map_err(|_| "request body must be valid utf-8")?;
    let parsed = json::parse(text).map_err(|err| format!("invalid json: {err}"))?;
    let owner_pubkey = parsed["owner_pubkey"]
        .as_str()
        .ok_or("owner_pubkey must be a string")?
        .trim()
        .to_string();
    let nonce_b64url = parsed["nonce_b64url"]
        .as_str()
        .ok_or("nonce_b64url must be a string")?
        .trim()
        .to_string();
    if owner_pubkey.is_empty() {
        return Err("owner_pubkey is required".to_string());
    }
    if nonce_b64url.is_empty() {
        return Err("nonce_b64url is required".to_string());
    }
    Ok(DeviceHandshakeRequest {
        owner_pubkey,
        nonce_b64url,
    })
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
    let value = u32::from_be_bytes([
        bytes[*cur],
        bytes[*cur + 1],
        bytes[*cur + 2],
        bytes[*cur + 3],
    ]);
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

#[cfg(test)]
mod tests {
    use edgerun_event_bus::edge_internal::{EdgeInternalClient, spawn_edge_internal_broker};
    use edgerun_event_bus::{EventTopic, OverlayNetwork, RuntimeEventBus};
    use edgerun_types::intent_pipeline::{
        EVENT_PAYLOAD_TYPE_TERM_SERVER_LIFECYCLE_START, EVENT_SCHEMA_VERSION_V1,
        EVENT_TOPIC_TERM_SERVER_LIFECYCLE_START, TermServerLifecycleEvent,
    };
    use tokio::time::{Duration, timeout};

    use super::publish_term_server_lifecycle_payload;

    #[tokio::test]
    async fn term_server_lifecycle_event_is_received_over_edge_internal_grpc() {
        let topic = EventTopic::new(
            OverlayNetwork::EdgeInternal,
            EVENT_TOPIC_TERM_SERVER_LIFECYCLE_START,
        )
        .expect("topic");
        let bus = RuntimeEventBus::with_topics(64, std::slice::from_ref(&topic)).expect("bus");
        let socket_path = std::env::temp_dir().join(format!(
            "edgerun-term-server-edge-internal-{}-{}.sock",
            std::process::id(),
            crate::now_unix_s()
        ));

        let _broker = spawn_edge_internal_broker(&socket_path, bus)
            .await
            .expect("spawn broker");
        let subscriber = EdgeInternalClient::connect(&socket_path)
            .await
            .expect("subscriber connect");
        let publisher = EdgeInternalClient::connect(&socket_path)
            .await
            .expect("publisher connect");

        let mut rx = subscriber.subscribe(&topic).await.expect("subscribe");
        let payload = TermServerLifecycleEvent {
            schema_version: EVENT_SCHEMA_VERSION_V1,
            backend: "software".to_string(),
            device_pubkey_b64url: "device-test-pubkey".to_string(),
            bind_addr: "127.0.0.1:5577".to_string(),
        };
        publish_term_server_lifecycle_payload(&publisher, payload.clone()).await;

        let event = timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("recv timeout")
            .expect("recv event");
        assert_eq!(
            event.payload_type,
            EVENT_PAYLOAD_TYPE_TERM_SERVER_LIFECYCLE_START
        );
        let decoded: TermServerLifecycleEvent =
            bincode::deserialize(&event.payload).expect("decode payload");
        assert_eq!(decoded.device_pubkey_b64url, payload.device_pubkey_b64url);
        assert_eq!(decoded.bind_addr, payload.bind_addr);

        let _ = tokio::fs::remove_file(&socket_path).await;
    }
}
