use std::collections::BTreeMap;
use std::env;
use std::future::Future;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use edgerun_crypto::{fill_random, sha::Sha256};
use edgerun_email::smtp::SmtpClient;
use edgerun_encoding::base64::{base64url_decode, base64url_nopad_encode};
use edgerun_encoding::hex::{bytes_to_hex, hex_to_bytes};
use edgerun_json::{Map, TapeValue, Value, from_slice, json, parse_json_tape, to_string_pretty};
use edgerun_node::http::{
    Handler, HttpServer, Method, Request, Response, StatusCode, TlsCertificate,
};
use edgerun_node::rt;
use edgerun_node::services::acme_runtime::NodeAcmeOrchestrator;
use edgerun_protocols::dhcp::{
    DhcpMessage, DhcpMessageType, DhcpServerConfig, DhcpServerCore, PxeClientArch,
};
use edgerun_protocols::dns::{
    DnsMessage, DnsRecord, DnsRecordData, DnsRecordType, DnsZone, handle_query_without_forwarding,
    resolve,
};
use edgerun_protocols::http::http1::multipart::{extract_boundary, is_multipart, parse_multipart};
use edgerun_protocols::oauth::pkce::PkcePair;
use edgerun_protocols::smtp::EmailBuilder;
use edgerun_protocols::websocket::{
    WS_BASE_HEADER_LEN, WS_OPCODE_CLOSE, WS_OPCODE_PONG, WebSocketMessage, decode_client_message,
    decode_frame_prefix, decode_payload_len, encode_server_control, encode_server_frame,
    encode_upgrade_response,
};
use edgerun_storage::{
    BlobKeySource, BlobStore, BlobStoreConfig, DbBatch, open_file_database_repairing_tail,
};
use edgerun_ui_core::gpu::{
    Color4, FontAtlas, GpuScene, ICON_VERTEX_FLOAT_STRIDE, POCKETBASE_ADMIN_COLLECTION_ROW_BASE_ID,
    PackedGpuScene, RECT_FLOAT_STRIDE, RectMode, TEXT_VERTEX_FLOAT_STRIDE, UiPocketBaseAdminState,
    UiPocketBaseAdminView, UiPocketBaseCollection, UiPocketBaseLog, UiPocketBaseRecord,
    build_pocketbase_admin_scene,
};
use edgerun_ui_core::initial_setup::{DEFAULT_KDF_ROUNDS, PASSWORD_MIN_LEN};
use edgerun_ui_core::tabler_svg_atlas_generated::{
    TABLER_SVG_ATLAS_ALPHA, TABLER_SVG_ATLAS_H, TABLER_SVG_ATLAS_W,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const PASSWORD_ITERATIONS: u32 = 100_000;
const DERIVED_DB_STATE_KEY: &[u8] = b"pocketbase/state";
const DERIVED_DB_PREFIX: &[u8] = b"pocketbase/";
const DERIVED_SECRET_KEY: &str = "pocketbase/secret";
const DERIVED_SETTINGS_KEY: &str = "pocketbase/settings";
const DERIVED_COLLECTION_PREFIX: &str = "pocketbase/collection/";
const DERIVED_RECORD_PREFIX: &str = "pocketbase/record/";
const DERIVED_ADMIN_PREFIX: &str = "pocketbase/admin/";
const DERIVED_MIGRATION_PREFIX: &str = "pocketbase/migration/";
const DERIVED_LOG_PREFIX: &str = "pocketbase/log/";
const DERIVED_CRON_PREFIX: &str = "pocketbase/cron/";
const DERIVED_AUTH_TOKEN_PREFIX: &str = "pocketbase/auth-token/";
const DERIVED_REALTIME_EVENT_PREFIX: &str = "pocketbase/realtime-event/";
const ADMIN_FONT_BYTES: &[u8] =
    include_bytes!("../../../utility/edgerun-ui-core/assets/Geist-Variable.ttf");

fn main() {
    let config = match Config::parse(env::args().skip(1)) {
        Ok(config) => config,
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
    };

    if config.version {
        println!("edgerun-pocketbase {VERSION}");
        return;
    }

    if let Some(command) = config.command {
        if let Err(error) = run_cli_command(config.data_dir, command) {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }

    let state = match PocketState::load(config.data_dir.clone()) {
        Ok(state) => Arc::new(Mutex::new(state)),
        Err(error) => {
            eprintln!("failed to load data store: {error}");
            std::process::exit(1);
        }
    };

    let bind = config.bind.clone();
    let admin_ws_bind = match admin_ws_bind_for(&bind) {
        Ok(bind) => bind,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };
    let https_bind = config.https_bind.clone();
    let workers = config.workers;
    let tls = match load_tls_certificate(&config) {
        Ok(tls) => tls,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    };
    if let Err(error) = start_admin_ws_server(admin_ws_bind.clone(), Arc::clone(&state)) {
        eprintln!("{error}");
        std::process::exit(1);
    }
    let runtime = match rt::Runtime::new_multi_thread()
        .worker_threads(workers)
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("failed to start runtime: {error}");
            std::process::exit(1);
        }
    };
    println!(
        "edgerun-pocketbase runtime workers: {}",
        runtime.worker_count()
    );
    let result = runtime.block_on(async move {
        if let (Some(https_bind), Some(cert)) = (https_bind.clone(), tls) {
            let https_state = Arc::clone(&state);
            let https_server = HttpServer::new(PocketHandler { state: https_state }).with_tls(cert);
            let https_bound = https_server.bind(https_bind.as_str()).await?;
            println!(
                "edgerun-pocketbase listening on https://{}",
                https_bound.local_addr()
            );
            rt::spawn(async move {
                if let Err(error) = https_bound.serve().await {
                    eprintln!("https server error: {error}");
                }
            });
        }
        let server = HttpServer::new(PocketHandler { state });
        let bound = server.bind(bind.as_str()).await?;
        println!(
            "edgerun-pocketbase listening on http://{}",
            bound.local_addr()
        );
        bound.serve().await
    });

    if let Err(error) = result {
        eprintln!("server error: {error}");
        std::process::exit(1);
    }
}

fn run_cli_command(data_dir: PathBuf, command: CliCommand) -> Result<(), String> {
    let mut state = PocketState::load(data_dir).map_err(|error| error.to_string())?;
    match command {
        CliCommand::SuperuserList => {
            for admin in state.admins.values() {
                println!("{}\t{}", admin.id, admin.email);
            }
            Ok(())
        }
        CliCommand::SuperuserCreate { email, password } => {
            if state.admins.values().any(|admin| admin.email == email) {
                return Err("superuser already exists".to_string());
            }
            state.insert_or_update_superuser(&email, &password, false)?;
            println!("created superuser {email}");
            Ok(())
        }
        CliCommand::SuperuserUpsert { email, password } => {
            let existed = state.admins.values().any(|admin| admin.email == email);
            state.insert_or_update_superuser(&email, &password, true)?;
            println!(
                "{} superuser {email}",
                if existed { "updated" } else { "created" }
            );
            Ok(())
        }
        CliCommand::SuperuserUpdate { email, password } => {
            if !state.admins.values().any(|admin| admin.email == email) {
                return Err("superuser not found".to_string());
            }
            state.insert_or_update_superuser(&email, &password, true)?;
            println!("updated superuser {email}");
            Ok(())
        }
        CliCommand::SuperuserDelete { email } => {
            let Some(id) = state
                .admins
                .iter()
                .find_map(|(id, admin)| (admin.email == email).then_some(id.clone()))
            else {
                return Err("superuser not found".to_string());
            };
            state.admins.remove(&id);
            state
                .persist_admin_delta(&id, true)
                .map_err(|error| error.to_string())?;
            println!("deleted superuser {email}");
            Ok(())
        }
    }
}

fn admin_ws_bind_for(http_bind: &str) -> Result<String, String> {
    let addr = http_bind.parse::<SocketAddr>().map_err(|error| {
        format!("cannot derive admin websocket bind from --bind {http_bind:?}: {error}")
    })?;
    let port = addr.port().checked_add(1).ok_or_else(|| {
        format!(
            "cannot derive admin websocket bind from --bind {http_bind:?}: port 65535 has no adjacent websocket port"
        )
    })?;
    Ok(SocketAddr::new(addr.ip(), port).to_string())
}

fn start_admin_ws_server(bind: String, state: Arc<Mutex<PocketState>>) -> Result<(), String> {
    let listener = TcpListener::bind(&bind)
        .map_err(|error| format!("admin websocket bind {bind} failed: {error}"))?;
    println!("edgerun-pocketbase admin websocket on ws://{bind}/_/admin-ws");
    thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let state = Arc::clone(&state);
                    thread::spawn(move || {
                        if let Err(error) = handle_admin_ws_client(stream, state) {
                            eprintln!("admin websocket client error: {error}");
                        }
                    });
                }
                Err(error) => eprintln!("admin websocket accept error: {error}"),
            }
        }
    });
    Ok(())
}

fn handle_admin_ws_client(
    mut stream: TcpStream,
    state: Arc<Mutex<PocketState>>,
) -> Result<(), String> {
    let request = read_ws_handshake(&mut stream)?;
    if !request.starts_with("GET /_/admin-ws ") {
        return Err("unsupported websocket path".to_string());
    }
    let response = encode_upgrade_response(&request, "edgerun-pocketbase-admin-v1")
        .map_err(|e| e.to_string())?;
    stream
        .write_all(response.as_bytes())
        .map_err(|error| error.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_millis(250)))
        .map_err(|error| error.to_string())?;

    let mut last_revision = String::new();
    loop {
        let snapshot = {
            let state = state
                .lock()
                .map_err(|_| "state lock poisoned".to_string())?;
            state.admin_ui_snapshot()
        };
        let revision = snapshot.get_str("revision").unwrap_or_default().to_string();
        if revision != last_revision {
            send_ws_json(&mut stream, snapshot)?;
            last_revision = revision;
        }

        match read_ws_message(&mut stream) {
            Ok(Some(WebSocketMessage::Binary(bytes))) => {
                handle_admin_ws_command(&mut stream, &state, &bytes)?;
            }
            Ok(Some(WebSocketMessage::Ping(payload))) => {
                let pong =
                    encode_server_control(WS_OPCODE_PONG, &payload).map_err(|e| e.to_string())?;
                stream.write_all(&pong).map_err(|error| error.to_string())?;
            }
            Ok(Some(WebSocketMessage::Close)) => {
                let close =
                    encode_server_control(WS_OPCODE_CLOSE, &[]).map_err(|e| e.to_string())?;
                let _ = stream.write_all(&close);
                return Ok(());
            }
            Ok(Some(WebSocketMessage::Pong)) | Ok(None) => {}
            Err(error) => return Err(error),
        }
    }
}

fn read_ws_handshake(stream: &mut TcpStream) -> Result<String, String> {
    let mut bytes = Vec::new();
    let mut buf = [0u8; 512];
    while !bytes.ends_with(b"\r\n\r\n") {
        if bytes.len() > 8192 {
            return Err("websocket handshake too large".to_string());
        }
        let len = stream.read(&mut buf).map_err(|error| error.to_string())?;
        if len == 0 {
            return Err("websocket handshake closed".to_string());
        }
        bytes.extend_from_slice(&buf[..len]);
    }
    String::from_utf8(bytes).map_err(|error| error.to_string())
}

fn read_ws_message(stream: &mut TcpStream) -> Result<Option<WebSocketMessage>, String> {
    let mut base = [0u8; WS_BASE_HEADER_LEN];
    match stream.read_exact(&mut base) {
        Ok(()) => {}
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
            ) =>
        {
            return Ok(None);
        }
        Err(error) => return Err(error.to_string()),
    }
    let prefix = decode_frame_prefix(&base)
        .and_then(|prefix| prefix.require_client_mask())
        .map_err(|error| error.to_string())?;
    let mut extended = vec![0u8; prefix.extended_len_bytes()];
    if !extended.is_empty() {
        stream
            .read_exact(&mut extended)
            .map_err(|error| error.to_string())?;
    }
    let payload_len =
        decode_payload_len(prefix, &extended, 1024 * 1024).map_err(|error| error.to_string())?;
    let mut mask = [0u8; 4];
    stream
        .read_exact(&mut mask)
        .map_err(|error| error.to_string())?;
    let mut payload = vec![0u8; payload_len];
    if payload_len > 0 {
        stream
            .read_exact(&mut payload)
            .map_err(|error| error.to_string())?;
    }
    decode_client_message(prefix.fin, prefix.opcode, mask, payload)
        .map(Some)
        .map_err(|error| error.to_string())
}

fn handle_admin_ws_command(
    stream: &mut TcpStream,
    state: &Arc<Mutex<PocketState>>,
    bytes: &[u8],
) -> Result<(), String> {
    match parse_admin_ws_command(bytes)? {
        AdminWsCommand::Snapshot => {
            let snapshot = state
                .lock()
                .map_err(|_| "state lock poisoned".to_string())?
                .admin_ui_snapshot();
            send_ws_json(stream, snapshot)
        }
        AdminWsCommand::Action { id } => {
            let action = id.to_string();
            if let Some(view) = admin_action_view(&action) {
                send_ws_json(
                    stream,
                    json!({
                        "type": "admin.view",
                        "id": action,
                        "view": view
                    }),
                )
            } else {
                let target = admin_action_target(&action).unwrap_or("/_/").to_string();
                send_ws_json(
                    stream,
                    json!({
                        "type": "admin.navigate",
                        "id": action,
                        "target": target
                    }),
                )
            }
        }
        AdminWsCommand::Collection { name } => send_ws_json(
            stream,
            json!({
                "type": "admin.collection",
                "collection": name
            }),
        ),
        AdminWsCommand::Unknown { .. } => send_ws_json(
            stream,
            json!({
                "type": "admin.error",
                "message": "unknown admin websocket command"
            }),
        ),
    }
}

fn parse_admin_ws_command(bytes: &[u8]) -> Result<AdminWsCommand, String> {
    let text = std::str::from_utf8(bytes).map_err(|error| error.to_string())?;
    let tape = parse_json_tape(text).map_err(|error| error.to_string())?;
    let root = tape
        .root(text)
        .ok_or_else(|| "empty admin websocket command".to_string())?;
    let kind = root.get_str("type").unwrap_or_default();
    Ok(match kind {
        "admin.hello" | "admin.refresh" => AdminWsCommand::Snapshot,
        "admin.action" => AdminWsCommand::Action {
            id: root.get_u64("id").unwrap_or_default(),
        },
        "admin.collection" => AdminWsCommand::Collection {
            name: root.get_str("collection").unwrap_or_default().to_string(),
        },
        _ => AdminWsCommand::Unknown {
            kind: kind.to_string(),
        },
    })
}

fn send_ws_json(stream: &mut TcpStream, value: Value) -> Result<(), String> {
    let text = edgerun_json::to_string(&value).map_err(|error| error.to_string())?;
    let frame = encode_server_frame(0x1, text.as_bytes()).map_err(|error| error.to_string())?;
    stream.write_all(&frame).map_err(|error| error.to_string())
}

fn admin_action_target(id: &str) -> Option<&'static str> {
    match id {
        "7106" => Some("/api/edgerun/dns/zones"),
        "7107" => Some("/api/edgerun/acme/http01"),
        "7108" => Some("/api/health"),
        "7109" => Some("/api/collections/import"),
        _ => None,
    }
}

fn admin_action_view(id: &str) -> Option<&'static str> {
    match id {
        "7101" => Some("collections"),
        "7102" => Some("settings"),
        "7103" => Some("logs"),
        "7104" => Some("backups"),
        "7105" => Some("crons"),
        _ => None,
    }
}

#[derive(Debug)]
struct Config {
    bind: String,
    https_bind: Option<String>,
    tls_cert: Option<PathBuf>,
    tls_key: Option<PathBuf>,
    autocert: bool,
    domains: Vec<String>,
    data_dir: PathBuf,
    workers: usize,
    version: bool,
    command: Option<CliCommand>,
}

#[derive(Debug)]
enum CliCommand {
    SuperuserCreate { email: String, password: String },
    SuperuserUpsert { email: String, password: String },
    SuperuserUpdate { email: String, password: String },
    SuperuserDelete { email: String },
    SuperuserList,
}

impl Config {
    fn parse(args: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut bind = "127.0.0.1:8090".to_string();
        let mut https_bind = None;
        let mut tls_cert = None;
        let mut tls_key = None;
        let mut autocert = false;
        let mut domains = Vec::new();
        let mut data_dir = PathBuf::from("pb_data");
        let mut workers = default_worker_threads();
        let mut version = false;
        let mut args = args.peekable();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--bind" => {
                    bind = args
                        .next()
                        .ok_or_else(|| "--bind requires an address".to_string())?;
                }
                "--https-bind" => {
                    https_bind = Some(
                        args.next()
                            .ok_or_else(|| "--https-bind requires an address".to_string())?,
                    );
                }
                "--tls-cert" => {
                    tls_cert =
                        Some(PathBuf::from(args.next().ok_or_else(|| {
                            "--tls-cert requires a PEM path".to_string()
                        })?));
                }
                "--tls-key" => {
                    tls_key =
                        Some(PathBuf::from(args.next().ok_or_else(|| {
                            "--tls-key requires a PEM path".to_string()
                        })?));
                }
                "--autocert" => autocert = true,
                "--domain" => {
                    domains.push(
                        args.next()
                            .ok_or_else(|| "--domain requires a hostname".to_string())?,
                    );
                }
                "--dir" => {
                    data_dir = PathBuf::from(
                        args.next()
                            .ok_or_else(|| "--dir requires a path".to_string())?,
                    );
                }
                "--workers" => {
                    workers = args
                        .next()
                        .ok_or_else(|| "--workers requires a positive integer".to_string())?
                        .parse::<usize>()
                        .map_err(|_| "--workers must be a positive integer".to_string())?
                        .max(1);
                }
                "--version" | "-V" => version = true,
                "--help" | "-h" => return Err(help()),
                "superuser" => {
                    let rest = args.collect::<Vec<_>>();
                    return Ok(Self {
                        bind,
                        https_bind,
                        tls_cert,
                        tls_key,
                        autocert,
                        domains,
                        data_dir,
                        workers,
                        version,
                        command: Some(parse_superuser_command(&rest)?),
                    });
                }
                other => return Err(format!("unknown option: {other}\n{}", help())),
            }
        }

        Ok(Self {
            bind,
            https_bind,
            tls_cert,
            tls_key,
            autocert,
            domains,
            data_dir,
            workers,
            version,
            command: None,
        })
    }
}

fn default_worker_threads() -> usize {
    thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .max(1)
}

fn help() -> String {
    "Usage: edgerun-pocketbase [--bind 127.0.0.1:8090] [--https-bind 0.0.0.0:443] [--tls-cert cert.pem --tls-key key.pem | --autocert --domain example.com] [--dir pb_data] [--workers N] [--version] [superuser create|upsert|update|delete|list ...]".to_string()
}

fn load_tls_certificate(config: &Config) -> Result<Option<TlsCertificate>, String> {
    match (&config.tls_cert, &config.tls_key, config.autocert) {
        (Some(cert), Some(key), _) => load_tls_certificate_from_files(cert, key).map(Some),
        (Some(_), None, _) | (None, Some(_), _) => {
            Err("--tls-cert and --tls-key must be provided together".to_string())
        }
        (None, None, true) => load_or_create_autocert(&config.data_dir, &config.domains).map(Some),
        (None, None, false) if config.https_bind.is_some() => {
            load_or_create_autocert(&config.data_dir, &config.domains).map(Some)
        }
        _ => Ok(None),
    }
}

fn load_tls_certificate_from_files(
    cert_path: &Path,
    key_path: &Path,
) -> Result<TlsCertificate, String> {
    let cert_pem = std::fs::read_to_string(cert_path)
        .map_err(|error| format!("failed to read TLS cert {}: {error}", cert_path.display()))?;
    let key_pem = std::fs::read_to_string(key_path)
        .map_err(|error| format!("failed to read TLS key {}: {error}", key_path.display()))?;
    TlsCertificate::from_pem(&format!("{cert_pem}\n{key_pem}"))
        .map_err(|error| format!("failed to parse TLS cert/key: {error}"))
}

fn load_or_create_autocert(data_dir: &Path, domains: &[String]) -> Result<TlsCertificate, String> {
    let dir = data_dir.join("autocert");
    let cert_path = dir.join("cert.pem");
    let key_path = dir.join("key.pem");
    if cert_path.exists() && key_path.exists() {
        return load_tls_certificate_from_files(&cert_path, &key_path);
    }
    std::fs::create_dir_all(&dir).map_err(|error| {
        format!(
            "failed to create autocert directory {}: {error}",
            dir.display()
        )
    })?;
    let names = autocert_names(domains);
    let refs = names.iter().map(String::as_str).collect::<Vec<_>>();
    let cert = edgerun_protocols::tls::generate_self_signed(&refs)
        .map_err(|error| format!("failed to generate TLS autocert: {error}"))?;
    std::fs::write(&cert_path, cert.cert_pem())
        .map_err(|error| format!("failed to write autocert {}: {error}", cert_path.display()))?;
    let key_pem = cert
        .key_pem()
        .map_err(|error| format!("failed to serialize autocert key: {error}"))?;
    std::fs::write(&key_path, key_pem).map_err(|error| {
        format!(
            "failed to write autocert key {}: {error}",
            key_path.display()
        )
    })?;
    Ok(cert)
}

fn autocert_names(domains: &[String]) -> Vec<String> {
    if domains.is_empty() {
        vec!["localhost".to_string(), "127.0.0.1".to_string()]
    } else {
        domains.to_vec()
    }
}

fn admin_shell_html() -> &'static str {
    r#"<!doctype html>
<html>
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width,initial-scale=1">
  <title>EdgeRun PocketBase Admin</title>
  <style>
    @font-face{font-family:EdgeRunAdmin;src:url('/_/assets/Geist-Variable.ttf') format('truetype');font-weight:100 900;font-display:block}
    html,body,#app{margin:0;width:100%;height:100%;overflow:hidden;background:#090909;font-family:EdgeRunAdmin,system-ui,sans-serif}
  </style>
</head>
<body>
  <div id="app"></div>
  <script type="module" src="/_/admin-ui.js"></script>
</body>
</html>"#
}

fn admin_ui_host_js() -> &'static str {
    r#"const root = document.getElementById('app');
let scene = null;
let gl = null, rectProgram = null, texProgram = null, quadVao = null, textTex = null, iconTex = null;
let fontAtlasAlpha = '', iconAtlasAlpha = '';
let adminSocket = null, adminSnapshot = null;
let wsReady = false;
let adminView = 'collections';
let adminCollection = '';
const adminViewById = new Map([[7101,'collections'],[7102,'settings'],[7103,'logs'],[7104,'backups'],[7105,'crons']]);
const encoder = new TextEncoder();
const rectVert = `#version 300 es
layout(location=0) in vec2 a_pos;
uniform vec2 u_screen;
uniform vec4 u_rect;
out vec2 v_local;
out vec2 v_size;
void main(){
  vec2 px = u_rect.xy + a_pos * u_rect.zw;
  vec2 ndc = vec2(px.x / u_screen.x * 2.0 - 1.0, 1.0 - px.y / u_screen.y * 2.0);
  gl_Position = vec4(ndc,0,1);
  v_local = a_pos * u_rect.zw;
  v_size = u_rect.zw;
}`;
const rectFrag = `#version 300 es
precision highp float;
in vec2 v_local;
in vec2 v_size;
out vec4 out_color;
uniform vec4 u_color;
uniform float u_radius;
uniform float u_shadow;
uniform int u_mode;
float rounded_box(vec2 p, vec2 b, float r){
  vec2 q = abs(p) - b + vec2(r);
  return length(max(q,0.0)) + min(max(q.x,q.y),0.0) - r;
}
void main(){
  vec2 p = v_local - v_size * 0.5;
  float d = rounded_box(p, v_size * 0.5, u_radius);
  float aa = max(fwidth(d), 0.75);
  float alpha = 1.0 - smoothstep(0.0, aa, d);
  if(u_mode == 1){
    float sd = rounded_box(p - vec2(0.0, -u_shadow * 0.18), v_size * 0.5, u_radius + u_shadow * 0.35);
    float blur = max(u_shadow, 1.0);
    alpha = 1.0 - smoothstep(-blur, blur, sd);
    out_color = vec4(u_color.rgb, u_color.a * alpha * 0.28);
  } else if(u_mode == 2){
    float inner = rounded_box(p, v_size * 0.5 - vec2(1.25), max(u_radius - 1.25, 0.0));
    float border = (1.0 - smoothstep(0.0, aa, d)) * smoothstep(0.0, aa, inner);
    out_color = vec4(u_color.rgb, u_color.a * border);
  } else {
    out_color = vec4(u_color.rgb, u_color.a * alpha);
  }
}`;
const texVert = `#version 300 es
layout(location=0) in vec2 a_pos;
layout(location=1) in vec2 a_uv;
layout(location=2) in vec4 a_color;
uniform vec2 u_screen;
out vec2 v_uv;
out vec4 v_color;
void main(){
  vec2 ndc = vec2(a_pos.x / u_screen.x * 2.0 - 1.0, 1.0 - a_pos.y / u_screen.y * 2.0);
  gl_Position = vec4(ndc,0,1);
  v_uv = a_uv;
  v_color = a_color;
}`;
const texFrag = `#version 300 es
precision highp float;
in vec2 v_uv;
in vec4 v_color;
out vec4 out_color;
uniform sampler2D u_tex;
void main(){
  float a = texture(u_tex, v_uv).r;
  out_color = vec4(v_color.rgb, v_color.a * a);
}`;
function shader(type, src){
  const s = gl.createShader(type);
  gl.shaderSource(s, src);
  gl.compileShader(s);
  if(!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(s));
  return s;
}
function program(vs, fs){
  const p = gl.createProgram();
  gl.attachShader(p, shader(gl.VERTEX_SHADER, vs));
  gl.attachShader(p, shader(gl.FRAGMENT_SHADER, fs));
  gl.linkProgram(p);
  if(!gl.getProgramParameter(p, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(p));
  return p;
}
function uploadAlphaTexture(width, height, bytes){
  const tex = gl.createTexture();
  gl.bindTexture(gl.TEXTURE_2D, tex);
  gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.R8, width, height, 0, gl.RED, gl.UNSIGNED_BYTE, bytes);
  return tex;
}
async function initGl(){
  root.replaceChildren();
  const canvas = document.createElement('canvas');
  canvas.id = 'scene';
  canvas.style.width = '100vw';
  canvas.style.height = '100vh';
  canvas.style.display = 'block';
  root.appendChild(canvas);
  gl = canvas.getContext('webgl2', {alpha:false, antialias:true});
  if(!gl) throw new Error('WebGL2 unavailable');
  rectProgram = program(rectVert, rectFrag);
  texProgram = program(texVert, texFrag);
  quadVao = gl.createVertexArray();
  const quad = gl.createBuffer();
  gl.bindVertexArray(quadVao);
  gl.bindBuffer(gl.ARRAY_BUFFER, quad);
  gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([0,0,1,0,1,1,0,0,1,1,0,1]), gl.STATIC_DRAW);
  gl.enableVertexAttribArray(0);
  gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);
  await uploadSceneTextures();
}
async function uploadSceneTextures(){
  if(textTex && fontAtlasAlpha !== scene.fontAtlas.alpha){
    gl.deleteTexture(textTex);
    textTex = null;
  }
  if(iconTex && iconAtlasAlpha !== scene.iconAtlas.alpha){
    gl.deleteTexture(iconTex);
    iconTex = null;
  }
  const fontBytes = new Uint8Array(await fetch(scene.fontAtlas.alpha, {cache:'no-store'}).then(r=>r.arrayBuffer()));
  textTex = uploadAlphaTexture(scene.fontAtlas.width, scene.fontAtlas.height, fontBytes);
  fontAtlasAlpha = scene.fontAtlas.alpha;
  if(!iconTex){
    const iconBytes = new Uint8Array(await fetch(scene.iconAtlas.alpha).then(r=>r.arrayBuffer()));
    iconTex = uploadAlphaTexture(scene.iconAtlas.width, scene.iconAtlas.height, iconBytes);
    iconAtlasAlpha = scene.iconAtlas.alpha;
  }
}
function resizeCanvas(){
  const canvas = gl.canvas;
  const dpr = window.devicePixelRatio || 1;
  canvas.width = Math.max(1, Math.floor(innerWidth * dpr));
  canvas.height = Math.max(1, Math.floor(innerHeight * dpr));
  gl.viewport(0,0,canvas.width,canvas.height);
}
function drawRects(rects){
  gl.useProgram(rectProgram);
  gl.bindVertexArray(quadVao);
  gl.uniform2f(gl.getUniformLocation(rectProgram, 'u_screen'), innerWidth, innerHeight);
  const uRect = gl.getUniformLocation(rectProgram, 'u_rect');
  const uColor = gl.getUniformLocation(rectProgram, 'u_color');
  const uRadius = gl.getUniformLocation(rectProgram, 'u_radius');
  const uShadow = gl.getUniformLocation(rectProgram, 'u_shadow');
  const uMode = gl.getUniformLocation(rectProgram, 'u_mode');
  const stride = scene.packed?.rectStride || 15;
  for(let i=0;i<rects.length;i+=stride){
    gl.uniform4f(uRect, rects[i], rects[i+1], rects[i+2], rects[i+3]);
    gl.uniform1f(uRadius, rects[i+4]);
    gl.uniform1f(uShadow, rects[i+5]);
    gl.uniform4f(uColor, rects[i+6], rects[i+7], rects[i+8], rects[i+9]);
    gl.uniform1i(uMode, rects[i+14] || 0);
    gl.drawArrays(gl.TRIANGLES, 0, 6);
  }
}
function drawTextured(vertices, tex){
  if(!vertices.length) return;
  const vao = gl.createVertexArray();
  const buffer = gl.createBuffer();
  gl.bindVertexArray(vao);
  gl.useProgram(texProgram);
  gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
  gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(vertices), gl.STREAM_DRAW);
  gl.enableVertexAttribArray(0);
  gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 32, 0);
  gl.enableVertexAttribArray(1);
  gl.vertexAttribPointer(1, 2, gl.FLOAT, false, 32, 8);
  gl.enableVertexAttribArray(2);
  gl.vertexAttribPointer(2, 4, gl.FLOAT, false, 32, 16);
  gl.uniform2f(gl.getUniformLocation(texProgram, 'u_screen'), innerWidth, innerHeight);
  gl.activeTexture(gl.TEXTURE0);
  gl.bindTexture(gl.TEXTURE_2D, tex);
  gl.uniform1i(gl.getUniformLocation(texProgram, 'u_tex'), 0);
  gl.drawArrays(gl.TRIANGLES, 0, vertices.length / 8);
  gl.deleteBuffer(buffer);
  gl.deleteVertexArray(vao);
  gl.bindVertexArray(null);
}
function render(){
  resizeCanvas();
  const c = scene.clear;
  gl.clearColor(c.r,c.g,c.b,c.a);
  gl.clear(gl.COLOR_BUFFER_BIT);
  gl.enable(gl.BLEND);
  gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
  drawRects(scene.packed.rects || []);
  drawTextured(scene.packed.iconVertices || [], iconTex);
  drawTextured(scene.packed.textVertices || [], textTex);
}
async function loadScene(){
  const dpr = window.devicePixelRatio || 1;
  scene = await fetch(`/_/admin-scene.json?w=${innerWidth}&h=${innerHeight}&dpr=${dpr}&view=${encodeURIComponent(adminView)}&collection=${encodeURIComponent(adminCollection)}`, {cache:'no-store'}).then(r=>r.json());
  if(scene.collection) adminCollection = scene.collection;
  if(!gl) await initGl();
  else if(fontAtlasAlpha !== scene.fontAtlas.alpha || iconAtlasAlpha !== scene.iconAtlas.alpha) await uploadSceneTextures();
  render();
}
function adminWsUrl(){
  const port = Number(location.port || (location.protocol === 'https:' ? 443 : 80)) + 1;
  const scheme = location.protocol === 'https:' ? 'wss:' : 'ws:';
  return `${scheme}//${location.hostname}:${port}/_/admin-ws`;
}
function sendAdmin(kind, payload = {}){
  if(!adminSocket || adminSocket.readyState !== WebSocket.OPEN) return false;
  adminSocket.send(encoder.encode(JSON.stringify({type: kind, ...payload})));
  return true;
}
function connectAdminSocket(){
  adminSocket = new WebSocket(adminWsUrl(), 'edgerun-pocketbase-admin-v1');
  adminSocket.binaryType = 'arraybuffer';
  adminSocket.addEventListener('open', () => {
    wsReady = true;
    sendAdmin('admin.hello', {width: innerWidth, height: innerHeight});
  });
  adminSocket.addEventListener('message', async event => {
    const text = typeof event.data === 'string' ? event.data : new TextDecoder().decode(event.data);
    const message = JSON.parse(text);
    if(message.type === 'admin.snapshot'){
      if(!adminSnapshot || adminSnapshot.revision !== message.revision){
        adminSnapshot = message;
        await loadScene();
      }
    } else if(message.type === 'admin.navigate' && message.target){
      location.href = message.target;
    } else if(message.type === 'admin.view' && message.view){
      adminView = message.view;
      await loadScene();
    } else if(message.type === 'admin.collection'){
      adminView = 'collections';
      adminCollection = message.collection || '';
      await loadScene();
    }
  });
  adminSocket.addEventListener('close', () => {
    wsReady = false;
    setTimeout(connectAdminSocket, 750);
  });
}
async function boot(){
  await loadScene();
  connectAdminSocket();
}
addEventListener('resize', () => {
  loadScene();
  sendAdmin('admin.refresh', {width: innerWidth, height: innerHeight});
});
root.addEventListener('click', ev => {
  if(!scene) return;
  const hit = [...scene.hits].reverse().find(h => ev.offsetX >= h.x && ev.offsetY >= h.y && ev.offsetX <= h.x + h.w && ev.offsetY <= h.y + h.h);
  if(hit && adminViewById.has(hit.id)){
    if(!sendAdmin('admin.action', {id: hit.id})){
      adminView = adminViewById.get(hit.id);
      loadScene();
    }
  } else if(hit && scene.collectionActions && scene.collectionActions[String(hit.id)]){
    const collection = scene.collectionActions[String(hit.id)];
    if(!sendAdmin('admin.collection', {collection})){
      adminView = 'collections';
      adminCollection = collection;
      loadScene();
    }
  } else if(hit && scene.actions && scene.actions[String(hit.id)]){
    if(!sendAdmin('admin.action', {id: hit.id})) location.href = scene.actions[String(hit.id)];
  }
});
boot().catch(err => console.error(err));"#
}

