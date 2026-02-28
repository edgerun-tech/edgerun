// SPDX-License-Identifier: Apache-2.0
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{header::CONTENT_TYPE, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::Router;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use clap::Parser;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use edgerun_runtime_proto::tunnel::v1::{
    CreatePairingCodeRequestV1, CreatePairingCodeResponseV1, RegisterEndpointRequestV1,
    RegisterEndpointResponseV1, RegisterWithPairingCodeRequestV1,
    RegisterWithPairingCodeResponseV1, ReserveDomainRequestV1, ReserveDomainResponseV1,
    TunnelHeartbeatRequestV1, TunnelHeartbeatResponseV1,
};
use prost::Message;
use rand::rngs::OsRng;
use rand::RngCore;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use tracing::info;

const DOMAIN_SUFFIX: &str = "users.edgerun.tech";
const SESSION_TTL_MS: u64 = 60_000;
const DEFAULT_PAIRING_TTL_SECONDS: u64 = 300;
const MAX_PAIRING_TTL_SECONDS: u64 = 900;
const DEFAULT_LEASE_TTL_SECONDS: u64 = 900;
const LEASE_SIGNING_SECRET_ENV: &str = "EDGERUN_RELAY_LEASE_HMAC_SECRET";

#[derive(Parser, Debug)]
#[command(
    name = "edgerun-tunnel-relay",
    about = "EdgeRun tunnel relay control service"
)]
struct Cli {
    #[arg(long, default_value = "127.0.0.1:9094")]
    listen: String,
}

#[derive(Clone, Debug)]
struct SessionRecord {
    domain: String,
    node_id: String,
    expires_unix_ms: u64,
}

#[derive(Clone, Debug)]
struct PairingRecord {
    domain: String,
    expires_unix_ms: u64,
    used: bool,
}

#[derive(Default)]
struct Registry {
    sessions: HashMap<String, SessionRecord>,
    pairings: HashMap<String, PairingRecord>,
}

#[derive(Clone, Default)]
struct AppState {
    inner: Arc<Mutex<Registry>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();
    let cli = Cli::parse();

    let app = Router::new()
        .route("/v1/tunnel/reserve-domain", post(handle_reserve_domain))
        .route(
            "/v1/tunnel/register-endpoint",
            post(handle_register_endpoint),
        )
        .route(
            "/v1/tunnel/create-pairing-code",
            post(handle_create_pairing_code),
        )
        .route(
            "/v1/tunnel/register-with-code",
            post(handle_register_with_code),
        )
        .route("/v1/tunnel/heartbeat", post(handle_heartbeat))
        .with_state(AppState::default());

    let listener = tokio::net::TcpListener::bind(&cli.listen).await?;
    info!(listen = %cli.listen, "tunnel relay control listening");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn handle_reserve_domain(State(_state): State<AppState>, body: Bytes) -> Response {
    let request = match ReserveDomainRequestV1::decode(body.as_ref()) {
        Ok(req) => req,
        Err(err) => {
            return proto_response(
                StatusCode::BAD_REQUEST,
                ReserveDomainResponseV1 {
                    ok: false,
                    error: format!("invalid protobuf request: {err}"),
                    ..Default::default()
                },
            )
        }
    };

    let profile_public_key = request.profile_public_key_b64url.trim();
    if profile_public_key.is_empty() {
        return proto_response(
            StatusCode::BAD_REQUEST,
            ReserveDomainResponseV1 {
                ok: false,
                error: "profile_public_key_b64url is required".to_string(),
                ..Default::default()
            },
        );
    }

    let user_id = derive_user_id(profile_public_key);
    let requested_label = sanitize_label(&request.requested_label);
    let label = if requested_label.is_empty() {
        user_id.clone()
    } else {
        requested_label
    };
    let domain = format!("{label}.{DOMAIN_SUFFIX}");

    let registration_token =
        match sign_lease_token(profile_public_key, &domain, DEFAULT_LEASE_TTL_SECONDS) {
            Ok(token) => token,
            Err(err) => {
                return proto_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ReserveDomainResponseV1 {
                        ok: false,
                        error: format!("lease signing failed: {err}"),
                        ..Default::default()
                    },
                )
            }
        };