fn parse_superuser_command(args: &[String]) -> Result<CliCommand, String> {
    match args {
        [action] if action == "list" => Ok(CliCommand::SuperuserList),
        [action, email] if action == "delete" => Ok(CliCommand::SuperuserDelete {
            email: email.clone(),
        }),
        [action, email, password] if action == "create" => Ok(CliCommand::SuperuserCreate {
            email: email.clone(),
            password: password.clone(),
        }),
        [action, email, password] if action == "upsert" => Ok(CliCommand::SuperuserUpsert {
            email: email.clone(),
            password: password.clone(),
        }),
        [action, email, password] if action == "update" => Ok(CliCommand::SuperuserUpdate {
            email: email.clone(),
            password: password.clone(),
        }),
        _ => Err("Usage: edgerun-pocketbase [--dir pb_data] superuser create|upsert|update <email> <password> | delete <email> | list".to_string()),
    }
}

#[derive(Clone)]
struct PocketHandler {
    state: Arc<Mutex<PocketState>>,
}

impl Handler for PocketHandler {
    fn handle(&self, request: Request) -> Pin<Box<dyn Future<Output = Response> + Send + '_>> {
        Box::pin(async move {
            if let Some(response) = stateless_response(&request) {
                return response;
            }
            let mut state = match self.state.lock() {
                Ok(state) => state,
                Err(_) => return error_response(500, "state lock poisoned"),
            };
            state.route(request)
        })
    }
}

fn stateless_response(request: &Request) -> Option<Response> {
    (request.method() == &Method::GET && request.uri().path() == "/api/health").then(|| {
        json_response(
            200,
            json!({"code":200,"message":"API is healthy.","data":{}}),
        )
    })
}

#[derive(Clone, Debug)]
struct PocketState {
    data_dir: PathBuf,
    secret: String,
    collections: BTreeMap<String, Collection>,
    admins: BTreeMap<String, Admin>,
    migrations: Vec<Migration>,
    settings: Value,
    logs: Vec<ActivityLog>,
    crons: Vec<CronJob>,
    auth_tokens: Vec<AuthFlowToken>,
    realtime_clients: BTreeMap<String, RealtimeClient>,
    realtime_events: Vec<RealtimeEvent>,
}

#[derive(Default)]
struct PersistDelta {
    delete_prefixes: Vec<Vec<u8>>,
    rows: BTreeMap<Vec<u8>, Vec<u8>>,
}

impl PersistDelta {
    fn delete_prefix(mut self, prefix: impl Into<Vec<u8>>) -> Self {
        self.delete_prefixes.push(prefix.into());
        self
    }

    fn put_json(&mut self, key: impl AsRef<str>, value: &Value) -> std::io::Result<()> {
        derived_row_json(&mut self.rows, key.as_ref(), value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum AdminWsCommand {
    Snapshot,
    Action { id: u64 },
    Collection { name: String },
    Unknown { kind: String },
}

#[derive(Clone, Debug)]
struct Collection {
    id: String,
    name: String,
    kind: CollectionKind,
    schema: Vec<Field>,
    records: BTreeMap<String, Record>,
    list_rule: Option<String>,
    view_rule: Option<String>,
    create_rule: Option<String>,
    update_rule: Option<String>,
    delete_rule: Option<String>,
    created: String,
    updated: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum CollectionKind {
    Base,
    Auth,
}

#[derive(Clone, Debug)]
struct Field {
    id: String,
    name: String,
    field_type: String,
    required: bool,
    system: bool,
    hidden: bool,
    options: Value,
}

#[derive(Clone, Debug)]
struct Record {
    id: String,
    data: BTreeMap<String, Value>,
    created: String,
    updated: String,
}

#[derive(Clone, Debug)]
struct Admin {
    id: String,
    email: String,
    password: String,
    created: String,
    updated: String,
}

#[derive(Clone, Debug)]
struct Migration {
    id: String,
    name: String,
    applied: String,
}

#[derive(Clone, Debug)]
struct ActivityLog {
    id: String,
    method: String,
    path: String,
    status: u16,
    actor: String,
    created: String,
}

#[derive(Clone, Debug)]
struct CronJob {
    id: String,
    expression: String,
    command: String,
    last_run: Option<String>,
}

#[derive(Clone, Debug)]
struct AuthFlowToken {
    token: String,
    collection: String,
    record_id: String,
    kind: String,
    email: Option<String>,
    payload: Option<String>,
    expires: u64,
}

#[derive(Clone, Debug)]
struct RealtimeClient {
    id: String,
    subscriptions: Vec<String>,
    created: String,
}

#[derive(Clone, Debug)]
struct RealtimeEvent {
    id: String,
    action: String,
    collection: String,
    record: Value,
    created: String,
}

#[derive(Clone, Debug)]
struct Actor {
    kind: String,
    id: String,
}

#[derive(Clone, Debug)]
struct FileUpload {
    field: String,
    name: String,
    bytes: Vec<u8>,
}

impl PocketState {
    fn load(data_dir: PathBuf) -> std::io::Result<Self> {
        std::fs::create_dir_all(&data_dir)?;
        if let Some(state) = Self::load_from_derived_db(&data_dir)? {
            return Ok(state);
        }
        let path = data_dir.join("store.json");
        if !path.exists() {
            let mut state = Self {
                data_dir,
                secret: id("sec"),
                collections: BTreeMap::new(),
                admins: BTreeMap::new(),
                migrations: Vec::new(),
                settings: default_settings(),
                logs: Vec::new(),
                crons: default_crons(),
                auth_tokens: Vec::new(),
                realtime_clients: BTreeMap::new(),
                realtime_events: Vec::new(),
            };
            state.ensure_builtin_auth_collection();
            state.save()?;
            return Ok(state);
        }

        let bytes = std::fs::read(&path)?;
        let value = from_slice(&bytes).map_err(invalid_data)?;
        let state = Self::from_json(data_dir, value);
        state.save()?;
        Ok(state)
    }

    fn load_from_derived_db(data_dir: &std::path::Path) -> std::io::Result<Option<Self>> {
        let path = data_dir.join("data.db");
        if !path.exists() {
            return Ok(None);
        }
        let db = open_file_database_repairing_tail(&path).map_err(invalid_data)?;
        if let Some(state) = Self::load_from_typed_derived_rows(data_dir, &db)? {
            return Ok(Some(state));
        }
        let Some(row) = db.get_meta(DERIVED_DB_STATE_KEY) else {
            return Ok(None);
        };
        let value = from_slice(&row.value).map_err(invalid_data)?;
        Ok(Some(Self::from_json(data_dir.to_path_buf(), value)))
    }

    fn load_from_typed_derived_rows(
        data_dir: &std::path::Path,
        db: &edgerun_storage::DerivedDb<edgerun_storage::FileStorage>,
    ) -> std::io::Result<Option<Self>> {
        let rows = db
            .meta_rows_with_prefix(DERIVED_DB_PREFIX)
            .collect::<Vec<_>>();
        if !rows
            .iter()
            .any(|row| row.key.as_slice() == DERIVED_SETTINGS_KEY.as_bytes())
        {
            return Ok(None);
        }
        let mut state = Self {
            data_dir: data_dir.to_path_buf(),
            secret: id("sec"),
            collections: BTreeMap::new(),
            admins: BTreeMap::new(),
            migrations: Vec::new(),
            settings: default_settings(),
            logs: Vec::new(),
            crons: Vec::new(),
            auth_tokens: Vec::new(),
            realtime_clients: BTreeMap::new(),
            realtime_events: Vec::new(),
        };
        for row in &rows {
            if row.key.as_slice() == DERIVED_SECRET_KEY.as_bytes() {
                state.secret = String::from_utf8(row.value.clone()).map_err(invalid_data)?;
            } else if row.key.as_slice() == DERIVED_SETTINGS_KEY.as_bytes() {
                state.settings = from_slice(&row.value).map_err(invalid_data)?;
            }
        }
        for row in &rows {
            let key = String::from_utf8_lossy(&row.key);
            let value = || from_slice(&row.value).map_err(invalid_data);
            if key.starts_with(DERIVED_COLLECTION_PREFIX) {
                if let Some(collection) = Collection::from_json(&value()?) {
                    state
                        .collections
                        .insert(collection.name.clone(), collection);
                }
            } else if let Some(rest) = key.strip_prefix(DERIVED_RECORD_PREFIX) {
                let Some((collection_name, _)) = rest.split_once('/') else {
                    continue;
                };
                if let Some(record) = Record::from_json(&value()?) {
                    if let Some(collection) = state.collections.get_mut(collection_name) {
                        collection.records.insert(record.id.clone(), record);
                    }
                }
            } else if key.starts_with(DERIVED_ADMIN_PREFIX) {
                if let Some(admin) = Admin::from_json(&value()?) {
                    state.admins.insert(admin.id.clone(), admin);
                }
            } else if key.starts_with(DERIVED_MIGRATION_PREFIX) {
                if let Some(migration) = Migration::from_json(&value()?) {
                    state.migrations.push(migration);
                }
            } else if key.starts_with(DERIVED_LOG_PREFIX) {
                if let Some(log) = ActivityLog::from_json(&value()?) {
                    state.logs.push(log);
                }
            } else if key.starts_with(DERIVED_CRON_PREFIX) {
                if let Some(cron) = CronJob::from_json(&value()?) {
                    state.crons.push(cron);
                }
            } else if key.starts_with(DERIVED_AUTH_TOKEN_PREFIX) {
                if let Some(token) = AuthFlowToken::from_json(&value()?) {
                    state.auth_tokens.push(token);
                }
            } else if key.starts_with(DERIVED_REALTIME_EVENT_PREFIX) {
                if let Some(event) = RealtimeEvent::from_json(&value()?) {
                    state.realtime_events.push(event);
                }
            }
        }
        if state.crons.is_empty() {
            state.crons = default_crons();
        }
        state.ensure_builtin_auth_collection();
        Ok(Some(state))
    }

    fn save(&self) -> std::io::Result<()> {
        let encoded = to_string_pretty(&self.to_json()).map_err(invalid_data)?;
        self.save_to_derived_db(encoded.as_bytes())?;
        self.export_state_json(&encoded)?;
        Ok(())
    }

    fn export_state_json(&self, encoded: &str) -> std::io::Result<()> {
        let path = self.data_dir.join("store.json");
        let tmp = self.data_dir.join("store.json.tmp");
        std::fs::write(&tmp, encoded)?;
        std::fs::rename(tmp, path)?;
        let _ = self.store_edge_blob("state/store.json", encoded.as_bytes());
        Ok(())
    }

    fn save_to_derived_db(&self, encoded: &[u8]) -> std::io::Result<()> {
        let mut db = open_file_database_repairing_tail(self.data_dir.join("data.db"))
            .map_err(invalid_data)?;
        let now = unix_seconds() as i64;
        let existing = db
            .meta_rows_with_prefix(DERIVED_DB_PREFIX)
            .map(|row| (row.key.clone(), row.value.clone()))
            .collect::<BTreeMap<_, _>>();
        let desired = self.derived_rows(encoded)?;
        let mut batch = DbBatch::new();
        for key in existing.keys() {
            if !desired.contains_key(key) {
                batch.delete_meta(key).map_err(invalid_data)?;
            }
        }
        for (key, value) in desired {
            if existing.get(&key) != Some(&value) {
                batch.put_meta(&key, &value, now).map_err(invalid_data)?;
            }
        }
        if batch.is_empty() {
            return Ok(());
        }
        db.apply_batch(batch).map_err(invalid_data)
    }

    fn derived_rows(&self, encoded: &[u8]) -> std::io::Result<BTreeMap<Vec<u8>, Vec<u8>>> {
        let mut rows = BTreeMap::new();
        rows.insert(DERIVED_DB_STATE_KEY.to_vec(), encoded.to_vec());
        derived_row(&mut rows, DERIVED_SECRET_KEY, self.secret.as_bytes());
        derived_row_json(&mut rows, DERIVED_SETTINGS_KEY, &self.settings)?;
        for collection in self.collections.values() {
            derived_row_json(
                &mut rows,
                &derived_collection_key(&collection.name),
                &collection.to_json(),
            )?;
            for record in collection.records.values() {
                derived_row_json(
                    &mut rows,
                    &derived_record_key(&collection.name, &record.id),
                    &record.to_store_json(),
                )?;
            }
        }
        for admin in self.admins.values() {
            derived_row_json(
                &mut rows,
                &derived_admin_key(&admin.id),
                &admin.to_store_json(),
            )?;
        }
        for migration in &self.migrations {
            derived_row_json(
                &mut rows,
                &derived_migration_key(&migration.id),
                &migration.to_json(),
            )?;
        }
        for log in &self.logs {
            derived_row_json(&mut rows, &derived_log_key(&log.id), &log.to_json())?;
        }
        for cron in &self.crons {
            derived_row_json(&mut rows, &derived_cron_key(&cron.id), &cron.to_json())?;
        }
        for token in &self.auth_tokens {
            derived_row_json(
                &mut rows,
                &derived_auth_token_key(&token.token),
                &token.to_json(),
            )?;
        }
        for event in &self.realtime_events {
            derived_row_json(
                &mut rows,
                &derived_realtime_event_key(&event.id),
                &event.to_json(),
            )?;
        }
        Ok(rows)
    }

    fn persist_record_delta_or_500(
        &self,
        value: Value,
        ok_status: u16,
        collection_name: &str,
        record_id: Option<String>,
        delete_record: bool,
        event_id: Option<String>,
    ) -> Response {
        let result = self.persist_record_delta(collection_name, record_id, delete_record, event_id);
        persist_response(value, ok_status, "record delta", result)
    }

    fn persist_record_delta(
        &self,
        collection_name: &str,
        record_id: Option<String>,
        delete_record: bool,
        event_id: Option<String>,
    ) -> std::io::Result<()> {
        let encoded = to_string_pretty(&self.to_json()).map_err(invalid_data)?;
        let mut db = open_file_database_repairing_tail(self.data_dir.join("data.db"))
            .map_err(invalid_data)?;
        let now = unix_seconds() as i64;
        let mut batch = DbBatch::new();
        batch
            .put_meta(DERIVED_DB_STATE_KEY, encoded.as_bytes(), now)
            .map_err(invalid_data)?;
        if let Some(collection) = self.collections.get(collection_name) {
            derived_batch_put_json(
                &mut batch,
                &derived_collection_key(collection_name),
                &collection.to_json(),
                now,
            )?;
            if let Some(record_id) = record_id {
                let key = derived_record_key(collection_name, &record_id);
                if delete_record {
                    batch.delete_meta(key.as_bytes()).map_err(invalid_data)?;
                } else if let Some(record) = collection.records.get(&record_id) {
                    derived_batch_put_json(&mut batch, &key, &record.to_store_json(), now)?;
                }
            }
        }
        if let Some(event_id) = event_id {
            if let Some(event) = self
                .realtime_events
                .iter()
                .find(|event| event.id == event_id)
            {
                derived_batch_put_json(
                    &mut batch,
                    &derived_realtime_event_key(&event_id),
                    &event.to_json(),
                    now,
                )?;
            }
        }
        db.apply_batch(batch).map_err(invalid_data)?;
        self.export_state_json(&encoded)
    }

    fn persist_collection_delta_or_500(
        &self,
        value: Value,
        ok_status: u16,
        collection_name: &str,
        delete_collection: bool,
        delete_records: bool,
    ) -> Response {
        match self.persist_collection_delta(collection_name, delete_collection, delete_records) {
            Ok(()) => json_response(ok_status, value),
            Err(error) => {
                error_response(500, &format!("failed to persist collection delta: {error}"))
            }
        }
    }

    fn persist_collection_delta(
        &self,
        collection_name: &str,
        delete_collection: bool,
        delete_records: bool,
    ) -> std::io::Result<()> {
        let mut delta = PersistDelta::default();
        if !delete_collection && let Some(collection) = self.collections.get(collection_name) {
            delta.put_json(
                derived_collection_key(collection_name),
                &collection.to_json(),
            )?;
            if !delete_records {
                for record in collection.records.values() {
                    delta.put_json(
                        derived_record_key(collection_name, &record.id),
                        &record.to_store_json(),
                    )?;
                }
            }
        }
        delta = if delete_collection || delete_records {
            delta
                .delete_prefix(derived_collection_key(collection_name).into_bytes())
                .delete_prefix(derived_record_prefix(collection_name).into_bytes())
        } else {
            delta.delete_prefix(derived_collection_key(collection_name).into_bytes())
        };
        self.persist_typed_delta(delta)
    }

    fn persist_imported_collections_delta(
        &self,
        collection_names: &[String],
    ) -> std::io::Result<()> {
        let mut delta = PersistDelta::default();
        for collection_name in collection_names {
            delta = delta
                .delete_prefix(derived_collection_key(collection_name).into_bytes())
                .delete_prefix(derived_record_prefix(collection_name).into_bytes());
            if let Some(collection) = self.collections.get(collection_name) {
                delta.put_json(
                    derived_collection_key(collection_name),
                    &collection.to_json(),
                )?;
                for record in collection.records.values() {
                    delta.put_json(
                        derived_record_key(collection_name, &record.id),
                        &record.to_store_json(),
                    )?;
                }
            }
        }
        self.persist_typed_delta(delta)
    }

    fn persist_settings_or_500(&self, value: Value, ok_status: u16) -> Response {
        persist_response(
            value,
            ok_status,
            "settings delta",
            self.persist_settings_delta(),
        )
    }

    fn persist_settings_delta(&self) -> std::io::Result<()> {
        let mut delta = PersistDelta::default();
        delta.put_json(DERIVED_SETTINGS_KEY, &self.settings)?;
        self.persist_typed_delta(delta)
    }

    fn persist_admin_or_500(
        &self,
        value: Value,
        ok_status: u16,
        admin_id: &str,
        delete: bool,
    ) -> Response {
        persist_response(
            value,
            ok_status,
            "admin delta",
            self.persist_admin_delta(admin_id, delete),
        )
    }

    fn persist_admin_delta(&self, admin_id: &str, delete: bool) -> std::io::Result<()> {
        let mut delta =
            PersistDelta::default().delete_prefix(derived_admin_key(admin_id).into_bytes());
        if !delete && let Some(admin) = self.admins.get(admin_id) {
            delta.put_json(derived_admin_key(admin_id), &admin.to_store_json())?;
        }
        self.persist_typed_delta(delta)
    }

    fn persist_migration_delta_or_500(
        &self,
        value: Value,
        ok_status: u16,
        migration_id: &str,
    ) -> Response {
        persist_response(
            value,
            ok_status,
            "migration delta",
            self.persist_migration_delta(migration_id),
        )
    }

    fn persist_migration_delta(&self, migration_id: &str) -> std::io::Result<()> {
        let mut delta = PersistDelta::default();
        if let Some(migration) = self.migrations.iter().find(|item| item.id == migration_id) {
            delta.put_json(derived_migration_key(migration_id), &migration.to_json())?;
        }
        self.persist_typed_delta(delta)
    }

    fn persist_cron_delta_or_500(&self, value: Value, ok_status: u16, cron_id: &str) -> Response {
        persist_response(
            value,
            ok_status,
            "cron delta",
            self.persist_cron_delta(cron_id),
        )
    }

    fn persist_cron_delta(&self, cron_id: &str) -> std::io::Result<()> {
        let mut delta = PersistDelta::default();
        if let Some(cron) = self.crons.iter().find(|item| item.id == cron_id) {
            delta.put_json(derived_cron_key(cron_id), &cron.to_json())?;
        }
        self.persist_typed_delta(delta)
    }

    fn persist_auth_tokens_delta(&self) -> std::io::Result<()> {
        let mut delta =
            PersistDelta::default().delete_prefix(DERIVED_AUTH_TOKEN_PREFIX.as_bytes().to_vec());
        for token in &self.auth_tokens {
            delta.put_json(derived_auth_token_key(&token.token), &token.to_json())?;
        }
        self.persist_typed_delta(delta)
    }

    fn persist_logs_delta(&self) -> std::io::Result<()> {
        let mut delta =
            PersistDelta::default().delete_prefix(DERIVED_LOG_PREFIX.as_bytes().to_vec());
        for log in &self.logs {
            delta.put_json(derived_log_key(&log.id), &log.to_json())?;
        }
        self.persist_typed_delta(delta)
    }

    fn persist_activity_log_delta(
        &self,
        log: &ActivityLog,
        removed_ids: &[String],
    ) -> std::io::Result<()> {
        let mut db = open_file_database_repairing_tail(self.data_dir.join("data.db"))
            .map_err(invalid_data)?;
        let now = unix_seconds() as i64;
        let mut batch = DbBatch::new();
        for id in removed_ids {
            batch
                .delete_meta(derived_log_key(id).as_bytes())
                .map_err(invalid_data)?;
        }
        if !removed_ids.iter().any(|id| id == &log.id) {
            derived_batch_put_json(&mut batch, &derived_log_key(&log.id), &log.to_json(), now)?;
        }
        db.apply_batch(batch).map_err(invalid_data)
    }

    fn persist_typed_delta(&self, mut delta: PersistDelta) -> std::io::Result<()> {
        let encoded = to_string_pretty(&self.to_json()).map_err(invalid_data)?;
        let mut db = open_file_database_repairing_tail(self.data_dir.join("data.db"))
            .map_err(invalid_data)?;
        let now = unix_seconds() as i64;
        let mut batch = DbBatch::new();
        batch
            .put_meta(DERIVED_DB_STATE_KEY, encoded.as_bytes(), now)
            .map_err(invalid_data)?;
        for prefix in &delta.delete_prefixes {
            for row in db.meta_rows_with_prefix(prefix) {
                if !delta.rows.contains_key(&row.key) {
                    batch.delete_meta(&row.key).map_err(invalid_data)?;
                }
            }
        }
        for (key, value) in core::mem::take(&mut delta.rows) {
            if db.get_meta(&key).as_ref().map(|row| row.value.as_slice()) != Some(value.as_slice())
            {
                batch.put_meta(&key, &value, now).map_err(invalid_data)?;
            }
        }
        db.apply_batch(batch).map_err(invalid_data)?;
        self.export_state_json(&encoded)
    }

    fn edge_blob_store(&self) -> Result<BlobStore, String> {
        let config = BlobStoreConfig {
            blob_dir: self.data_dir.join("edgerun-storage").join("blobs"),
        };
        BlobStore::open(
            &config,
            BlobKeySource::Software {
                private_key_bytes: self.secret.as_bytes().to_vec(),
            },
        )
        .map_err(|error| error.to_string())
    }

    fn store_edge_blob(&self, label: &str, bytes: &[u8]) -> Result<String, String> {
        let store = self.edge_blob_store()?;
        let recipients = vec![store.node_identity().to_vec()];
        let blob_id = store
            .store(bytes, &recipients)
            .map_err(|error| error.to_string())?;
        let manifest = self.data_dir.join("edgerun-storage").join("manifest.jsonl");
        if let Some(parent) = manifest.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let line = format!(
            "{{\"label\":\"{}\",\"blobId\":\"{}\",\"bytes\":{},\"created\":\"{}\"}}\n",
            json_escape(label),
            blob_id,
            bytes.len(),
            timestamp()
        );
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(manifest)
            .and_then(|mut file| {
                use std::io::Write;
                file.write_all(line.as_bytes())
            })
            .map_err(|error| error.to_string())?;
        Ok(blob_id)
    }

    fn route(&mut self, request: Request) -> Response {
        let method = request.method().clone();
        let path = request.uri().path().trim_matches('/').to_string();
        let logged_method = method.as_str().to_string();
        let logged_path = request.uri().path().to_string();
        let origin = request
            .headers()
            .get("origin")
            .or_else(|| request.headers().get("Origin"))
            .map(|value| value.as_str().to_string());
        if let Some(body) = request.body() {
            let max = self.request_body_limit();
            if body.len() > max {
                let mut response = error_response(413, "request body exceeds configured limit");
                apply_cors_headers(&mut response, origin.as_deref(), &self.settings);
                self.record_activity(logged_method, logged_path, 413, None);
                return response;
            }
        }
        if method == Method::OPTIONS {
            let mut response = Response::text(status(204), "");
            apply_cors_headers(&mut response, origin.as_deref(), &self.settings);
            if should_record_activity(&logged_path) {
                self.record_activity(logged_method, logged_path, 204, None);
            }
            return response;
        }
        let actor = self.authenticate_request(&request);
        let mut response = self.dispatch(method, path, request, actor.as_ref());
        apply_cors_headers(&mut response, origin.as_deref(), &self.settings);
        if should_record_activity(&logged_path) {
            self.record_activity(
                logged_method,
                logged_path,
                response.status().as_u16(),
                actor.as_ref(),
            );
        }
        response
    }

    fn request_body_limit(&self) -> usize {
        self.settings
            .get_object("request")
            .and_then(|request| request.get("maxBodyBytes"))
            .and_then(Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(32 * 1024 * 1024)
    }

    fn admin_ui_manifest(&self) -> Value {
        json!({
            "uiCore": {
                "crate": "edgerun-ui-core",
                "passwordMinLen": PASSWORD_MIN_LEN,
                "defaultKdfRounds": DEFAULT_KDF_ROUNDS,
                "surface": "pocketbase-admin",
                "shadcn": {
                    "demoCount": edgerun_ui_core::gpu::shadcn_exact_demo_count(),
                    "nativeCount": edgerun_ui_core::gpu::shadcn_native_demo_count(),
                    "exactParityCount": edgerun_ui_core::gpu::shadcn_exact_parity_count()
                }
            },
            "counts": {
                "collections": self.collections.len(),
                "superusers": self.admins.len(),
                "logs": self.logs.len(),
                "crons": self.crons.len(),
                "dnsZones": dns_zones_from_settings(&self.settings).len()
            },
            "routes": {
                "collections": "/api/collections",
                "settings": "/api/settings",
                "logs": "/api/logs",
                "backups": "/api/backups",
                "crons": "/api/crons",
                "dnsZones": "/api/edgerun/dns/zones",
                "dnsQuery": "/api/edgerun/dns/query",
                "dhcpDiscover": "/api/edgerun/dhcp/discover",
                "acmeHttp01": "/api/edgerun/acme/http01",
                "acmeDns01Plan": "/api/edgerun/acme/dns01",
                "acmeTlsAlpn01Plan": "/api/edgerun/acme/tls-alpn01",
                "hooksRun": "/api/edgerun/hooks/run"
            }
        })
    }

    fn admin_ui_snapshot(&self) -> Value {
        let record_count = self
            .collections
            .values()
            .map(|collection| collection.records.len())
            .sum::<usize>();
        let latest_log_id = self
            .logs
            .iter()
            .filter(|log| !log.path.starts_with("/_/"))
            .next_back()
            .map(|log| log.id.as_str())
            .unwrap_or_default();
        let visible_log_count = self
            .logs
            .iter()
            .filter(|log| !log.path.starts_with("/_/"))
            .count();
        let revision = format!(
            "{}:{}:{}:{}:{}",
            self.collections.len(),
            record_count,
            self.admins.len(),
            visible_log_count,
            latest_log_id
        );
        json!({
            "type": "admin.snapshot",
            "revision": revision,
            "counts": {
                "collections": self.collections.len(),
                "records": record_count,
                "superusers": self.admins.len(),
                "logs": visible_log_count,
                "crons": self.crons.len(),
                "dnsZones": dns_zones_from_settings(&self.settings).len()
            },
            "collections": self.collections.values().take(24).map(|collection| {
                json!({
                    "id": collection.id.clone(),
                    "name": collection.name.clone(),
                    "type": collection.kind.as_str(),
                    "records": collection.records.len(),
                    "fields": collection.schema.len()
                })
            }).collect::<Vec<_>>(),
            "logs": self.logs.iter().rev().take(20).map(ActivityLog::to_json).collect::<Vec<_>>()
        })
    }

    fn admin_ui_scene(&self, query: Option<&str>) -> Result<Value, String> {
        let width = query_usize(query, "w").unwrap_or(1280).clamp(360, 3840) as f32;
        let height = query_usize(query, "h").unwrap_or(820).clamp(480, 2160) as f32;
        let dpr = query_f32(query, "dpr").unwrap_or(1.0).clamp(1.0, 4.0);
        let view = admin_view_from_query(query);
        let requested_collection = query_string(query, "collection").unwrap_or_default();
        let selected_collection = self.admin_selected_collection(&requested_collection);
        let atlas = FontAtlas::load_geist_for_device_scale(16.0, dpr)
            .map_err(|error| format!("font atlas failed: {error}"))?;
        let font_atlas_alpha =
            format!("/_/assets/admin-font-alpha-geist-gvar-v2-16.bin?dpr={dpr:.3}");
        let mut scene = GpuScene::new(Color4::rgb_u8(9, 9, 9));
        let admin_state = self.admin_ui_projection(view, &selected_collection);
        build_pocketbase_admin_scene(&mut scene, &atlas, &admin_state, width, height);
        let mut packed = PackedGpuScene::default();
        packed.pack(&scene);
        let packed_stats = packed.stats();
        let collection_actions = self
            .collections
            .values()
            .take(12)
            .enumerate()
            .map(|(index, collection)| {
                (
                    (POCKETBASE_ADMIN_COLLECTION_ROW_BASE_ID + index as u32).to_string(),
                    collection.name.clone(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        Ok(json!({
            "surface": "pocketbase-admin",
            "source": "edgerun-ui-core::gpu::pocketbase_admin",
            "renderer": "webgl2",
            "view": view.as_str(),
            "collection": selected_collection,
            "width": width,
            "height": height,
            "clear": color_json(scene.clear),
            "rects": scene.rects().iter().map(gpu_rect_json).collect::<Vec<_>>(),
            "hits": scene.hits().iter().map(gpu_hit_json).collect::<Vec<_>>(),
            "font": {
                "family": "EdgeRunAdmin",
                "asset": "/_/assets/Geist-Variable.ttf",
                "source": "edgerun-ui-core/assets/Geist-Variable.ttf"
            },
            "fontAtlas": {
                "width": atlas.width,
                "height": atlas.height,
                "cssPx": 16.0,
                "rasterPx": 16.0 * dpr,
                "devicePixelRatio": dpr,
                "alpha": font_atlas_alpha
            },
            "iconAtlas": {
                "provider": "tabler",
                "width": TABLER_SVG_ATLAS_W,
                "height": TABLER_SVG_ATLAS_H,
                "alpha": "/_/assets/tabler-alpha.bin"
            },
            "packed": {
                "rectStride": RECT_FLOAT_STRIDE,
                "textStride": TEXT_VERTEX_FLOAT_STRIDE,
                "iconStride": ICON_VERTEX_FLOAT_STRIDE,
                "rects": packed.rects,
                "textVertices": packed.text_vertices,
                "iconVertices": packed.icon_vertices
            },
            "actions": {
                "7106": "/api/edgerun/dns/zones",
                "7107": "/api/edgerun/acme/http01",
                "7108": "/api/health",
                "7109": "/api/collections/import"
            },
            "collectionActions": collection_actions,
            "stats": {
                "rects": scene.rects().len(),
                "hits": scene.hits().len(),
                "textVertices": packed_stats.text_vertex_count,
                "iconVertices": packed_stats.icon_vertex_count
            }
        }))
    }

    fn admin_ui_projection(
        &self,
        active_view: UiPocketBaseAdminView,
        selected_collection: &str,
    ) -> UiPocketBaseAdminState {
        UiPocketBaseAdminState {
            active_view,
            selected_collection: selected_collection.to_string(),
            superuser_count: self.admins.len(),
            collections: self
                .collections
                .values()
                .map(|collection| UiPocketBaseCollection {
                    name: collection.name.clone(),
                    kind: collection.kind.as_str().to_string(),
                    field_count: collection.schema.len(),
                    record_count: collection.records.len(),
                    records: collection
                        .records
                        .values()
                        .take(10)
                        .map(|record| UiPocketBaseRecord {
                            id: record.id.clone(),
                            email: record
                                .data
                                .get("email")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_string(),
                            created: record.created.clone(),
                            updated: record.updated.clone(),
                        })
                        .collect(),
                })
                .collect(),
            logs: self
                .logs
                .iter()
                .rev()
                .take(20)
                .map(|log| UiPocketBaseLog {
                    method: log.method.clone(),
                    path: log.path.clone(),
                    status: log.status,
                    created: log.created.clone(),
                })
                .collect(),
        }
    }

    fn admin_selected_collection(&self, requested: &str) -> String {
        if !requested.is_empty() && self.collections.contains_key(requested) {
            return requested.to_string();
        }
        self.collections.keys().next().cloned().unwrap_or_default()
    }

    fn dispatch(
        &mut self,
        method: Method,
        path: String,
        request: Request,
        actor: Option<&Actor>,
    ) -> Response {
        let parts = path.split('/').collect::<Vec<_>>();

        if method == Method::GET && (path.is_empty() || path == "_/" || path == "_") {
            return Response::html(status(200), admin_shell_html());
        }
        if method == Method::GET && path == "_/admin-ui.js" {
            return Response::from_parts(
                status(200),
                Default::default(),
                admin_ui_host_js().as_bytes().to_vec(),
            )
            .with_header("Content-Type", "text/javascript; charset=utf-8")
            .with_header("Cache-Control", "no-store");
        }
        if method == Method::GET && path == "_/assets/Geist-Variable.ttf" {
            return Response::from_parts(
                status(200),
                Default::default(),
                ADMIN_FONT_BYTES.to_vec(),
            )
            .with_header("Content-Type", "font/ttf")
            .with_header("Cache-Control", "public, max-age=31536000, immutable");
        }
        if method == Method::GET
            && (path == "_/assets/admin-font-alpha-geist-gvar-v2-16.bin"
                || path == "_/assets/admin-font-alpha-geist-gvar-v1-16.bin"
                || path == "_/assets/admin-font-alpha-16.bin"
                || path == "_/assets/admin-font-alpha-multi.bin"
                || path == "_/assets/admin-font-alpha.bin")
        {
            let dpr = query_f32(request.uri().query(), "dpr")
                .unwrap_or(1.0)
                .clamp(1.0, 4.0);
            let atlas = match FontAtlas::load_geist_for_device_scale(16.0, dpr) {
                Ok(atlas) => atlas,
                Err(error) => return error_response(500, &format!("font atlas failed: {error}")),
            };
            return Response::from_parts(status(200), Default::default(), atlas.alpha)
                .with_header("Content-Type", "application/octet-stream")
                .with_header("Cache-Control", "no-store");
        }
        if method == Method::GET && path == "_/assets/tabler-alpha.bin" {
            return Response::from_parts(
                status(200),
                Default::default(),
                TABLER_SVG_ATLAS_ALPHA.to_vec(),
            )
            .with_header("Content-Type", "application/octet-stream")
            .with_header("Cache-Control", "public, max-age=31536000, immutable");
        }
        if method == Method::GET && path == "_/edgerun-ui-core.json" {
            return json_response(200, self.admin_ui_manifest());
        }
        if method == Method::GET && path == "_/admin-scene.json" {
            return match self.admin_ui_scene(request.uri().query()) {
                Ok(scene) => json_response(200, scene),
                Err(error) => error_response(500, &error),
            };
        }
        if method == Method::GET && path == "api/health" {
            return json_response(
                200,
                json!({"code":200,"message":"API is healthy.","data":{}}),
            );
        }
        if method == Method::GET
            && let Some(token) = path.strip_prefix(".well-known/acme-challenge/")
        {
            return self.route_acme_http01_challenge(token);
        }

        match parts.as_slice() {
            ["api", "collections"] => self.route_collections(method, request, actor),
            ["api", "collections", "import"] => {
                self.route_collections_import(method, request, actor)
            }
            ["api", "collections", "meta", "scaffolds"] => {
                self.route_collection_scaffolds(method, actor)
            }
            ["api", "collections", "meta", "oauth2-providers"] => {
                self.route_oauth2_providers(method, actor)
            }
            ["api", "collections", "meta", "dry-run-view"] => {
                self.route_dry_run_view(method, request, actor)
            }
            ["api", "collections", collection] => {
                self.route_collection(method, collection, request, actor)
            }
            ["api", "collections", collection, "truncate"] => {
                self.route_collection_truncate(method, collection, actor)
            }
            ["api", "collections", collection, "records"] => {
                self.route_records(method, collection, None, request, actor)
            }
            ["api", "collections", collection, "records", id] => {
                self.route_records(method, collection, Some(*id), request, actor)
            }
            ["api", "files", collection, record, name] => {
                self.route_file(method, collection, record, name, actor)
            }
            ["api", "files", "token"] => self.route_file_token(method, actor),
            ["api", "collections", collection, "auth-with-password"] => {
                self.route_record_auth(method, collection, request)
            }
            ["api", "collections", collection, "auth-with-oauth2"] => {
                self.route_auth_with_oauth2(method, collection, request)
            }
            ["api", "collections", collection, "auth-refresh"] => {
                self.route_auth_refresh(method, collection, actor)
            }
            ["api", "collections", collection, "auth-methods"] => {
                self.route_auth_methods(method, collection)
            }
            ["api", "collections", collection, "request-otp"] => {
                self.route_request_otp(method, collection, request)
            }
            ["api", "collections", collection, "auth-with-otp"] => {
                self.route_auth_with_otp(method, collection, request)
            }
            ["api", "collections", collection, "request-password-reset"] => {
                self.route_request_auth_token(method, collection, request, "passwordReset")
            }
            ["api", "collections", collection, "confirm-password-reset"] => {
                self.route_confirm_password_reset(method, collection, request)
            }
            ["api", "collections", collection, "request-verification"] => {
                self.route_request_auth_token(method, collection, request, "verification")
            }
            ["api", "collections", collection, "confirm-verification"] => {
                self.route_confirm_verification(method, collection, request)
            }
            ["api", "collections", collection, "request-email-change"] => {
                self.route_request_email_change(method, collection, request, actor)
            }
            ["api", "collections", collection, "confirm-email-change"] => {
                self.route_confirm_email_change(method, collection, request)
            }
            ["api", "collections", collection, "impersonate", id] => {
                self.route_impersonate(method, collection, id, actor)
            }
            ["api", "realtime"] => self.route_realtime(method, request, actor),
            ["api", "admins"] => self.route_admins(method, request, actor),
            ["api", "admins", "auth-with-password"] => self.route_admin_auth(method, request),
            ["api", "backups"] => self.route_backups(method, request, actor),
            ["api", "backups", "upload"] => self.route_backup_upload(method, request, actor),
            ["api", "backups", name] => self.route_backup(method, name, actor),
            ["api", "backups", name, "restore"] => self.route_backup_restore(method, name, actor),
            ["api", "migrations"] => self.route_migrations(method, request, actor),
            ["api", "settings"] => self.route_settings(method, request, actor),
            ["api", "settings", "test", "s3"] => self.route_settings_test(method, "s3", actor),
            ["api", "settings", "test", "email"] => {
                self.route_settings_test(method, "email", actor)
            }
            ["api", "settings", "apple", "generate-client-secret"] => {
                self.route_apple_client_secret(method, actor)
            }
            ["api", "logs"] => self.route_logs(method, request, actor),
            ["api", "logs", "stats"] => self.route_log_stats(method, actor),
            ["api", "logs", id] => self.route_log(method, id, actor),
            ["api", "crons"] => self.route_crons(method, actor),
            ["api", "crons", id] => self.route_cron(method, id, actor),
            ["api", "batch"] => self.route_batch(method, request, actor),
            ["api", "edgerun", "dns", "zones"] => self.route_dns_zones(method, request, actor),
            ["api", "edgerun", "dns", "query"] => self.route_dns_query(method, request, actor),
            ["api", "edgerun", "dhcp", "discover"] => {
                self.route_dhcp_discover(method, request, actor)
            }
            ["api", "edgerun", "dhcp", "request"] => {
                self.route_dhcp_request(method, request, actor)
            }
            ["api", "edgerun", "acme", "http01"] => self.route_acme_http01(method, request, actor),
            ["api", "edgerun", "acme", "http01", token] => {
                self.route_acme_http01_token(method, token, actor)
            }
            ["api", "edgerun", "acme", "dns01"] => self.route_acme_dns01(method, request, actor),
            ["api", "edgerun", "acme", "tls-alpn01"] => {
                self.route_acme_tls_alpn01(method, request, actor)
            }
            ["api", "edgerun", "hooks", "run"] => self.route_hook_run(method, request, actor),
            _ => error_response(404, "route not found"),
        }
    }

    fn route_collections(
        &mut self,
        method: Method,
        request: Request,
        actor: Option<&Actor>,
    ) -> Response {
        match method {
            Method::GET => {
                if let Err(response) = self.require_superuser(actor) {
                    return response;
                }
                let items = self
                    .collections
                    .values()
                    .map(Collection::to_json)
                    .collect::<Vec<_>>();
                json_response(
                    200,
                    json!({"page":1,"perPage":items.len(),"totalItems":items.len(),"totalPages":1,"items":items}),
                )
            }
            Method::POST => {
                if let Err(response) = self.require_superuser(actor) {
                    return response;
                }
                let body = match parse_body(&request) {
                    Ok(body) => body,
                    Err(response) => return response,
                };
                let name = match body.get_str("name") {
                    Some(name) if valid_name(name) => name.to_string(),
                    _ => return error_response(400, "collection name is required"),
                };
                if self.collections.contains_key(&name) {
                    return error_response(400, "collection already exists");
                }
                let now = timestamp();
                let collection = Collection {
                    id: id("col"),
                    name: name.clone(),
                    kind: parse_collection_kind(&body),
                    schema: parse_schema(&body),
                    records: BTreeMap::new(),
                    list_rule: opt_string(&body, "listRule"),
                    view_rule: opt_string(&body, "viewRule"),
                    create_rule: opt_string(&body, "createRule"),
                    update_rule: opt_string(&body, "updateRule"),
                    delete_rule: opt_string(&body, "deleteRule"),
                    created: now.clone(),
                    updated: now,
                };
                let response = collection.to_json();
                self.collections.insert(name.clone(), collection);
                self.persist_collection_delta_or_500(response, 200, &name, false, false)
            }
            _ => error_response(405, "method not allowed"),
        }
    }

    fn route_collection(
        &mut self,
        method: Method,
        name: &str,
        request: Request,
        actor: Option<&Actor>,
    ) -> Response {
        if let Err(response) = self.require_superuser(actor) {
            return response;
        }
        match method {
            Method::GET => match self.collections.get(name) {
                Some(collection) => json_response(200, collection.to_json()),
                None => error_response(404, "collection not found"),
            },
            Method::PATCH => {
                let body = match parse_body(&request) {
                    Ok(body) => body,
                    Err(response) => return response,
                };
                let Some(collection) = self.collections.get_mut(name) else {
                    return error_response(404, "collection not found");
                };
                if let Some(schema) = body.get_array("schema") {
                    collection.schema = fields_from_array(schema);
                }
                patch_rule(&body, "listRule", &mut collection.list_rule);
                patch_rule(&body, "viewRule", &mut collection.view_rule);
                patch_rule(&body, "createRule", &mut collection.create_rule);
                patch_rule(&body, "updateRule", &mut collection.update_rule);
                patch_rule(&body, "deleteRule", &mut collection.delete_rule);
                collection.updated = timestamp();
                let response = collection.to_json();
                self.persist_collection_delta_or_500(response, 200, name, false, false)
            }
            Method::DELETE => {
                if self.collections.remove(name).is_none() {
                    return error_response(404, "collection not found");
                }
                self.persist_collection_delta_or_500(json!({}), 204, name, true, false)
            }
            _ => error_response(405, "method not allowed"),
        }
    }

    fn route_collection_truncate(
        &mut self,
        method: Method,
        name: &str,
        actor: Option<&Actor>,
    ) -> Response {
        if method != Method::DELETE {
            return error_response(405, "method not allowed");
        }
        if let Err(response) = self.require_superuser(actor) {
            return response;
        }
        let Some(collection) = self.collections.get_mut(name) else {
            return error_response(404, "collection not found");
        };
        collection.records.clear();
        collection.updated = timestamp();
        self.persist_collection_delta_or_500(json!({}), 204, name, false, true)
    }

    fn route_collections_import(
        &mut self,
        method: Method,
        request: Request,
        actor: Option<&Actor>,
    ) -> Response {
        if method != Method::PUT {
            return error_response(405, "method not allowed");
        }
        if let Err(response) = self.require_superuser(actor) {
            return response;
        }
        let body = match parse_body(&request) {
            Ok(body) => body,
            Err(response) => return response,
        };
        let Some(items) = body
            .get_array("collections")
            .or_else(|| body.get_array("items"))
        else {
            return error_response(400, "collections array is required");
        };
        let mut imported = Vec::new();
        for item in items {
            let Some(collection) = Collection::from_json(item) else {
                return error_response(400, "invalid collection import entry");
            };
            imported.push(collection.name.clone());
            self.collections.insert(collection.name.clone(), collection);
        }
        match self.persist_imported_collections_delta(&imported) {
            Ok(()) => json_response(200, json!({"imported": imported})),
            Err(error) => error_response(
                500,
                &format!("failed to persist collection import delta: {error}"),
            ),
        }
    }

    fn route_collection_scaffolds(&self, method: Method, actor: Option<&Actor>) -> Response {
        if method != Method::GET {
            return error_response(405, "method not allowed");
        }
        if let Err(response) = self.require_superuser(actor) {
            return response;
        }
        json_response(
            200,
            json!({
                "base": {"type":"base","schema":[]},
                "auth": {"type":"auth","schema":[
                    {"name":"email","type":"email","required":true},
                    {"name":"password","type":"password","required":true},
                    {"name":"verified","type":"bool","required":false}
                ]},
                "view": {"type":"view","schema":[]}
            }),
        )
    }

    fn route_oauth2_providers(&self, method: Method, actor: Option<&Actor>) -> Response {
        if method != Method::GET {
            return error_response(405, "method not allowed");
        }
        if let Err(response) = self.require_superuser(actor) {
            return response;
        }
        json_response(200, json!({"providers": oauth_provider_names()}))
    }

    fn route_dry_run_view(
        &self,
        method: Method,
        request: Request,
        actor: Option<&Actor>,
    ) -> Response {
        if method != Method::POST {
            return error_response(405, "method not allowed");
        }
        if let Err(response) = self.require_superuser(actor) {
            return response;
        }
        let body = match parse_body(&request) {
            Ok(body) => body,
            Err(response) => return response,
        };
        let Some(query) = body.get_str("query") else {
            return error_response(400, "query is required");
        };
        if !query
            .trim_start()
            .to_ascii_lowercase()
            .starts_with("select ")
        {
            return error_response(400, "view query must be a select statement");
        }
        json_response(200, json!({"query": query, "valid": true, "schema": []}))
    }

    fn route_records(
        &mut self,
        method: Method,
        collection_name: &str,
        record_id: Option<&str>,
        request: Request,
        actor: Option<&Actor>,
    ) -> Response {
        match (method, record_id) {
            (Method::GET, None) => {
                let Some(collection) = self.collections.get(collection_name) else {
                    return error_response(404, "collection not found");
                };
                if !rule_allows(collection.list_rule.as_deref(), actor, None) {
                    return error_response(403, "request rejected by collection list rule");
                }
                let mut records = collection
                    .records
                    .values()
                    .filter(|record| record_matches_query(record, request.uri().query()))
                    .cloned()
                    .collect::<Vec<_>>();
                apply_sort(&mut records, request.uri().query());
                let page = query_usize(request.uri().query(), "page")
                    .unwrap_or(1)
                    .max(1);
                let per_page = query_usize(request.uri().query(), "perPage")
                    .unwrap_or(30)
                    .clamp(1, 500);
                let skip_total = query_bool(request.uri().query(), "skipTotal").unwrap_or(false);
                let total = records.len();
                let start = (page - 1).saturating_mul(per_page).min(total);
                let end = start.saturating_add(per_page).min(total);
                let items = records[start..end]
                    .iter()
                    .map(Record::to_json)
                    .map(|value| project_fields(value, request.uri().query()))
                    .collect::<Vec<_>>();
                let total_items = if skip_total { 0 } else { total };
                let total_pages = if skip_total {
                    0
                } else {
                    total.div_ceil(per_page)
                };
                json_response(
                    200,
                    json!({"page":page,"perPage":per_page,"totalItems":total_items,"totalPages":total_pages,"items":items}),
                )
            }
            (Method::POST, None) => {
                let Some(collection) = self.collections.get_mut(collection_name) else {
                    return error_response(404, "collection not found");
                };
                let (body, files) = match parse_record_request(&request) {
                    Ok(parsed) => parsed,
                    Err(response) => return response,
                };
                if !rule_allows(collection.create_rule.as_deref(), actor, Some(&body)) {
                    return error_response(403, "request rejected by collection create rule");
                }
                if let Err(message) = collection.validate_record(&body) {
                    return error_response(400, &message);
                }
                let body = prepare_record_data(body);
                let now = timestamp();
                let record = Record {
                    id: body
                        .get_str("id")
                        .map(str::to_string)
                        .unwrap_or_else(|| id("rec")),
                    data: object_to_record_data(body),
                    created: now.clone(),
                    updated: now,
                };
                let record = match save_uploads(&self.data_dir, collection_name, record, files) {
                    Ok(record) => record,
                    Err(error) => {
                        return error_response(
                            500,
                            &format!("failed to save uploaded file: {error}"),
                        );
                    }
                };
                let response = record.to_json();
                let record_id = record.id.clone();
                collection.records.insert(record.id.clone(), record);
                let event = RealtimeEvent::new("create", collection_name, response.clone());
                let event_id = event.id.clone();
                self.realtime_events.push(event);
                self.persist_record_delta_or_500(
                    response,
                    200,
                    collection_name,
                    Some(record_id),
                    false,
                    Some(event_id),
                )
            }
            (Method::GET, Some(id)) => {
                let Some(collection) = self.collections.get(collection_name) else {
                    return error_response(404, "collection not found");
                };
                match collection.records.get(id) {
                    Some(record)
                        if rule_allows(
                            collection.view_rule.as_deref(),
                            actor,
                            Some(&record.to_store_json()),
                        ) =>
                    {
                        json_response(200, record.to_json())
                    }
                    Some(_) => error_response(403, "request rejected by collection view rule"),
                    None => error_response(404, "record not found"),
                }
            }
            (Method::PATCH, Some(id)) => {
                let Some(collection) = self.collections.get_mut(collection_name) else {
                    return error_response(404, "collection not found");
                };
                let (body, files) = match parse_record_request(&request) {
                    Ok(parsed) => parsed,
                    Err(response) => return response,
                };
                if !rule_allows(collection.update_rule.as_deref(), actor, Some(&body)) {
                    return error_response(403, "request rejected by collection update rule");
                }
                if let Err(message) = collection.validate_record(&body) {
                    return error_response(400, &message);
                }
                let Some(record) = collection.records.get_mut(id) else {
                    return error_response(404, "record not found");
                };
                let body = prepare_record_data(body);
                for (key, value) in body.object_entries().into_iter().flatten() {
                    if !is_system_record_field(key) {
                        record.data.insert(key.to_string(), value.clone());
                    }
                }
                let saved =
                    match save_uploads(&self.data_dir, collection_name, record.clone(), files) {
                        Ok(record) => record,
                        Err(error) => {
                            return error_response(
                                500,
                                &format!("failed to save uploaded file: {error}"),
                            );
                        }
                    };
                *record = saved;
                record.updated = timestamp();
                let response = record.to_json();
                let record_id = record.id.clone();
                let event = RealtimeEvent::new("update", collection_name, response.clone());
                let event_id = event.id.clone();
                self.realtime_events.push(event);
                self.persist_record_delta_or_500(
                    response,
                    200,
                    collection_name,
                    Some(record_id),
                    false,
                    Some(event_id),
                )
            }
            (Method::DELETE, Some(id)) => {
                let Some(collection) = self.collections.get_mut(collection_name) else {
                    return error_response(404, "collection not found");
                };
                let Some(record) = collection.records.get(id) else {
                    return error_response(404, "record not found");
                };
                if !rule_allows(
                    collection.delete_rule.as_deref(),
                    actor,
                    Some(&record.to_store_json()),
                ) {
                    return error_response(403, "request rejected by collection delete rule");
                }
                let Some(removed) = collection.records.remove(id) else {
                    return error_response(404, "record not found");
                };
                let removed_json = removed.to_json();
                let _ = std::fs::remove_dir_all(
                    self.data_dir.join("files").join(collection_name).join(id),
                );
                let event = RealtimeEvent::new("delete", collection_name, removed_json);
                let event_id = event.id.clone();
                self.realtime_events.push(event);
                self.persist_record_delta_or_500(
                    json!({}),
                    204,
                    collection_name,
                    Some(id.to_string()),
                    true,
                    Some(event_id),
                )
            }
            _ => error_response(405, "method not allowed"),
        }
    }

    fn route_record_auth(
        &mut self,
        method: Method,
        collection_name: &str,
        request: Request,
    ) -> Response {
        if method != Method::POST {
            return error_response(405, "method not allowed");
        }
        let body = match parse_body(&request) {
            Ok(body) => body,
            Err(response) => return response,
        };
        let identity = body
            .get_str("identity")
            .or_else(|| body.get_str("email"))
            .unwrap_or_default();
        let password = body.get_str("password").unwrap_or_default();

        let Some(collection) = self.collections.get(collection_name) else {
            return error_response(404, "collection not found");
        };
        if collection.kind != CollectionKind::Auth {
            return error_response(400, "collection is not auth-enabled");
        }

        for record in collection.records.values() {
            let email_ok = record
                .data
                .get("email")
                .and_then(Value::as_str)
                .is_some_and(|email| email == identity);
            let user_ok = record
                .data
                .get("username")
                .and_then(Value::as_str)
                .is_some_and(|username| username == identity);
            let pass_ok = record
                .data
                .get("password")
                .and_then(Value::as_str)
                .is_some_and(|stored| verify_password(password, stored));
            if (email_ok || user_ok) && pass_ok {
                return json_response(
                    200,
                    json!({"token": self.sign_token("record", &record.id), "record": record.to_json()}),
                );
            }
        }
        error_response(400, "invalid identity or password")
    }

    fn route_auth_with_oauth2(
        &mut self,
        method: Method,
        collection_name: &str,
        request: Request,
    ) -> Response {
        if let Err(response) = require_method(&method, Method::POST) {
            return response;
        }
        let body = match parse_body(&request) {
            Ok(body) => body,
            Err(response) => return response,
        };
        let provider = body.get_str("provider").unwrap_or_default();
        let provider_id = body
            .get_str("providerId")
            .or_else(|| body.get_str("provider_id"))
            .or_else(|| body.get_str("id"))
            .unwrap_or_default();
        if provider.is_empty() || provider_id.is_empty() {
            return error_response(400, "provider and providerId are required");
        }
        if !oauth_provider_names()
            .iter()
            .any(|item| item.get_str("name") == Some(provider))
        {
            return error_response(400, "unsupported oauth2 provider");
        }
        let email = body.get_str("email").unwrap_or_default();
        let username = body
            .get_str("username")
            .or_else(|| body.get_str("name"))
            .unwrap_or_default();
        let now = timestamp();
        let Some(collection) = self.collections.get_mut(collection_name) else {
            return error_response(404, "collection not found");
        };
        if collection.kind != CollectionKind::Auth {
            return error_response(400, "collection is not auth-enabled");
        }
        let external = json!({"provider": provider, "providerId": provider_id});
        let mut record_id = collection
            .records
            .values()
            .find(|record| record_has_external_auth(record, provider, provider_id))
            .map(|record| record.id.clone());
        if record_id.is_none() && !email.is_empty() {
            record_id = collection.records.values().find_map(|record| {
                (record.data.get("email").and_then(Value::as_str) == Some(email))
                    .then(|| record.id.clone())
            });
        }
        let record = if let Some(record_id) = record_id {
            let Some(record) = collection.records.get_mut(&record_id) else {
                return error_response(500, "matched oauth2 record disappeared");
            };
            append_external_auth(record, external);
            if !email.is_empty() {
                record
                    .data
                    .insert("email".to_string(), Value::String(email.to_string()));
            }
            if !username.is_empty() {
                record
                    .data
                    .insert("username".to_string(), Value::String(username.to_string()));
            }
            record.updated = now;
            record.clone()
        } else {
            if email.is_empty() {
                return error_response(400, "email is required for new oauth2 auth records");
            }
            let mut data = BTreeMap::new();
            data.insert("email".to_string(), Value::String(email.to_string()));
            data.insert("verified".to_string(), Value::Bool(true));
            data.insert("externalAuths".to_string(), Value::Array(vec![external]));
            if !username.is_empty() {
                data.insert("username".to_string(), Value::String(username.to_string()));
            }
            let record = Record {
                id: id("rec"),
                data,
                created: now.clone(),
                updated: now,
            };
            collection.records.insert(record.id.clone(), record.clone());
            record
        };
        let response =
            json!({"token": self.sign_token("record", &record.id), "record": record.to_json()});
        let record_id = record.id.clone();
        let event = RealtimeEvent::new("auth", collection_name, record.to_json());
        let event_id = event.id.clone();
        self.realtime_events.push(event);
        self.persist_record_delta_or_500(
            response,
            200,
            collection_name,
            Some(record_id),
            false,
            Some(event_id),
        )
    }

    fn route_auth_refresh(
        &self,
        method: Method,
        collection_name: &str,
        actor: Option<&Actor>,
    ) -> Response {
        if method != Method::POST {
            return error_response(405, "method not allowed");
        }
        let Some(actor) = actor else {
            return error_response(401, "auth token is required");
        };
        if actor.kind != "record" {
            return error_response(403, "record auth token is required");
        }
        let Some(collection) = self.collections.get(collection_name) else {
            return error_response(404, "collection not found");
        };
        let Some(record) = collection.records.get(&actor.id) else {
            return error_response(404, "auth record not found");
        };
        json_response(
            200,
            json!({"token": self.sign_token("record", &record.id), "record": record.to_json()}),
        )
    }

    fn route_auth_methods(&self, method: Method, collection_name: &str) -> Response {
        if method != Method::GET {
            return error_response(405, "method not allowed");
        }
        let Some(collection) = self.collections.get(collection_name) else {
            return error_response(404, "collection not found");
        };
        if collection.kind != CollectionKind::Auth {
            return error_response(400, "collection is not auth-enabled");
        }
        json_response(
            200,
            json!({
                "password": {"enabled": true, "identityFields": ["email", "username"]},
                "oauth2": {"enabled": true, "providers": oauth_provider_names(), "pkce": pkce_metadata()},
                "otp": {"enabled": true, "duration": 300},
                "mfa": {"enabled": false}
            }),
        )
    }

    fn route_request_otp(
        &mut self,
        method: Method,
        collection_name: &str,
        request: Request,
    ) -> Response {
        if let Err(response) = require_method(&method, Method::POST) {
            return response;
        }
        let body = match parse_body(&request) {
            Ok(body) => body,
            Err(response) => return response,
        };
        let identity = body
            .get_str("identity")
            .or_else(|| body.get_str("email"))
            .unwrap_or_default();
        let Some((record_id, record)) =
            self.find_auth_record_by_identity(collection_name, identity)
        else {
            return error_response(404, "auth record not found");
        };
        let code = otp_code();
        let token = self.issue_flow_token_with_payload(
            collection_name,
            &record_id,
            "otp",
            record
                .data
                .get("email")
                .and_then(Value::as_str)
                .map(str::to_string),
            Some(code.clone()),
            300,
        );
        let mail = build_system_email(
            &self.settings,
            record
                .data
                .get("email")
                .and_then(Value::as_str)
                .unwrap_or(identity),
            "Your login code",
            &format!("Your login code is {code}. It expires in 5 minutes."),
        );
        let sent = record
            .data
            .get("email")
            .and_then(Value::as_str)
            .map(|email| self.maybe_send_auth_flow_email(email, "otp", &code))
            .unwrap_or(false);
        json_response(
            200,
            json!({"otpId": token, "code": code, "email": record.data.get("email").and_then(Value::as_str).unwrap_or_default(), "message": String::from_utf8_lossy(&mail).to_string(), "emailSent": sent}),
        )
    }

    fn route_auth_with_otp(
        &mut self,
        method: Method,
        collection_name: &str,
        request: Request,
    ) -> Response {
        if let Err(response) = require_method(&method, Method::POST) {
            return response;
        }
        let body = match parse_body(&request) {
            Ok(body) => body,
            Err(response) => return response,
        };
        let token = body
            .get_str("otpId")
            .or_else(|| body.get_str("otp_id"))
            .or_else(|| body.get_str("token"))
            .unwrap_or_default();
        let password = body
            .get_str("password")
            .or_else(|| body.get_str("code"))
            .unwrap_or_default();
        let Some(flow) = self.take_flow_token(collection_name, token, "otp") else {
            return error_response(400, "invalid or expired otp token");
        };
        if !constant_time_eq(
            flow.payload.unwrap_or_default().as_bytes(),
            password.as_bytes(),
        ) {
            return error_response(400, "invalid otp code");
        }
        let Some(collection) = self.collections.get(collection_name) else {
            return error_response(404, "collection not found");
        };
        let Some(record) = collection.records.get(&flow.record_id) else {
            return error_response(404, "auth record not found");
        };
        json_response(
            200,
            json!({"token": self.sign_token("record", &record.id), "record": record.to_json()}),
        )
    }

    fn route_request_auth_token(
        &mut self,
        method: Method,
        collection_name: &str,
        request: Request,
        kind: &str,
    ) -> Response {
        if let Err(response) = require_method(&method, Method::POST) {
            return response;
        }
        let body = match parse_body(&request) {
            Ok(body) => body,
            Err(response) => return response,
        };
        let email = body
            .get_str("email")
            .or_else(|| body.get_str("identity"))
            .unwrap_or_default();
        let Some((record_id, _)) = self.find_auth_record_by_identity(collection_name, email) else {
            return error_response(404, "auth record not found");
        };
        let token = self.issue_flow_token(collection_name, &record_id, kind, None);
        let sent = self.maybe_send_auth_flow_email(email, kind, &token);
        json_response(200, json!({"token": token, "emailSent": sent}))
    }

    fn route_confirm_password_reset(
        &mut self,
        method: Method,
        collection_name: &str,
        request: Request,
    ) -> Response {
        if let Err(response) = require_method(&method, Method::POST) {
            return response;
        }
        let body = match parse_body(&request) {
            Ok(body) => body,
            Err(response) => return response,
        };
        let token = body.get_str("token").unwrap_or_default();
        let password = body
            .get_str("password")
            .or_else(|| body.get_str("newPassword"))
            .unwrap_or_default();
        if password.is_empty() {
            return error_response(400, "new password is required");
        }
        let Some(flow) = self.take_flow_token(collection_name, token, "passwordReset") else {
            return error_response(400, "invalid or expired password reset token");
        };
        let Some(collection) = self.collections.get_mut(collection_name) else {
            return error_response(404, "collection not found");
        };
        let Some(record) = collection.records.get_mut(&flow.record_id) else {
            return error_response(404, "auth record not found");
        };
        record.data.insert(
            "password".to_string(),
            Value::String(hash_password(password)),
        );
        record.updated = timestamp();
        let response = record.to_json();
        self.persist_record_delta_or_500(
            response,
            200,
            collection_name,
            Some(flow.record_id),
            false,
            None,
        )
    }

    fn route_confirm_verification(
        &mut self,
        method: Method,
        collection_name: &str,
        request: Request,
    ) -> Response {
        if let Err(response) = require_method(&method, Method::POST) {
            return response;
        }
        let body = match parse_body(&request) {
            Ok(body) => body,
            Err(response) => return response,
        };
        let token = body.get_str("token").unwrap_or_default();
        let Some(flow) = self.take_flow_token(collection_name, token, "verification") else {
            return error_response(400, "invalid or expired verification token");
        };
        let Some(collection) = self.collections.get_mut(collection_name) else {
            return error_response(404, "collection not found");
        };
        let Some(record) = collection.records.get_mut(&flow.record_id) else {
            return error_response(404, "auth record not found");
        };
        record
            .data
            .insert("verified".to_string(), Value::Bool(true));
        record.updated = timestamp();
        let response = record.to_json();
        self.persist_record_delta_or_500(
            response,
            200,
            collection_name,
            Some(flow.record_id),
            false,
            None,
        )
    }

    fn route_request_email_change(
        &mut self,
        method: Method,
        collection_name: &str,
        request: Request,
        actor: Option<&Actor>,
    ) -> Response {
        if let Err(response) = require_method(&method, Method::POST) {
            return response;
        }
        let Some(actor) = actor else {
            return error_response(401, "auth token is required");
        };
        if actor.kind != "record" {
            return error_response(403, "record auth token is required");
        }
        let body = match parse_body(&request) {
            Ok(body) => body,
            Err(response) => return response,
        };
        let Some(new_email) = body.get_str("newEmail").or_else(|| body.get_str("email")) else {
            return error_response(400, "new email is required");
        };
        if !is_valid_email(new_email) {
            return error_response(400, "new email must be valid");
        }
        let Some(collection) = self.collections.get(collection_name) else {
            return error_response(404, "collection not found");
        };
        if !collection.records.contains_key(&actor.id) {
            return error_response(404, "auth record not found");
        }
        let token = self.issue_flow_token(
            collection_name,
            &actor.id,
            "emailChange",
            Some(new_email.to_string()),
        );
        json_response(200, json!({"token": token}))
    }

    fn route_confirm_email_change(
        &mut self,
        method: Method,
        collection_name: &str,
        request: Request,
    ) -> Response {
        if let Err(response) = require_method(&method, Method::POST) {
            return response;
        }
        let body = match parse_body(&request) {
            Ok(body) => body,
            Err(response) => return response,
        };
        let token = body.get_str("token").unwrap_or_default();
        let Some(flow) = self.take_flow_token(collection_name, token, "emailChange") else {
            return error_response(400, "invalid or expired email change token");
        };
        let Some(new_email) = flow.email else {
            return error_response(400, "email change token has no email");
        };
        let Some(collection) = self.collections.get_mut(collection_name) else {
            return error_response(404, "collection not found");
        };
        let Some(record) = collection.records.get_mut(&flow.record_id) else {
            return error_response(404, "auth record not found");
        };
        record
            .data
            .insert("email".to_string(), Value::String(new_email));
        record
            .data
            .insert("verified".to_string(), Value::Bool(false));
        record.updated = timestamp();
        let response = record.to_json();
        self.persist_record_delta_or_500(
            response,
            200,
            collection_name,
            Some(flow.record_id),
            false,
            None,
        )
    }

    fn route_impersonate(
        &self,
        method: Method,
        collection_name: &str,
        record_id: &str,
        actor: Option<&Actor>,
    ) -> Response {
        if let Err(response) = require_method(&method, Method::POST) {
            return response;
        }
        if let Err(response) = self.require_superuser(actor) {
            return response;
        }
        let Some(collection) = self.collections.get(collection_name) else {
            return error_response(404, "collection not found");
        };
        if collection.kind != CollectionKind::Auth {
            return error_response(400, "collection is not auth-enabled");
        }
        let Some(record) = collection.records.get(record_id) else {
            return error_response(404, "auth record not found");
        };
        json_response(
            200,
            json!({"token": self.sign_token("record", &record.id), "record": record.to_json()}),
        )
    }

    fn route_admins(
        &mut self,
        method: Method,
        request: Request,
        actor: Option<&Actor>,
    ) -> Response {
        match method {
            Method::GET => {
                if let Err(response) = self.require_superuser(actor) {
                    return response;
                }
                let items = self.admins.values().map(Admin::to_json).collect::<Vec<_>>();
                json_response(
                    200,
                    json!({"page":1,"perPage":items.len(),"totalItems":items.len(),"totalPages":1,"items":items}),
                )
            }
            Method::POST => {
                if !self.admins.is_empty() && !self.is_superuser(actor) {
                    return error_response(403, "superuser authorization is required");
                }
                let body = match parse_body(&request) {
                    Ok(body) => body,
                    Err(response) => return response,
                };
                let Some(email) = body.get_str("email") else {
                    return error_response(400, "admin email is required");
                };
                let Some(password) = body.get_str("password") else {
                    return error_response(400, "admin password is required");
                };
                if self.admins.values().any(|admin| admin.email == email) {
                    return error_response(400, "admin already exists");
                }
                let now = timestamp();
                let admin = Admin {
                    id: id("adm"),
                    email: email.to_string(),
                    password: hash_password(password),
                    created: now.clone(),
                    updated: now,
                };
                let response = admin.to_json();
                let admin_id = admin.id.clone();
                self.admins.insert(admin_id.clone(), admin);
                self.persist_admin_or_500(response, 200, &admin_id, false)
            }
            _ => error_response(405, "method not allowed"),
        }
    }

    fn route_admin_auth(&mut self, method: Method, request: Request) -> Response {
        if method != Method::POST {
            return error_response(405, "method not allowed");
        }
        let body = match parse_body(&request) {
            Ok(body) => body,
            Err(response) => return response,
        };
        let identity = body
            .get_str("identity")
            .or_else(|| body.get_str("email"))
            .unwrap_or_default();
        let password = body.get_str("password").unwrap_or_default();
        for admin in self.admins.values() {
            if admin.email == identity && verify_password(password, &admin.password) {
                return json_response(
                    200,
                    json!({"token": self.sign_token("admin", &admin.id), "admin": admin.to_json()}),
                );
            }
        }
        error_response(400, "invalid identity or password")
    }

    fn route_file(
        &mut self,
        method: Method,
        collection_name: &str,
        record_id: &str,
        name: &str,
        actor: Option<&Actor>,
    ) -> Response {
        if method != Method::GET {
            return error_response(405, "method not allowed");
        }
        let Some(collection) = self.collections.get(collection_name) else {
            return error_response(404, "collection not found");
        };
        let Some(record) = collection.records.get(record_id) else {
            return error_response(404, "record not found");
        };
        if !rule_allows(
            collection.view_rule.as_deref(),
            actor,
            Some(&record.to_store_json()),
        ) {
            return error_response(403, "request rejected by collection view rule");
        }
        let file_dir = self
            .data_dir
            .join("files")
            .join(collection_name)
            .join(record_id);
        let path = file_dir.join(safe_file_name(name));
        match std::fs::read(&path) {
            Ok(bytes) => Response::from_parts(status(200), Default::default(), bytes),
            Err(_) => error_response(404, "file not found"),
        }
    }

    fn route_file_token(&self, method: Method, actor: Option<&Actor>) -> Response {
        if method != Method::POST {
            return error_response(405, "method not allowed");
        }
        let Some(actor) = actor else {
            return error_response(401, "auth token is required");
        };
        json_response(200, json!({"token": self.sign_token("file", &actor.id)}))
    }

    fn route_backups(
        &mut self,
        method: Method,
        _request: Request,
        actor: Option<&Actor>,
    ) -> Response {
        if let Err(response) = self.require_superuser(actor) {
            return response;
        }
        match method {
            Method::GET => {
                let items = match self.list_backups() {
                    Ok(items) => items,
                    Err(error) => {
                        return error_response(500, &format!("failed to list backups: {error}"));
                    }
                };
                json_response(200, json!({"items": items}))
            }
            Method::POST => {
                let name = format!("backup-{}.json", timestamp());
                match self.create_backup(&name) {
                    Ok(()) => json_response(200, json!({"name": name})),
                    Err(error) => error_response(500, &format!("failed to create backup: {error}")),
                }
            }
            _ => error_response(405, "method not allowed"),
        }
    }

    fn route_backup(&mut self, method: Method, name: &str, actor: Option<&Actor>) -> Response {
        if let Err(response) = self.require_superuser(actor) {
            return response;
        }
        let name = safe_file_name(name);
        let path = self.data_dir.join("backups").join(&name);
        match method {
            Method::GET => match std::fs::read(&path) {
                Ok(bytes) => Response::from_parts(status(200), Default::default(), bytes),
                Err(_) => error_response(404, "backup not found"),
            },
            Method::DELETE => match std::fs::remove_file(&path) {
                Ok(()) => json_response(204, json!({})),
                Err(_) => error_response(404, "backup not found"),
            },
            _ => error_response(405, "method not allowed"),
        }
    }

    fn route_backup_upload(
        &mut self,
        method: Method,
        request: Request,
        actor: Option<&Actor>,
    ) -> Response {
        if method != Method::POST {
            return error_response(405, "method not allowed");
        }
        if let Err(response) = self.require_superuser(actor) {
            return response;
        }
        let body = match parse_body(&request) {
            Ok(body) => body,
            Err(response) => return response,
        };
        let name = body
            .get_str("name")
            .map(safe_file_name)
            .unwrap_or_else(|| format!("upload-{}.json", timestamp()));
        let data = body.get("data").cloned().unwrap_or_else(|| self.to_json());
        let dir = self.data_dir.join("backups");
        let upload_result = std::fs::create_dir_all(&dir).and_then(|_| {
            let encoded = to_string_pretty(&data).map_err(invalid_data)?;
            std::fs::write(dir.join(&name), encoded)
        });
        if let Err(error) = upload_result {
            return error_response(500, &format!("failed to upload backup: {error}"));
        }
        json_response(200, json!({"name": name}))
    }

    fn route_backup_restore(
        &mut self,
        method: Method,
        name: &str,
        actor: Option<&Actor>,
    ) -> Response {
        if method != Method::POST {
            return error_response(405, "method not allowed");
        }
        if let Err(response) = self.require_superuser(actor) {
            return response;
        }
        let path = self.data_dir.join("backups").join(safe_file_name(name));
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(_) => return error_response(404, "backup not found"),
        };
        let value = match from_slice(&bytes) {
            Ok(value) => value,
            Err(error) => return error_response(400, &format!("invalid backup JSON: {error}")),
        };
        let restored = Self::from_json(self.data_dir.clone(), value);
        *self = restored;
        self.persist_or_500(json!({}), 200)
    }

    fn route_migrations(
        &mut self,
        method: Method,
        request: Request,
        actor: Option<&Actor>,
    ) -> Response {
        if let Err(response) = self.require_superuser(actor) {
            return response;
        }
        match method {
            Method::GET => {
                let items = self
                    .migrations
                    .iter()
                    .map(Migration::to_json)
                    .collect::<Vec<_>>();
                json_response(200, json!({"items": items}))
            }
            Method::POST => {
                let body = match parse_body(&request) {
                    Ok(body) => body,
                    Err(response) => return response,
                };
                let Some(name) = body.get_str("name") else {
                    return error_response(400, "migration name is required");
                };
                let migration = Migration {
                    id: id("mig"),
                    name: name.to_string(),
                    applied: timestamp(),
                };
                let response = migration.to_json();
                let migration_id = migration.id.clone();
                self.migrations.push(migration);
                self.persist_migration_delta_or_500(response, 200, &migration_id)
            }
            _ => error_response(405, "method not allowed"),
        }
    }

    fn route_settings(
        &mut self,
        method: Method,
        request: Request,
        actor: Option<&Actor>,
    ) -> Response {
        if let Err(response) = self.require_superuser(actor) {
            return response;
        }
        match method {
            Method::GET => json_response(200, self.settings.clone()),
            Method::PATCH => {
                let body = match parse_body(&request) {
                    Ok(body) => body,
                    Err(response) => return response,
                };
                merge_json(&mut self.settings, &body);
                self.persist_settings_or_500(self.settings.clone(), 200)
            }
            _ => error_response(405, "method not allowed"),
        }
    }

    fn route_settings_test(&self, method: Method, name: &str, actor: Option<&Actor>) -> Response {
        if method != Method::POST {
            return error_response(405, "method not allowed");
        }
        if let Err(response) = self.require_superuser(actor) {
            return response;
        }
        match name {
            "s3" => {
                if self
                    .settings
                    .get_object("s3")
                    .and_then(|s3| s3.get("enabled"))
                    .and_then(Value::as_bool)
                    == Some(true)
                {
                    error_response(
                        501,
                        "s3 connectivity test is not implemented without an S3 storage backend",
                    )
                } else {
                    error_response(400, "s3 storage is not enabled")
                }
            }
            "email" => {
                if self
                    .settings
                    .get_object("smtp")
                    .and_then(|smtp| smtp.get("enabled"))
                    .and_then(Value::as_bool)
                    == Some(true)
                {
                    match rt::block_on(self.send_system_email(
                        "test@example.invalid",
                        "EdgeRun PocketBase test email",
                        "SMTP connectivity test from EdgeRun PocketBase.",
                    )) {
                        Ok(()) => json_response(200, json!({"sent": true})),
                        Err(error) => error_response(502, &format!("email test failed: {error}")),
                    }
                } else {
                    error_response(400, "smtp mailer is not enabled")
                }
            }
            _ => error_response(404, "settings test target not found"),
        }
    }

    fn route_apple_client_secret(&self, method: Method, actor: Option<&Actor>) -> Response {
        if method != Method::POST {
            return error_response(405, "method not allowed");
        }
        if let Err(response) = self.require_superuser(actor) {
            return response;
        }
        error_response(
            400,
            "apple client secret generation requires configured key material",
        )
    }

    fn route_logs(&self, method: Method, request: Request, actor: Option<&Actor>) -> Response {
        if method != Method::GET {
            return error_response(405, "method not allowed");
        }
        if let Err(response) = self.require_superuser(actor) {
            return response;
        }
        let page = query_usize(request.uri().query(), "page")
            .unwrap_or(1)
            .max(1);
        let per_page = query_usize(request.uri().query(), "perPage")
            .unwrap_or(30)
            .clamp(1, 500);
        let total = self.logs.len();
        let start = (page - 1).saturating_mul(per_page).min(total);
        let end = start.saturating_add(per_page).min(total);
        let items = self.logs[start..end]
            .iter()
            .map(ActivityLog::to_json)
            .collect::<Vec<_>>();
        json_response(
            200,
            json!({"page":page,"perPage":per_page,"totalItems":total,"totalPages":total.div_ceil(per_page),"items":items}),
        )
    }

    fn route_log_stats(&self, method: Method, actor: Option<&Actor>) -> Response {
        if method != Method::GET {
            return error_response(405, "method not allowed");
        }
        if let Err(response) = self.require_superuser(actor) {
            return response;
        }
        let mut total = 0usize;
        let mut errors = 0usize;
        for log in &self.logs {
            total += 1;
            if log.status >= 400 {
                errors += 1;
            }
        }
        json_response(200, json!({"total": total, "errors": errors}))
    }

    fn route_log(&self, method: Method, id: &str, actor: Option<&Actor>) -> Response {
        if method != Method::GET {
            return error_response(405, "method not allowed");
        }
        if let Err(response) = self.require_superuser(actor) {
            return response;
        }
        match self.logs.iter().find(|log| log.id == id) {
            Some(log) => json_response(200, log.to_json()),
            None => error_response(404, "log not found"),
        }
    }

    fn route_crons(&self, method: Method, actor: Option<&Actor>) -> Response {
        if method != Method::GET {
            return error_response(405, "method not allowed");
        }
        if let Err(response) = self.require_superuser(actor) {
            return response;
        }
        let items = self.crons.iter().map(CronJob::to_json).collect::<Vec<_>>();
        json_response(200, json!({"items": items}))
    }

    fn route_cron(&mut self, method: Method, id: &str, actor: Option<&Actor>) -> Response {
        if method != Method::POST {
            return error_response(405, "method not allowed");
        }
        if let Err(response) = self.require_superuser(actor) {
            return response;
        }
        let Some(index) = self.crons.iter().position(|cron| cron.id == id) else {
            return error_response(404, "cron not found");
        };
        self.crons[index].last_run = Some(timestamp());
        let response = self.crons[index].to_json();
        self.persist_cron_delta_or_500(response, 200, id)
    }

    fn route_batch(&mut self, method: Method, request: Request, actor: Option<&Actor>) -> Response {
        if method != Method::POST {
            return error_response(405, "method not allowed");
        }
        let (staged, responses) = match with_body_tape(&request, |body| {
            let requests = body
                .get_array("requests")
                .ok_or_else(|| error_response(400, "requests array is required"))?;
            let mut staged = self.clone();
            let mut responses = Vec::new();
            for item in requests {
                let method = match item.get_str("method").unwrap_or("GET") {
                    "GET" => Method::GET,
                    "POST" => Method::POST,
                    "PATCH" => Method::PATCH,
                    "PUT" => Method::PUT,
                    "DELETE" => Method::DELETE,
                    other => Method::Extension(other.to_string()),
                };
                let Some(url) = item.get_str("url").or_else(|| item.get_str("path")) else {
                    return Err(error_response(400, "batch request url is required"));
                };
                let body = item.get("body").and_then(|value| value.to_json_value());
                let body = body
                    .filter(|value| *value != Value::Null)
                    .and_then(|value| edgerun_json::to_string(&value).ok());
                let req = match build_internal_request(method.clone(), url, body.as_deref(), actor)
                {
                    Ok(req) => req,
                    Err(message) => return Err(error_response(400, &message)),
                };
                let path = req.uri().path().trim_matches('/').to_string();
                let response = staged.dispatch(method, path, req, actor);
                let status = response.status().as_u16();
                responses.push(json!({"status": status, "body": from_slice(response.body()).unwrap_or(Value::Null)}));
                if status >= 400 {
                    return Err(json_response(400, json!({"responses": responses})));
                }
            }
            Ok((staged, responses))
        }) {
            Ok(result) => result,
            Err(response) => return response,
        };
        *self = staged;
        match self.save() {
            Ok(()) => json_response(200, json!({"responses": responses})),
            Err(error) => error_response(500, &format!("failed to persist batch: {error}")),
        }
    }

    fn route_realtime(
        &mut self,
        method: Method,
        request: Request,
        actor: Option<&Actor>,
    ) -> Response {
        match method {
            Method::GET => {
                let client_id = id("rt");
                self.realtime_clients.insert(
                    client_id.clone(),
                    RealtimeClient {
                        id: client_id.clone(),
                        subscriptions: Vec::new(),
                        created: timestamp(),
                    },
                );
                let body = format!(
                    "event: PB_CONNECT\ndata: {{\"clientId\":\"{}\"}}\n\n",
                    json_escape(&client_id)
                );
                Response::from_parts(status(200), Default::default(), body.into_bytes())
                    .with_header("Content-Type", "text/event-stream")
                    .with_header("Cache-Control", "no-cache")
            }
            Method::POST => {
                let body = match parse_body(&request) {
                    Ok(body) => body,
                    Err(response) => return response,
                };
                let Some(client_id) = body
                    .get_str("clientId")
                    .or_else(|| body.get_str("client_id"))
                else {
                    return error_response(400, "clientId is required");
                };
                let subscriptions = body
                    .get_array("subscriptions")
                    .or_else(|| body.get_array("subs"))
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(|item| item.as_str().map(str::to_string))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                for subscription in &subscriptions {
                    if let Some(collection_name) = subscription.split('/').next() {
                        if let Some(collection) = self.collections.get(collection_name) {
                            if !rule_allows(collection.list_rule.as_deref(), actor, None) {
                                return error_response(
                                    403,
                                    "request rejected by realtime subscription rule",
                                );
                            }
                        }
                    }
                }
                let client = self
                    .realtime_clients
                    .entry(client_id.to_string())
                    .or_insert_with(|| RealtimeClient {
                        id: client_id.to_string(),
                        subscriptions: Vec::new(),
                        created: timestamp(),
                    });
                client.subscriptions = subscriptions.clone();
                let mut response = client.to_json();
                let events = self.realtime_events_for_subscriptions(&subscriptions);
                if let Some(object) = response.as_object_mut() {
                    object.push_field("events", Value::Array(events));
                }
                json_response(200, response)
            }
            _ => error_response(405, "method not allowed"),
        }
    }

    fn realtime_events_for_subscriptions(&self, subscriptions: &[String]) -> Vec<Value> {
        self.realtime_events
            .iter()
            .filter(|event| {
                subscriptions
                    .iter()
                    .any(|subscription| realtime_subscription_matches(subscription, event))
            })
            .map(RealtimeEvent::to_json)
            .collect()
    }

    fn route_dns_zones(
        &mut self,
        method: Method,
        request: Request,
        actor: Option<&Actor>,
    ) -> Response {
        match method {
            Method::GET => {
                if let Err(response) = self.require_superuser(actor) {
                    return response;
                }
                json_response(200, json!({"items": dns_zone_values(&self.settings)}))
            }
            Method::POST | Method::PUT => {
                if let Err(response) = self.require_superuser(actor) {
                    return response;
                }
                let body = match parse_body(&request) {
                    Ok(body) => body,
                    Err(response) => return response,
                };
                let zones = if let Some(items) = body.get_array("zones") {
                    Value::Array(items.to_vec())
                } else {
                    Value::Array(vec![body])
                };
                if let Err(error) = validate_dns_zones_value(&zones) {
                    return error_response(400, &error);
                }
                if let Err(error) =
                    set_nested_setting(&mut self.settings, &["edgerun", "dns", "zones"], zones)
                {
                    return error_response(500, &error);
                }
                self.persist_settings_or_500(json!({"items": dns_zone_values(&self.settings)}), 200)
            }
            _ => error_response(405, "method not allowed"),
        }
    }

    fn route_dns_query(&self, method: Method, request: Request, actor: Option<&Actor>) -> Response {
        if let Err(response) = require_method(&method, Method::POST) {
            return response;
        }
        if let Err(response) = self.require_superuser(actor) {
            return response;
        }
        let body = match parse_body(&request) {
            Ok(body) => body,
            Err(response) => return response,
        };
        let name = body.get_str("name").unwrap_or_default();
        let qtype = body.get_str("type").unwrap_or("A");
        let Some(qtype) = dns_record_type(qtype) else {
            return error_response(400, "unsupported DNS record type");
        };
        let zones = dns_zones_from_settings(&self.settings);
        let records = resolve(name, qtype, &zones);
        let query = DnsMessage::query(0x4544, name.to_string(), qtype);
        let query_wire = query.to_wire();
        let response_wire = handle_query_without_forwarding(&query_wire, &zones)
            .map(|(wire, _)| wire)
            .unwrap_or_default();
        json_response(
            200,
            json!({
                "name": name,
                "type": qtype.as_str(),
                "records": records.iter().map(dns_record_to_json).collect::<Vec<_>>(),
                "queryWireHex": bytes_to_hex(&query_wire),
                "responseWireHex": bytes_to_hex(&response_wire)
            }),
        )
    }

    fn route_dhcp_discover(
        &self,
        method: Method,
        request: Request,
        actor: Option<&Actor>,
    ) -> Response {
        if let Err(response) = require_method(&method, Method::POST) {
            return response;
        }
        if let Err(response) = self.require_superuser(actor) {
            return response;
        }
        let (mac, xid) = match with_body_tape(&request, |body| {
            let mac = parse_mac(body.get_str("mac").unwrap_or("02:00:00:00:00:01"))
                .ok_or_else(|| error_response(400, "mac must be six hex octets"))?;
            Ok((mac, body.get_u64("xid").unwrap_or(0x4544_0001) as u32))
        }) {
            Ok(parsed) => parsed,
            Err(response) => return response,
        };
        let mut core = match dhcp_core_from_settings(&self.settings) {
            Ok(core) => core,
            Err(error) => return error_response(400, &error),
        };
        let discover = DhcpMessage::discover(xid, mac);
        dhcp_exchange_response(&mut core, discover)
    }

    fn route_dhcp_request(
        &self,
        method: Method,
        request: Request,
        actor: Option<&Actor>,
    ) -> Response {
        if let Err(response) = require_method(&method, Method::POST) {
            return response;
        }
        if let Err(response) = self.require_superuser(actor) {
            return response;
        }
        let (mac, xid, requested_ip, server_id) = match with_body_tape(&request, |body| {
            let mac = parse_mac(body.get_str("mac").unwrap_or("02:00:00:00:00:01"))
                .ok_or_else(|| error_response(400, "mac must be six hex octets"))?;
            let requested_ip = tape_str_any(body, &["requestedIp", "requested_ip"])
                .and_then(parse_ipv4)
                .ok_or_else(|| error_response(400, "requestedIp is required"))?;
            let server_id = tape_str_any(body, &["serverId", "server_id"])
                .and_then(parse_ipv4)
                .or_else(|| dhcp_config_value(&self.settings, "serverIp").and_then(parse_ipv4))
                .ok_or_else(|| error_response(400, "serverId is required"))?;
            Ok((
                mac,
                body.get_u64("xid").unwrap_or(0x4544_0002) as u32,
                requested_ip,
                server_id,
            ))
        }) {
            Ok(parsed) => parsed,
            Err(response) => return response,
        };
        let mut core = match dhcp_core_from_settings(&self.settings) {
            Ok(core) => core,
            Err(error) => return error_response(400, &error),
        };
        let discover = DhcpMessage::discover(xid, mac);
        let _ = core.handle_message(&discover);
        let request = DhcpMessage::request(xid, mac, requested_ip, server_id);
        dhcp_exchange_response(&mut core, request)
    }

    fn route_acme_http01_challenge(&self, token: &str) -> Response {
        match acme_http01_values(&self.settings)
            .into_iter()
            .find(|challenge| challenge.get_str("token") == Some(token))
            .and_then(|challenge| {
                challenge
                    .get_str("responseBody")
                    .or_else(|| challenge.get_str("keyAuthorization"))
                    .map(str::to_string)
            }) {
            Some(body) => Response::text(status(200), &body)
                .with_header("Content-Type", "text/plain")
                .with_header("Cache-Control", "no-store"),
            None => error_response(404, "ACME challenge not found"),
        }
    }

    fn route_acme_http01(
        &mut self,
        method: Method,
        request: Request,
        actor: Option<&Actor>,
    ) -> Response {
        if let Err(response) = self.require_superuser(actor) {
            return response;
        }
        match method {
            Method::GET => json_response(200, json!({"items": acme_http01_values(&self.settings)})),
            Method::POST | Method::PUT => {
                let (domain, token, thumbprint) = match acme_challenge_fields(&request) {
                    Ok(fields) => fields,
                    Err(response) => return response,
                };
                let plan =
                    NodeAcmeOrchestrator::default().http01_plan(&domain, &token, &thumbprint);
                let challenge = json!({
                    "domain": plan.domain,
                    "token": token.clone(),
                    "path": plan.route_path,
                    "responseBody": plan.response_body,
                    "contentType": plan.content_type,
                    "created": timestamp()
                });
                let mut items = acme_http01_values(&self.settings)
                    .into_iter()
                    .filter(|item| item.get_str("token") != Some(token.as_str()))
                    .collect::<Vec<_>>();
                items.push(challenge.clone());
                if let Err(error) = set_nested_setting(
                    &mut self.settings,
                    &["edgerun", "acme", "http01"],
                    Value::Array(items),
                ) {
                    return error_response(500, &error);
                }
                self.persist_settings_or_500(challenge, 200)
            }
            _ => error_response(405, "method not allowed"),
        }
    }

    fn route_acme_http01_token(
        &mut self,
        method: Method,
        token: &str,
        actor: Option<&Actor>,
    ) -> Response {
        if let Err(response) = self.require_superuser(actor) {
            return response;
        }
        match method {
            Method::GET => self.route_acme_http01_challenge(token),
            Method::DELETE => {
                let items = acme_http01_values(&self.settings)
                    .into_iter()
                    .filter(|item| item.get_str("token") != Some(token))
                    .collect::<Vec<_>>();
                if let Err(error) = set_nested_setting(
                    &mut self.settings,
                    &["edgerun", "acme", "http01"],
                    Value::Array(items),
                ) {
                    return error_response(500, &error);
                }
                self.persist_settings_or_500(Value::Null, 204)
            }
            _ => error_response(405, "method not allowed"),
        }
    }

    fn route_acme_dns01(
        &self,
        method: Method,
        request: Request,
        actor: Option<&Actor>,
    ) -> Response {
        if let Err(response) = require_method(&method, Method::POST) {
            return response;
        }
        if let Err(response) = self.require_superuser(actor) {
            return response;
        }
        let (domain, token, thumbprint) = match acme_challenge_fields(&request) {
            Ok(fields) => fields,
            Err(response) => return response,
        };
        let plan = NodeAcmeOrchestrator::default().dns01_plan(&domain, &token, &thumbprint);
        json_response(
            200,
            json!({
                "domain": plan.domain,
                "recordName": plan.record_name,
                "recordType": "TXT",
                "recordValue": plan.record_value,
                "ttl": plan.ttl_secs
            }),
        )
    }

    fn route_acme_tls_alpn01(
        &self,
        method: Method,
        request: Request,
        actor: Option<&Actor>,
    ) -> Response {
        if let Err(response) = require_method(&method, Method::POST) {
            return response;
        }
        if let Err(response) = self.require_superuser(actor) {
            return response;
        }
        let (domain, token, thumbprint) = match acme_challenge_fields(&request) {
            Ok(fields) => fields,
            Err(response) => return response,
        };
        let plan = NodeAcmeOrchestrator::default().tls_alpn01_plan(&domain, &token, &thumbprint);
        json_response(
            200,
            json!({
                "domain": plan.domain,
                "alpnProtocol": String::from_utf8_lossy(&plan.alpn_protocol).to_string(),
                "challengeValue": plan.challenge_value
            }),
        )
    }

    fn route_hook_run(&self, method: Method, request: Request, actor: Option<&Actor>) -> Response {
        if let Err(response) = require_method(&method, Method::POST) {
            return response;
        }
        if let Err(response) = self.require_superuser(actor) {
            return response;
        }
        let body = match parse_body(&request) {
            Ok(body) => body,
            Err(response) => return response,
        };
        let script = body.get_str("script").unwrap_or_default();
        match eval_js_hook(script, &body) {
            Ok((value, runtime)) => {
                json_response(200, json!({"result": value, "runtime": runtime}))
            }
            Err(error) => error_response(400, &error),
        }
    }

    fn persist_or_500(&self, value: Value, ok_status: u16) -> Response {
        match self.save() {
            Ok(()) => json_response(ok_status, value),
            Err(error) => error_response(500, &format!("failed to persist state: {error}")),
        }
    }

    fn is_superuser(&self, actor: Option<&Actor>) -> bool {
        let Some(actor) = actor else {
            return false;
        };
        actor.kind == "admin" && self.admins.contains_key(&actor.id)
    }

    fn require_superuser(&self, actor: Option<&Actor>) -> Result<(), Response> {
        if self.is_superuser(actor) {
            Ok(())
        } else {
            Err(error_response(403, "superuser authorization is required"))
        }
    }

    fn insert_or_update_superuser(
        &mut self,
        email: &str,
        password: &str,
        allow_existing: bool,
    ) -> Result<(), String> {
        if !is_valid_email(email) {
            return Err("superuser email must be valid".to_string());
        }
        if password.is_empty() {
            return Err("superuser password is required".to_string());
        }
        let now = timestamp();
        let admin_id =
            if let Some(admin) = self.admins.values_mut().find(|admin| admin.email == email) {
                if !allow_existing {
                    return Err("superuser already exists".to_string());
                }
                admin.password = hash_password(password);
                admin.updated = now;
                admin.id.clone()
            } else {
                let admin = Admin {
                    id: id("adm"),
                    email: email.to_string(),
                    password: hash_password(password),
                    created: now.clone(),
                    updated: now,
                };
                let admin_id = admin.id.clone();
                self.admins.insert(admin_id.clone(), admin);
                admin_id
            };
        self.persist_admin_delta(&admin_id, false)
            .map_err(|error| error.to_string())
    }

    async fn send_system_email(&self, to: &str, subject: &str, body: &str) -> Result<(), String> {
        let smtp = self
            .settings
            .get_object("smtp")
            .ok_or_else(|| "smtp settings are missing".to_string())?;
        if smtp.get("enabled").and_then(Value::as_bool) != Some(true) {
            return Err("smtp mailer is not enabled".to_string());
        }
        let host = smtp
            .get("host")
            .and_then(Value::as_str)
            .unwrap_or("127.0.0.1");
        let port = smtp.get("port").and_then(Value::as_u64).unwrap_or(25);
        let from = smtp
            .get("senderAddress")
            .and_then(Value::as_str)
            .unwrap_or("noreply@localhost");
        let addr = format!("{host}:{port}");
        let mut client = SmtpClient::connect_no_tls(&addr)
            .await
            .map_err(|error| error.to_string())?;
        if let (Some(username), Some(password)) = (
            smtp.get("username").and_then(Value::as_str),
            smtp.get("password").and_then(Value::as_str),
        ) {
            if !username.is_empty() {
                client
                    .auth_plain(&format!("\0{username}\0{password}"))
                    .await
                    .map_err(|error| error.to_string())?;
            }
        }
        let message = build_system_email(&self.settings, to, subject, body);
        client
            .mail_from(from)
            .await
            .map_err(|error| error.to_string())?;
        client
            .rcpt_to(to)
            .await
            .map_err(|error| error.to_string())?;
        client
            .data(&message)
            .await
            .map_err(|error| error.to_string())?;
        let _ = client.quit().await;
        Ok(())
    }

    fn maybe_send_auth_flow_email(&self, to: &str, kind: &str, token: &str) -> bool {
        if self
            .settings
            .get_object("smtp")
            .and_then(|smtp| smtp.get("enabled"))
            .and_then(Value::as_bool)
            != Some(true)
        {
            return false;
        }
        let (subject, body) = match kind {
            "passwordReset" => (
                "Reset your password",
                format!("Use this token to reset your password: {token}"),
            ),
            "verification" => (
                "Verify your email",
                format!("Use this token to verify your email: {token}"),
            ),
            "emailChange" => (
                "Confirm your email change",
                format!("Use this token to confirm your email change: {token}"),
            ),
            "otp" => ("Your login code", format!("Your login code is {token}")),
            _ => ("PocketBase auth token", format!("Auth token: {token}")),
        };
        rt::block_on(self.send_system_email(to, subject, &body)).is_ok()
    }

    fn record_activity(
        &mut self,
        method: String,
        path: String,
        status: u16,
        actor: Option<&Actor>,
    ) {
        let actor = actor
            .map(|actor| format!("{}:{}", actor.kind, actor.id))
            .unwrap_or_else(|| "guest".to_string());
        let log = ActivityLog {
            id: id("log"),
            method,
            path,
            status,
            actor,
            created: timestamp(),
        };
        self.logs.push(log.clone());
        let retention = self
            .settings
            .get("logs")
            .and_then(|logs| logs.get_usize("maxEntries"))
            .unwrap_or(1000);
        let mut removed_ids = Vec::new();
        if self.logs.len() > retention {
            let remove = self.logs.len() - retention;
            removed_ids.extend(self.logs.drain(0..remove).map(|log| log.id));
        }
        let _ = self.persist_activity_log_delta(&log, &removed_ids);
    }

    fn find_auth_record_by_identity(
        &self,
        collection_name: &str,
        identity: &str,
    ) -> Option<(String, Record)> {
        let collection = self.collections.get(collection_name)?;
        if collection.kind != CollectionKind::Auth {
            return None;
        }
        collection.records.values().find_map(|record| {
            let email_ok = record
                .data
                .get("email")
                .and_then(Value::as_str)
                .is_some_and(|email| email == identity);
            let user_ok = record
                .data
                .get("username")
                .and_then(Value::as_str)
                .is_some_and(|username| username == identity);
            (email_ok || user_ok).then(|| (record.id.clone(), record.clone()))
        })
    }

    fn issue_flow_token(
        &mut self,
        collection_name: &str,
        record_id: &str,
        kind: &str,
        email: Option<String>,
    ) -> String {
        self.issue_flow_token_with_payload(collection_name, record_id, kind, email, None, 3600)
    }

    fn issue_flow_token_with_payload(
        &mut self,
        collection_name: &str,
        record_id: &str,
        kind: &str,
        email: Option<String>,
        payload: Option<String>,
        ttl_seconds: u64,
    ) -> String {
        self.prune_expired_flow_tokens();
        let token = self.sign_token(kind, record_id);
        self.auth_tokens.push(AuthFlowToken {
            token: token.clone(),
            collection: collection_name.to_string(),
            record_id: record_id.to_string(),
            kind: kind.to_string(),
            email,
            payload,
            expires: unix_seconds() + ttl_seconds,
        });
        let _ = self.persist_auth_tokens_delta();
        token
    }

    fn take_flow_token(
        &mut self,
        collection_name: &str,
        token: &str,
        kind: &str,
    ) -> Option<AuthFlowToken> {
        self.prune_expired_flow_tokens();
        let index = self.auth_tokens.iter().position(|candidate| {
            candidate.collection == collection_name
                && candidate.kind == kind
                && constant_time_eq(candidate.token.as_bytes(), token.as_bytes())
        })?;
        let token = self.auth_tokens.remove(index);
        let _ = self.persist_auth_tokens_delta();
        Some(token)
    }

    fn prune_expired_flow_tokens(&mut self) {
        let now = unix_seconds();
        self.auth_tokens.retain(|token| token.expires > now);
    }

    fn ensure_builtin_auth_collection(&mut self) {
        if self.collections.contains_key("users") {
            return;
        }
        let now = timestamp();
        self.collections.insert(
            "users".to_string(),
            Collection {
                id: id("col"),
                name: "users".to_string(),
                kind: CollectionKind::Auth,
                schema: vec![
                    Field::new("email", "email", true),
                    Field::new("password", "password", true),
                    Field::new("username", "text", false),
                    Field::new("verified", "bool", false),
                ],
                records: BTreeMap::new(),
                list_rule: None,
                view_rule: None,
                create_rule: None,
                update_rule: None,
                delete_rule: None,
                created: now.clone(),
                updated: now,
            },
        );
    }

    fn from_json(data_dir: PathBuf, value: Value) -> Self {
        let mut state = Self {
            data_dir,
            secret: value
                .get_str("secret")
                .map(str::to_string)
                .unwrap_or_else(|| id("sec")),
            collections: BTreeMap::new(),
            admins: BTreeMap::new(),
            migrations: Vec::new(),
            settings: value
                .get("settings")
                .cloned()
                .unwrap_or_else(default_settings),
            logs: Vec::new(),
            crons: Vec::new(),
            auth_tokens: Vec::new(),
            realtime_clients: BTreeMap::new(),
            realtime_events: Vec::new(),
        };
        if let Some(collections) = value.get_array("collections") {
            for value in collections {
                if let Some(collection) = Collection::from_json(value) {
                    state
                        .collections
                        .insert(collection.name.clone(), collection);
                }
            }
        }
        if let Some(admins) = value.get_array("admins") {
            for value in admins {
                if let Some(admin) = Admin::from_json(value) {
                    state.admins.insert(admin.id.clone(), admin);
                }
            }
        }
        if let Some(migrations) = value.get_array("migrations") {
            state.migrations = migrations.iter().filter_map(Migration::from_json).collect();
        }
        if let Some(logs) = value.get_array("logs") {
            state.logs = logs.iter().filter_map(ActivityLog::from_json).collect();
        }
        if let Some(crons) = value.get_array("crons") {
            state.crons = crons.iter().filter_map(CronJob::from_json).collect();
        }
        if let Some(tokens) = value.get_array("authTokens") {
            state.auth_tokens = tokens.iter().filter_map(AuthFlowToken::from_json).collect();
        }
        if let Some(events) = value.get_array("realtimeEvents") {
            state.realtime_events = events.iter().filter_map(RealtimeEvent::from_json).collect();
        }
        if state.crons.is_empty() {
            state.crons = default_crons();
        }
        state.ensure_builtin_auth_collection();
        state
    }

    fn to_json(&self) -> Value {
        json!({
            "secret": self.secret.clone(),
            "collections": self.collections.values().map(Collection::to_store_json).collect::<Vec<_>>(),
            "admins": self.admins.values().map(Admin::to_store_json).collect::<Vec<_>>(),
            "migrations": self.migrations.iter().map(Migration::to_json).collect::<Vec<_>>(),
            "settings": self.settings.clone(),
            "logs": self.logs.iter().map(ActivityLog::to_json).collect::<Vec<_>>(),
            "crons": self.crons.iter().map(CronJob::to_json).collect::<Vec<_>>(),
            "authTokens": self.auth_tokens.iter().map(AuthFlowToken::to_json).collect::<Vec<_>>(),
            "realtimeEvents": self.realtime_events.iter().map(RealtimeEvent::to_json).collect::<Vec<_>>()
        })
    }

    fn authenticate_request(&self, request: &Request) -> Option<Actor> {
        let header = request
            .headers()
            .get("authorization")
            .or_else(|| request.headers().get("Authorization"))?
            .as_str();
        let token = header.strip_prefix("Bearer ")?;
        self.verify_token(token)
    }

    fn sign_token(&self, kind: &str, id: &str) -> String {
        let iat = unix_seconds();
        let exp = iat + 604_800;
        let header = base64url_nopad_encode(br#"{"alg":"HS256","typ":"JWT"}"#);
        let payload = json!({"sub": id, "type": kind, "iat": iat, "exp": exp});
        let payload = base64url_nopad_encode(
            edgerun_json::to_string(&payload)
                .unwrap_or_default()
                .as_bytes(),
        );
        let signing_input = format!("{header}.{payload}");
        let signature = base64url_nopad_encode(&hmac_sha256(
            self.secret.as_bytes(),
            signing_input.as_bytes(),
        ));
        format!("{signing_input}.{signature}")
    }

    fn verify_token(&self, token: &str) -> Option<Actor> {
        let mut parts = token.split('.');
        let header = parts.next()?;
        let payload = parts.next()?;
        let signature = parts.next()?;
        if parts.next().is_some() {
            return None;
        }
        let signing_input = format!("{header}.{payload}");
        let expected = base64url_nopad_encode(&hmac_sha256(
            self.secret.as_bytes(),
            signing_input.as_bytes(),
        ));
        if !constant_time_eq(expected.as_bytes(), signature.as_bytes()) {
            return None;
        }
        let payload_bytes = base64url_decode(payload).ok()?;
        let payload_text = std::str::from_utf8(&payload_bytes).ok()?;
        let tape = parse_json_tape(payload_text).ok()?;
        let payload = tape.root(payload_text)?;
        let exp = payload.get_u64("exp").unwrap_or(0);
        if exp < unix_seconds() {
            return None;
        }
        Some(Actor {
            kind: payload.get_str("type")?.to_string(),
            id: payload.get_str("sub")?.to_string(),
        })
    }

    fn list_backups(&self) -> std::io::Result<Vec<Value>> {
        let dir = self.data_dir.join("backups");
        std::fs::create_dir_all(&dir)?;
        let mut items = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            if metadata.is_file() {
                items.push(json!({
                    "name": entry.file_name().to_string_lossy().to_string(),
                    "size": metadata.len(),
                }));
            }
        }
        Ok(items)
    }

    fn create_backup(&self, name: &str) -> std::io::Result<()> {
        let dir = self.data_dir.join("backups");
        std::fs::create_dir_all(&dir)?;
        let encoded = to_string_pretty(&self.to_json()).map_err(invalid_data)?;
        std::fs::write(dir.join(safe_file_name(name)), encoded)
    }
}

impl Collection {
    fn to_json(&self) -> Value {
        json!({
            "id": self.id.clone(),
            "name": self.name.clone(),
            "type": self.kind.as_str(),
            "schema": self.schema.iter().map(Field::to_json).collect::<Vec<_>>(),
            "listRule": self.list_rule.clone().unwrap_or_default(),
            "viewRule": self.view_rule.clone().unwrap_or_default(),
            "createRule": self.create_rule.clone().unwrap_or_default(),
            "updateRule": self.update_rule.clone().unwrap_or_default(),
            "deleteRule": self.delete_rule.clone().unwrap_or_default(),
            "created": self.created.clone(),
            "updated": self.updated.clone(),
        })
    }

    fn to_store_json(&self) -> Value {
        json!({
            "id": self.id.clone(),
            "name": self.name.clone(),
            "type": self.kind.as_str(),
            "schema": self.schema.iter().map(Field::to_json).collect::<Vec<_>>(),
            "records": self.records.values().map(Record::to_store_json).collect::<Vec<_>>(),
            "listRule": self.list_rule.clone().unwrap_or_default(),
            "viewRule": self.view_rule.clone().unwrap_or_default(),
            "createRule": self.create_rule.clone().unwrap_or_default(),
            "updateRule": self.update_rule.clone().unwrap_or_default(),
            "deleteRule": self.delete_rule.clone().unwrap_or_default(),
            "created": self.created.clone(),
            "updated": self.updated.clone(),
        })
    }

    fn from_json(value: &Value) -> Option<Self> {
        let name = value.get_str("name")?.to_string();
        Some(Self {
            id: value
                .get_str("id")
                .map(str::to_string)
                .unwrap_or_else(|| id("col")),
            name,
            kind: match value.get_str("type") {
                Some("auth") => CollectionKind::Auth,
                _ => CollectionKind::Base,
            },
            schema: value
                .get_array("schema")
                .map(|items| fields_from_array(items))
                .unwrap_or_default(),
            records: value
                .get_array("records")
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Record::from_json)
                        .map(|record| (record.id.clone(), record))
                        .collect()
                })
                .unwrap_or_default(),
            list_rule: opt_string(value, "listRule"),
            view_rule: opt_string(value, "viewRule"),
            create_rule: opt_string(value, "createRule"),
            update_rule: opt_string(value, "updateRule"),
            delete_rule: opt_string(value, "deleteRule"),
            created: value
                .get_str("created")
                .map(str::to_string)
                .unwrap_or_else(timestamp),
            updated: value
                .get_str("updated")
                .map(str::to_string)
                .unwrap_or_else(timestamp),
        })
    }

    fn validate_record(&self, body: &Value) -> Result<(), String> {
        for field in &self.schema {
            let value = body.get(&field.name);
            if field.required && value.is_none() {
                return Err(format!("missing required field `{}`", field.name));
            }
            let Some(value) = value else {
                continue;
            };
            field.validate_value(value)?;
        }
        Ok(())
    }
}

impl CollectionKind {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Base => "base",
            Self::Auth => "auth",
        }
    }
}

impl Field {
    fn new(name: &str, field_type: &str, required: bool) -> Self {
        Self {
            id: id("fld"),
            name: name.to_string(),
            field_type: field_type.to_string(),
            required,
            system: false,
            hidden: false,
            options: Value::empty_object(),
        }
    }

    fn to_json(&self) -> Value {
        let mut value = json!({
            "id": self.id.clone(),
            "name": self.name.clone(),
            "type": self.field_type.clone(),
            "required": self.required,
            "system": self.system,
            "hidden": self.hidden
        });
        merge_json(&mut value, &self.options);
        value
    }

    fn from_json(value: &Value) -> Option<Self> {
        Some(Self {
            id: value
                .get_str("id")
                .map(str::to_string)
                .unwrap_or_else(|| id("fld")),
            name: value.get_str("name")?.to_string(),
            field_type: value.get_str("type").unwrap_or("text").to_string(),
            required: value.get_bool("required").unwrap_or(false),
            system: value.get_bool("system").unwrap_or(false),
            hidden: value.get_bool("hidden").unwrap_or(false),
            options: field_options(value),
        })
    }

    fn validate_value(&self, value: &Value) -> Result<(), String> {
        match self.field_type.as_str() {
            "text" | "editor" | "url" => {
                let Some(text) = value.as_str() else {
                    return Err(format!("field `{}` must be a string", self.name));
                };
                if self.field_type == "url" && !is_valid_url(text) {
                    return Err(format!("field `{}` must be a URL", self.name));
                }
                validate_string_bounds(&self.name, text, &self.options)
            }
            "email" => {
                let Some(text) = value.as_str() else {
                    return Err(format!("field `{}` must be a string", self.name));
                };
                if !is_valid_email(text) {
                    return Err(format!("field `{}` must be an email", self.name));
                }
                Ok(())
            }
            "number" => {
                let Some(number) = value.as_f64() else {
                    return Err(format!("field `{}` must be a number", self.name));
                };
                if let Some(min) = self
                    .options
                    .get_f64("min")
                    .or_else(|| self.options.get_f64("minValue"))
                {
                    if number < min {
                        return Err(format!("field `{}` is below minimum", self.name));
                    }
                }
                if let Some(max) = self
                    .options
                    .get_f64("max")
                    .or_else(|| self.options.get_f64("maxValue"))
                {
                    if number > max {
                        return Err(format!("field `{}` exceeds maximum", self.name));
                    }
                }
                Ok(())
            }
            "bool" => value
                .as_bool()
                .map(|_| ())
                .ok_or_else(|| format!("field `{}` must be a boolean", self.name)),
            "date" | "autodate" => {
                let Some(text) = value.as_str() else {
                    return Err(format!("field `{}` must be a string", self.name));
                };
                if text.parse::<u64>().is_ok() || looks_like_iso_date(text) {
                    Ok(())
                } else {
                    Err(format!("field `{}` must be a date", self.name))
                }
            }
            "select" => validate_select(&self.name, value, &self.options),
            "json" => Ok(()),
            "file" => match value {
                Value::String(_) | Value::Array(_) => Ok(()),
                _ => Err(format!("field `{}` must be a file name or list", self.name)),
            },
            "relation" => match value {
                Value::String(_) | Value::Array(_) => Ok(()),
                _ => Err(format!("field `{}` must be a record id or list", self.name)),
            },
            "geoPoint" => {
                let lat = value.get("lat").and_then(Value::as_f64);
                let lon = value
                    .get("lon")
                    .or_else(|| value.get("lng"))
                    .and_then(Value::as_f64);
                if lat.is_some() && lon.is_some() {
                    Ok(())
                } else {
                    Err(format!("field `{}` must contain lat and lon", self.name))
                }
            }
            "password" => value
                .as_str()
                .map(|_| ())
                .ok_or_else(|| format!("field `{}` must be a string", self.name)),
            _ => Ok(()),
        }
    }
}

impl Record {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.push_field("id", self.id.clone());
        object.push_field("created", self.created.clone());
        object.push_field("updated", self.updated.clone());
        for (key, value) in &self.data {
            if key != "password" {
                object.push_field(key.clone(), value.clone());
            }
        }
        Value::Object(object)
    }

    fn to_store_json(&self) -> Value {
        let mut object = Map::new();
        object.push_field("id", self.id.clone());
        object.push_field("created", self.created.clone());
        object.push_field("updated", self.updated.clone());
        for (key, value) in &self.data {
            object.push_field(key.clone(), value.clone());
        }
        Value::Object(object)
    }

    fn from_json(value: &Value) -> Option<Self> {
        Some(Self {
            id: value.get_str("id")?.to_string(),
            data: object_to_record_data(value.clone()),
            created: value
                .get_str("created")
                .map(str::to_string)
                .unwrap_or_else(timestamp),
            updated: value
                .get_str("updated")
                .map(str::to_string)
                .unwrap_or_else(timestamp),
        })
    }
}

impl Admin {
    fn to_json(&self) -> Value {
        json!({
            "id": self.id.clone(),
            "email": self.email.clone(),
            "created": self.created.clone(),
            "updated": self.updated.clone()
        })
    }

    fn to_store_json(&self) -> Value {
        json!({
            "id": self.id.clone(),
            "email": self.email.clone(),
            "password": self.password.clone(),
            "created": self.created.clone(),
            "updated": self.updated.clone(),
        })
    }

    fn from_json(value: &Value) -> Option<Self> {
        Some(Self {
            id: value.get_str("id")?.to_string(),
            email: value.get_str("email")?.to_string(),
            password: value.get_str("password").unwrap_or_default().to_string(),
            created: value
                .get_str("created")
                .map(str::to_string)
                .unwrap_or_else(timestamp),
            updated: value
                .get_str("updated")
                .map(str::to_string)
                .unwrap_or_else(timestamp),
        })
    }
}

impl Migration {
    fn to_json(&self) -> Value {
        json!({
            "id": self.id.clone(),
            "name": self.name.clone(),
            "applied": self.applied.clone(),
        })
    }

    fn from_json(value: &Value) -> Option<Self> {
        Some(Self {
            id: value.get_str("id")?.to_string(),
            name: value.get_str("name")?.to_string(),
            applied: value
                .get_str("applied")
                .map(str::to_string)
                .unwrap_or_else(timestamp),
        })
    }
}

impl ActivityLog {
    fn to_json(&self) -> Value {
        json!({
            "id": self.id.clone(),
            "method": self.method.clone(),
            "path": self.path.clone(),
            "status": self.status,
            "actor": self.actor.clone(),
            "created": self.created.clone(),
        })
    }

    fn from_json(value: &Value) -> Option<Self> {
        Some(Self {
            id: value.get_str("id")?.to_string(),
            method: value.get_str("method").unwrap_or_default().to_string(),
            path: value.get_str("path").unwrap_or_default().to_string(),
            status: value.get_u64("status").unwrap_or(0) as u16,
            actor: value.get_str("actor").unwrap_or("guest").to_string(),
            created: value
                .get_str("created")
                .map(str::to_string)
                .unwrap_or_else(timestamp),
        })
    }
}

impl CronJob {
    fn to_json(&self) -> Value {
        json!({
            "id": self.id.clone(),
            "expression": self.expression.clone(),
            "command": self.command.clone(),
            "lastRun": self.last_run.clone().unwrap_or_default(),
        })
    }

    fn from_json(value: &Value) -> Option<Self> {
        Some(Self {
            id: value.get_str("id")?.to_string(),
            expression: value.get_str("expression").unwrap_or_default().to_string(),
            command: value.get_str("command").unwrap_or_default().to_string(),
            last_run: opt_string(value, "lastRun"),
        })
    }
}

impl AuthFlowToken {
    fn to_json(&self) -> Value {
        json!({
            "token": self.token.clone(),
            "collection": self.collection.clone(),
            "recordId": self.record_id.clone(),
            "type": self.kind.clone(),
            "email": self.email.clone().unwrap_or_default(),
            "payload": self.payload.clone().unwrap_or_default(),
            "expires": self.expires,
        })
    }

    fn from_json(value: &Value) -> Option<Self> {
        Some(Self {
            token: value.get_str("token")?.to_string(),
            collection: value.get_str("collection")?.to_string(),
            record_id: value.get_str("recordId")?.to_string(),
            kind: value.get_str("type")?.to_string(),
            email: opt_string(value, "email"),
            payload: opt_string(value, "payload"),
            expires: value.get_u64("expires").unwrap_or(0),
        })
    }
}

impl RealtimeClient {
    fn to_json(&self) -> Value {
        json!({
            "clientId": self.id.clone(),
            "subscriptions": self.subscriptions.clone(),
            "created": self.created.clone(),
        })
    }
}

impl RealtimeEvent {
    fn new(action: &str, collection: &str, record: Value) -> Self {
        Self {
            id: id("rte"),
            action: action.to_string(),
            collection: collection.to_string(),
            record,
            created: timestamp(),
        }
    }

    fn to_json(&self) -> Value {
        json!({
            "id": self.id.clone(),
            "action": self.action.clone(),
            "collection": self.collection.clone(),
            "record": self.record.clone(),
            "created": self.created.clone(),
        })
    }

    fn from_json(value: &Value) -> Option<Self> {
        Some(Self {
            id: value.get_str("id")?.to_string(),
            action: value.get_str("action")?.to_string(),
            collection: value.get_str("collection")?.to_string(),
            record: value.get("record").cloned().unwrap_or(Value::Null),
            created: value
                .get_str("created")
                .map(str::to_string)
                .unwrap_or_else(timestamp),
        })
    }
}

fn parse_body(request: &Request) -> Result<Value, Response> {
    let Some(body) = request.body() else {
        return Ok(Value::empty_object());
    };
    from_slice(body).map_err(|error| error_response(400, &format!("invalid JSON body: {error}")))
}

fn with_body_tape<T>(
    request: &Request,
    f: impl FnOnce(TapeValue<'_>) -> Result<T, Response>,
) -> Result<T, Response> {
    let Some(body) = request.body() else {
        let tape = parse_json_tape("{}")
            .map_err(|error| error_response(400, &format!("invalid JSON body: {error}")))?;
        let root = tape
            .root("{}")
            .ok_or_else(|| error_response(400, "invalid JSON body: empty document"))?;
        return f(root);
    };
    let text = std::str::from_utf8(body)
        .map_err(|error| error_response(400, &format!("invalid JSON body: {error}")))?;
    let tape = parse_json_tape(text)
        .map_err(|error| error_response(400, &format!("invalid JSON body: {error}")))?;
    let root = tape
        .root(text)
        .ok_or_else(|| error_response(400, "invalid JSON body: empty document"))?;
    f(root)
}

fn tape_str_any<'a>(value: TapeValue<'a>, keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|key| value.get_str(key))
}

fn acme_challenge_fields(request: &Request) -> Result<(String, String, String), Response> {
    with_body_tape(request, |body| {
        let domain = body
            .get_str("domain")
            .ok_or_else(|| error_response(400, "domain is required"))?;
        let token = body
            .get_str("token")
            .ok_or_else(|| error_response(400, "token is required"))?;
        let thumbprint = tape_str_any(
            body,
            &["accountThumbprint", "account_thumbprint", "thumbprint"],
        )
        .ok_or_else(|| error_response(400, "accountThumbprint is required"))?;
        Ok((
            domain.to_string(),
            token.to_string(),
            thumbprint.to_string(),
        ))
    })
}

fn color_json(color: Color4) -> Value {
    json!({"r": color.r, "g": color.g, "b": color.b, "a": color.a})
}

fn gpu_rect_json(rect: &edgerun_ui_core::gpu::GpuRect) -> Value {
    json!({
        "x": rect.x,
        "y": rect.y,
        "w": rect.w,
        "h": rect.h,
        "radius": rect.radius,
        "mode": rect_mode_name(rect.mode),
        "color": color_json(rect.color)
    })
}

fn gpu_hit_json(hit: &edgerun_ui_core::gpu::GpuHit) -> Value {
    json!({
        "kind": format!("{:?}", hit.kind),
        "id": hit.id,
        "x": hit.x,
        "y": hit.y,
        "w": hit.w,
        "h": hit.h
    })
}

fn rect_mode_name(mode: RectMode) -> &'static str {
    match mode {
        RectMode::Fill => "Fill",
        RectMode::Shadow => "Shadow",
        RectMode::Border => "Border",
        RectMode::LinearGradient => "LinearGradient",
    }
}