    proto_response(
        StatusCode::OK,
        ReserveDomainResponseV1 {
            ok: true,
            error: String::new(),
            user_id,
            domain,
            status: "lease_issued".to_string(),
            registration_token,
        },
    )
}

async fn handle_register_endpoint(State(state): State<AppState>, body: Bytes) -> Response {
    let request = match RegisterEndpointRequestV1::decode(body.as_ref()) {
        Ok(req) => req,
        Err(err) => {
            return proto_response(
                StatusCode::BAD_REQUEST,
                RegisterEndpointResponseV1 {
                    ok: false,
                    error: format!("invalid protobuf request: {err}"),
                    ..Default::default()
                },
            )
        }
    };

    if request.domain.trim().is_empty() || request.node_id.trim().is_empty() {
        return proto_response(
            StatusCode::BAD_REQUEST,
            RegisterEndpointResponseV1 {
                ok: false,
                error: "domain and node_id are required".to_string(),
                ..Default::default()
            },
        );
    }

    let mut registry = match state.inner.lock() {
        Ok(lock) => lock,
        Err(_) => {
            return proto_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                RegisterEndpointResponseV1 {
                    ok: false,
                    error: "registry lock poisoned".to_string(),
                    ..Default::default()
                },
            )
        }
    };

    let domain = request.domain.trim().to_string();

    if request.registration_token.trim().is_empty() {
        return proto_response(
            StatusCode::BAD_REQUEST,
            RegisterEndpointResponseV1 {
                ok: false,
                error: "registration_token is required".to_string(),
                ..Default::default()
            },
        );
    }

    if let Err(err) = verify_lease_token(request.registration_token.trim(), &domain) {
        return proto_response(
            StatusCode::FORBIDDEN,
            RegisterEndpointResponseV1 {
                ok: false,
                error: format!("registration lease invalid: {err}"),
                ..Default::default()
            },
        );
    }

    if let Err(err) = verify_endpoint_registration_signature(&request) {
        return proto_response(
            StatusCode::FORBIDDEN,
            RegisterEndpointResponseV1 {
                ok: false,
                error: format!("invalid registration signature: {err}"),
                ..Default::default()
            },
        );
    }

    let now = unix_ms_now();
    let expires = now.saturating_add(SESSION_TTL_MS);
    let session_seed = format!(
        "{}:{}:{}:{}",
        request.domain, request.node_id, request.tunnel_nonce_b64url, now
    );
    let session_id = blake3::hash(session_seed.as_bytes()).to_hex()[0..24].to_string();

    registry.sessions.insert(
        session_id.clone(),
        SessionRecord {
            domain: domain.clone(),
            node_id: request.node_id.trim().to_string(),
            expires_unix_ms: expires,
        },
    );

    proto_response(
        StatusCode::OK,
        RegisterEndpointResponseV1 {
            ok: true,
            error: String::new(),
            session_id,
            relay_url: format!("https://{domain}"),
            expires_unix_ms: expires,
        },
    )
}