fn parse_record_request(request: &Request) -> Result<(Value, Vec<FileUpload>), Response> {
    let content_type = request
        .headers()
        .get("content-type")
        .map(|value| value.as_str())
        .unwrap_or_default();
    if is_multipart(content_type) {
        parse_multipart_record(request.body().unwrap_or_default(), content_type)
    } else {
        parse_body(request).map(|body| (prepare_record_data(body), Vec::new()))
    }
}

fn parse_multipart_record(
    body: &[u8],
    content_type: &str,
) -> Result<(Value, Vec<FileUpload>), Response> {
    let Some(boundary) = extract_boundary(content_type) else {
        return Err(error_response(400, "multipart boundary is required"));
    };
    let parts = parse_multipart(body, &boundary)
        .map_err(|error| error_response(400, &format!("invalid multipart body: {error}")))?;
    let mut object = Map::new();
    let mut files = Vec::new();
    for part in parts {
        if let Some(filename) = part.filename {
            files.push(FileUpload {
                field: part.name,
                name: safe_file_name(&filename),
                bytes: part.data,
            });
        } else {
            let value = String::from_utf8(part.data)
                .map_err(|_| error_response(400, "multipart field value must be UTF-8"))?;
            object.push_field(part.name, value);
        }
    }
    Ok((prepare_record_data(Value::Object(object)), files))
}

fn save_uploads(
    data_dir: &std::path::Path,
    collection_name: &str,
    mut record: Record,
    files: Vec<FileUpload>,
) -> std::io::Result<Record> {
    if files.is_empty() {
        return Ok(record);
    }
    let dir = data_dir
        .join("files")
        .join(collection_name)
        .join(&record.id);
    std::fs::create_dir_all(&dir)?;
    for upload in files {
        let name = if upload.name.is_empty() {
            id("file")
        } else {
            upload.name
        };
        std::fs::write(dir.join(&name), &upload.bytes)?;
        let _ = store_upload_blob(data_dir, collection_name, &record.id, &name, &upload.bytes);
        match record.data.get_mut(&upload.field) {
            Some(Value::Array(values)) => values.push(Value::String(name)),
            Some(Value::String(existing)) => {
                let previous = existing.clone();
                record.data.insert(
                    upload.field,
                    Value::Array(vec![Value::String(previous), Value::String(name)]),
                );
            }
            _ => {
                record.data.insert(upload.field, Value::String(name));
            }
        }
    }
    Ok(record)
}

fn store_upload_blob(
    data_dir: &std::path::Path,
    collection_name: &str,
    record_id: &str,
    name: &str,
    bytes: &[u8],
) -> Result<String, String> {
    let secret = std::fs::read_to_string(data_dir.join("store.json"))
        .ok()
        .and_then(|stored| {
            let tape = parse_json_tape(&stored).ok()?;
            tape.root(&stored)?.get_str("secret").map(str::to_string)
        })
        .unwrap_or_else(|| "edgerun-pocketbase-files".to_string());
    let config = BlobStoreConfig {
        blob_dir: data_dir.join("edgerun-storage").join("blobs"),
    };
    let store = BlobStore::open(
        &config,
        BlobKeySource::Software {
            private_key_bytes: secret.into_bytes(),
        },
    )
    .map_err(|error| error.to_string())?;
    let recipients = vec![store.node_identity().to_vec()];
    let blob_id = store
        .store(bytes, &recipients)
        .map_err(|error| error.to_string())?;
    let sidecar = data_dir
        .join("files")
        .join(collection_name)
        .join(record_id)
        .join(format!("{name}.edgerun-blob"));
    std::fs::write(&sidecar, &blob_id).map_err(|error| error.to_string())?;
    Ok(blob_id)
}