async fn handle_create_pairing_code(State(state): State<AppState>, body: Bytes) -> Response {
    let request = match CreatePairingCodeRequestV1::decode(body.as_ref()) {
        Ok(req) => req,
        Err(err) => {
            return proto_response(
                StatusCode::BAD_REQUEST,
                CreatePairingCodeResponseV1 {
                    ok: false,
                    error: format!("invalid protobuf request: {err}"),
                    ..Default::default()
                },
            )
        }
    };

    if request.domain.trim().is_empty() || request.registration_token.trim().is_empty() {
        return proto_response(
            StatusCode::BAD_REQUEST,
            CreatePairingCodeResponseV1 {
                ok: false,
                error: "domain and registration_token are required".to_string(),
                ..Default::default()
            },
        );
    }

    let mut registry = match state.inner.lock() {
        Ok(lock) => lock,
        Err(_) => {
            return proto_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                CreatePairingCodeResponseV1 {
                    ok: false,
                    error: "registry lock poisoned".to_string(),
                    ..Default::default()
                },
            )
        }
    };
    evict_expired_pairings(&mut registry);

    let domain = request.domain.trim().to_string();
    if let Err(err) = verify_lease_token(request.registration_token.trim(), &domain) {
        return proto_response(
            StatusCode::FORBIDDEN,
            CreatePairingCodeResponseV1 {
                ok: false,
                error: format!("registration lease invalid: {err}"),
                ..Default::default()
            },
        );
    }

    let ttl_seconds = if request.ttl_seconds == 0 {
        DEFAULT_PAIRING_TTL_SECONDS
    } else {
        request.ttl_seconds.min(MAX_PAIRING_TTL_SECONDS)
    };
    let now = unix_ms_now();
    let expires_unix_ms = now.saturating_add(ttl_seconds.saturating_mul(1000));

    let pairing_code = loop {
        let candidate = random_pairing_code();
        if !registry.pairings.contains_key(&candidate) {
            break candidate;
        }
    };
    registry.pairings.insert(
        pairing_code.clone(),
        PairingRecord {
            domain: request.domain.trim().to_string(),
            expires_unix_ms,
            used: false,
        },
    );
    let device_command = format!(
        "edgerun-node-manager tunnel-connect --pairing-code {}",
        pairing_code
    );

    proto_response(
        StatusCode::OK,
        CreatePairingCodeResponseV1 {
            ok: true,
            error: String::new(),
            pairing_code,
            expires_unix_ms,
            device_command,
        },
    )
}

async fn handle_register_with_code(State(state): State<AppState>, body: Bytes) -> Response {
    let request = match RegisterWithPairingCodeRequestV1::decode(body.as_ref()) {
        Ok(req) => req,
        Err(err) => {
            return proto_response(
                StatusCode::BAD_REQUEST,
                RegisterWithPairingCodeResponseV1 {
                    ok: false,
                    error: format!("invalid protobuf request: {err}"),
                    ..Default::default()
                },
            )
        }
    };
    if request.pairing_code.trim().is_empty() || request.node_id.trim().is_empty() {
        return proto_response(
            StatusCode::BAD_REQUEST,
            RegisterWithPairingCodeResponseV1 {
                ok: false,
                error: "pairing_code and node_id are required".to_string(),
                ..Default::default()
            },
        );
    }

    let mut registry = match state.inner.lock() {
        Ok(lock) => lock,
        Err(_) => {
            return proto_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                RegisterWithPairingCodeResponseV1 {
                    ok: false,
                    error: "registry lock poisoned".to_string(),
                    ..Default::default()
                },
            )
        }
    };
    evict_expired_pairings(&mut registry);

    let now = unix_ms_now();
    let domain = {
        let Some(pairing) = registry.pairings.get_mut(request.pairing_code.trim()) else {
            return proto_response(
                StatusCode::NOT_FOUND,
                RegisterWithPairingCodeResponseV1 {
                    ok: false,
                    error: "unknown pairing code".to_string(),
                    ..Default::default()
                },
            );
        };
        if pairing.used {
            return proto_response(
                StatusCode::GONE,
                RegisterWithPairingCodeResponseV1 {
                    ok: false,
                    error: "pairing code already used".to_string(),
                    ..Default::default()
                },
            );
        }
        if pairing.expires_unix_ms < now {
            return proto_response(
                StatusCode::GONE,
                RegisterWithPairingCodeResponseV1 {
                    ok: false,
                    error: "pairing code expired".to_string(),
                    ..Default::default()
                },
            );
        }
        pairing.used = true;
        pairing.domain.clone()
    };

    if let Err(err) = verify_pairing_registration_signature(&request) {
        return proto_response(
            StatusCode::FORBIDDEN,
            RegisterWithPairingCodeResponseV1 {
                ok: false,
                error: format!("invalid pairing registration signature: {err}"),
                ..Default::default()
            },
        );
    }
    let expires = now.saturating_add(SESSION_TTL_MS);
    let session_seed = format!(
        "{}:{}:{}:{}",
        domain, request.node_id, request.tunnel_nonce_b64url, now
    );
    let session_id = blake3::hash(session_seed.as_bytes()).to_hex()[0..24].to_string();
    registry.sessions.insert(
        session_id.clone(),
        SessionRecord {
            domain: domain.clone(),
            node_id: request.node_id.trim().to_string(),
            expires_unix_ms: expires,
        },
    );

    proto_response(
        StatusCode::OK,
        RegisterWithPairingCodeResponseV1 {
            ok: true,
            error: String::new(),
            domain: domain.clone(),
            session_id,
            relay_url: format!("https://{domain}"),
            expires_unix_ms: expires,
        },
    )
}