fn parse_collection_kind(body: &Value) -> CollectionKind {
    match body.get_str("type") {
        Some("auth") => CollectionKind::Auth,
        _ => CollectionKind::Base,
    }
}

fn parse_schema(body: &Value) -> Vec<Field> {
    body.get_array("schema")
        .map(|items| fields_from_array(items))
        .unwrap_or_default()
}

fn default_settings() -> Value {
    json!({
        "meta": {"appName": "EdgeRun PocketBase", "appUrl": "http://127.0.0.1:8090"},
        "logs": {"maxEntries": 1000},
        "smtp": {"enabled": false},
        "s3": {"enabled": false},
        "backups": {"enabled": true},
        "trustedProxy": {"enabled": false, "headers": []},
        "rateLimits": {"enabled": false, "rules": []},
        "batch": {"enabled": true, "maxRequests": 50},
        "cors": {"enabled": false, "origins": ["*"], "methods": ["GET","POST","PUT","PATCH","DELETE","OPTIONS"], "headers": ["Authorization","Content-Type"]},
        "request": {"maxBodyBytes": 33554432},
        "edgerun": {
            "acme": {"http01": []},
            "dns": {"zones": []},
            "dhcp": {
                "serverIp": "10.77.0.1",
                "subnetMask": "255.255.255.0",
                "router": "10.77.0.1",
                "dnsServers": ["10.77.0.1"],
                "leaseTime": 3600,
                "poolStart": "10.77.0.50",
                "poolEnd": "10.77.0.200"
            }
        },
        "superusers": {"allowedIps": []},
        "mailTemplates": {
            "verification": {},
            "passwordReset": {},
            "emailChange": {},
            "otp": {},
            "authAlert": {}
        }
    })
}

fn default_crons() -> Vec<CronJob> {
    vec![CronJob {
        id: "logs_cleanup".to_string(),
        expression: "0 0 * * *".to_string(),
        command: "logs.cleanup".to_string(),
        last_run: None,
    }]
}

fn pkce_metadata() -> Value {
    let sample = PkcePair::from_verifier_bytes(b"edgerun-pocketbase-pkce-sample-32");
    json!({
        "method": "S256",
        "verifierLength": sample.code_verifier.len(),
        "challengeLength": sample.code_challenge.len(),
    })
}

fn otp_code() -> String {
    let mut bytes = [0u8; 8];
    let value = if fill_random(&mut bytes).is_ok() {
        u64::from_be_bytes(bytes)
    } else {
        unix_seconds()
    };
    format!("{:06}", value % 1_000_000)
}

fn build_system_email(settings: &Value, to: &str, subject: &str, body: &str) -> Vec<u8> {
    let app_name = settings
        .get_object("meta")
        .and_then(|meta| meta.get("appName"))
        .and_then(Value::as_str)
        .unwrap_or("EdgeRun PocketBase");
    let from = settings
        .get_object("smtp")
        .and_then(|smtp| smtp.get("senderAddress"))
        .and_then(Value::as_str)
        .unwrap_or("noreply@localhost");
    EmailBuilder::new()
        .from(from)
        .to(to)
        .subject(subject)
        .header("X-Application", app_name)
        .message_id(&format!("<{}@edgerun-pocketbase.local>", id("msg")))
        .body(body)
        .build()
}

fn derived_row(rows: &mut BTreeMap<Vec<u8>, Vec<u8>>, key: &str, bytes: &[u8]) {
    rows.insert(key.as_bytes().to_vec(), bytes.to_vec());
}

fn derived_row_json(
    rows: &mut BTreeMap<Vec<u8>, Vec<u8>>,
    key: &str,
    value: &Value,
) -> std::io::Result<()> {
    let encoded = edgerun_json::to_string(value).map_err(invalid_data)?;
    derived_row(rows, key, encoded.as_bytes());
    Ok(())
}

fn derived_collection_key(collection: &str) -> String {
    format!("{DERIVED_COLLECTION_PREFIX}{collection}")
}

fn derived_record_prefix(collection: &str) -> String {
    format!("{DERIVED_RECORD_PREFIX}{collection}/")
}

fn derived_record_key(collection: &str, record: &str) -> String {
    format!("{}{record}", derived_record_prefix(collection))
}

fn derived_admin_key(admin_id: &str) -> String {
    format!("{DERIVED_ADMIN_PREFIX}{admin_id}")
}

fn derived_migration_key(migration_id: &str) -> String {
    format!("{DERIVED_MIGRATION_PREFIX}{migration_id}")
}

fn derived_log_key(log_id: &str) -> String {
    format!("{DERIVED_LOG_PREFIX}{log_id}")
}

fn derived_cron_key(cron_id: &str) -> String {
    format!("{DERIVED_CRON_PREFIX}{cron_id}")
}

fn derived_auth_token_key(token: &str) -> String {
    format!("{DERIVED_AUTH_TOKEN_PREFIX}{}", token_key(token))
}

fn derived_realtime_event_key(event_id: &str) -> String {
    format!("{DERIVED_REALTIME_EVENT_PREFIX}{event_id}")
}

fn derived_batch_put_json(
    batch: &mut DbBatch,
    key: &str,
    value: &Value,
    updated_at: i64,
) -> std::io::Result<()> {
    let encoded = edgerun_json::to_string(value).map_err(invalid_data)?;
    batch
        .put_meta(key.as_bytes(), encoded.as_bytes(), updated_at)
        .map(|_| ())
        .map_err(invalid_data)
}

fn token_key(token: &str) -> String {
    bytes_to_hex(&Sha256::digest(token.as_bytes()))
}

fn dns_zone_values(settings: &Value) -> Vec<Value> {
    settings
        .get_object("edgerun")
        .and_then(|edgerun| edgerun.get("dns"))
        .and_then(Value::as_object)
        .and_then(|dns| dns.get("zones"))
        .and_then(Value::as_array)
        .map(|zones| zones.to_vec())
        .unwrap_or_default()
}

fn acme_http01_values(settings: &Value) -> Vec<Value> {
    settings
        .get_object("edgerun")
        .and_then(|edgerun| edgerun.get("acme"))
        .and_then(Value::as_object)
        .and_then(|acme| acme.get("http01"))
        .and_then(Value::as_array)
        .map(|items| items.to_vec())
        .unwrap_or_default()
}

fn dns_zones_from_settings(settings: &Value) -> BTreeMap<String, DnsZone> {
    let mut zones = BTreeMap::new();
    for value in dns_zone_values(settings) {
        if let Some(zone) = dns_zone_from_value(&value) {
            zones.insert(zone.origin.clone(), zone);
        }
    }
    zones
}

fn validate_dns_zones_value(value: &Value) -> Result<(), String> {
    let Some(zones) = value.as_array() else {
        return Err("zones must be an array".to_string());
    };
    for zone in zones {
        dns_zone_from_value(zone).ok_or_else(|| "invalid DNS zone".to_string())?;
    }
    Ok(())
}

fn dns_zone_from_value(value: &Value) -> Option<DnsZone> {
    let origin = value.get_str("origin")?;
    let mut zone = DnsZone::new(origin);
    if let Some(ttl) = value.get_u64("ttl").and_then(|ttl| u32::try_from(ttl).ok()) {
        zone.set_default_ttl(ttl);
    }
    if let Some(records) = value.get_array("records") {
        for record in records {
            add_dns_record_from_value(&mut zone, record)?;
        }
    }
    Some(zone)
}

fn add_dns_record_from_value(zone: &mut DnsZone, value: &Value) -> Option<()> {
    let name = value.get_str("name").unwrap_or("@");
    let ttl = value
        .get_u64("ttl")
        .and_then(|ttl| u32::try_from(ttl).ok())
        .unwrap_or(300);
    match value.get_str("type").unwrap_or("A") {
        "A" => zone.add_a(name, parse_ipv4(value.get_str("value")?)?, ttl),
        "AAAA" => zone.add_aaaa(name, value.get_str("value")?.parse().ok()?, ttl),
        "CNAME" => zone.add_cname(name, value.get_str("value")?, ttl),
        "NS" => zone.add_ns(value.get_str("value")?),
        "MX" => zone.add_mx(
            name,
            value.get_u64("priority").unwrap_or(10) as u16,
            value.get_str("value")?,
            ttl,
        ),
        "TXT" => zone.add_txt(name, value.get_str("value")?, ttl),
        "PTR" => zone.add_ptr(name, value.get_str("value")?, ttl),
        "SOA" => zone.add_soa(
            value.get_str("mname").unwrap_or("ns1.local"),
            value.get_str("rname").unwrap_or("admin.local"),
        ),
        _ => return None,
    }
    Some(())
}

fn dns_record_type(value: &str) -> Option<DnsRecordType> {
    match value {
        "A" => Some(DnsRecordType::A),
        "AAAA" => Some(DnsRecordType::AAAA),
        "CNAME" => Some(DnsRecordType::CNAME),
        "NS" => Some(DnsRecordType::NS),
        "MX" => Some(DnsRecordType::MX),
        "TXT" => Some(DnsRecordType::TXT),
        "PTR" => Some(DnsRecordType::PTR),
        "SOA" => Some(DnsRecordType::SOA),
        _ => None,
    }
}

fn dns_record_to_json(record: &DnsRecord) -> Value {
    json!({
        "name": record.name.clone(),
        "type": record.rtype.as_str(),
        "ttl": record.ttl,
        "value": dns_record_data_value(&record.data)
    })
}

fn dns_record_data_value(data: &DnsRecordData) -> String {
    match data {
        DnsRecordData::A(ip) => ip.to_string(),
        DnsRecordData::AAAA(ip) => ip.to_string(),
        DnsRecordData::CNAME(value)
        | DnsRecordData::NS(value)
        | DnsRecordData::TXT(value)
        | DnsRecordData::PTR(value) => value.clone(),
        DnsRecordData::MX { priority, exchange } => format!("{priority} {exchange}"),
        DnsRecordData::SOA { mname, rname, .. } => format!("{mname} {rname}"),
        other => format!("{other:?}"),
    }
}

fn dhcp_core_from_settings(settings: &Value) -> Result<DhcpServerCore, String> {
    let server_ip = dhcp_config_value(settings, "serverIp")
        .and_then(parse_ipv4)
        .ok_or_else(|| "edgerun.dhcp.serverIp is invalid".to_string())?;
    let subnet_mask = dhcp_config_value(settings, "subnetMask")
        .and_then(parse_ipv4)
        .ok_or_else(|| "edgerun.dhcp.subnetMask is invalid".to_string())?;
    let router = dhcp_config_value(settings, "router")
        .and_then(parse_ipv4)
        .ok_or_else(|| "edgerun.dhcp.router is invalid".to_string())?;
    let pool_start = dhcp_config_value(settings, "poolStart")
        .and_then(parse_ipv4)
        .ok_or_else(|| "edgerun.dhcp.poolStart is invalid".to_string())?;
    let pool_end = dhcp_config_value(settings, "poolEnd")
        .and_then(parse_ipv4)
        .ok_or_else(|| "edgerun.dhcp.poolEnd is invalid".to_string())?;
    let dns_servers = settings
        .get_object("edgerun")
        .and_then(|edgerun| edgerun.get("dhcp"))
        .and_then(Value::as_object)
        .and_then(|dhcp| dhcp.get("dnsServers"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .filter_map(parse_ipv4)
                .collect::<Vec<_>>()
        })
        .filter(|items| !items.is_empty())
        .unwrap_or_else(|| vec![server_ip]);
    let lease_time = settings
        .get_object("edgerun")
        .and_then(|edgerun| edgerun.get("dhcp"))
        .and_then(Value::as_object)
        .and_then(|dhcp| dhcp.get("leaseTime"))
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or(3600);
    let config = DhcpServerConfig {
        server_ip,
        subnet_mask,
        router,
        dns_servers,
        lease_time,
        tftp_server: None,
        default_bootfile: None,
        bootfile_by_arch: BTreeMap::new(),
    };
    Ok(DhcpServerCore::new(config, pool_start, pool_end))
}

fn dhcp_config_value<'a>(settings: &'a Value, key: &str) -> Option<&'a str> {
    settings
        .get_object("edgerun")
        .and_then(|edgerun| edgerun.get("dhcp"))
        .and_then(Value::as_object)
        .and_then(|dhcp| dhcp.get(key))
        .and_then(Value::as_str)
}

fn dhcp_exchange_response(core: &mut DhcpServerCore, message: DhcpMessage) -> Response {
    let request_wire = message.to_wire();
    let Some(datagram) = core.handle_message(&message) else {
        return json_response(
            200,
            json!({"handled": false, "requestWireHex": bytes_to_hex(&request_wire)}),
        );
    };
    let parsed = match DhcpMessage::from_wire(&datagram.wire) {
        Ok(parsed) => parsed,
        Err(error) => return error_response(500, &format!("DHCP response decode failed: {error}")),
    };
    json_response(
        200,
        json!({
            "handled": true,
            "dest": datagram.dest.to_string(),
            "port": datagram.port,
            "requestWireHex": bytes_to_hex(&request_wire),
            "responseWireHex": bytes_to_hex(&datagram.wire),
            "response": dhcp_message_to_json(&parsed)
        }),
    )
}

fn dhcp_message_to_json(message: &DhcpMessage) -> Value {
    json!({
        "type": dhcp_message_type_name(message.options.message_type),
        "xid": message.xid,
        "yiaddr": message.yiaddr.to_string(),
        "serverId": message.options.server_id.map(|ip| ip.to_string()).unwrap_or_default(),
        "leaseTime": message.options.lease_time.unwrap_or_default(),
        "router": message.options.router.map(|ip| ip.to_string()).unwrap_or_default(),
        "dnsServers": message.options.dns_servers.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "bootfile": message.options.bootfile_name.clone().unwrap_or_default(),
        "clientArch": message.options.client_arch.map(PxeClientArch::as_str).unwrap_or_default()
    })
}

fn dhcp_message_type_name(value: Option<DhcpMessageType>) -> &'static str {
    match value {
        Some(DhcpMessageType::Discover) => "discover",
        Some(DhcpMessageType::Offer) => "offer",
        Some(DhcpMessageType::Request) => "request",
        Some(DhcpMessageType::Decline) => "decline",
        Some(DhcpMessageType::Ack) => "ack",
        Some(DhcpMessageType::Nak) => "nak",
        Some(DhcpMessageType::Release) => "release",
        Some(DhcpMessageType::Inform) => "inform",
        None => "unknown",
    }
}

fn parse_ipv4(value: &str) -> Option<Ipv4Addr> {
    value.parse().ok()
}

fn parse_mac(value: &str) -> Option<[u8; 6]> {
    let parts = value.split([':', '-']).collect::<Vec<_>>();
    if parts.len() != 6 {
        return None;
    }
    let mut out = [0u8; 6];
    for (index, part) in parts.iter().enumerate() {
        out[index] = u8::from_str_radix(part, 16).ok()?;
    }
    Some(out)
}

fn apply_cors_headers(response: &mut Response, origin: Option<&str>, settings: &Value) {
    let Some(cors) = settings.get_object("cors") else {
        return;
    };
    if cors.get("enabled").and_then(Value::as_bool) != Some(true) {
        return;
    }
    let allow_origin = cors
        .get("origins")
        .and_then(Value::as_array)
        .and_then(|origins| {
            if origins.iter().any(|value| value.as_str() == Some("*")) {
                Some("*")
            } else {
                origin.filter(|origin| origins.iter().any(|value| value.as_str() == Some(*origin)))
            }
        })
        .unwrap_or("*");
    let methods = settings_list(cors, "methods", "GET,POST,PUT,PATCH,DELETE,OPTIONS");
    let headers = settings_list(cors, "headers", "Authorization,Content-Type");
    let _ = response
        .headers_mut()
        .insert("Access-Control-Allow-Origin", allow_origin);
    let _ = response
        .headers_mut()
        .insert("Access-Control-Allow-Methods", &methods);
    let _ = response
        .headers_mut()
        .insert("Access-Control-Allow-Headers", &headers);
}

fn settings_list(object: &Map, key: &str, default_value: &str) -> String {
    object
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(",")
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default_value.to_string())
}

fn merge_json(target: &mut Value, patch: &Value) {
    match (target.as_object_mut(), patch.as_object()) {
        (Some(target), Some(patch)) => {
            for (key, value) in patch.fields() {
                if let Some(existing) = target.get_mut(key) {
                    merge_json(existing, value);
                } else {
                    target.push_field(key.to_string(), value.clone());
                }
            }
        }
        _ => *target = patch.clone(),
    }
}

fn set_nested_setting(root: &mut Value, path: &[&str], value: Value) -> Result<(), String> {
    if path.is_empty() {
        *root = value;
        return Ok(());
    }
    let mut current = root;
    for key in &path[..path.len() - 1] {
        if current.as_object().is_none() {
            *current = Value::empty_object();
        }
        let Some(object) = current.as_object_mut() else {
            return Err(format!("settings path segment {key:?} is not an object"));
        };
        if object.get(key).is_none() {
            object.push_field((*key).to_string(), Value::empty_object());
        }
        let Some(next) = object.get_mut(key) else {
            return Err(format!("settings path segment {key:?} was not created"));
        };
        current = next;
    }
    if current.as_object().is_none() {
        *current = Value::empty_object();
    }
    let Some(object) = current.as_object_mut() else {
        return Err("settings leaf parent is not an object".to_string());
    };
    if let Some(existing) = object.get_mut(path[path.len() - 1]) {
        *existing = value;
    } else {
        object.push_field(path[path.len() - 1].to_string(), value);
    }
    Ok(())
}

enum JsHookRuntime {
    QuickJs(PathBuf),
    Node(PathBuf),
}

impl JsHookRuntime {
    fn name(&self) -> &'static str {
        match self {
            Self::QuickJs(_) => "quickjs",
            Self::Node(_) => "node",
        }
    }
}

fn eval_js_hook(script: &str, body: &Value) -> Result<(Value, &'static str), String> {
    let runtime = js_hook_runtime()?;
    let context = edgerun_json::to_string(body).map_err(|error| error.to_string())?;
    let program = format!(
        "const request = {context}; const context = request; const result = (() => {{ {script} }})(); console.log(JSON.stringify(result === undefined ? null : result));"
    );
    let runtime_name = runtime.name();
    let mut command = match runtime {
        JsHookRuntime::QuickJs(path) => {
            let mut command = Command::new(path);
            command
                .arg("--memory-limit")
                .arg("8192")
                .arg("--stack-size")
                .arg("512")
                .arg("-e");
            command
        }
        JsHookRuntime::Node(path) => {
            let mut command = Command::new(path);
            command.arg("-e");
            command
        }
    };
    let output = command
        .arg(program)
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(stderr.trim().to_string());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .last()
        .ok_or_else(|| "JavaScript hook runtime produced no result".to_string())?;
    let value = from_slice(line.as_bytes()).map_err(|error| error.to_string())?;
    Ok((value, runtime_name))
}

fn js_hook_runtime() -> Result<JsHookRuntime, String> {
    if let Some(path) = env::var_os("EDGERUN_QUICKJS")
        .map(PathBuf::from)
        .filter(|path| path.exists())
    {
        return Ok(JsHookRuntime::QuickJs(path));
    }

    if let Some(path) = env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths)
            .map(|dir| dir.join("qjs"))
            .find(|path| path.exists())
    }) {
        return Ok(JsHookRuntime::QuickJs(path));
    }

    if let Some(path) = env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths)
            .map(|dir| dir.join("node"))
            .find(|path| path.exists())
    }) {
        return Ok(JsHookRuntime::Node(path));
    }

    Err("JavaScript hook execution requires EDGERUN_QUICKJS, qjs, or node on PATH".to_string())
}

fn field_options(value: &Value) -> Value {
    let mut options = Map::new();
    for (key, item) in value.object_entries().into_iter().flatten() {
        if !matches!(
            key,
            "id" | "name" | "type" | "required" | "system" | "hidden"
        ) {
            options.push_field(key.to_string(), item.clone());
        }
    }
    Value::Object(options)
}

fn opt_string(value: &Value, key: &str) -> Option<String> {
    value
        .get_str(key)
        .map(str::to_string)
        .filter(|value| !value.is_empty())
}

fn patch_rule(body: &Value, key: &str, target: &mut Option<String>) {
    if body.get(key).is_some() {
        *target = opt_string(body, key);
    }
}

fn prepare_record_data(mut body: Value) -> Value {
    if let Some(object) = body.as_object_mut() {
        if let Some(password) = object
            .get("password")
            .and_then(Value::as_str)
            .map(str::to_string)
        {
            object.push_field("password", hash_password(&password));
        }
    }
    body
}

fn fields_from_array(values: &[Value]) -> Vec<Field> {
    values.iter().filter_map(Field::from_json).collect()
}

fn object_to_record_data(value: Value) -> BTreeMap<String, Value> {
    value
        .object_entries()
        .into_iter()
        .flatten()
        .filter(|(key, _)| !is_system_record_field(key))
        .map(|(key, value)| (key.to_string(), value.clone()))
        .collect()
}

fn record_has_external_auth(record: &Record, provider: &str, provider_id: &str) -> bool {
    record
        .data
        .get("externalAuths")
        .and_then(Value::as_array)
        .is_some_and(|items| {
            items.iter().any(|item| {
                item.get_str("provider") == Some(provider)
                    && item
                        .get_str("providerId")
                        .or_else(|| item.get_str("provider_id"))
                        == Some(provider_id)
            })
        })
}

fn append_external_auth(record: &mut Record, external: Value) {
    let Some(provider) = external.get_str("provider") else {
        return;
    };
    let Some(provider_id) = external
        .get_str("providerId")
        .or_else(|| external.get_str("provider_id"))
    else {
        return;
    };
    if record_has_external_auth(record, provider, provider_id) {
        return;
    }
    match record.data.get_mut("externalAuths") {
        Some(Value::Array(items)) => items.push(external),
        _ => {
            record
                .data
                .insert("externalAuths".to_string(), Value::Array(vec![external]));
        }
    }
}

fn is_system_record_field(key: &str) -> bool {
    matches!(
        key,
        "id" | "created" | "updated" | "collectionId" | "collectionName"
    )
}

fn rule_allows(rule: Option<&str>, actor: Option<&Actor>, record: Option<&Value>) -> bool {
    let Some(rule) = rule.map(str::trim).filter(|rule| !rule.is_empty()) else {
        return true;
    };
    if rule == "true" {
        return true;
    }
    if rule == "false" {
        return false;
    }
    if rule == "@request.auth.id != ''" || rule == "@request.auth.id != \"\"" {
        return actor.is_some();
    }
    if rule == "@request.auth.id = id" || rule == "@request.auth.id == id" {
        return actor
            .zip(record)
            .and_then(|(actor, record)| record.get_str("id").map(|id| actor.id == id))
            .unwrap_or(false);
    }
    if let Some((field, expected)) = split_rule_eq(rule, "!=") {
        return record
            .and_then(|record| record.get(field))
            .map(|value| value_to_filter_string(value) != expected)
            .unwrap_or(true);
    }
    if let Some((field, expected)) = split_rule_eq(rule, "==").or_else(|| split_rule_eq(rule, "="))
    {
        if field == "@request.auth.type" {
            return actor.map(|actor| actor.kind == expected).unwrap_or(false);
        }
        if field == "@request.auth.id" {
            return actor.map(|actor| actor.id == expected).unwrap_or(false);
        }
        return record
            .and_then(|record| record.get(field))
            .map(|value| value_to_filter_string(value) == expected)
            .unwrap_or(false);
    }
    false
}

fn split_rule_eq<'a>(rule: &'a str, op: &str) -> Option<(&'a str, String)> {
    let (left, right) = rule.split_once(op)?;
    Some((left.trim(), unquote(right.trim())))
}

fn unquote(value: &str) -> String {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
        .unwrap_or(value)
        .to_string()
}

fn record_matches_query(record: &Record, query: Option<&str>) -> bool {
    let Some(filter) = query_pairs(query)
        .into_iter()
        .find_map(|(key, value)| (key == "filter").then_some(value))
    else {
        return true;
    };
    eval_filter_expression(record, &filter)
}

fn eval_filter_expression(record: &Record, filter: &str) -> bool {
    filter.split("||").any(|or_part| {
        or_part.split("&&").all(|expr| {
            let expr = expr.trim().trim_matches(|ch| ch == '(' || ch == ')');
            eval_filter_atom(record, expr).unwrap_or(false)
        })
    })
}

fn eval_filter_atom(record: &Record, expr: &str) -> Option<bool> {
    for op in [">=", "<=", "!=", "~", "=", ">", "<"] {
        if let Some((field, expected)) = expr.split_once(op) {
            let field = field.trim();
            let expected = unquote(expected.trim());
            let actual = if field == "id" {
                Some(record.id.clone())
            } else if field == "created" {
                Some(record.created.clone())
            } else if field == "updated" {
                Some(record.updated.clone())
            } else {
                record.data.get(field).map(value_to_filter_string)
            }?;
            return Some(match op {
                "=" => actual == expected,
                "!=" => actual != expected,
                "~" => actual.contains(&expected),
                ">" => compare_filter_values(&actual, &expected).is_gt(),
                "<" => compare_filter_values(&actual, &expected).is_lt(),
                ">=" => !compare_filter_values(&actual, &expected).is_lt(),
                "<=" => !compare_filter_values(&actual, &expected).is_gt(),
                _ => false,
            });
        }
    }
    None
}

fn compare_filter_values(left: &str, right: &str) -> std::cmp::Ordering {
    match (left.parse::<f64>(), right.parse::<f64>()) {
        (Ok(left), Ok(right)) => left
            .partial_cmp(&right)
            .unwrap_or(std::cmp::Ordering::Equal),
        _ => left.cmp(right),
    }
}

fn realtime_subscription_matches(subscription: &str, event: &RealtimeEvent) -> bool {
    subscription == "*"
        || subscription == event.collection
        || subscription
            .strip_prefix(event.collection.as_str())
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn apply_sort(records: &mut [Record], query: Option<&str>) {
    let Some(sort) = query_pairs(query)
        .into_iter()
        .find_map(|(key, value)| (key == "sort").then_some(value))
    else {
        return;
    };
    let fields = sort
        .split(',')
        .map(str::trim)
        .filter(|field| !field.is_empty())
        .collect::<Vec<_>>();
    records.sort_by(|left, right| {
        for field in &fields {
            let descending = field.starts_with('-');
            let field = field.trim_start_matches(['-', '+']);
            let left = record_sort_value(left, field);
            let right = record_sort_value(right, field);
            let ordering = left.cmp(&right);
            if ordering != std::cmp::Ordering::Equal {
                return if descending {
                    ordering.reverse()
                } else {
                    ordering
                };
            }
        }
        std::cmp::Ordering::Equal
    });
}

fn record_sort_value(record: &Record, field: &str) -> String {
    match field {
        "id" => record.id.clone(),
        "created" => record.created.clone(),
        "updated" => record.updated.clone(),
        other => record
            .data
            .get(other)
            .map(value_to_filter_string)
            .unwrap_or_default(),
    }
}

fn project_fields(value: Value, query: Option<&str>) -> Value {
    let Some(fields) = query_pairs(query)
        .into_iter()
        .find_map(|(key, value)| (key == "fields").then_some(value))
    else {
        return value;
    };
    let selected = fields.split(',').map(str::trim).collect::<Vec<_>>();
    let mut object = Map::new();
    for field in selected {
        if let Some(value) = value.get(field) {
            object.push_field(field.to_string(), value.clone());
        }
    }
    Value::Object(object)
}

fn query_usize(query: Option<&str>, key: &str) -> Option<usize> {
    query_pairs(query).into_iter().find_map(|(name, value)| {
        if name == key {
            value.parse().ok()
        } else {
            None
        }
    })
}

fn query_f32(query: Option<&str>, key: &str) -> Option<f32> {
    query_pairs(query).into_iter().find_map(|(name, value)| {
        if name == key {
            value.parse().ok()
        } else {
            None
        }
    })
}

fn query_bool(query: Option<&str>, key: &str) -> Option<bool> {
    query_pairs(query).into_iter().find_map(|(name, value)| {
        if name == key {
            Some(value == "true" || value == "1")
        } else {
            None
        }
    })
}

fn query_string(query: Option<&str>, key: &str) -> Option<String> {
    query_pairs(query)
        .into_iter()
        .find_map(|(name, value)| (name == key).then_some(value))
}

fn admin_view_from_query(query: Option<&str>) -> UiPocketBaseAdminView {
    query_pairs(query)
        .into_iter()
        .find_map(|(name, value)| {
            if name == "view" {
                Some(match value.as_str() {
                    "logs" => UiPocketBaseAdminView::Logs,
                    "settings" => UiPocketBaseAdminView::Settings,
                    "backups" => UiPocketBaseAdminView::Backups,
                    "crons" => UiPocketBaseAdminView::Crons,
                    _ => UiPocketBaseAdminView::Collections,
                })
            } else {
                None
            }
        })
        .unwrap_or(UiPocketBaseAdminView::Collections)
}

fn query_pairs(query: Option<&str>) -> Vec<(String, String)> {
    query
        .unwrap_or_default()
        .split('&')
        .filter_map(|pair| {
            let (key, value) = pair.split_once('=')?;
            Some((key.to_string(), unquote(value)))
        })
        .collect()
}

fn value_to_filter_string(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        _ => edgerun_json::to_string(value).unwrap_or_default(),
    }
}

fn validate_string_bounds(field: &str, text: &str, options: &Value) -> Result<(), String> {
    if let Some(min) = options
        .get_usize("min")
        .or_else(|| options.get_usize("minLength"))
    {
        if text.chars().count() < min {
            return Err(format!("field `{field}` is shorter than minimum length"));
        }
    }
    if let Some(max) = options
        .get_usize("max")
        .or_else(|| options.get_usize("maxLength"))
    {
        if text.chars().count() > max {
            return Err(format!("field `{field}` exceeds maximum length"));
        }
    }
    if let Some(pattern) = options.get_str("pattern") {
        if !pattern.is_empty() && !text.contains(pattern) {
            return Err(format!("field `{field}` does not match pattern"));
        }
    }
    Ok(())
}

fn validate_select(field: &str, value: &Value, options: &Value) -> Result<(), String> {
    let allowed = options.get_array("values").cloned().unwrap_or_default();
    let max_select = options.get_usize("maxSelect").unwrap_or(1);
    let values = match value {
        Value::String(item) => vec![item.clone()],
        Value::Array(items) => items
            .iter()
            .filter_map(|item| item.as_str().map(str::to_string))
            .collect::<Vec<_>>(),
        _ => return Err(format!("field `{field}` must be a select value")),
    };
    if values.len() > max_select {
        return Err(format!("field `{field}` has too many selected values"));
    }
    if !allowed.is_empty() {
        for selected in values {
            if !allowed
                .iter()
                .any(|value| value.as_str() == Some(selected.as_str()))
            {
                return Err(format!("field `{field}` has an unsupported value"));
            }
        }
    }
    Ok(())
}

fn is_valid_email(value: &str) -> bool {
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    !local.is_empty() && domain.contains('.') && !domain.ends_with('.')
}

fn is_valid_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}

fn looks_like_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 10
        && bytes.get(4) == Some(&b'-')
        && bytes.get(7) == Some(&b'-')
        && bytes[..4].iter().all(u8::is_ascii_digit)
        && bytes[5..7].iter().all(u8::is_ascii_digit)
        && bytes[8..10].iter().all(u8::is_ascii_digit)
}

fn oauth_provider_names() -> Vec<Value> {
    [
        "apple",
        "bitbucket",
        "box",
        "discord",
        "facebook",
        "gitea",
        "gitee",
        "github",
        "gitlab",
        "google",
        "instagram",
        "kakao",
        "lark",
        "linear",
        "livechat",
        "mailcow",
        "microsoft",
        "monday",
        "notion",
        "oidc",
        "patreon",
        "planningcenter",
        "spotify",
        "strava",
        "trakt",
        "twitch",
        "twitter",
        "vk",
        "wakatime",
        "yandex",
    ]
    .iter()
    .map(|name| json!({"name": *name, "displayName": *name, "enabled": false}))
    .collect()
}

fn build_internal_request(
    method: Method,
    path: &str,
    body: Option<&str>,
    actor: Option<&Actor>,
) -> Result<Request, String> {
    let mut builder = Request::builder()
        .method(method)
        .uri(format!("http://127.0.0.1/{}", path.trim_start_matches('/')));
    if let Some(body) = body {
        builder = builder.json_body(body);
    }
    if let Some(actor) = actor {
        builder = builder.header(
            "Authorization",
            &format!("Bearer internal.{}.{}", actor.kind, actor.id),
        );
    }
    builder
        .build()
        .map_err(|error| format!("invalid internal batch request: {error}"))
}

fn require_method(actual: &Method, expected: Method) -> Result<(), Response> {
    if *actual == expected {
        Ok(())
    } else {
        Err(error_response(405, "method not allowed"))
    }
}

fn json_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out
}