async fn handle_heartbeat(State(state): State<AppState>, body: Bytes) -> Response {
    let request = match TunnelHeartbeatRequestV1::decode(body.as_ref()) {
        Ok(req) => req,
        Err(err) => {
            return proto_response(
                StatusCode::BAD_REQUEST,
                TunnelHeartbeatResponseV1 {
                    ok: false,
                    error: format!("invalid protobuf request: {err}"),
                    ..Default::default()
                },
            )
        }
    };

    let mut registry = match state.inner.lock() {
        Ok(lock) => lock,
        Err(_) => {
            return proto_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                TunnelHeartbeatResponseV1 {
                    ok: false,
                    error: "registry lock poisoned".to_string(),
                    ..Default::default()
                },
            )
        }
    };

    let Some(session) = registry.sessions.get_mut(request.session_id.trim()) else {
        return proto_response(
            StatusCode::NOT_FOUND,
            TunnelHeartbeatResponseV1 {
                ok: false,
                error: "unknown session".to_string(),
                ..Default::default()
            },
        );
    };

    if session.domain != request.domain.trim() || session.node_id != request.node_id.trim() {
        return proto_response(
            StatusCode::FORBIDDEN,
            TunnelHeartbeatResponseV1 {
                ok: false,
                error: "session binding mismatch".to_string(),
                ..Default::default()
            },
        );
    }

    let now = unix_ms_now();
    if session.expires_unix_ms < now {
        return proto_response(
            StatusCode::GONE,
            TunnelHeartbeatResponseV1 {
                ok: false,
                error: "session expired".to_string(),
                ..Default::default()
            },
        );
    }

    session.expires_unix_ms = now.saturating_add(SESSION_TTL_MS);
    proto_response(
        StatusCode::OK,
        TunnelHeartbeatResponseV1 {
            ok: true,
            error: String::new(),
            expires_unix_ms: session.expires_unix_ms,
        },
    )
}

fn proto_response<T: Message>(status: StatusCode, message: T) -> Response {
    let mut encoded = Vec::with_capacity(message.encoded_len());
    if message.encode(&mut encoded).is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR, "encode error").into_response();
    }
    let mut response = (status, encoded).into_response();
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/protobuf"),
    );
    response
}

fn derive_user_id(profile_public_key_b64url: &str) -> String {
    blake3::hash(profile_public_key_b64url.as_bytes()).to_hex()[0..16].to_string()
}