fn valid_name(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn id(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("{prefix}_{nanos:x}")
}

fn timestamp() -> String {
    unix_seconds().to_string()
}

fn unix_seconds() -> u64 {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    seconds
}

fn hash_password(password: &str) -> String {
    let salt = id("salt");
    let hash = pbkdf2_hmac_sha256(
        password.as_bytes(),
        salt.as_bytes(),
        PASSWORD_ITERATIONS,
        32,
    );
    format!(
        "pbkdf2_sha256${PASSWORD_ITERATIONS}${salt}${}",
        bytes_to_hex(&hash)
    )
}

fn verify_password(password: &str, stored: &str) -> bool {
    let parts = stored.split('$').collect::<Vec<_>>();
    if parts.len() != 4 || parts[0] != "pbkdf2_sha256" {
        return false;
    }
    let Ok(iterations) = parts[1].parse::<u32>() else {
        return false;
    };
    let Ok(expected) = hex_to_bytes(parts[3]) else {
        return false;
    };
    let actual = pbkdf2_hmac_sha256(
        password.as_bytes(),
        parts[2].as_bytes(),
        iterations,
        expected.len(),
    );
    constant_time_eq(&actual, &expected)
}

fn pbkdf2_hmac_sha256(password: &[u8], salt: &[u8], iterations: u32, len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(len);
    let mut block_index = 1u32;
    while out.len() < len {
        let mut input = Vec::with_capacity(salt.len() + 4);
        input.extend_from_slice(salt);
        input.extend_from_slice(&block_index.to_be_bytes());
        let mut u = hmac_sha256(password, &input);
        let mut block = u;
        for _ in 1..iterations {
            u = hmac_sha256(password, &u);
            for (dst, src) in block.iter_mut().zip(u) {
                *dst ^= src;
            }
        }
        out.extend_from_slice(&block);
        block_index = block_index.saturating_add(1);
    }
    out.truncate(len);
    out
}

fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    let mut key_block = [0u8; 64];
    if key.len() > 64 {
        key_block[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        key_block[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0x36u8; 64];
    let mut opad = [0x5cu8; 64];
    for index in 0..64 {
        ipad[index] ^= key_block[index];
        opad[index] ^= key_block[index];
    }
    let mut inner = Vec::with_capacity(64 + message.len());
    inner.extend_from_slice(&ipad);
    inner.extend_from_slice(message);
    let inner_hash = Sha256::digest(&inner);
    let mut outer = Vec::with_capacity(96);
    outer.extend_from_slice(&opad);
    outer.extend_from_slice(&inner_hash);
    Sha256::digest(&outer)
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut diff = 0u8;
    for (left, right) in left.iter().zip(right) {
        diff |= left ^ right;
    }
    diff == 0
}

fn safe_file_name(name: &str) -> String {
    name.chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
        .collect::<String>()
}

fn json_response(code: u16, value: Value) -> Response {
    let body = edgerun_json::to_string(&value).unwrap_or_else(|_| "{}".to_string());
    Response::json(status(code), &body)
}

fn persist_response(
    value: Value,
    ok_status: u16,
    label: &str,
    result: std::io::Result<()>,
) -> Response {
    match result {
        Ok(()) => json_response(ok_status, value),
        Err(error) => error_response(500, &format!("failed to persist {label}: {error}")),
    }
}

fn error_response(code: u16, message: &str) -> Response {
    json_response(code, json!({"code": code, "message": message, "data": {}}))
}

fn should_record_activity(path: &str) -> bool {
    path != "/api/health" && !path.starts_with("/_/assets/")
}

fn status(code: u16) -> StatusCode {
    StatusCode::new(code).unwrap_or_else(|_| StatusCode::new(500).unwrap())
}

fn invalid_data(error: impl std::fmt::Display) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admin_ws_bind_uses_adjacent_port() {
        assert_eq!(
            admin_ws_bind_for("127.0.0.1:8090").unwrap(),
            "127.0.0.1:8091"
        );
        assert_eq!(admin_ws_bind_for("[::1]:8090").unwrap(), "[::1]:8091");
    }

    #[test]
    fn admin_ws_bind_rejects_ambiguous_or_overflowing_bind() {
        assert!(admin_ws_bind_for("localhost:8090").is_err());
        assert!(admin_ws_bind_for("127.0.0.1:65535").is_err());
    }

    #[test]
    fn collection_record_crud_and_reload_persist_records() {
        let dir = temp_dir("crud");
        let mut state = PocketState::load(dir.clone()).unwrap();
        let token = create_admin_token(&mut state);

        let response = state.route(auth_request(
            Method::POST,
            "/api/collections",
            Some(r#"{"name":"posts","schema":[{"name":"title","type":"text","required":true}]}"#),
            &token,
        ));
        assert_eq!(response.status().as_u16(), 200);

        let response = state.route(request(
            Method::POST,
            "/api/collections/posts/records",
            Some(r#"{"title":"First"}"#),
        ));
        assert_eq!(response.status().as_u16(), 200);
        let created = parse_response(response);
        let id = created.get_str("id").unwrap().to_string();

        let reloaded = PocketState::load(dir).unwrap();
        let record = reloaded
            .collections
            .get("posts")
            .unwrap()
            .records
            .get(&id)
            .unwrap();
        assert_eq!(
            record.data.get("title").and_then(Value::as_str),
            Some("First")
        );
    }

    #[test]
    fn derived_db_is_canonical_when_json_export_is_missing() {
        let dir = temp_dir("derived-db");
        let mut state = PocketState::load(dir.clone()).unwrap();
        let token = create_admin_token(&mut state);

        let created = state.route(auth_request(
            Method::POST,
            "/api/collections",
            Some(r#"{"name":"notes","schema":[{"name":"title","type":"text","required":true}]}"#),
            &token,
        ));
        assert_eq!(created.status().as_u16(), 200);
        let record = state.route(request(
            Method::POST,
            "/api/collections/notes/records",
            Some(r#"{"title":"from-derived-db"}"#),
        ));
        assert_eq!(record.status().as_u16(), 200);
        assert!(dir.join("data.db").exists());
        std::fs::remove_file(dir.join("store.json")).unwrap();
        let mut db = open_file_database_repairing_tail(dir.join("data.db")).unwrap();
        assert!(
            db.meta_rows_with_prefix(b"pocketbase/record/notes/")
                .count()
                > 0
        );
        assert!(db.delete_meta(DERIVED_DB_STATE_KEY).unwrap());

        let reloaded = PocketState::load(dir).unwrap();
        let notes = reloaded.collections.get("notes").unwrap();
        assert_eq!(notes.records.len(), 1);
        assert_eq!(
            notes
                .records
                .values()
                .next()
                .unwrap()
                .data
                .get("title")
                .and_then(Value::as_str),
            Some("from-derived-db")
        );
    }

    #[test]
    fn derived_db_save_skips_unchanged_projection_rows() {
        let dir = temp_dir("derived-db-diff");
        let state = PocketState::load(dir.clone()).unwrap();
        let first_len = std::fs::metadata(dir.join("data.db")).unwrap().len();
        state.save().unwrap();
        let second_len = std::fs::metadata(dir.join("data.db")).unwrap().len();
        assert_eq!(first_len, second_len);
    }

    #[test]
    fn record_update_persists_exact_typed_row_delta() {
        let dir = temp_dir("record-delta");
        let mut state = PocketState::load(dir.clone()).unwrap();
        let token = create_admin_token(&mut state);
        assert_eq!(
            state
                .route(auth_request(
                    Method::POST,
                    "/api/collections",
                    Some(r#"{"name":"notes","schema":[{"name":"title","type":"text"}]}"#),
                    &token,
                ))
                .status()
                .as_u16(),
            200
        );
        let created = state.route(request(
            Method::POST,
            "/api/collections/notes/records",
            Some(r#"{"title":"one"}"#),
        ));
        let id = parse_response(created).get_str("id").unwrap().to_string();
        let before = std::fs::metadata(dir.join("data.db")).unwrap().len();
        let updated = state.route(request(
            Method::PATCH,
            &format!("/api/collections/notes/records/{id}"),
            Some(r#"{"title":"two"}"#),
        ));
        assert_eq!(updated.status().as_u16(), 200);
        let after = std::fs::metadata(dir.join("data.db")).unwrap().len();
        assert!(after > before);
        assert!(after - before < 12_000);

        let reloaded = PocketState::load(dir).unwrap();
        assert_eq!(
            reloaded
                .collections
                .get("notes")
                .unwrap()
                .records
                .get(&id)
                .unwrap()
                .data
                .get("title")
                .and_then(Value::as_str),
            Some("two")
        );
    }

    #[test]
    fn auth_collection_issues_record_token_without_leaking_password() {
        let dir = temp_dir("auth");
        let mut state = PocketState::load(dir).unwrap();
        let response = state.route(request(
            Method::POST,
            "/api/collections/users/records",
            Some(r#"{"email":"user@example.test","password":"secret"}"#),
        ));
        assert_eq!(response.status().as_u16(), 200);

        let response = state.route(request(
            Method::POST,
            "/api/collections/users/auth-with-password",
            Some(r#"{"identity":"user@example.test","password":"secret"}"#),
        ));
        assert_eq!(response.status().as_u16(), 200);
        let body = parse_response(response);
        assert_eq!(body.get_str("token").unwrap().split('.').count(), 3);
        assert!(body.get("record").unwrap().get("password").is_none());
        let stored = state
            .collections
            .get("users")
            .unwrap()
            .records
            .values()
            .next()
            .unwrap()
            .data
            .get("password")
            .and_then(Value::as_str)
            .unwrap();
        assert!(stored.starts_with("pbkdf2_sha256$"));
        assert!(verify_password("secret", stored));
    }

    #[test]
    fn admin_auth_survives_reload() {
        let dir = temp_dir("admin");
        let mut state = PocketState::load(dir.clone()).unwrap();
        let response = state.route(request(
            Method::POST,
            "/api/admins",
            Some(r#"{"email":"admin@example.test","password":"secret"}"#),
        ));
        assert_eq!(response.status().as_u16(), 200);

        let mut reloaded = PocketState::load(dir).unwrap();
        let response = reloaded.route(request(
            Method::POST,
            "/api/admins/auth-with-password",
            Some(r#"{"identity":"admin@example.test","password":"secret"}"#),
        ));
        assert_eq!(response.status().as_u16(), 200);
    }

    #[test]
    fn signed_token_authorizes_collection_rule() {
        let dir = temp_dir("rules");
        let mut state = PocketState::load(dir).unwrap();
        let admin_token = create_admin_token(&mut state);
        let response = state.route(request(
            Method::POST,
            "/api/collections/users/records",
            Some(r#"{"email":"rule@example.test","password":"secret"}"#),
        ));
        assert_eq!(response.status().as_u16(), 200);
        let auth = state.route(request(
            Method::POST,
            "/api/collections/users/auth-with-password",
            Some(r#"{"identity":"rule@example.test","password":"secret"}"#),
        ));
        let token = parse_response(auth).get_str("token").unwrap().to_string();

        let response = state.route(auth_request(
            Method::POST,
            "/api/collections",
            Some(r#"{"name":"private","listRule":"@request.auth.id != ''","schema":[{"name":"title","type":"text","required":true}]}"#),
            &admin_token,
        ));
        assert_eq!(response.status().as_u16(), 200);

        let response = state.route(request(
            Method::GET,
            "/api/collections/private/records",
            None,
        ));
        assert_eq!(response.status().as_u16(), 403);

        let response = state.route(request_with_header(
            Method::GET,
            "/api/collections/private/records",
            None,
            "Authorization",
            &format!("Bearer {token}"),
        ));
        assert_eq!(response.status().as_u16(), 200);
    }

    #[test]
    fn backups_and_migrations_are_durable() {
        let dir = temp_dir("backup");
        let mut state = PocketState::load(dir.clone()).unwrap();
        let token = create_admin_token(&mut state);
        let response = state.route(auth_request(
            Method::POST,
            "/api/migrations",
            Some(r#"{"name":"init"}"#),
            &token,
        ));
        assert_eq!(response.status().as_u16(), 200);
        let response = state.route(auth_request(Method::POST, "/api/backups", None, &token));
        assert_eq!(response.status().as_u16(), 200);
        let response = state.route(auth_request(Method::GET, "/api/backups", None, &token));
        let body = parse_response(response);
        assert_eq!(body.get_array("items").unwrap().len(), 1);

        let reloaded = PocketState::load(dir).unwrap();
        assert_eq!(reloaded.migrations.len(), 1);
        assert_eq!(reloaded.migrations[0].name, "init");
    }

    #[test]
    fn multipart_file_upload_round_trips_binary_bytes() {
        let dir = temp_dir("files");
        let mut state = PocketState::load(dir).unwrap();
        let token = create_admin_token(&mut state);
        let response = state.route(auth_request(
            Method::POST,
            "/api/collections",
            Some(r#"{"name":"docs","schema":[{"name":"title","type":"text","required":true},{"name":"asset","type":"file","required":false}]}"#),
            &token,
        ));
        assert_eq!(response.status().as_u16(), 200);

        let body = b"--edge\r\nContent-Disposition: form-data; name=\"title\"\r\n\r\nDoc\r\n--edge\r\nContent-Disposition: form-data; name=\"asset\"; filename=\"bytes.bin\"\r\nContent-Type: application/octet-stream\r\n\r\n\x00\xffABC\r\n--edge--\r\n".to_vec();
        let response = state.route(raw_request(
            Method::POST,
            "/api/collections/docs/records",
            body,
            "multipart/form-data; boundary=edge",
        ));
        assert_eq!(response.status().as_u16(), 200);
        let record = parse_response(response);
        let id = record.get_str("id").unwrap().to_string();
        assert_eq!(record.get_str("asset"), Some("bytes.bin"));
        let blob_sidecar = state
            .data_dir
            .join("files")
            .join("docs")
            .join(&id)
            .join("bytes.bin.edgerun-blob");
        assert!(blob_sidecar.exists());

        let response = state.route(request(
            Method::GET,
            &format!("/api/files/docs/{id}/bytes.bin"),
            None,
        ));
        assert_eq!(response.status().as_u16(), 200);
        assert_eq!(response.body(), b"\x00\xffABC");

        let delete = state.route(request(
            Method::DELETE,
            &format!("/api/collections/docs/records/{id}"),
            None,
        ));
        assert_eq!(delete.status().as_u16(), 204);
        let response = state.route(request(
            Method::GET,
            &format!("/api/files/docs/{id}/bytes.bin"),
            None,
        ));
        assert_eq!(response.status().as_u16(), 404);
        assert!(!blob_sidecar.exists());
    }

    #[test]
    fn superuser_routes_require_admin_token_and_settings_logs_crons_persist() {
        let dir = temp_dir("superuser");
        let mut state = PocketState::load(dir.clone()).unwrap();
        assert_eq!(
            state
                .route(request(Method::GET, "/api/settings", None))
                .status()
                .as_u16(),
            403
        );
        let token = create_admin_token(&mut state);

        let response = state.route(auth_request(
            Method::PATCH,
            "/api/settings",
            Some(r#"{"meta":{"appName":"Parity"},"logs":{"maxEntries":50}}"#),
            &token,
        ));
        assert_eq!(response.status().as_u16(), 200);
        assert_eq!(
            parse_response(response)
                .get_object("meta")
                .unwrap()
                .get("appName")
                .and_then(Value::as_str),
            Some("Parity")
        );

        let response = state.route(auth_request(Method::GET, "/api/logs", None, &token));
        assert_eq!(response.status().as_u16(), 200);
        let response = state.route(auth_request(Method::GET, "/api/crons", None, &token));
        assert_eq!(response.status().as_u16(), 200);

        let user = state.route(request(
            Method::POST,
            "/api/collections/users/records",
            Some(r#"{"email":"imp@example.test","password":"secret"}"#),
        ));
        let user_id = parse_response(user).get_str("id").unwrap().to_string();
        let impersonated = state.route(auth_request(
            Method::POST,
            &format!("/api/collections/users/impersonate/{user_id}"),
            None,
            &token,
        ));
        assert_eq!(impersonated.status().as_u16(), 200);
        assert!(parse_response(impersonated).get_str("token").is_some());

        let reloaded = PocketState::load(dir).unwrap();
        assert_eq!(
            reloaded
                .settings
                .get_object("meta")
                .unwrap()
                .get("appName")
                .and_then(Value::as_str),
            Some("Parity")
        );
        assert!(!reloaded.logs.is_empty());

        let manifest = state.route(request(Method::GET, "/_/edgerun-ui-core.json", None));
        assert_eq!(manifest.status().as_u16(), 200);
        assert_eq!(
            parse_response(manifest)
                .get_object("uiCore")
                .unwrap()
                .get("crate")
                .and_then(Value::as_str),
            Some("edgerun-ui-core")
        );
        let shell = state.route(request(Method::GET, "/_/", None));
        assert_eq!(shell.status().as_u16(), 200);
        let shell = String::from_utf8(shell.body().to_vec()).unwrap();
        assert!(shell.contains("<div id=\"app\""));
        assert!(shell.contains("/_/admin-ui.js"));
        assert!(shell.contains("/_/assets/Geist-Variable.ttf"));
        assert!(!shell.contains("<section>"));
        assert!(!shell.contains("<canvas"));
        let host = state.route(request(Method::GET, "/_/admin-ui.js", None));
        assert_eq!(host.status().as_u16(), 200);
        let host = String::from_utf8(host.body().to_vec()).unwrap();
        assert!(host.contains("/_/admin-scene.json"));
        assert!(host.contains("adminViewById"));
        assert!(host.contains("collectionActions"));
        assert!(host.contains("admin.view"));
        assert!(host.contains("admin.collection"));
        assert!(host.contains("scene.fontAtlas.alpha"));
        assert!(host.contains("webgl2"));
        assert!(host.contains("gl.drawArrays"));
        let font = state.route(request(Method::GET, "/_/assets/Geist-Variable.ttf", None));
        assert_eq!(font.status().as_u16(), 200);
        assert!(font.body().len() > 10_000);
        let font_alpha = state.route(request(
            Method::GET,
            "/_/assets/admin-font-alpha-geist-gvar-v2-16.bin?dpr=2",
            None,
        ));
        assert_eq!(font_alpha.status().as_u16(), 200);
        assert!(font_alpha.body().len() > 100_000);
        let icon_alpha = state.route(request(Method::GET, "/_/assets/tabler-alpha.bin", None));
        assert_eq!(icon_alpha.status().as_u16(), 200);
        assert!(icon_alpha.body().len() > 100_000);
        let scene = state.route(request(
            Method::GET,
            "/_/admin-scene.json?w=900&h=640&dpr=2",
            None,
        ));
        assert_eq!(scene.status().as_u16(), 200);
        let scene = parse_response(scene);
        assert_eq!(scene.get_str("surface"), Some("pocketbase-admin"));
        assert_eq!(scene.get_str("renderer"), Some("webgl2"));
        assert_eq!(scene.get_str("view"), Some("collections"));
        assert_eq!(scene.get_str("collection"), Some("users"));
        assert_eq!(
            scene
                .get_object("fontAtlas")
                .and_then(|font| font.get("alpha"))
                .and_then(Value::as_str),
            Some("/_/assets/admin-font-alpha-geist-gvar-v2-16.bin?dpr=2.000")
        );
        assert_eq!(
            scene
                .get_object("fontAtlas")
                .and_then(|font| font.get("rasterPx"))
                .and_then(Value::as_f64),
            Some(32.0)
        );
        assert_eq!(
            scene
                .get_object("font")
                .and_then(|font| font.get("family"))
                .and_then(Value::as_str),
            Some("EdgeRunAdmin")
        );
        assert!(
            scene
                .get_object("packed")
                .and_then(|packed| packed.get("textVertices"))
                .and_then(Value::as_array)
                .is_some_and(|vertices| vertices.len() > 100)
        );
        assert!(
            scene
                .get_object("packed")
                .and_then(|packed| packed.get("iconVertices"))
                .and_then(Value::as_array)
                .is_some_and(|vertices| vertices.len() > 40)
        );
        assert!(
            scene
                .get_array("rects")
                .is_some_and(|rects| rects.len() > 20)
        );
        assert!(
            scene
                .get_array("hits")
                .is_some_and(|hits| hits.iter().any(|hit| hit.get_u64("id") == Some(7101)))
        );
        assert!(
            scene
                .get_object("collectionActions")
                .is_some_and(|actions| actions.get("7200").and_then(Value::as_str) == Some("users"))
        );
        let selected_scene = state.route(request(
            Method::GET,
            "/_/admin-scene.json?w=900&h=640&collection=users",
            None,
        ));
        assert_eq!(selected_scene.status().as_u16(), 200);
        assert_eq!(
            parse_response(selected_scene).get_str("collection"),
            Some("users")
        );
        let logs_scene = state.route(request(
            Method::GET,
            "/_/admin-scene.json?w=900&h=640&view=logs",
            None,
        ));
        assert_eq!(logs_scene.status().as_u16(), 200);
        let logs_scene = parse_response(logs_scene);
        assert_eq!(logs_scene.get_str("view"), Some("logs"));
        assert!(logs_scene.get_array("hits").is_some_and(|hits| {
            hits.iter()
                .any(|hit| hit.get_u64("id").is_some_and(|id| id >= 7600))
        }));
    }

    #[test]
    fn cors_body_limit_and_superuser_cli_have_real_behavior() {
        let dir = temp_dir("middleware-cli");
        run_cli_command(
            dir.clone(),
            CliCommand::SuperuserCreate {
                email: "cli@example.test".to_string(),
                password: "secret".to_string(),
            },
        )
        .unwrap();
        let mut state = PocketState::load(dir.clone()).unwrap();
        assert_eq!(state.admins.len(), 1);
        assert!(
            run_cli_command(
                dir.clone(),
                CliCommand::SuperuserUpdate {
                    email: "missing@example.test".to_string(),
                    password: "secret".to_string(),
                },
            )
            .is_err()
        );

        let token = state
            .route(request(
                Method::POST,
                "/api/admins/auth-with-password",
                Some(r#"{"identity":"cli@example.test","password":"secret"}"#),
            ))
            .body()
            .to_vec();
        let token = from_slice(&token)
            .unwrap()
            .get_str("token")
            .unwrap()
            .to_string();
        let patched = state.route(auth_request(
            Method::PATCH,
            "/api/settings",
            Some(r#"{"cors":{"enabled":true,"origins":["https://app.example"],"methods":["GET","POST"],"headers":["Authorization"]},"request":{"maxBodyBytes":10}}"#),
            &token,
        ));
        assert_eq!(patched.status().as_u16(), 200);

        let preflight = state.route(request_with_header(
            Method::OPTIONS,
            "/api/collections/users/records",
            None,
            "Origin",
            "https://app.example",
        ));
        assert_eq!(preflight.status().as_u16(), 204);
        assert_eq!(
            preflight
                .headers()
                .get("Access-Control-Allow-Origin")
                .unwrap()
                .as_str(),
            "https://app.example"
        );

        let oversized = state.route(request(
            Method::POST,
            "/api/collections/users/records",
            Some(r#"{"email":"too-large@example.test"}"#),
        ));
        assert_eq!(oversized.status().as_u16(), 413);
    }

    #[test]
    fn query_params_batch_and_auth_refresh_have_real_behavior() {
        let dir = temp_dir("query-batch");
        let mut state = PocketState::load(dir).unwrap();
        let token = create_admin_token(&mut state);
        let response = state.route(auth_request(
            Method::POST,
            "/api/collections",
            Some(r#"{"name":"tasks","schema":[{"name":"title","type":"text","required":true},{"name":"rank","type":"number","min":0,"max":10}]}"#),
            &token,
        ));
        assert_eq!(response.status().as_u16(), 200);
        assert_eq!(
            state
                .route(request(
                    Method::POST,
                    "/api/collections/tasks/records",
                    Some(r#"{"title":"bad","rank":11}"#)
                ))
                .status()
                .as_u16(),
            400
        );
        assert_eq!(
            state
                .route(request(
                    Method::POST,
                    "/api/collections/tasks/records",
                    Some(r#"{"title":"A","rank":1}"#)
                ))
                .status()
                .as_u16(),
            200
        );
        assert_eq!(
            state
                .route(request(
                    Method::POST,
                    "/api/collections/tasks/records",
                    Some(r#"{"title":"B","rank":2}"#)
                ))
                .status()
                .as_u16(),
            200
        );
        let response = state.route(request(
            Method::GET,
            "/api/collections/tasks/records?filter=rank>1&sort=-rank&fields=title",
            None,
        ));
        assert_eq!(response.status().as_u16(), 200);
        let body = parse_response(response);
        let items = body.get_array("items").unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].get_str("title"), Some("B"));
        assert!(items[0].get("rank").is_none());
        let response = state.route(request(
            Method::GET,
            "/api/collections/tasks/records?filter=rank=1||rank=2",
            None,
        ));
        assert_eq!(response.status().as_u16(), 200);
        assert_eq!(
            parse_response(response).get_array("items").unwrap().len(),
            2
        );

        let response = state.route(auth_request(
            Method::POST,
            "/api/batch",
            Some(r#"{"requests":[{"method":"POST","url":"/api/collections/tasks/records","body":{"title":"C","rank":3}},{"method":"GET","url":"/api/collections/tasks/records?filter=title='C'"}]}"#),
            &token,
        ));
        assert_eq!(response.status().as_u16(), 200);

        let response = state.route(request(
            Method::POST,
            "/api/collections/users/records",
            Some(r#"{"email":"refresh@example.test","password":"secret"}"#),
        ));
        assert_eq!(response.status().as_u16(), 200);
        let auth = state.route(request(
            Method::POST,
            "/api/collections/users/auth-with-password",
            Some(r#"{"identity":"refresh@example.test","password":"secret"}"#),
        ));
        let record_token = parse_response(auth).get_str("token").unwrap().to_string();
        let refresh = state.route(auth_request(
            Method::POST,
            "/api/collections/users/auth-refresh",
            None,
            &record_token,
        ));
        assert_eq!(refresh.status().as_u16(), 200);
    }

    #[test]
    fn auth_flow_tokens_mutate_records_and_realtime_subscribes() {
        let dir = temp_dir("auth-flows");
        let mut state = PocketState::load(dir).unwrap();
        let response = state.route(request(
            Method::POST,
            "/api/collections/users/records",
            Some(r#"{"email":"flow@example.test","password":"secret","verified":false}"#),
        ));
        assert_eq!(response.status().as_u16(), 200);

        let reset = state.route(request(
            Method::POST,
            "/api/collections/users/request-password-reset",
            Some(r#"{"email":"flow@example.test"}"#),
        ));
        assert_eq!(reset.status().as_u16(), 200);
        let reset_token = parse_response(reset).get_str("token").unwrap().to_string();
        assert_eq!(
            state
                .route(request(
                    Method::POST,
                    "/api/collections/users/confirm-password-reset",
                    Some(&format!(
                        r#"{{"token":"{reset_token}","password":"changed"}}"#
                    )),
                ))
                .status()
                .as_u16(),
            200
        );
        assert_eq!(
            state
                .route(request(
                    Method::POST,
                    "/api/collections/users/auth-with-password",
                    Some(r#"{"identity":"flow@example.test","password":"changed"}"#),
                ))
                .status()
                .as_u16(),
            200
        );

        let verification = state.route(request(
            Method::POST,
            "/api/collections/users/request-verification",
            Some(r#"{"email":"flow@example.test"}"#),
        ));
        let verification_token = parse_response(verification)
            .get_str("token")
            .unwrap()
            .to_string();
        let verified = state.route(request(
            Method::POST,
            "/api/collections/users/confirm-verification",
            Some(&format!(r#"{{"token":"{verification_token}"}}"#)),
        ));
        assert_eq!(parse_response(verified).get_bool("verified"), Some(true));

        let auth = state.route(request(
            Method::POST,
            "/api/collections/users/auth-with-password",
            Some(r#"{"identity":"flow@example.test","password":"changed"}"#),
        ));
        let record_token = parse_response(auth).get_str("token").unwrap().to_string();
        let email_change = state.route(auth_request(
            Method::POST,
            "/api/collections/users/request-email-change",
            Some(r#"{"newEmail":"new-flow@example.test"}"#),
            &record_token,
        ));
        let email_token = parse_response(email_change)
            .get_str("token")
            .unwrap()
            .to_string();
        let changed = state.route(request(
            Method::POST,
            "/api/collections/users/confirm-email-change",
            Some(&format!(r#"{{"token":"{email_token}"}}"#)),
        ));
        let changed = parse_response(changed);
        assert_eq!(changed.get_str("email"), Some("new-flow@example.test"));
        assert_eq!(changed.get_bool("verified"), Some(false));

        let methods = state.route(request(
            Method::GET,
            "/api/collections/users/auth-methods",
            None,
        ));
        let methods = parse_response(methods);
        assert_eq!(
            methods
                .get_object("otp")
                .unwrap()
                .get("enabled")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            methods
                .get_object("oauth2")
                .unwrap()
                .get_object("pkce")
                .unwrap()
                .get("method")
                .and_then(Value::as_str),
            Some("S256")
        );

        let otp = state.route(request(
            Method::POST,
            "/api/collections/users/request-otp",
            Some(r#"{"identity":"new-flow@example.test"}"#),
        ));
        assert_eq!(otp.status().as_u16(), 200);
        let otp = parse_response(otp);
        let otp_id = otp.get_str("otpId").unwrap().to_string();
        let code = otp.get_str("code").unwrap().to_string();
        let otp_auth = state.route(request(
            Method::POST,
            "/api/collections/users/auth-with-otp",
            Some(&format!(r#"{{"otpId":"{otp_id}","password":"{code}"}}"#)),
        ));
        assert_eq!(otp_auth.status().as_u16(), 200);
        assert!(parse_response(otp_auth).get_str("token").is_some());

        let oauth = state.route(request(
            Method::POST,
            "/api/collections/users/auth-with-oauth2",
            Some(
                r#"{"provider":"github","providerId":"gh_1","email":"oauth@example.test","username":"octo"}"#,
            ),
        ));
        assert_eq!(oauth.status().as_u16(), 200);
        let oauth = parse_response(oauth);
        assert!(oauth.get_str("token").is_some());
        assert_eq!(
            oauth.get_object("record").unwrap().get_str("email"),
            Some("oauth@example.test")
        );
        let oauth = state.route(request(
            Method::POST,
            "/api/collections/users/auth-with-oauth2",
            Some(
                r#"{"provider":"github","providerId":"gh_1","email":"oauth@example.test","username":"octo2"}"#,
            ),
        ));
        assert_eq!(oauth.status().as_u16(), 200);
        assert_eq!(
            parse_response(oauth)
                .get_object("record")
                .unwrap()
                .get_str("username"),
            Some("octo2")
        );

        let connect = state.route(request(Method::GET, "/api/realtime", None));
        assert_eq!(connect.status().as_u16(), 200);
        let text = String::from_utf8(connect.body().to_vec()).unwrap();
        let client_id = text
            .split("\"clientId\":\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap()
            .to_string();
        let subscribe = state.route(request(
            Method::POST,
            "/api/realtime",
            Some(&format!(
                r#"{{"clientId":"{client_id}","subscriptions":["users"]}}"#
            )),
        ));
        assert_eq!(subscribe.status().as_u16(), 200);
        assert_eq!(
            parse_response(subscribe)
                .get_array("subscriptions")
                .unwrap()[0]
                .as_str(),
            Some("users")
        );
        let subscribe = state.route(request(
            Method::POST,
            "/api/realtime",
            Some(&format!(
                r#"{{"clientId":"{client_id}","subscriptions":["users"]}}"#
            )),
        ));
        let subscribe = parse_response(subscribe);
        assert!(
            subscribe
                .get_array("events")
                .unwrap()
                .iter()
                .any(|event| event.get_str("collection") == Some("users"))
        );
    }

    #[test]
    fn edgerun_protocol_apis_use_dns_dhcp_and_hook_runtime() {
        let dir = temp_dir("protocol-apis");
        let mut state = PocketState::load(dir).unwrap();
        let token = create_admin_token(&mut state);

        let zones = state.route(auth_request(
            Method::POST,
            "/api/edgerun/dns/zones",
            Some(r#"{"origin":"example.test","records":[{"name":"@","type":"SOA","mname":"ns1.example.test","rname":"admin.example.test"},{"name":"@","type":"A","value":"10.77.0.10","ttl":60},{"name":"@","type":"TXT","value":"edgerun"}]}"#),
            &token,
        ));
        assert_eq!(zones.status().as_u16(), 200);
        let dns = state.route(auth_request(
            Method::POST,
            "/api/edgerun/dns/query",
            Some(r#"{"name":"example.test","type":"A"}"#),
            &token,
        ));
        assert_eq!(dns.status().as_u16(), 200);
        let dns = parse_response(dns);
        assert_eq!(dns.get_array("records").unwrap().len(), 1);
        assert!(dns.get_str("responseWireHex").unwrap().len() > 20);

        let dhcp = state.route(auth_request(
            Method::POST,
            "/api/edgerun/dhcp/discover",
            Some(r#"{"mac":"02:00:00:00:00:09","xid":99}"#),
            &token,
        ));
        assert_eq!(dhcp.status().as_u16(), 200);
        let dhcp = parse_response(dhcp);
        assert_eq!(
            dhcp.get_object("response").unwrap().get_str("type"),
            Some("offer")
        );
        assert_eq!(
            dhcp.get_object("response").unwrap().get_str("yiaddr"),
            Some("10.77.0.50")
        );

        let hook = state.route(auth_request(
            Method::POST,
            "/api/edgerun/hooks/run",
            Some(r#"{"script":"return request.method == 'POST';","method":"POST"}"#),
            &token,
        ));
        assert_eq!(hook.status().as_u16(), 200);
        assert_eq!(parse_response(hook).get_bool("result"), Some(true));
    }

    #[test]
    fn acme_http_dns_tls_alpn_and_autocert_have_real_outputs() {
        let dir = temp_dir("acme-https");
        let mut state = PocketState::load(dir.clone()).unwrap();
        let token = create_admin_token(&mut state);

        let http = state.route(auth_request(
            Method::POST,
            "/api/edgerun/acme/http01",
            Some(r#"{"domain":"example.test","token":"tok_1","accountThumbprint":"thumb_1"}"#),
            &token,
        ));
        assert_eq!(http.status().as_u16(), 200);
        let http = parse_response(http);
        assert_eq!(
            http.get_str("path"),
            Some("/.well-known/acme-challenge/tok_1")
        );
        assert_eq!(http.get_str("responseBody"), Some("tok_1.thumb_1"));
        let challenge = state.route(request(
            Method::GET,
            "/.well-known/acme-challenge/tok_1",
            None,
        ));
        assert_eq!(challenge.status().as_u16(), 200);
        assert_eq!(challenge.body(), b"tok_1.thumb_1");

        let dns = state.route(auth_request(
            Method::POST,
            "/api/edgerun/acme/dns01",
            Some(r#"{"domain":"example.test","token":"tok_1","accountThumbprint":"thumb_1"}"#),
            &token,
        ));
        assert_eq!(dns.status().as_u16(), 200);
        let dns = parse_response(dns);
        assert_eq!(
            dns.get_str("recordName"),
            Some("_acme-challenge.example.test")
        );
        assert_eq!(dns.get_str("recordType"), Some("TXT"));
        assert!(
            dns.get_str("recordValue")
                .is_some_and(|value| !value.is_empty())
        );

        let tls_alpn = state.route(auth_request(
            Method::POST,
            "/api/edgerun/acme/tls-alpn01",
            Some(r#"{"domain":"example.test","token":"tok_1","accountThumbprint":"thumb_1"}"#),
            &token,
        ));
        assert_eq!(tls_alpn.status().as_u16(), 200);
        let tls_alpn = parse_response(tls_alpn);
        assert_eq!(tls_alpn.get_str("alpnProtocol"), Some("acme-tls/1"));
        assert!(
            tls_alpn
                .get_str("challengeValue")
                .is_some_and(|value| !value.is_empty())
        );

        let cert = load_or_create_autocert(&dir, &["example.test".to_string()]).unwrap();
        assert!(!cert.cert_der.is_empty());
        assert!(dir.join("autocert").join("cert.pem").exists());
        assert!(dir.join("autocert").join("key.pem").exists());
        let reloaded = load_or_create_autocert(&dir, &["example.test".to_string()]).unwrap();
        assert_eq!(cert.cert_der, reloaded.cert_der);
    }

    #[test]
    fn admin_ws_commands_use_tape_parse_for_small_control_frames() {
        assert_eq!(
            parse_admin_ws_command(br#"{"type":"admin.hello","width":1280}"#).unwrap(),
            AdminWsCommand::Snapshot
        );
        assert_eq!(
            parse_admin_ws_command(br#"{"type":"admin.refresh"}"#).unwrap(),
            AdminWsCommand::Snapshot
        );
        assert_eq!(
            parse_admin_ws_command(br#"{"type":"admin.action","id":7103}"#).unwrap(),
            AdminWsCommand::Action { id: 7103 }
        );
        assert_eq!(
            parse_admin_ws_command(br#"{"type":"admin.noop"}"#).unwrap(),
            AdminWsCommand::Unknown {
                kind: "admin.noop".to_string()
            }
        );
    }

    fn request(method: Method, path: &str, body: Option<&str>) -> Request {
        let mut builder = Request::builder()
            .method(method)
            .uri(format!("http://127.0.0.1{path}"));
        if let Some(body) = body {
            builder = builder.json_body(body);
        }
        builder.build().unwrap()
    }

    fn request_with_header(
        method: Method,
        path: &str,
        body: Option<&str>,
        header: &str,
        value: &str,
    ) -> Request {
        let mut builder = Request::builder()
            .method(method)
            .uri(format!("http://127.0.0.1{path}"))
            .header(header, value);
        if let Some(body) = body {
            builder = builder.json_body(body);
        }
        builder.build().unwrap()
    }

    fn auth_request(method: Method, path: &str, body: Option<&str>, token: &str) -> Request {
        request_with_header(
            method,
            path,
            body,
            "Authorization",
            &format!("Bearer {token}"),
        )
    }

    fn raw_request(method: Method, path: &str, body: Vec<u8>, content_type: &str) -> Request {
        Request::builder()
            .method(method)
            .uri(format!("http://127.0.0.1{path}"))
            .header("Content-Type", content_type)
            .body(body)
            .build()
            .unwrap()
    }

    fn parse_response(response: Response) -> Value {
        from_slice(response.body()).unwrap()
    }

    fn temp_dir(name: &str) -> PathBuf {
        let path = env::temp_dir().join(format!("edgerun-pocketbase-{name}-{}", id("test")));
        let _ = std::fs::remove_dir_all(&path);
        path
    }

    fn create_admin_token(state: &mut PocketState) -> String {
        let response = state.route(request(
            Method::POST,
            "/api/admins",
            Some(r#"{"email":"admin@example.test","password":"secret"}"#),
        ));
        assert_eq!(response.status().as_u16(), 200);
        let response = state.route(request(
            Method::POST,
            "/api/admins/auth-with-password",
            Some(r#"{"identity":"admin@example.test","password":"secret"}"#),
        ));
        assert_eq!(response.status().as_u16(), 200);
        parse_response(response)
            .get_str("token")
            .unwrap()
            .to_string()
    }
}