fn random_registration_token() -> String {
    let mut bytes = [0_u8; 32];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn lease_secret_bytes() -> Result<Vec<u8>> {
    let raw = std::env::var(LEASE_SIGNING_SECRET_ENV)
        .map_err(|_| anyhow::anyhow!("{LEASE_SIGNING_SECRET_ENV} is not configured"))?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        anyhow::bail!("{LEASE_SIGNING_SECRET_ENV} is empty");
    }
    Ok(trimmed.as_bytes().to_vec())
}

fn sign_lease_token(profile_public_key_b64url: &str, domain: &str, ttl_seconds: u64) -> Result<String> {
    let now = unix_ms_now();
    let expires_unix_ms = now.saturating_add(ttl_seconds.saturating_mul(1000));
    let nonce = random_registration_token();
    let payload = format!(
        "lease_v1\nprofile_public_key_b64url={profile_public_key_b64url}\ndomain={domain}\nissued_unix_ms={now}\nexpires_unix_ms={expires_unix_ms}\nnonce={nonce}"
    );
    let payload_b64 = URL_SAFE_NO_PAD.encode(payload.as_bytes());
    let secret = lease_secret_bytes()?;
    let mut mac = Hmac::<Sha256>::new_from_slice(&secret)
        .map_err(|err| anyhow::anyhow!("invalid lease secret: {err}"))?;
    mac.update(payload_b64.as_bytes());
    let signature_b64 = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
    Ok(format!("{payload_b64}.{signature_b64}"))
}

fn verify_lease_token(token: &str, expected_domain: &str) -> Result<()> {
    let token = token.trim();
    if token.is_empty() {
        anyhow::bail!("empty token");
    }
    let (payload_b64, signature_b64) = token
        .split_once('.')
        .ok_or_else(|| anyhow::anyhow!("token format invalid"))?;
    let secret = lease_secret_bytes()?;
    let mut mac = Hmac::<Sha256>::new_from_slice(&secret)
        .map_err(|err| anyhow::anyhow!("invalid lease secret: {err}"))?;
    mac.update(payload_b64.as_bytes());
    let signature = URL_SAFE_NO_PAD
        .decode(signature_b64.trim())
        .map_err(|err| anyhow::anyhow!("signature decode failed: {err}"))?;
    mac.verify_slice(&signature)
        .map_err(|_| anyhow::anyhow!("signature mismatch"))?;

    let payload = URL_SAFE_NO_PAD
        .decode(payload_b64.trim())
        .map_err(|err| anyhow::anyhow!("payload decode failed: {err}"))?;
    let payload_text = String::from_utf8(payload).map_err(|err| anyhow::anyhow!("payload utf8 invalid: {err}"))?;
    let mut seen_domain = String::new();
    let mut expires_unix_ms = 0_u64;
    for line in payload_text.lines() {
        if let Some(value) = line.strip_prefix("domain=") {
            seen_domain = value.trim().to_string();
        }
        if let Some(value) = line.strip_prefix("expires_unix_ms=") {
            expires_unix_ms = value.trim().parse::<u64>().unwrap_or(0);
        }
    }
    if seen_domain.is_empty() {
        anyhow::bail!("domain claim missing");
    }
    if seen_domain != expected_domain {
        anyhow::bail!("domain claim mismatch");
    }
    let now = unix_ms_now();
    if expires_unix_ms == 0 || expires_unix_ms < now {
        anyhow::bail!("lease expired");
    }
    Ok(())
}

fn random_pairing_code() -> String {
    let mut bytes = [0_u8; 20];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

fn verify_endpoint_registration_signature(request: &RegisterEndpointRequestV1) -> Result<()> {
    if request.endpoint_public_key_b64url.trim().is_empty()
        || request.registration_signature_b64url.trim().is_empty()
        || request.tunnel_nonce_b64url.trim().is_empty()
    {
        anyhow::bail!(
            "endpoint_public_key_b64url, tunnel_nonce_b64url, and registration_signature_b64url are required"
        );
    }
    let public_key_bytes = URL_SAFE_NO_PAD
        .decode(request.endpoint_public_key_b64url.trim())
        .map_err(|err| anyhow::anyhow!("invalid endpoint_public_key_b64url: {err}"))?;
    let signature_bytes = URL_SAFE_NO_PAD
        .decode(request.registration_signature_b64url.trim())
        .map_err(|err| anyhow::anyhow!("invalid registration_signature_b64url: {err}"))?;

    let public_key_array: [u8; 32] = public_key_bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!("invalid endpoint public key length"))?;
    let signature_array: [u8; 64] = signature_bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!("invalid signature length"))?;

    let verifying_key = VerifyingKey::from_bytes(&public_key_array)
        .map_err(|err| anyhow::anyhow!("invalid endpoint public key: {err}"))?;
    let signature = Signature::from_bytes(&signature_array);
    let message = tunnel_registration_message(
        request.domain.trim(),
        request.node_id.trim(),
        request.tunnel_nonce_b64url.trim(),
    );
    verifying_key
        .verify(message.as_bytes(), &signature)
        .map_err(|err| anyhow::anyhow!("signature verification failed: {err}"))?;
    Ok(())
}

fn verify_pairing_registration_signature(request: &RegisterWithPairingCodeRequestV1) -> Result<()> {
    if request.endpoint_public_key_b64url.trim().is_empty()
        || request.registration_signature_b64url.trim().is_empty()
        || request.tunnel_nonce_b64url.trim().is_empty()
    {
        anyhow::bail!(
            "endpoint_public_key_b64url, tunnel_nonce_b64url, and registration_signature_b64url are required"
        );
    }
    let public_key_bytes = URL_SAFE_NO_PAD
        .decode(request.endpoint_public_key_b64url.trim())
        .map_err(|err| anyhow::anyhow!("invalid endpoint_public_key_b64url: {err}"))?;
    let signature_bytes = URL_SAFE_NO_PAD
        .decode(request.registration_signature_b64url.trim())
        .map_err(|err| anyhow::anyhow!("invalid registration_signature_b64url: {err}"))?;
    let public_key_array: [u8; 32] = public_key_bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!("invalid endpoint public key length"))?;
    let signature_array: [u8; 64] = signature_bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!("invalid signature length"))?;
    let verifying_key = VerifyingKey::from_bytes(&public_key_array)
        .map_err(|err| anyhow::anyhow!("invalid endpoint public key: {err}"))?;
    let signature = Signature::from_bytes(&signature_array);
    let message = tunnel_pairing_registration_message(
        request.pairing_code.trim(),
        request.node_id.trim(),
        request.tunnel_nonce_b64url.trim(),
    );
    verifying_key
        .verify(message.as_bytes(), &signature)
        .map_err(|err| anyhow::anyhow!("signature verification failed: {err}"))?;
    Ok(())
}

fn tunnel_registration_message(domain: &str, node_id: &str, nonce_b64url: &str) -> String {
    format!("edgerun-tunnel-register-v1\ndomain={domain}\nnode_id={node_id}\nnonce={nonce_b64url}")
}

fn tunnel_pairing_registration_message(
    pairing_code: &str,
    node_id: &str,
    nonce_b64url: &str,
) -> String {
    format!(
        "edgerun-tunnel-connect-v1\npairing_code={pairing_code}\nnode_id={node_id}\nnonce={nonce_b64url}"
    )
}

fn sanitize_label(raw: &str) -> String {
    raw.chars()
        .filter_map(|c| {
            if c.is_ascii_alphanumeric() {
                Some(c.to_ascii_lowercase())
            } else if c == '-' {
                Some(c)
            } else {
                None
            }
        })
        .collect::<String>()
}

fn unix_ms_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn evict_expired_pairings(registry: &mut Registry) {
    let now = unix_ms_now();
    registry
        .pairings
        .retain(|_, record| !record.used && record.expires_unix_ms >= now);
}
