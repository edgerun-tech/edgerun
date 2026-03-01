// SPDX-License-Identifier: Apache-2.0
use std::ffi::CString;
use std::fs;
use std::io::Write;
use std::net::SocketAddr;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use axum::extract::ws::{Message as WsMessage, WebSocket, WebSocketUpgrade};
use axum::extract::{Json, Path as AxumPath, Query, State};
use axum::http::header::{
    ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN,
    CACHE_CONTROL, CONTENT_TYPE as AXUM_CONTENT_TYPE, LOCATION,
};
use axum::http::{HeaderValue, StatusCode as AxumStatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, options, post};
use axum::Router;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use clap::{Parser, Subcommand};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use edgerun_hwvault_primitives::hardware::{
    load_or_create_device_signer, random_token_b64url, tpm_nv_available, tpm_nv_read_blob,
    tpm_nv_write_blob, DeviceSigner, HardwareBackend, HardwareSecurityMode,
};
use edgerun_runtime_proto::local::v1::{LocalEventEnvelopeV1, LocalNodeInfoResponseV1};
use edgerun_runtime_proto::tunnel::v1::{
    CreatePairingCodeRequestV1, CreatePairingCodeResponseV1, RegisterEndpointRequestV1,
    RegisterEndpointResponseV1, RegisterWithPairingCodeRequestV1,
    RegisterWithPairingCodeResponseV1, ReserveDomainRequestV1, ReserveDomainResponseV1,
    TunnelHeartbeatRequestV1, TunnelHeartbeatResponseV1,
};
use futures_util::StreamExt;
use prost::Message;
use reqwest::{header::CONTENT_TYPE, Client, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sonic_rs::{JsonContainerTrait, JsonValueMutTrait, JsonValueTrait, Value};
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::process::Command as TokioCommand;
use tokio::sync::broadcast;
use tokio::sync::Mutex;
use tokio::time::sleep;

mod local_bridge;
use local_bridge::{
    handle_local_cloudflare_access_apps, handle_local_cloudflare_dns_records,
    handle_local_cloudflare_dns_upsert, handle_local_cloudflare_pages,
    handle_local_cloudflare_tunnels, handle_local_cloudflare_verify,
    handle_local_cloudflare_workers, handle_local_cloudflare_zones,
    handle_local_docker_container_state, handle_local_docker_summary,
    handle_local_github_workflow_runner_run, handle_local_github_workflow_runner_runs,
    handle_local_github_workflow_runs,
};

const SECURITY_MODE: HardwareSecurityMode = HardwareSecurityMode::TpmRequired;
const CONFIG_TPM_NV_INDEX: u32 = 0x0150_0026;
const CONFIG_TPM_NV_SIZE: usize = 1024;
const CREDENTIALS_TPM_NV_INDEX: u32 = 0x0150_0027;
const CREDENTIALS_TPM_NV_SIZE: usize = 1024;
const GOOGLE_OAUTH_TPM_NV_INDEX: u32 = 0x0150_0028;
const GOOGLE_OAUTH_TPM_NV_SIZE: usize = 1024;
const DEFAULT_API_BASE: &str = "https://api.edgerun.tech";
const DEFAULT_RPC_URL: &str = "local://edgerun";
const DEFAULT_WORKER_BIN: &str = "/usr/bin/edgerun-worker";
const DEFAULT_CRUN_BIN: &str = "/usr/bin/crun";
const WORKER_PID_FILE: &str = "/run/edgerun/edgerun-worker.pid";
const WORKER_RUNTIME_MARKER_FILE: &str = "/run/edgerun/worker-runtime.ready";
const DEFAULT_WORKER_MAX_CONCURRENCY: u32 = 2;
const DEFAULT_WORKER_MEM_BYTES: u64 = 2_147_483_648;
const REQUIRED_CMDLINE_LOCK_TOKEN: &str = "edgerun.locked_cmdline=1";
const EDGE_SECUREBOOT_CERT_DER_PATH: &str =
    "/etc/edgerun/secureboot/edgerun-secureboot-db-cert.der";
const EDGE_SECUREBOOT_CERT_PEM_PATH: &str =
    "/etc/edgerun/secureboot/edgerun-secureboot-db-cert.pem";
const EFI_UPDATEVAR_BIN: &str = "/usr/bin/efi-updatevar";
const BOOT_POLICY_VERIFY_KEY_B64URL_ENV: &str = "EDGERUN_BOOT_POLICY_VERIFY_KEY_B64URL";
const RUNTIME_IMAGE_POLICY_SIGNING_CONTEXT: &str = "edgerun-runtime-image-policy-v1";
const RUNTIME_IMAGE_REQUEST_SIGNING_CONTEXT: &str = "edgerun-runtime-image-request-v1";
const DEFAULT_TUNNEL_CONTROL_BASE: &str = "https://relay.edgerun.tech";
const DEFAULT_LOCAL_BRIDGE_LISTEN: &str = "127.0.0.1:7777";
const LOCAL_BRIDGE_EVENTBUS_PATH: &str = "/v1/local/eventbus/ws";
const LOCAL_DOCKER_SUMMARY_PATH: &str = "/v1/local/docker/summary";
const LOCAL_DOCKER_CONTAINER_STATE_PATH: &str = "/v1/local/docker/container/state";
const LOCAL_FS_ROOT_ENV: &str = "EDGERUN_LOCAL_FS_ROOT";
const LOCAL_FS_META_PATH: &str = "/v1/local/fs/meta";
const LOCAL_FS_LIST_PATH: &str = "/v1/local/fs/list";
const LOCAL_FS_READ_PATH: &str = "/v1/local/fs/read";
const LOCAL_FS_WRITE_PATH: &str = "/v1/local/fs/write";
const LOCAL_FS_MKDIR_PATH: &str = "/v1/local/fs/mkdir";
const LOCAL_FS_DELETE_PATH: &str = "/v1/local/fs/delete";
const LOCAL_FS_MOVE_PATH: &str = "/v1/local/fs/move";
const LOCAL_FS_COPY_PATH: &str = "/v1/local/fs/copy";
const LOCAL_FS_ARCHIVE_PATH: &str = "/v1/local/fs/archive";
const LOCAL_FS_EXTRACT_PATH: &str = "/v1/local/fs/extract";
const LOCAL_CREDENTIALS_STATUS_PATH: &str = "/v1/local/credentials/status";
const LOCAL_CREDENTIALS_LIST_PATH: &str = "/v1/local/credentials/list";
const LOCAL_CREDENTIALS_STORE_PATH: &str = "/v1/local/credentials/store";
const LOCAL_CREDENTIALS_DELETE_PATH: &str = "/v1/local/credentials/delete";
const LOCAL_CREDENTIALS_LOCK_PATH: &str = "/v1/local/credentials/lock";
const LOCAL_CREDENTIALS_UNLOCK_PATH: &str = "/v1/local/credentials/unlock";
const LOCAL_CREDENTIALS_INTEGRATION_TOKEN_PATH: &str = "/v1/local/credentials/integration-token";
const LOCAL_CREDENTIALS_GOOGLE_OAUTH_PATH: &str = "/v1/local/credentials/google-oauth";
const LOCAL_MCP_START_PATH: &str = "/v1/local/mcp/integration/start";
const LOCAL_MCP_STOP_PATH: &str = "/v1/local/mcp/integration/stop";
const LOCAL_MCP_STATUS_PATH: &str = "/v1/local/mcp/integration/status";
const LOCAL_MCP_PREFLIGHT_PATH: &str = "/v1/local/mcp/integration/preflight";
const LOCAL_CLOUDFLARE_VERIFY_PATH: &str = "/v1/local/cloudflare/verify";
const LOCAL_CLOUDFLARE_ZONES_PATH: &str = "/v1/local/cloudflare/zones";
const LOCAL_CLOUDFLARE_TUNNELS_PATH: &str = "/v1/local/cloudflare/tunnels";
const LOCAL_CLOUDFLARE_ACCESS_APPS_PATH: &str = "/v1/local/cloudflare/access/apps";
const LOCAL_CLOUDFLARE_WORKERS_PATH: &str = "/v1/local/cloudflare/workers";
const LOCAL_CLOUDFLARE_PAGES_PATH: &str = "/v1/local/cloudflare/pages";
const LOCAL_CLOUDFLARE_DNS_RECORDS_PATH: &str = "/v1/local/cloudflare/dns/records";
const LOCAL_CLOUDFLARE_DNS_UPSERT_PATH: &str = "/v1/local/cloudflare/dns/records/upsert";
const LOCAL_GITHUB_WORKFLOW_RUNS_PATH: &str = "/v1/local/github/workflow/runs";
const LOCAL_GITHUB_WORKFLOW_RUNNER_RUNS_PATH: &str = "/v1/local/github/workflow/runner/runs";
const LOCAL_GITHUB_WORKFLOW_RUNNER_RUN_PATH: &str = "/v1/local/github/workflow/runner/run";
const LOCAL_BEEPER_VERIFY_PATH: &str = "/v1/local/beeper/verify";
const LOCAL_BEEPER_CHATS_PATH: &str = "/v1/local/beeper/chats";
const LOCAL_BEEPER_MESSAGES_PATH: &str = "/v1/local/beeper/messages";
const LOCAL_BEEPER_IMPORTED_PATH: &str = "/v1/local/beeper/imported";
const LOCAL_BEEPER_MEDIA_PATH: &str = "/v1/local/beeper/media";
const LOCAL_BEEPER_SEND_PATH: &str = "/v1/local/beeper/send";
const LOCAL_TAILSCALE_DEVICES_PATH: &str = "/v1/local/tailscale/devices";
const LOCAL_ONVIF_DISCOVER_PATH: &str = "/v1/local/onvif/discover";
const LOCAL_GOOGLE_MESSAGES_PATH: &str = "/v1/local/google/messages";
const LOCAL_GOOGLE_MESSAGE_PATH: &str = "/v1/local/google/message/{id}";
const LOCAL_GOOGLE_EVENTS_PATH: &str = "/v1/local/google/events";
const LOCAL_GOOGLE_CONTACTS_PATH: &str = "/v1/local/google/contacts";
const LOCAL_GOOGLE_DRIVE_FILES_PATH: &str = "/v1/local/google/drive/files";
const LOCAL_GOOGLE_DRIVE_FILE_PATH: &str = "/v1/local/google/drive/file/{id}";
const LOCAL_GOOGLE_PHOTOS_PATH: &str = "/v1/local/google/photos";
const LOCAL_GOOGLE_REFRESH_PATH: &str = "/v1/local/google/refresh";
const LOCAL_GOOGLE_OAUTH_START_PATH: &str = "/v1/local/google/oauth/start";
const LOCAL_GOOGLE_OAUTH_CALLBACK_PATH: &str = "/v1/local/google/oauth/callback";
const EVENTBUS_NATS_URL_ENV: &str = "EDGERUN_EVENTBUS_NATS_URL";
const EVENTBUS_NATS_URL_DEFAULT: &str = "nats://127.0.0.1:4222";
const INTEGRATION_TOPIC_ROOT_ENV: &str = "EDGERUN_INTEGRATION_TOPIC_ROOT";
const INTEGRATION_TOPIC_ROOT_DEFAULT: &str = "edgerun.integrations";
const LOCAL_ASSISTANT_PATH: &str = "/v1/local/assistant";
const LOCAL_DOCKER_EVENTS_TOPIC: &str = "local.docker.events";
const OPENCODE_CLI_CONTAINER_NAME: &str = "edgerun-opencode-cli";
const OPENCODE_CONFIG_PATH: &str = "/home/ken/.config/opencode/opencode.jsonb";
const OPENCODE_MCP_SCHEMA_URL: &str = "https://opencode.ai/config.json";
const OPENCODE_MANAGED_MCP_PREFIX: &str = "edgerun-";
const MCP_IMAGE_GITHUB_DEFAULT: &str = "ghcr.io/github/github-mcp-server:latest";
const MCP_IMAGE_GOOGLE_MESSAGES_DEFAULT: &str = "dock.mau.dev/mautrix/gmessages:latest";
const MCP_IMAGE_GVOICE_DEFAULT: &str = "dock.mau.dev/mautrix/gvoice:latest";
const MCP_IMAGE_GOOGLECHAT_DEFAULT: &str = "dock.mau.dev/mautrix/googlechat:latest";
const BEEPER_DESKTOP_API_BASE_ENV: &str = "BEEPER_DESKTOP_API_BASE";
const BEEPER_DESKTOP_API_BASE_DEFAULT: &str = "http://127.0.0.1:23373";
const BEEPER_IMPORT_DIR_ENV: &str = "BEEPER_IMPORT_DIR";
const BEEPER_IMPORT_DIR_DEFAULT: &str = "/workspace/edgerun/out/imports/fbmessages";
const BEEPER_MEDIA_ROOT_ENV: &str = "BEEPER_MEDIA_ROOT";
const BEEPER_MEDIA_ROOT_DEFAULT: &str = "/home/ken/.config/BeeperTexts/media";
const GOOGLE_OAUTH_CLIENT_ID_ENV: &str = "GOOGLE_OAUTH_CLIENT_ID";
const GOOGLE_OAUTH_CLIENT_SECRET_ENV: &str = "GOOGLE_OAUTH_CLIENT_SECRET";
const GOOGLE_OAUTH_REDIRECT_ORIGIN_ENV: &str = "GOOGLE_OAUTH_REDIRECT_ORIGIN";
const GOOGLE_OAUTH_REDIRECT_ORIGIN_DEFAULT: &str = "https://osdev.edgerun.tech";
const GOOGLE_OAUTH_SCOPES_BASE: &str = "openid profile https://www.googleapis.com/auth/userinfo.email";
const GOOGLE_OAUTH_SCOPES_PRODUCTIVITY: &str = "https://www.googleapis.com/auth/gmail.readonly https://www.googleapis.com/auth/contacts.readonly https://www.googleapis.com/auth/calendar.readonly https://www.googleapis.com/auth/drive.readonly";
const GOOGLE_OAUTH_SCOPES_PHOTOS_READONLY: &str = "https://www.googleapis.com/auth/photoslibrary.readonly";
const GOOGLE_OAUTH_SCOPES_PHOTOS_APPEND: &str = "https://www.googleapis.com/auth/photoslibrary.appendonly";
const GOOGLE_OAUTH_SCOPES_PHOTOS_READONLY_APPCREATED: &str = "https://www.googleapis.com/auth/photoslibrary.readonly.appcreateddata";
const GOOGLE_OAUTH_SCOPES_PHOTOS_EDIT_APPCREATED: &str = "https://www.googleapis.com/auth/photoslibrary.edit.appcreateddata";
const LOCAL_BRIDGE_VERSION: &str = "v1";

#[derive(Parser, Debug)]
#[command(name = "edgerun-node-manager", about = "EdgeRun TPM node manager")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Configure {
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        rpc_url: String,
        #[arg(long, default_value_t = 15)]
        heartbeat_secs: u64,
    },
    Bond {
        #[arg(long)]
        owner_pubkey: String,
    },
    Identity,
    Register {
        #[arg(long)]
        owner_pubkey: String,
    },
    TunnelReserve {
        #[arg(long, default_value = DEFAULT_TUNNEL_CONTROL_BASE)]
        relay_control_base: String,
        #[arg(long, default_value = "")]
        requested_label: String,
    },
    TunnelRegister {
        #[arg(long, default_value = DEFAULT_TUNNEL_CONTROL_BASE)]
        relay_control_base: String,
        #[arg(long)]
        domain: String,
        #[arg(long)]
        node_id: String,
        #[arg(long)]
        registration_token: String,
    },
    TunnelCreateCode {
        #[arg(long, default_value = DEFAULT_TUNNEL_CONTROL_BASE)]
        relay_control_base: String,
        #[arg(long)]
        domain: String,
        #[arg(long)]
        registration_token: String,
        #[arg(long, default_value_t = 300)]
        ttl_seconds: u64,
    },
    TunnelConnect {
        #[arg(long, default_value = DEFAULT_TUNNEL_CONTROL_BASE)]
        relay_control_base: String,
        #[arg(long)]
        pairing_code: String,
        #[arg(long, default_value = "")]
        node_id: String,
    },
    TunnelHeartbeat {
        #[arg(long, default_value = DEFAULT_TUNNEL_CONTROL_BASE)]
        relay_control_base: String,
        #[arg(long)]
        domain: String,
        #[arg(long)]
        node_id: String,
        #[arg(long)]
        session_id: String,
    },
    Run {
        #[arg(long, default_value = DEFAULT_LOCAL_BRIDGE_LISTEN)]
        local_bridge_listen: String,
    },
}

#[derive(Debug, Deserialize)]
struct DeviceChallengeResponse {
    nonce_b64url: String,
}

#[derive(Debug, Serialize)]
struct DeviceHandshakeRequest<'a> {
    owner_pubkey: &'a str,
    nonce_b64url: &'a str,
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
    ok: bool,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RuntimeImageResponse {
    ok: bool,
    #[serde(default)]
    image_tag: Option<String>,
    #[serde(default)]
    image_ref: Option<String>,
    #[serde(default)]
    request_nonce_b64url: Option<String>,
    #[serde(default)]
    issued_at_unix_s: Option<u64>,
    #[serde(default)]
    valid_until_unix_s: Option<u64>,
    #[serde(default)]
    rollback_index: Option<u64>,
    #[serde(default)]
    signature_b64url: Option<String>,
    #[serde(default)]
    signing_key_id: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct BondRequest<'a> {
    owner_pubkey: &'a str,
    device_pubkey_b64url: &'a str,
}

#[derive(Debug, Serialize)]
struct NodeInitRequest<'a> {
    owner_pubkey: &'a str,
    device_pubkey_b64url: &'a str,
    rpc_url: &'a str,
}

#[derive(Debug, Serialize)]
struct NodeHeartbeatRequest<'a> {
    owner_pubkey: &'a str,
    device_pubkey_b64url: &'a str,
    rpc_url: &'a str,
    version: &'a str,
    pid1: bool,
}

#[derive(Debug, Serialize)]
struct RuntimeImageRequest<'a> {
    owner_pubkey: &'a str,
    device_pubkey_b64url: &'a str,
    rpc_url: &'a str,
    request_nonce_b64url: &'a str,
    request_issued_at_unix_s: u64,
    request_signature_b64url: &'a str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ManagerConfig {
    api_base: String,
    rpc_url: String,
    worker_max_concurrency: u32,
    worker_mem_bytes: u64,
    #[serde(default)]
    runtime_image_ref: Option<String>,
    #[serde(default)]
    runtime_image_pulled: bool,
    #[serde(default)]
    runtime_policy_rollback_index: u64,
    heartbeat_secs: u64,
    bonded: bool,
    node_initialized: bool,
    owner_pubkey: Option<String>,
    #[serde(default)]
    credentials: Vec<LocalCredentialEntry>,
}

#[derive(Debug)]
struct BootPolicy {
    owner_pubkey: Option<String>,
}

struct LocalBridgeState {
    node_id: String,
    device_pubkey_b64url: String,
    local_fs_root: PathBuf,
    started_unix_ms: u64,
    tx: broadcast::Sender<LocalEventEnvelopeV1>,
    credentials_lock: Mutex<()>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LocalCredentialEntry {
    entry_id: String,
    credential_type: String,
    name: String,
    username: String,
    secret: String,
    url: String,
    note: String,
    tags: String,
    folder: String,
    created_unix_ms: u64,
    updated_unix_ms: u64,
}

#[derive(Debug, Serialize)]
struct LocalDockerEventPayload {
    event_type: String,
    action: String,
    container_id: String,
    container_name: String,
    message: String,
    ts_unix_ms: u64,
}

#[derive(Debug, Serialize)]
struct LocalBridgeEventPayloadEnvelope {
    payload: sonic_rs::Value,
    meta: sonic_rs::Value,
}

#[derive(Debug, Deserialize)]
struct LocalFsQuery {
    #[serde(default)]
    path: Option<String>,
    #[serde(default, alias = "nodeId")]
    node_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LocalFsWriteRequest {
    path: String,
    content: String,
    #[serde(default, alias = "nodeId")]
    node_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LocalFsPathRequest {
    path: String,
    #[serde(default, alias = "nodeId")]
    node_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LocalFsMoveRequest {
    from: String,
    to: String,
    #[serde(default, alias = "nodeId")]
    node_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LocalFsArchiveRequest {
    path: String,
    #[serde(default)]
    format: Option<String>,
    #[serde(default, alias = "nodeId")]
    node_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct LocalFsResponse<T>
where
    T: Serialize,
{
    ok: bool,
    error: String,
    #[serde(flatten)]
    data: T,
}

#[derive(Debug, Serialize)]
struct LocalFsMetaData {
    #[serde(rename = "localFsRoot")]
    local_fs_root: String,
}

#[derive(Debug, Serialize)]
struct LocalFsListData {
    entries: Vec<LocalFsEntry>,
}

#[derive(Debug, Serialize)]
struct LocalFsReadData {
    content: String,
}

#[derive(Debug, Serialize)]
struct LocalFsArchiveData {
    archive_path: String,
}

#[derive(Debug, Serialize)]
struct LocalFsEmptyData {}

#[derive(Debug, Serialize)]
struct LocalCredentialsStatusData {
    installed: bool,
    locked: bool,
    count: u64,
    backend: String,
}

#[derive(Debug, Serialize)]
struct LocalCredentialsListData {
    entries: Vec<LocalCredentialEntry>,
    count: u64,
    locked: bool,
}

#[derive(Debug, Serialize)]
struct LocalCredentialTokenData {
    integration_id: String,
    token: String,
}

#[derive(Debug, Serialize)]
struct LocalFsEntry {
    id: String,
    provider: String,
    path: String,
    name: String,
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct LocalMcpStartRequest {
    integration_id: String,
    token: String,
    #[serde(default, alias = "nodeId")]
    node_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LocalCredentialStoreRequest {
    #[serde(default, alias = "credentialType")]
    credential_type: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    secret: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    note: Option<String>,
    #[serde(default)]
    tags: Option<String>,
    #[serde(default)]
    folder: Option<String>,
    #[serde(default, alias = "entryId")]
    entry_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LocalCredentialDeleteRequest {
    #[serde(default, alias = "entryId")]
    entry_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LocalCredentialUnlockRequest {
    #[serde(default, rename = "reason")]
    _reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LocalGoogleOauthCredentialStoreRequest {
    client_id: String,
    client_secret: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct GoogleOauthStoredSecrets {
    #[serde(default)]
    client_id: String,
    #[serde(default)]
    client_secret: String,
}

#[derive(Debug, Deserialize)]
struct LocalCredentialTokenQuery {
    #[serde(default, alias = "integrationId")]
    integration_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LocalMcpStopRequest {
    integration_id: String,
    #[serde(default, alias = "nodeId")]
    node_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LocalMcpStatusQuery {
    integration_id: String,
    #[serde(default, alias = "nodeId")]
    node_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LocalMcpPreflightQuery {
    integration_id: String,
    #[serde(default, alias = "nodeId")]
    node_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LocalBeeperVerifyRequest {
    #[serde(default)]
    token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LocalBeeperChatsQuery {
    token: String,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct LocalBeeperMessagesQuery {
    token: String,
    #[serde(default, alias = "chatId")]
    chat_id: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct LocalBeeperImportedQuery {
    #[serde(default)]
    limit_threads: Option<usize>,
    #[serde(default)]
    limit_messages: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct LocalBeeperMediaQuery {
    uri: String,
}

#[derive(Debug, Deserialize)]
struct LocalBeeperSendRequest {
    token: String,
    #[serde(default, alias = "chatId")]
    chat_id: Option<String>,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ImportedFbThread {
    #[serde(default)]
    participants: Vec<String>,
    #[serde(default, alias = "threadName")]
    thread_name: String,
    #[serde(default)]
    messages: Vec<ImportedFbMessage>,
}

#[derive(Debug, Deserialize)]
struct ImportedFbMessage {
    #[serde(default, alias = "senderName")]
    sender_name: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    timestamp: i64,
    #[serde(default, alias = "type")]
    message_type: String,
    #[serde(default)]
    media: Vec<ImportedFbMedia>,
}

#[derive(Debug, Deserialize)]
struct ImportedFbMedia {
    #[serde(default)]
    uri: String,
}

#[derive(Debug, Deserialize)]
struct LocalTailscaleDevicesRequest {
    #[serde(default, alias = "apiKey")]
    api_key: Option<String>,
    #[serde(default)]
    tailnet: Option<String>,
}

#[derive(Debug, Serialize)]
struct LocalMcpStatusData {
    integration_id: String,
    container_name: String,
    running: bool,
    status: String,
}

#[derive(Debug, Serialize)]
struct LocalMcpPreflightData {
    integration_id: String,
    container_name: String,
    token_env: String,
    image: String,
    image_resolved: bool,
}

#[derive(Debug, Deserialize)]
struct LocalGoogleMessagesQuery {
    token: String,
    #[serde(default)]
    after: Option<u64>,
    #[serde(default, alias = "maxResults")]
    max_results: Option<u32>,
    #[serde(default)]
    limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct LocalGoogleMessageQuery {
    token: String,
}

#[derive(Debug, Deserialize)]
struct LocalGoogleEventsQuery {
    token: String,
    #[serde(default)]
    limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct LocalGoogleContactsQuery {
    token: String,
    #[serde(default)]
    limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct LocalGoogleDriveFilesQuery {
    token: String,
    #[serde(default, alias = "parentId")]
    parent_id: Option<String>,
    #[serde(default, alias = "pageSize")]
    page_size: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct LocalGoogleDriveFileQuery {
    token: String,
}

#[derive(Debug, Deserialize)]
struct LocalGooglePhotosQuery {
    token: String,
    #[serde(default, alias = "pageSize")]
    page_size: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct LocalOnvifDiscoverQuery {
    #[serde(default, alias = "waitMs")]
    wait_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct LocalGoogleRefreshRequest {
    refresh_token: String,
}

#[derive(Debug, Deserialize)]
struct LocalGoogleOauthStartQuery {
    #[serde(default, alias = "returnTo")]
    return_to: Option<String>,
    #[serde(default, alias = "integrationId")]
    integration_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LocalGoogleOauthCallbackQuery {
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LocalAssistantRequest {
    message: String,
    #[serde(default)]
    provider: Option<String>,
    #[serde(default, alias = "sessionId")]
    session_id: Option<String>,
    #[serde(default, alias = "threadId")]
    thread_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct LocalAssistantResponse {
    ok: bool,
    error: String,
    message: String,
    #[serde(default)]
    actions: Vec<String>,
    #[serde(default, rename = "statusEvents")]
    status_events: Vec<LocalAssistantStatusEvent>,
    #[serde(default, rename = "sessionId")]
    session_id: String,
    #[serde(default, rename = "threadId")]
    thread_id: String,
}

#[derive(Debug, Serialize)]
struct LocalAssistantStatusEvent {
    #[serde(rename = "type")]
    event_type: String,
    label: String,
    detail: String,
}

#[derive(Debug, Deserialize)]
struct OpenCodeRunPart {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    text: Option<String>,
    #[serde(default, rename = "sessionID")]
    session_id: Option<String>,
    #[serde(default, rename = "sessionId")]
    session_id_alt: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenCodeRunEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default, rename = "sessionID")]
    session_id: Option<String>,
    #[serde(default, rename = "sessionId")]
    session_id_alt: Option<String>,
    #[serde(default)]
    part: Option<OpenCodeRunPart>,
}

#[tokio::main]
async fn main() {
    if let Err(err) = run_main().await {
        eprintln!("Error: {err:#}");
        if std::process::id() == 1 {
            eprintln!("pid1 entering hold loop after fatal startup error");
            loop {
                std::thread::sleep(Duration::from_secs(60));
            }
        }
        std::process::exit(1);
    }
}

async fn run_main() -> Result<()> {
    bootstrap_pid1_runtime();
    let cli = Cli::parse();
    match cli.command.unwrap_or(Commands::Run {
        local_bridge_listen: DEFAULT_LOCAL_BRIDGE_LISTEN.to_string(),
    }) {
        Commands::Configure {
            rpc_url,
            heartbeat_secs,
        } => cmd_configure(&rpc_url, heartbeat_secs),
        Commands::Bond { owner_pubkey } => cmd_bond(&owner_pubkey),
        Commands::Identity => cmd_identity(),
        Commands::Register { owner_pubkey } => cmd_register(&owner_pubkey).await,
        Commands::TunnelReserve {
            relay_control_base,
            requested_label,
        } => cmd_tunnel_reserve(&relay_control_base, &requested_label).await,
        Commands::TunnelRegister {
            relay_control_base,
            domain,
            node_id,
            registration_token,
        } => cmd_tunnel_register(&relay_control_base, &domain, &node_id, &registration_token).await,
        Commands::TunnelCreateCode {
            relay_control_base,
            domain,
            registration_token,
            ttl_seconds,
        } => {
            cmd_tunnel_create_code(
                &relay_control_base,
                &domain,
                &registration_token,
                ttl_seconds,
            )
            .await
        }
        Commands::TunnelConnect {
            relay_control_base,
            pairing_code,
            node_id,
        } => cmd_tunnel_connect(&relay_control_base, &pairing_code, &node_id).await,
        Commands::TunnelHeartbeat {
            relay_control_base,
            domain,
            node_id,
            session_id,
        } => cmd_tunnel_heartbeat(&relay_control_base, &domain, &node_id, &session_id).await,
        Commands::Run {
            local_bridge_listen,
        } => cmd_run(&local_bridge_listen).await,
    }
}

fn bootstrap_pid1_runtime() {
    if std::process::id() != 1 {
        return;
    }
    std::env::set_var("PATH", "/usr/bin:/usr/sbin:/bin:/sbin");
    for dir in ["/proc", "/sys", "/dev", "/run"] {
        let _ = fs::create_dir_all(dir);
    }
    let _ = mount_fs("proc", "/proc", "proc");
    let _ = mount_fs("sysfs", "/sys", "sysfs");
    let _ = mount_fs("devtmpfs", "/dev", "devtmpfs");
    let _ = fs::create_dir_all("/sys/firmware/efi/efivars");
    let _ = mount_fs("efivarfs", "/sys/firmware/efi/efivars", "efivarfs");
    let _ = mount_fs("tmpfs", "/run", "tmpfs");
}

fn mount_fs(source: &str, target: &str, fstype: &str) -> Result<()> {
    let target_display = target.to_string();
    let source = CString::new(source).context("invalid source cstring")?;
    let target = CString::new(target).context("invalid target cstring")?;
    let fstype = CString::new(fstype).context("invalid fstype cstring")?;
    // SAFETY: pointers are valid for the duration of the call and nul-terminated.
    let rc = unsafe {
        libc::mount(
            source.as_ptr(),
            target.as_ptr(),
            fstype.as_ptr(),
            0,
            std::ptr::null(),
        )
    };
    if rc == 0 {
        return Ok(());
    }
    let err = std::io::Error::last_os_error();
    if err.raw_os_error() == Some(libc::EBUSY) {
        return Ok(());
    }
    Err(anyhow!("mount failed for {target_display}: {err}"))
}

fn load_tpm_signer() -> Result<DeviceSigner> {
    wait_for_tpm_ready();
    let signer = load_or_create_device_signer(SECURITY_MODE)
        .context("failed to initialize TPM-backed device signer")?;
    if signer.backend != HardwareBackend::Tpm {
        return Err(anyhow!(
            "TPM is required; refusing non-TPM backend: {:?}",
            signer.backend
        ));
    }
    Ok(signer)
}

fn wait_for_tpm_ready() {
    if tpm_nv_available() {
        return;
    }
    // Early PID1 startup can race TPM char device creation in initramfs boot.
    for _ in 0..120 {
        std::thread::sleep(Duration::from_millis(250));
        if tpm_nv_available() {
            return;
        }
    }
}

fn cmd_identity() -> Result<()> {
    let signer = load_tpm_signer()?;
    println!("backend=tpm");
    println!("device_pubkey_b64url={}", signer.public_key_b64url);
    Ok(())
}

fn decode_nv_json_blob<T: DeserializeOwned>(blob: &[u8], nv_size: usize, label: &str) -> Result<T> {
    if blob.len() < 4 {
        return Err(anyhow!("invalid TPM {label} blob"));
    }
    let len = u32::from_le_bytes([blob[0], blob[1], blob[2], blob[3]]) as usize;
    if len > (nv_size - 4) {
        return Err(anyhow!("invalid TPM {label} length: {len}"));
    }
    let raw = &blob[4..4 + len];
    sonic_rs::from_slice(raw).with_context(|| format!("failed to parse TPM {label} json"))
}

fn encode_nv_json_blob<T: Serialize + ?Sized>(value: &T, nv_size: usize, label: &str) -> Result<Vec<u8>> {
    let payload = sonic_rs::to_vec(value).with_context(|| format!("failed to encode TPM {label} json"))?;
    if payload.len() > (nv_size - 4) {
        return Err(anyhow!(
            "{label} too large for TPM NV ({} > {})",
            payload.len(),
            nv_size - 4
        ));
    }
    let mut blob = vec![0_u8; nv_size];
    let len = payload.len() as u32;
    blob[0..4].copy_from_slice(&len.to_le_bytes());
    blob[4..4 + payload.len()].copy_from_slice(&payload);
    Ok(blob)
}

fn load_credentials_from_nv() -> Result<Vec<LocalCredentialEntry>> {
    let blob = tpm_nv_read_blob(CREDENTIALS_TPM_NV_INDEX, CREDENTIALS_TPM_NV_SIZE)
        .context("failed to read credentials from TPM NV")?;
    decode_nv_json_blob(&blob, CREDENTIALS_TPM_NV_SIZE, "credentials")
}

fn save_credentials_to_nv(credentials: &[LocalCredentialEntry]) -> Result<()> {
    let blob = encode_nv_json_blob(credentials, CREDENTIALS_TPM_NV_SIZE, "credentials")?;
    tpm_nv_write_blob(CREDENTIALS_TPM_NV_INDEX, &blob, CREDENTIALS_TPM_NV_SIZE)
        .context("failed to store credentials in TPM NV")?;
    Ok(())
}

fn load_google_oauth_secrets_from_nv() -> Result<GoogleOauthStoredSecrets> {
    let blob = tpm_nv_read_blob(GOOGLE_OAUTH_TPM_NV_INDEX, GOOGLE_OAUTH_TPM_NV_SIZE)
        .context("failed to read google oauth secrets from TPM NV")?;
    decode_nv_json_blob(&blob, GOOGLE_OAUTH_TPM_NV_SIZE, "google oauth secrets")
}

fn save_google_oauth_secrets_to_nv(secrets: &GoogleOauthStoredSecrets) -> Result<()> {
    let blob = encode_nv_json_blob(secrets, GOOGLE_OAUTH_TPM_NV_SIZE, "google oauth secrets")?;
    tpm_nv_write_blob(GOOGLE_OAUTH_TPM_NV_INDEX, &blob, GOOGLE_OAUTH_TPM_NV_SIZE)
        .context("failed to store google oauth secrets in TPM NV")?;
    Ok(())
}

fn load_config() -> Result<ManagerConfig> {
    let blob = tpm_nv_read_blob(CONFIG_TPM_NV_INDEX, CONFIG_TPM_NV_SIZE)
        .context("failed to read manager config from TPM NV")?;
    let mut cfg: ManagerConfig = decode_nv_json_blob(&blob, CONFIG_TPM_NV_SIZE, "config")?;
    if let Ok(credentials) = load_credentials_from_nv() {
        cfg.credentials = credentials;
    }
    Ok(cfg)
}

fn default_manager_config() -> ManagerConfig {
    ManagerConfig {
        api_base: DEFAULT_API_BASE.to_string(),
        rpc_url: DEFAULT_RPC_URL.to_string(),
        worker_max_concurrency: DEFAULT_WORKER_MAX_CONCURRENCY,
        worker_mem_bytes: DEFAULT_WORKER_MEM_BYTES,
        runtime_image_ref: None,
        runtime_image_pulled: false,
        runtime_policy_rollback_index: 0,
        heartbeat_secs: 15,
        bonded: false,
        node_initialized: false,
        owner_pubkey: None,
        credentials: Vec::new(),
    }
}

fn save_config(cfg: &ManagerConfig) -> Result<()> {
    let mut config_without_credentials = cfg.clone();
    let credentials = config_without_credentials.credentials.clone();
    config_without_credentials.credentials = Vec::new();
    let blob = encode_nv_json_blob(&config_without_credentials, CONFIG_TPM_NV_SIZE, "config")?;
    tpm_nv_write_blob(CONFIG_TPM_NV_INDEX, &blob, CONFIG_TPM_NV_SIZE)
        .context("failed to store manager config in TPM NV")?;
    save_credentials_to_nv(&credentials)?;
    Ok(())
}

fn cmd_configure(_rpc_url: &str, heartbeat_secs: u64) -> Result<()> {
    if !tpm_nv_available() {
        return Err(anyhow!("TPM NV storage unavailable"));
    }
    if heartbeat_secs == 0 {
        return Err(anyhow!("heartbeat_secs must be greater than zero"));
    }
    if let Ok(existing) = load_config() {
        if existing.bonded {
            return Err(anyhow!(
                "configuration is immutable after bonding (node already bonded)"
            ));
        }
    }
    let mut cfg = default_manager_config();
    cfg.heartbeat_secs = heartbeat_secs;
    save_config(&cfg)?;
    println!("status=configured");
    println!("config_tpm_nv_index=0x{CONFIG_TPM_NV_INDEX:08x}");
    Ok(())
}

fn cmd_bond(owner_pubkey: &str) -> Result<()> {
    let mut cfg = load_config()?;
    if cfg.bonded {
        println!("status=already-bonded");
        println!("owner_pubkey={}", cfg.owner_pubkey.unwrap_or_default());
        return Ok(());
    }
    if owner_pubkey.trim().is_empty() {
        return Err(anyhow!("owner_pubkey is required"));
    }
    cfg.bonded = true;
    cfg.owner_pubkey = Some(owner_pubkey.trim().to_string());
    save_config(&cfg)?;
    println!("status=bonded");
    println!("owner_pubkey={owner_pubkey}");
    Ok(())
}

async fn cmd_register(owner_pubkey: &str) -> Result<()> {
    let signer = load_tpm_signer()?;
    let client = Client::new();
    register_device(
        &client,
        DEFAULT_API_BASE,
        &signer.public_key_b64url,
        owner_pubkey,
    )
    .await?;

    let registration_link = format!(
        "{}/register?device={}&owner={}",
        DEFAULT_API_BASE, signer.public_key_b64url, owner_pubkey
    );
    println!("status=registered");
    println!("device_pubkey_b64url={}", signer.public_key_b64url);
    println!("owner_pubkey={owner_pubkey}");
    println!("registration_url={registration_link}");
    Ok(())
}

async fn cmd_tunnel_reserve(relay_control_base: &str, requested_label: &str) -> Result<()> {
    let signer = load_tpm_signer()?;
    let client = Client::new();
    let url = format!(
        "{}/v1/tunnel/reserve-domain",
        relay_control_base.trim_end_matches('/')
    );
    let payload = ReserveDomainRequestV1 {
        profile_public_key_b64url: signer.public_key_b64url.clone(),
        requested_label: requested_label.trim().to_string(),
    };
    let response: ReserveDomainResponseV1 = post_protobuf(&client, &url, &payload).await?;
    if !response.ok {
        return Err(anyhow!("tunnel reserve failed: {}", response.error.trim()));
    }
    println!("status=tunnel-domain-reserved");
    println!("user_id={}", response.user_id);
    println!("domain={}", response.domain);
    println!("reservation_status={}", response.status);
    println!("registration_token={}", response.registration_token);
    Ok(())
}

fn tunnel_registration_message(domain: &str, node_id: &str, nonce_b64url: &str) -> String {
    format!("edgerun-tunnel-register-v1\ndomain={domain}\nnode_id={node_id}\nnonce={nonce_b64url}")
}

async fn cmd_tunnel_register(
    relay_control_base: &str,
    domain: &str,
    node_id: &str,
    registration_token: &str,
) -> Result<()> {
    let signer = load_tpm_signer()?;
    if domain.trim().is_empty() {
        return Err(anyhow!("domain is required"));
    }
    if node_id.trim().is_empty() {
        return Err(anyhow!("node_id is required"));
    }
    if registration_token.trim().is_empty() {
        return Err(anyhow!("registration_token is required"));
    }
    let nonce = random_token_b64url(24);
    let signature = signer
        .sign_b64url(tunnel_registration_message(domain.trim(), node_id.trim(), &nonce).as_bytes());
    let payload = RegisterEndpointRequestV1 {
        domain: domain.trim().to_string(),
        node_id: node_id.trim().to_string(),
        endpoint_public_key_b64url: signer.public_key_b64url.clone(),
        tunnel_nonce_b64url: nonce,
        registration_signature_b64url: signature,
        capability_scopes: vec![],
        registration_token: registration_token.trim().to_string(),
    };
    let client = Client::new();
    let url = format!(
        "{}/v1/tunnel/register-endpoint",
        relay_control_base.trim_end_matches('/')
    );
    let response: RegisterEndpointResponseV1 = post_protobuf(&client, &url, &payload).await?;
    if !response.ok {
        return Err(anyhow!("tunnel register failed: {}", response.error.trim()));
    }
    println!("status=tunnel-endpoint-registered");
    println!("domain={}", domain.trim());
    println!("node_id={}", node_id.trim());
    println!("session_id={}", response.session_id);
    println!("relay_url={}", response.relay_url);
    println!("expires_unix_ms={}", response.expires_unix_ms);
    Ok(())
}

async fn cmd_tunnel_heartbeat(
    relay_control_base: &str,
    domain: &str,
    node_id: &str,
    session_id: &str,
) -> Result<()> {
    if domain.trim().is_empty() || node_id.trim().is_empty() || session_id.trim().is_empty() {
        return Err(anyhow!("domain, node_id, and session_id are required"));
    }
    let client = Client::new();
    let url = format!(
        "{}/v1/tunnel/heartbeat",
        relay_control_base.trim_end_matches('/')
    );
    let payload = TunnelHeartbeatRequestV1 {
        domain: domain.trim().to_string(),
        node_id: node_id.trim().to_string(),
        session_id: session_id.trim().to_string(),
    };
    let response: TunnelHeartbeatResponseV1 = post_protobuf(&client, &url, &payload).await?;
    if !response.ok {
        return Err(anyhow!(
            "tunnel heartbeat failed: {}",
            response.error.trim()
        ));
    }
    println!("status=tunnel-heartbeat-ok");
    println!("expires_unix_ms={}", response.expires_unix_ms);
    Ok(())
}

async fn cmd_tunnel_create_code(
    relay_control_base: &str,
    domain: &str,
    registration_token: &str,
    ttl_seconds: u64,
) -> Result<()> {
    if domain.trim().is_empty() {
        return Err(anyhow!("domain is required"));
    }
    if registration_token.trim().is_empty() {
        return Err(anyhow!("registration_token is required"));
    }
    let client = Client::new();
    let url = format!(
        "{}/v1/tunnel/create-pairing-code",
        relay_control_base.trim_end_matches('/')
    );
    let payload = CreatePairingCodeRequestV1 {
        domain: domain.trim().to_string(),
        registration_token: registration_token.trim().to_string(),
        ttl_seconds,
    };
    let response: CreatePairingCodeResponseV1 = post_protobuf(&client, &url, &payload).await?;
    if !response.ok {
        return Err(anyhow!(
            "tunnel create pairing code failed: {}",
            response.error.trim()
        ));
    }
    println!("status=tunnel-pairing-code-issued");
    println!("pairing_code={}", response.pairing_code);
    println!("expires_unix_ms={}", response.expires_unix_ms);
    println!("device_command={}", response.device_command);
    Ok(())
}

fn derive_default_node_id(device_pubkey_b64url: &str) -> String {
    let short = device_pubkey_b64url
        .chars()
        .take(12)
        .collect::<String>()
        .to_ascii_lowercase();
    format!("node-{short}")
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

async fn cmd_tunnel_connect(
    relay_control_base: &str,
    pairing_code: &str,
    node_id: &str,
) -> Result<()> {
    let signer = load_tpm_signer()?;
    if pairing_code.trim().is_empty() {
        return Err(anyhow!("pairing_code is required"));
    }
    let effective_node_id = if node_id.trim().is_empty() {
        derive_default_node_id(&signer.public_key_b64url)
    } else {
        node_id.trim().to_string()
    };
    let nonce = random_token_b64url(24);
    let signature = signer.sign_b64url(
        tunnel_pairing_registration_message(pairing_code.trim(), &effective_node_id, &nonce)
            .as_bytes(),
    );
    let payload = RegisterWithPairingCodeRequestV1 {
        pairing_code: pairing_code.trim().to_string(),
        node_id: effective_node_id.clone(),
        endpoint_public_key_b64url: signer.public_key_b64url.clone(),
        tunnel_nonce_b64url: nonce,
        registration_signature_b64url: signature,
        capability_scopes: vec![],
    };
    let client = Client::new();
    let url = format!(
        "{}/v1/tunnel/register-with-code",
        relay_control_base.trim_end_matches('/')
    );
    let response: RegisterWithPairingCodeResponseV1 =
        post_protobuf(&client, &url, &payload).await?;
    if !response.ok {
        return Err(anyhow!("tunnel connect failed: {}", response.error.trim()));
    }
    println!("status=tunnel-connected");
    println!("domain={}", response.domain);
    println!("node_id={effective_node_id}");
    println!("session_id={}", response.session_id);
    println!("relay_url={}", response.relay_url);
    println!("expires_unix_ms={}", response.expires_unix_ms);
    Ok(())
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn local_bridge_envelope(topic: &str, source: &str) -> LocalEventEnvelopeV1 {
    LocalEventEnvelopeV1 {
        event_id: format!("local-{}", now_unix_ms()),
        topic: topic.to_string(),
        payload: vec![],
        source: source.to_string(),
        ts_unix_ms: now_unix_ms(),
    }
}

fn with_local_cors_headers(mut response: Response) -> Response {
    let headers = response.headers_mut();
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"));
    headers.insert(
        ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("content-type"),
    );
    headers.insert(
        ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("GET, POST, OPTIONS"),
    );
    response
}

fn local_json_ok<T>(data: T) -> Response
where
    T: Serialize,
{
    let payload = sonic_rs::to_string(&LocalFsResponse {
        ok: true,
        error: String::new(),
        data,
    })
    .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode failed\"}".to_string());
    with_local_cors_headers(
        (
            AxumStatusCode::OK,
            [(
                AXUM_CONTENT_TYPE,
                HeaderValue::from_static("application/json; charset=utf-8"),
            )],
            payload,
        )
            .into_response(),
    )
}

fn local_json_error(status: AxumStatusCode, message: &str) -> Response {
    let payload = sonic_rs::to_string(&LocalFsResponse {
        ok: false,
        error: message.to_string(),
        data: LocalFsEmptyData {},
    })
    .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode failed\"}".to_string());
    with_local_cors_headers(
        (
            status,
            [(
                AXUM_CONTENT_TYPE,
                HeaderValue::from_static("application/json; charset=utf-8"),
            )],
            payload,
        )
            .into_response(),
    )
}

fn local_json_value(status: AxumStatusCode, value: sonic_rs::Value) -> Response {
    let payload = sonic_rs::to_string(&value)
        .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode failed\"}".to_string());
    with_local_cors_headers(
        (
            status,
            [(
                AXUM_CONTENT_TYPE,
                HeaderValue::from_static("application/json; charset=utf-8"),
            )],
            payload,
        )
            .into_response(),
    )
}

fn google_token_from(raw: &str) -> Result<String> {
    let token = raw.trim();
    if token.len() < 12 {
        return Err(anyhow!("google token is missing or invalid"));
    }
    Ok(token.to_string())
}

fn google_oauth_redirect_origin() -> String {
    std::env::var(GOOGLE_OAUTH_REDIRECT_ORIGIN_ENV)
        .ok()
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| GOOGLE_OAUTH_REDIRECT_ORIGIN_DEFAULT.to_string())
}

fn google_oauth_redirect_uri() -> String {
    format!("{}/api/google/oauth/callback", google_oauth_redirect_origin())
}

fn google_oauth_scopes_for(integration_id: Option<&str>) -> String {
    let id = integration_id.unwrap_or("google").trim().to_ascii_lowercase();
    let mut scopes = vec![GOOGLE_OAUTH_SCOPES_BASE.to_string()];
    match id.as_str() {
        "google_photos" => {
            scopes.push(GOOGLE_OAUTH_SCOPES_PHOTOS_READONLY.to_string());
            scopes.push(GOOGLE_OAUTH_SCOPES_PHOTOS_APPEND.to_string());
            scopes.push(GOOGLE_OAUTH_SCOPES_PHOTOS_READONLY_APPCREATED.to_string());
            scopes.push(GOOGLE_OAUTH_SCOPES_PHOTOS_EDIT_APPCREATED.to_string());
        }
        "email" => {
            scopes.push("https://www.googleapis.com/auth/gmail.readonly".to_string());
        }
        _ => {
            scopes.push(GOOGLE_OAUTH_SCOPES_PRODUCTIVITY.to_string());
            scopes.push(GOOGLE_OAUTH_SCOPES_PHOTOS_READONLY.to_string());
            scopes.push(GOOGLE_OAUTH_SCOPES_PHOTOS_APPEND.to_string());
            scopes.push(GOOGLE_OAUTH_SCOPES_PHOTOS_READONLY_APPCREATED.to_string());
            scopes.push(GOOGLE_OAUTH_SCOPES_PHOTOS_EDIT_APPCREATED.to_string());
        }
    }
    scopes.join(" ")
}

fn sanitize_return_to(raw: Option<&str>) -> String {
    let value = raw.unwrap_or("/").trim();
    if value.starts_with('/') {
        value.to_string()
    } else {
        "/".to_string()
    }
}

fn encode_google_oauth_state(return_to: &str) -> String {
    URL_SAFE_NO_PAD.encode(return_to.as_bytes())
}

fn decode_google_oauth_state(state: Option<&str>) -> String {
    let Some(raw) = state else { return "/".to_string(); };
    let decoded = URL_SAFE_NO_PAD.decode(raw.as_bytes()).ok();
    let text = decoded
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .unwrap_or_else(|| "/".to_string());
    sanitize_return_to(Some(&text))
}

fn append_query_pair(url: &mut String, key: &str, value: &str) {
    if value.is_empty() {
        return;
    }
    let sep = if url.contains('?') { '&' } else { '?' };
    let encoded = beeper_encode_path_segment(value).replace("%2F", "/");
    url.push(sep);
    url.push_str(key);
    url.push('=');
    url.push_str(&encoded);
}

fn google_oauth_redirect_with_result(return_to: &str, ok: bool, message: &str, access_token: &str, refresh_token: &str) -> Response {
    let mut location = format!("{}{}", google_oauth_redirect_origin(), sanitize_return_to(Some(return_to)).as_str());
    append_query_pair(&mut location, "google_oauth", if ok { "ok" } else { "error" });
    append_query_pair(&mut location, "google_oauth_message", message);
    append_query_pair(&mut location, "google_access_token", access_token);
    append_query_pair(&mut location, "google_refresh_token", refresh_token);
    let mut response = AxumStatusCode::FOUND.into_response();
    if let Ok(value) = HeaderValue::from_str(&location) {
        response.headers_mut().insert(LOCATION, value);
    }
    response
}

fn beeper_token_from(raw: &str) -> Result<String> {
    let token = raw.trim();
    if token.len() < 8 {
        return Err(anyhow!("beeper access token is missing or invalid"));
    }
    Ok(token.to_string())
}

fn beeper_api_base() -> String {
    std::env::var(BEEPER_DESKTOP_API_BASE_ENV)
        .ok()
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| BEEPER_DESKTOP_API_BASE_DEFAULT.to_string())
}

fn beeper_import_dir() -> String {
    std::env::var(BEEPER_IMPORT_DIR_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| BEEPER_IMPORT_DIR_DEFAULT.to_string())
}

fn beeper_media_root() -> PathBuf {
    std::env::var(BEEPER_MEDIA_ROOT_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(BEEPER_MEDIA_ROOT_DEFAULT))
}

async fn beeper_api_verify_token(token: &str) -> Result<sonic_rs::Value> {
    let client = Client::new();
    let base = beeper_api_base();
    let candidates = ["/v0/accounts", "/v1/accounts"];
    let mut last_error = String::new();
    for path in candidates {
        let url = format!("{base}{path}");
        let response = match client
            .get(&url)
            .header("Authorization", format!("Bearer {token}"))
            .header("Accept", "application/json")
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(err) => {
                last_error = format!("beeper accounts request failed: {url}: {err}");
                continue;
            }
        };
        let status = response.status();
        let bytes = response
            .bytes()
            .await
            .with_context(|| format!("failed to read beeper accounts response: {url}"))?;
        if !status.is_success() {
            let detail = String::from_utf8_lossy(&bytes).to_string();
            last_error = format!("beeper token verify failed ({status}) at {url}: {detail}");
            continue;
        }
        return sonic_rs::from_slice(&bytes)
            .with_context(|| format!("failed to parse beeper accounts json: {url}"));
    }
    Err(anyhow!(
        "unable to verify Beeper token at {} (tried /v0/accounts and /v1/accounts): {}",
        base,
        last_error
    ))
}

async fn beeper_api_get_chats(token: &str, limit: usize) -> Result<sonic_rs::Value> {
    let client = Client::new();
    let base = beeper_api_base();
    let candidates = ["/v1/chats", "/v0/chats"];
    let mut last_error = String::new();
    for path in candidates {
        let url = format!("{base}{path}?limit={limit}");
        let response = match client
            .get(&url)
            .header("Authorization", format!("Bearer {token}"))
            .header("Accept", "application/json")
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(err) => {
                last_error = format!("beeper chats request failed: {url}: {err}");
                continue;
            }
        };
        let status = response.status();
        let bytes = response
            .bytes()
            .await
            .with_context(|| format!("failed to read beeper chats response: {url}"))?;
        if !status.is_success() {
            let detail = String::from_utf8_lossy(&bytes).to_string();
            last_error = format!("beeper chats request failed ({status}) at {url}: {detail}");
            continue;
        }
        return sonic_rs::from_slice(&bytes)
            .with_context(|| format!("failed to parse beeper chats json: {url}"));
    }
    Err(anyhow!(
        "unable to fetch Beeper chats at {} (tried /v1/chats and /v0/chats): {}",
        base,
        last_error
    ))
}

fn beeper_encode_path_segment(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| {
            let ch = *byte as char;
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' || ch == '~' {
                ch.to_string()
            } else {
                format!("%{:02X}", byte)
            }
        })
        .collect::<String>()
}

fn beeper_media_content_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .as_deref()
    {
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        Some("mp4") => "video/mp4",
        Some("webm") => "video/webm",
        Some("mov") => "video/quicktime",
        Some("mp3") => "audio/mpeg",
        Some("wav") => "audio/wav",
        Some("ogg") => "audio/ogg",
        Some("pdf") => "application/pdf",
        _ => "application/octet-stream",
    }
}

fn resolve_beeper_media_path(raw_uri: &str) -> Result<PathBuf> {
    let uri = raw_uri.trim();
    if uri.is_empty() {
        return Err(anyhow!("uri is required"));
    }
    let media_root = beeper_media_root();
    let candidate = if let Some(path) = uri.strip_prefix("file://") {
        PathBuf::from(path)
    } else if let Some(rest) = uri.strip_prefix("mxc://") {
        let mut parts = rest.splitn(2, '/');
        let host = parts.next().unwrap_or_default().trim();
        let media_id = parts.next().unwrap_or_default().trim();
        if host.is_empty() || media_id.is_empty() {
            return Err(anyhow!("invalid mxc uri"));
        }
        media_root.join(host).join(media_id)
    } else {
        return Err(anyhow!("unsupported beeper media uri"));
    };

    let root_display = media_root.display().to_string();
    let candidate_display = candidate.display().to_string();
    if !candidate_display.starts_with(&root_display) {
        return Err(anyhow!("media uri is outside allowed root"));
    }
    Ok(candidate)
}

async fn beeper_api_get_chat_messages(token: &str, chat_id: &str, limit: usize) -> Result<sonic_rs::Value> {
    let client = Client::new();
    let base = beeper_api_base();
    let encoded_chat = beeper_encode_path_segment(chat_id);
    let candidates = [
        format!("/v1/chats/{encoded_chat}/messages?limit={limit}"),
        format!("/v0/chats/{encoded_chat}/messages?limit={limit}"),
    ];
    let mut last_error = String::new();
    for path in candidates {
        let url = format!("{base}{path}");
        let response = match client
            .get(&url)
            .header("Authorization", format!("Bearer {token}"))
            .header("Accept", "application/json")
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(err) => {
                last_error = format!("beeper messages request failed: {url}: {err}");
                continue;
            }
        };
        let status = response.status();
        let bytes = response
            .bytes()
            .await
            .with_context(|| format!("failed to read beeper messages response: {url}"))?;
        if !status.is_success() {
            let detail = String::from_utf8_lossy(&bytes).to_string();
            last_error = format!("beeper messages request failed ({status}) at {url}: {detail}");
            continue;
        }
        return sonic_rs::from_slice(&bytes)
            .with_context(|| format!("failed to parse beeper messages json: {url}"));
    }
    Err(anyhow!(
        "unable to fetch Beeper chat messages for {} at {}: {}",
        chat_id,
        base,
        last_error
    ))
}

async fn beeper_api_send_chat_message(token: &str, chat_id: &str, text: &str) -> Result<sonic_rs::Value> {
    let client = Client::new();
    let base = beeper_api_base();
    let encoded_chat = beeper_encode_path_segment(chat_id);
    let candidates = [
        format!("/v1/chats/{encoded_chat}/messages"),
        format!("/v0/chats/{encoded_chat}/messages"),
    ];
    let payload = sonic_rs::to_vec(&sonic_rs::json!({ "text": text }))
        .context("failed to encode beeper send payload")?;
    let mut last_error = String::new();
    for path in candidates {
        let url = format!("{base}{path}");
        let response = match client
            .post(&url)
            .header("Authorization", format!("Bearer {token}"))
            .header("Accept", "application/json")
            .header(CONTENT_TYPE, "application/json")
            .body(payload.clone())
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(err) => {
                last_error = format!("beeper send request failed: {url}: {err}");
                continue;
            }
        };
        let status = response.status();
        let bytes = response
            .bytes()
            .await
            .with_context(|| format!("failed to read beeper send response: {url}"))?;
        if !status.is_success() {
            let detail = String::from_utf8_lossy(&bytes).to_string();
            last_error = format!("beeper send failed ({status}) at {url}: {detail}");
            continue;
        }
        return sonic_rs::from_slice(&bytes)
            .with_context(|| format!("failed to parse beeper send json: {url}"));
    }
    Err(anyhow!(
        "unable to send Beeper message for chat {} at {}: {}",
        chat_id,
        base,
        last_error
    ))
}

fn sanitize_import_thread_id(raw: &str) -> String {
    let compact = raw
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' { ch } else { '-' })
        .collect::<String>();
    if compact.trim_matches('-').is_empty() {
        "thread".to_string()
    } else {
        compact
    }
}

fn load_imported_fb_threads(limit_threads: usize, limit_messages: usize) -> Result<Vec<sonic_rs::Value>> {
    let dir = PathBuf::from(beeper_import_dir());
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut threads = Vec::new();
    for entry in fs::read_dir(&dir).with_context(|| format!("failed to read beeper import dir {}", dir.display()))? {
        let entry = match entry {
            Ok(value) => value,
            Err(_) => continue,
        };
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let raw = match fs::read(&path) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let thread: ImportedFbThread = match sonic_rs::from_slice(&raw) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let thread_name = if thread.thread_name.trim().is_empty() {
            path.file_stem().and_then(|name| name.to_str()).unwrap_or("Imported Thread").to_string()
        } else {
            thread.thread_name.trim().to_string()
        };
        let participants = thread
            .participants
            .into_iter()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();
        let subtitle = participants
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        let mut messages = thread
            .messages
            .into_iter()
            .map(|item| {
                let first_media = item
                    .media
                    .iter()
                    .find_map(|media| {
                        let uri = media.uri.trim();
                        if uri.is_empty() {
                            None
                        } else {
                            Some(uri.to_string())
                        }
                    })
                    .unwrap_or_default();
                let body = if !item.text.trim().is_empty() {
                    item.text.trim().to_string()
                } else if !first_media.is_empty() {
                    format!("[Media]\n{}", first_media)
                } else {
                    format!("[{}]", if item.message_type.trim().is_empty() { "message" } else { item.message_type.trim() })
                };
                sonic_rs::json!({
                    "id": format!("{}-{}", sanitize_import_thread_id(&thread_name), item.timestamp),
                    "role": "contact",
                    "text": body,
                    "createdAt": item.timestamp,
                    "channel": "beeper",
                    "author": if item.sender_name.trim().is_empty() { "Unknown" } else { item.sender_name.trim() },
                })
            })
            .collect::<Vec<_>>();
        messages.sort_by(|a, b| {
            let a_ts = a["createdAt"].as_i64().unwrap_or_default();
            let b_ts = b["createdAt"].as_i64().unwrap_or_default();
            a_ts.cmp(&b_ts)
        });
        if messages.len() > limit_messages {
            messages = messages[messages.len().saturating_sub(limit_messages)..].to_vec();
        }
        let updated_at = messages
            .last()
            .and_then(|message| message["createdAt"].as_i64())
            .unwrap_or_default();
        let preview = messages
            .last()
            .and_then(|message| message["text"].as_str())
            .unwrap_or_default()
            .lines()
            .next()
            .unwrap_or_default()
            .to_string();
        threads.push(sonic_rs::json!({
            "id": format!("bridge-beeper-import-{}", sanitize_import_thread_id(&thread_name)),
            "kind": "bridge",
            "channel": "beeper",
            "channels": ["beeper", "facebook_export"],
            "title": thread_name,
            "subtitle": if subtitle.is_empty() { "Imported from Facebook export".to_string() } else { subtitle },
            "updatedAt": updated_at,
            "preview": preview,
            "messages": messages,
            "participants": participants,
            "imported": true,
        }));
    }
    threads.sort_by(|a, b| {
        let a_ts = a["updatedAt"].as_i64().unwrap_or_default();
        let b_ts = b["updatedAt"].as_i64().unwrap_or_default();
        b_ts.cmp(&a_ts)
    });
    if threads.len() > limit_threads {
        threads.truncate(limit_threads);
    }
    Ok(threads)
}

fn tailscale_api_key_from(raw: &str) -> Result<String> {
    let key = raw.trim();
    if key.len() < 12 {
        return Err(anyhow!("tailscale api key is missing or invalid"));
    }
    Ok(key.to_string())
}

fn tailscale_tailnet_from(raw: &str) -> Result<String> {
    let tailnet = raw.trim();
    if tailnet.is_empty() {
        return Err(anyhow!("tailnet is required"));
    }
    if tailnet.len() > 255 {
        return Err(anyhow!("tailnet is too long"));
    }
    Ok(tailnet.to_string())
}

fn tailscale_encode_path_segment(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| {
            let ch = *byte as char;
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' || ch == '~' {
                ch.to_string()
            } else {
                format!("%{:02X}", byte)
            }
        })
        .collect::<String>()
}

async fn tailscale_api_get_devices(api_key: &str, tailnet: &str) -> Result<sonic_rs::Value> {
    let client = Client::new();
    let encoded_tailnet = tailscale_encode_path_segment(tailnet);
    let url = format!(
        "https://api.tailscale.com/api/v2/tailnet/{encoded_tailnet}/devices"
    );
    let response = client
        .get(&url)
        .basic_auth(api_key, Some(""))
        .header("Accept", "application/json")
        .send()
        .await
        .with_context(|| format!("tailscale api request failed: {url}"))?;
    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .with_context(|| format!("failed to read tailscale api response: {url}"))?;
    if !status.is_success() {
        let detail = String::from_utf8_lossy(&bytes).to_string();
        return Err(anyhow!("tailscale api request failed ({status}): {detail}"));
    }
    sonic_rs::from_slice(&bytes).with_context(|| format!("failed to parse tailscale api json: {url}"))
}

async fn google_api_get_json(token: &str, url: &str) -> Result<sonic_rs::Value> {
    let client = Client::new();
    let response = client
        .get(url)
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/json")
        .send()
        .await
        .with_context(|| format!("google api request failed: {url}"))?;
    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .with_context(|| format!("failed to read google api response: {url}"))?;
    if !status.is_success() {
        let detail = String::from_utf8_lossy(&bytes).to_string();
        return Err(anyhow!("google api request failed ({status}): {detail}"));
    }
    sonic_rs::from_slice(&bytes).with_context(|| format!("failed to parse google api json: {url}"))
}

async fn google_api_get_bytes(token: &str, url: &str) -> Result<Vec<u8>> {
    let client = Client::new();
    let response = client
        .get(url)
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .with_context(|| format!("google api request failed: {url}"))?;
    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .with_context(|| format!("failed to read google api response: {url}"))?;
    if !status.is_success() {
        let detail = String::from_utf8_lossy(&bytes).to_string();
        return Err(anyhow!("google api request failed ({status}): {detail}"));
    }
    Ok(bytes.to_vec())
}

fn gmail_header(payload: &sonic_rs::Value, name: &str) -> String {
    payload["headers"]
        .as_array()
        .and_then(|headers| {
            headers.iter().find_map(|header| {
                let key = header["name"].as_str().unwrap_or_default();
                if key.eq_ignore_ascii_case(name) {
                    Some(header["value"].as_str().unwrap_or_default().to_string())
                } else {
                    None
                }
            })
        })
        .unwrap_or_default()
}

fn decode_gmail_part_body(part: &sonic_rs::Value) -> String {
    let data = part["body"]["data"].as_str().unwrap_or_default().trim();
    if data.is_empty() {
        return String::new();
    }
    match URL_SAFE_NO_PAD.decode(data.as_bytes()) {
        Ok(bytes) => String::from_utf8(bytes).unwrap_or_default(),
        Err(_) => String::new(),
    }
}

fn extract_gmail_text(payload: &sonic_rs::Value) -> (String, String) {
    let mime_type = payload["mimeType"].as_str().unwrap_or_default();
    if mime_type.eq_ignore_ascii_case("text/plain") {
        return (decode_gmail_part_body(payload), String::new());
    }
    if mime_type.eq_ignore_ascii_case("text/html") {
        return (String::new(), decode_gmail_part_body(payload));
    }

    let mut plain = String::new();
    let mut html = String::new();
    let mut stack: Vec<&sonic_rs::Value> = vec![payload];
    while let Some(node) = stack.pop() {
        let node_mime = node["mimeType"].as_str().unwrap_or_default();
        if node_mime.eq_ignore_ascii_case("text/plain") && plain.is_empty() {
            plain = decode_gmail_part_body(node);
        } else if node_mime.eq_ignore_ascii_case("text/html") && html.is_empty() {
            html = decode_gmail_part_body(node);
        }
        if let Some(parts) = node["parts"].as_array() {
            for part in parts {
                stack.push(part);
            }
        }
    }
    (plain, html)
}

fn normalized_relative_path(raw: &str) -> Result<PathBuf> {
    let mut rel = PathBuf::new();
    for component in Path::new(raw).components() {
        match component {
            Component::Prefix(_) | Component::ParentDir => {
                return Err(anyhow!("path traversal is not allowed"));
            }
            Component::CurDir => {}
            Component::RootDir => {}
            Component::Normal(part) => rel.push(part),
        }
    }
    Ok(rel)
}

fn local_fs_display_path(raw: &str) -> Result<String> {
    let rel = normalized_relative_path(raw)?;
    if rel.as_os_str().is_empty() {
        return Ok("/".to_string());
    }
    Ok(format!("/{}", rel.to_string_lossy()))
}

fn local_fs_abs_path(root: &Path, raw: &str) -> Result<PathBuf> {
    let rel = normalized_relative_path(raw)?;
    let candidate = root.join(rel);
    Ok(candidate)
}

fn assert_path_under_root(root: &Path, candidate: &Path) -> Result<()> {
    if candidate.starts_with(root) {
        return Ok(());
    }
    Err(anyhow!("path escapes local filesystem root"))
}

fn enforce_local_node(state: &LocalBridgeState, node_id: Option<&str>) -> Result<()> {
    let requested = node_id.unwrap_or("").trim();
    if requested.is_empty() || requested == state.node_id {
        return Ok(());
    }
    Err(anyhow!(
        "selected node is not local; local bridge filesystem is only available for node {}",
        state.node_id
    ))
}

fn local_fs_entry(root: &Path, path: &Path) -> Result<LocalFsEntry> {
    let metadata =
        fs::metadata(path).with_context(|| format!("failed to stat path {}", path.display()))?;
    let rel = path
        .strip_prefix(root)
        .with_context(|| format!("path {} is outside root {}", path.display(), root.display()))?;
    let display = if rel.as_os_str().is_empty() {
        "/".to_string()
    } else {
        format!("/{}", rel.to_string_lossy())
    };
    let name = if rel.as_os_str().is_empty() {
        "/".to_string()
    } else {
        path.file_name()
            .map(|v| v.to_string_lossy().to_string())
            .unwrap_or_else(|| rel.to_string_lossy().to_string())
    };
    let kind = if metadata.is_dir() { "dir" } else { "file" }.to_string();
    Ok(LocalFsEntry {
        id: format!("local:{display}"),
        provider: "local".to_string(),
        path: display,
        name,
        kind,
        size: if metadata.is_file() {
            Some(metadata.len())
        } else {
            None
        },
    })
}

fn local_fs_copy_recursive(from: &Path, to: &Path) -> Result<()> {
    let metadata =
        fs::metadata(from).with_context(|| format!("failed to stat source {}", from.display()))?;
    if metadata.is_dir() {
        fs::create_dir_all(to)
            .with_context(|| format!("failed to create destination dir {}", to.display()))?;
        for entry in
            fs::read_dir(from).with_context(|| format!("failed to read dir {}", from.display()))?
        {
            let entry =
                entry.with_context(|| format!("failed to read dir entry {}", from.display()))?;
            let source_child = entry.path();
            let target_child = to.join(entry.file_name());
            local_fs_copy_recursive(&source_child, &target_child)?;
        }
    } else {
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create destination parent {}", parent.display())
            })?;
        }
        fs::copy(from, to).with_context(|| {
            format!(
                "failed to copy source {} to destination {}",
                from.display(),
                to.display()
            )
        })?;
    }
    Ok(())
}

fn local_fs_archive_path(path: &Path, format: Option<&str>) -> PathBuf {
    let selected = format.unwrap_or("tar.gz").trim();
    let suffix = if selected.eq_ignore_ascii_case("tar.gz") || selected.eq_ignore_ascii_case("tgz")
    {
        "tar.gz"
    } else {
        "tar.gz"
    };
    let name = path
        .file_name()
        .map(|v| v.to_string_lossy().to_string())
        .unwrap_or_else(|| "archive".to_string());
    path.with_file_name(format!("{name}.{suffix}"))
}

fn mcp_container_name(integration_id: &str) -> String {
    let normalized = integration_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    format!("edgerun-mcp-{}", normalized.trim_matches('-'))
}

fn mcp_image_for(integration_id: &str) -> Option<String> {
    let key = integration_id.trim().to_ascii_uppercase().replace('-', "_");
    let specific_env = format!("EDGERUN_MCP_{}_IMAGE", key);
    if let Ok(value) = std::env::var(&specific_env) {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    match integration_id.trim() {
        "github" => Some(MCP_IMAGE_GITHUB_DEFAULT.to_string()),
        "google_messages" => Some(MCP_IMAGE_GOOGLE_MESSAGES_DEFAULT.to_string()),
        "gvoice" => Some(MCP_IMAGE_GVOICE_DEFAULT.to_string()),
        "googlechat" => Some(MCP_IMAGE_GOOGLECHAT_DEFAULT.to_string()),
        _ => None,
    }
}

fn mcp_token_env_for(integration_id: &str) -> &'static str {
    match integration_id.trim() {
        "github" => "GITHUB_PERSONAL_ACCESS_TOKEN",
        _ => "MCP_API_TOKEN",
    }
}

fn mcp_container_command_for(integration_id: &str) -> Vec<String> {
    match integration_id.trim() {
        "github" => vec!["http".to_string(), "--port".to_string(), "8082".to_string()],
        _ => Vec::new(),
    }
}

fn integration_topic_root() -> String {
    std::env::var(INTEGRATION_TOPIC_ROOT_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| INTEGRATION_TOPIC_ROOT_DEFAULT.to_string())
}

fn integration_topic_for(integration_id: &str, lane: &str) -> String {
    let lane = lane
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    format!("{}.{}.{}", integration_topic_root(), integration_id, lane)
}

fn list_running_mcp_integrations() -> Result<Vec<String>> {
    let output = Command::new("docker")
        .args([
            "ps",
            "--filter",
            "name=^edgerun-mcp-",
            "--format",
            "{{.Names}}",
        ])
        .output()
        .context("failed to list MCP containers")?;
    if !output.status.success() {
        return Err(anyhow!(
            "failed to list MCP containers: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let mut integrations: Vec<String> = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let container_name = line.trim();
        if !container_name.starts_with("edgerun-mcp-") {
            continue;
        }
        let integration = container_name
            .trim_start_matches("edgerun-mcp-")
            .trim()
            .to_ascii_lowercase();
        if integration.is_empty() {
            continue;
        }
        integrations.push(integration);
    }
    integrations.sort();
    integrations.dedup();
    Ok(integrations)
}

fn read_opencode_config_text() -> Result<String> {
    let output = Command::new("docker")
        .args(["exec", OPENCODE_CLI_CONTAINER_NAME, "cat", OPENCODE_CONFIG_PATH])
        .output()
        .context("failed to read OpenCode config from container")?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).to_string());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
    if stderr.contains("no such file") {
        return Ok(String::new());
    }
    Err(anyhow!(
        "failed to read OpenCode config from {}: {}",
        OPENCODE_CLI_CONTAINER_NAME,
        String::from_utf8_lossy(&output.stderr).trim()
    ))
}

fn opencode_cli_container_available() -> bool {
    let output = Command::new("docker")
        .args(["inspect", OPENCODE_CLI_CONTAINER_NAME])
        .output();
    let Ok(output) = output else {
        return false;
    };
    output.status.success()
}

fn require_opencode_cli_container() -> Result<()> {
    if opencode_cli_container_available() {
        return Ok(());
    }
    Err(anyhow!(
        "OpenCode runtime is unavailable: container {} is not running",
        OPENCODE_CLI_CONTAINER_NAME
    ))
}

fn write_opencode_config_text(config: &str) -> Result<()> {
    let mkdir_output = Command::new("docker")
        .args([
            "exec",
            OPENCODE_CLI_CONTAINER_NAME,
            "mkdir",
            "-p",
            "/home/ken/.config/opencode",
        ])
        .output()
        .context("failed to ensure OpenCode config directory")?;
    if !mkdir_output.status.success() {
        return Err(anyhow!(
            "failed to create OpenCode config directory: {}",
            String::from_utf8_lossy(&mkdir_output.stderr).trim()
        ));
    }

    let mut child = Command::new("docker")
        .args([
            "exec",
            "-i",
            OPENCODE_CLI_CONTAINER_NAME,
            "sh",
            "-lc",
            &format!("cat > {}", OPENCODE_CONFIG_PATH),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to launch OpenCode config write command")?;
    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow!("failed to open stdin for OpenCode config write"))?;
        stdin
            .write_all(config.as_bytes())
            .context("failed to stream OpenCode config content")?;
    }
    let output = child
        .wait_with_output()
        .context("failed to complete OpenCode config write command")?;
    if !output.status.success() {
        return Err(anyhow!(
            "failed to write OpenCode config in {}: {}",
            OPENCODE_CLI_CONTAINER_NAME,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

fn apply_managed_opencode_mcp_entries(
    existing: &str,
    running_integrations: &[String],
) -> Result<String> {
    let mut config: Value = if existing.trim().is_empty() {
        sonic_rs::json!({})
    } else {
        sonic_rs::from_str(existing).context("failed to parse existing OpenCode config")?
    };

    let root = config
        .as_object_mut()
        .ok_or_else(|| anyhow!("OpenCode config root must be a JSON object"))?;
    let schema = root
        .get(&"$schema")
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .trim()
        .to_string();
    if schema.is_empty() {
        root.insert("$schema", OPENCODE_MCP_SCHEMA_URL);
    }

    if !root
        .get(&"mcp")
        .map(|value| value.is_object())
        .unwrap_or(false)
    {
        root.insert("mcp", sonic_rs::json!({}));
    }

    let mcp = root
        .get_mut(&"mcp")
        .and_then(|value| value.as_object_mut())
        .ok_or_else(|| anyhow!("OpenCode config mcp field must be a JSON object"))?;
    let managed_keys: Vec<String> = mcp
        .iter()
        .filter_map(|(name, _)| {
            let key = name.to_string();
            if key.starts_with(OPENCODE_MANAGED_MCP_PREFIX) {
                Some(key)
            } else {
                None
            }
        })
        .collect();
    for key in managed_keys {
        mcp.remove(&key);
    }

    if running_integrations.iter().any(|id| id == "github") {
        mcp.insert(
            "edgerun-github",
            sonic_rs::json!({
                "command": "docker",
                "args": ["exec", "-i", "edgerun-mcp-github", "server", "stdio"],
                "enabled": true,
            }),
        );
    }

    sonic_rs::to_string_pretty(&config).context("failed to encode OpenCode config")
}

fn sync_opencode_mcp_config() -> Result<()> {
    if !opencode_cli_container_available() {
        eprintln!(
            "opencode MCP sync skipped: container {} is not running",
            OPENCODE_CLI_CONTAINER_NAME
        );
        return Ok(());
    }
    let running_integrations = list_running_mcp_integrations()?;
    let existing = read_opencode_config_text()?;
    let updated = apply_managed_opencode_mcp_entries(&existing, &running_integrations)?;
    write_opencode_config_text(&updated)?;
    Ok(())
}

fn start_local_bridge(local_bridge_listen: &str, device_pubkey_b64url: &str) -> Result<()> {
    let addr: SocketAddr = local_bridge_listen
        .parse()
        .with_context(|| format!("invalid local bridge listen addr: {local_bridge_listen}"))?;
    if !addr.ip().is_loopback() {
        return Err(anyhow!(
            "local bridge must bind to loopback only, got {local_bridge_listen}"
        ));
    }
    let local_fs_root = std::env::var(LOCAL_FS_ROOT_ENV).unwrap_or_else(|_| "/".to_string());
    let local_fs_root = fs::canonicalize(Path::new(local_fs_root.trim()))
        .with_context(|| format!("failed to resolve {}", LOCAL_FS_ROOT_ENV))?;
    if !local_fs_root.is_dir() {
        return Err(anyhow!(
            "{} must resolve to an existing directory",
            LOCAL_FS_ROOT_ENV
        ));
    }
    let state = LocalBridgeState {
        node_id: derive_default_node_id(device_pubkey_b64url),
        device_pubkey_b64url: device_pubkey_b64url.to_string(),
        local_fs_root,
        started_unix_ms: now_unix_ms(),
        tx: broadcast::channel(512).0,
        credentials_lock: Mutex::new(()),
    };
    let state = Arc::new(state);
    spawn_local_docker_events(Arc::clone(&state));
    let app = Router::new()
        .route("/v1/local/node/info.pb", get(handle_local_node_info))
        .route("/v1/local/node/info.pb", options(handle_local_options))
        .route(LOCAL_BRIDGE_EVENTBUS_PATH, get(handle_local_eventbus_ws))
        .route(LOCAL_DOCKER_SUMMARY_PATH, get(handle_local_docker_summary))
        .route(LOCAL_DOCKER_SUMMARY_PATH, options(handle_local_options))
        .route(
            LOCAL_DOCKER_CONTAINER_STATE_PATH,
            post(handle_local_docker_container_state),
        )
        .route(
            LOCAL_DOCKER_CONTAINER_STATE_PATH,
            options(handle_local_options),
        )
        .route(LOCAL_FS_META_PATH, get(handle_local_fs_meta))
        .route(LOCAL_FS_META_PATH, options(handle_local_options))
        .route(LOCAL_FS_LIST_PATH, get(handle_local_fs_list))
        .route(LOCAL_FS_LIST_PATH, options(handle_local_options))
        .route(LOCAL_FS_READ_PATH, get(handle_local_fs_read))
        .route(LOCAL_FS_READ_PATH, options(handle_local_options))
        .route(LOCAL_FS_WRITE_PATH, post(handle_local_fs_write))
        .route(LOCAL_FS_WRITE_PATH, options(handle_local_options))
        .route(LOCAL_FS_MKDIR_PATH, post(handle_local_fs_mkdir))
        .route(LOCAL_FS_MKDIR_PATH, options(handle_local_options))
        .route(LOCAL_FS_DELETE_PATH, post(handle_local_fs_delete))
        .route(LOCAL_FS_DELETE_PATH, options(handle_local_options))
        .route(LOCAL_FS_MOVE_PATH, post(handle_local_fs_move))
        .route(LOCAL_FS_MOVE_PATH, options(handle_local_options))
        .route(LOCAL_FS_COPY_PATH, post(handle_local_fs_copy))
        .route(LOCAL_FS_COPY_PATH, options(handle_local_options))
        .route(LOCAL_FS_ARCHIVE_PATH, post(handle_local_fs_archive))
        .route(LOCAL_FS_ARCHIVE_PATH, options(handle_local_options))
        .route(LOCAL_FS_EXTRACT_PATH, post(handle_local_fs_extract))
        .route(LOCAL_FS_EXTRACT_PATH, options(handle_local_options))
        .route(
            LOCAL_CREDENTIALS_STATUS_PATH,
            get(handle_local_credentials_status),
        )
        .route(LOCAL_CREDENTIALS_STATUS_PATH, options(handle_local_options))
        .route(
            LOCAL_CREDENTIALS_LIST_PATH,
            get(handle_local_credentials_list),
        )
        .route(LOCAL_CREDENTIALS_LIST_PATH, options(handle_local_options))
        .route(
            LOCAL_CREDENTIALS_STORE_PATH,
            post(handle_local_credentials_store),
        )
        .route(LOCAL_CREDENTIALS_STORE_PATH, options(handle_local_options))
        .route(
            LOCAL_CREDENTIALS_DELETE_PATH,
            post(handle_local_credentials_delete),
        )
        .route(LOCAL_CREDENTIALS_DELETE_PATH, options(handle_local_options))
        .route(
            LOCAL_CREDENTIALS_LOCK_PATH,
            post(handle_local_credentials_lock),
        )
        .route(LOCAL_CREDENTIALS_LOCK_PATH, options(handle_local_options))
        .route(
            LOCAL_CREDENTIALS_UNLOCK_PATH,
            post(handle_local_credentials_unlock),
        )
        .route(LOCAL_CREDENTIALS_UNLOCK_PATH, options(handle_local_options))
        .route(
            LOCAL_CREDENTIALS_INTEGRATION_TOKEN_PATH,
            get(handle_local_credentials_integration_token),
        )
        .route(
            LOCAL_CREDENTIALS_INTEGRATION_TOKEN_PATH,
            options(handle_local_options),
        )
        .route(
            LOCAL_CREDENTIALS_GOOGLE_OAUTH_PATH,
            post(handle_local_credentials_google_oauth_store),
        )
        .route(
            LOCAL_CREDENTIALS_GOOGLE_OAUTH_PATH,
            options(handle_local_options),
        )
        .route(LOCAL_MCP_START_PATH, post(handle_local_mcp_start))
        .route(LOCAL_MCP_START_PATH, options(handle_local_options))
        .route(LOCAL_MCP_STOP_PATH, post(handle_local_mcp_stop))
        .route(LOCAL_MCP_STOP_PATH, options(handle_local_options))
        .route(LOCAL_MCP_STATUS_PATH, get(handle_local_mcp_status))
        .route(LOCAL_MCP_STATUS_PATH, options(handle_local_options))
        .route(LOCAL_MCP_PREFLIGHT_PATH, get(handle_local_mcp_preflight))
        .route(LOCAL_MCP_PREFLIGHT_PATH, options(handle_local_options))
        .route(
            LOCAL_CLOUDFLARE_VERIFY_PATH,
            post(handle_local_cloudflare_verify),
        )
        .route(LOCAL_CLOUDFLARE_VERIFY_PATH, options(handle_local_options))
        .route(LOCAL_CLOUDFLARE_ZONES_PATH, get(handle_local_cloudflare_zones))
        .route(LOCAL_CLOUDFLARE_ZONES_PATH, options(handle_local_options))
        .route(
            LOCAL_CLOUDFLARE_TUNNELS_PATH,
            get(handle_local_cloudflare_tunnels),
        )
        .route(LOCAL_CLOUDFLARE_TUNNELS_PATH, options(handle_local_options))
        .route(
            LOCAL_CLOUDFLARE_ACCESS_APPS_PATH,
            get(handle_local_cloudflare_access_apps),
        )
        .route(
            LOCAL_CLOUDFLARE_ACCESS_APPS_PATH,
            options(handle_local_options),
        )
        .route(
            LOCAL_CLOUDFLARE_WORKERS_PATH,
            get(handle_local_cloudflare_workers),
        )
        .route(
            LOCAL_CLOUDFLARE_WORKERS_PATH,
            options(handle_local_options),
        )
        .route(
            LOCAL_CLOUDFLARE_PAGES_PATH,
            get(handle_local_cloudflare_pages),
        )
        .route(LOCAL_CLOUDFLARE_PAGES_PATH, options(handle_local_options))
        .route(
            LOCAL_CLOUDFLARE_DNS_RECORDS_PATH,
            get(handle_local_cloudflare_dns_records),
        )
        .route(
            LOCAL_CLOUDFLARE_DNS_RECORDS_PATH,
            options(handle_local_options),
        )
        .route(
            LOCAL_CLOUDFLARE_DNS_UPSERT_PATH,
            post(handle_local_cloudflare_dns_upsert),
        )
        .route(
            LOCAL_CLOUDFLARE_DNS_UPSERT_PATH,
            options(handle_local_options),
        )
        .route(
            LOCAL_GITHUB_WORKFLOW_RUNS_PATH,
            get(handle_local_github_workflow_runs),
        )
        .route(
            LOCAL_GITHUB_WORKFLOW_RUNS_PATH,
            options(handle_local_options),
        )
        .route(
            LOCAL_GITHUB_WORKFLOW_RUNNER_RUNS_PATH,
            get(handle_local_github_workflow_runner_runs),
        )
        .route(
            LOCAL_GITHUB_WORKFLOW_RUNNER_RUNS_PATH,
            options(handle_local_options),
        )
        .route(
            LOCAL_GITHUB_WORKFLOW_RUNNER_RUN_PATH,
            post(handle_local_github_workflow_runner_run),
        )
        .route(
            LOCAL_GITHUB_WORKFLOW_RUNNER_RUN_PATH,
            options(handle_local_options),
        )
        .route(LOCAL_BEEPER_VERIFY_PATH, post(handle_local_beeper_verify))
        .route(LOCAL_BEEPER_VERIFY_PATH, options(handle_local_options))
        .route(LOCAL_BEEPER_CHATS_PATH, get(handle_local_beeper_chats))
        .route(LOCAL_BEEPER_CHATS_PATH, options(handle_local_options))
        .route(LOCAL_BEEPER_MESSAGES_PATH, get(handle_local_beeper_messages))
        .route(LOCAL_BEEPER_MESSAGES_PATH, options(handle_local_options))
        .route(LOCAL_BEEPER_IMPORTED_PATH, get(handle_local_beeper_imported))
        .route(LOCAL_BEEPER_IMPORTED_PATH, options(handle_local_options))
        .route(LOCAL_BEEPER_MEDIA_PATH, get(handle_local_beeper_media))
        .route(LOCAL_BEEPER_MEDIA_PATH, options(handle_local_options))
        .route(LOCAL_BEEPER_SEND_PATH, post(handle_local_beeper_send))
        .route(LOCAL_BEEPER_SEND_PATH, options(handle_local_options))
        .route(
            LOCAL_TAILSCALE_DEVICES_PATH,
            post(handle_local_tailscale_devices),
        )
        .route(LOCAL_TAILSCALE_DEVICES_PATH, options(handle_local_options))
        .route(LOCAL_ONVIF_DISCOVER_PATH, get(handle_local_onvif_discover))
        .route(LOCAL_ONVIF_DISCOVER_PATH, options(handle_local_options))
        .route(LOCAL_GOOGLE_MESSAGES_PATH, get(handle_local_google_messages))
        .route(LOCAL_GOOGLE_MESSAGES_PATH, options(handle_local_options))
        .route(LOCAL_GOOGLE_MESSAGE_PATH, get(handle_local_google_message))
        .route(LOCAL_GOOGLE_MESSAGE_PATH, options(handle_local_options))
        .route(LOCAL_GOOGLE_EVENTS_PATH, get(handle_local_google_events))
        .route(LOCAL_GOOGLE_EVENTS_PATH, options(handle_local_options))
        .route(LOCAL_GOOGLE_CONTACTS_PATH, get(handle_local_google_contacts))
        .route(LOCAL_GOOGLE_CONTACTS_PATH, options(handle_local_options))
        .route(LOCAL_GOOGLE_DRIVE_FILES_PATH, get(handle_local_google_drive_files))
        .route(LOCAL_GOOGLE_DRIVE_FILES_PATH, options(handle_local_options))
        .route(LOCAL_GOOGLE_DRIVE_FILE_PATH, get(handle_local_google_drive_file))
        .route(LOCAL_GOOGLE_DRIVE_FILE_PATH, options(handle_local_options))
        .route(LOCAL_GOOGLE_PHOTOS_PATH, get(handle_local_google_photos))
        .route(LOCAL_GOOGLE_PHOTOS_PATH, options(handle_local_options))
        .route(LOCAL_GOOGLE_REFRESH_PATH, post(handle_local_google_refresh))
        .route(LOCAL_GOOGLE_REFRESH_PATH, options(handle_local_options))
        .route(LOCAL_GOOGLE_OAUTH_START_PATH, get(handle_local_google_oauth_start))
        .route(LOCAL_GOOGLE_OAUTH_START_PATH, options(handle_local_options))
        .route(LOCAL_GOOGLE_OAUTH_CALLBACK_PATH, get(handle_local_google_oauth_callback))
        .route(LOCAL_GOOGLE_OAUTH_CALLBACK_PATH, options(handle_local_options))
        .route(LOCAL_ASSISTANT_PATH, post(handle_local_assistant))
        .route(LOCAL_ASSISTANT_PATH, options(handle_local_options))
        .with_state(state);
    tokio::spawn(async move {
        let listener = match tokio::net::TcpListener::bind(addr).await {
            Ok(listener) => listener,
            Err(err) => {
                eprintln!("local_bridge_bind_error={err}");
                return;
            }
        };
        if let Err(err) = axum::serve(listener, app).await {
            eprintln!("local_bridge_serve_error={err}");
        }
    });
    Ok(())
}

async fn handle_local_node_info(State(state): State<Arc<LocalBridgeState>>) -> Response {
    let payload = LocalNodeInfoResponseV1 {
        ok: true,
        error: String::new(),
        node_id: state.node_id.clone(),
        device_pubkey_b64url: state.device_pubkey_b64url.clone(),
        bridge_version: LOCAL_BRIDGE_VERSION.to_string(),
        started_unix_ms: state.started_unix_ms,
        eventbus_ws_path: LOCAL_BRIDGE_EVENTBUS_PATH.to_string(),
    }
    .encode_to_vec();
    (
        AxumStatusCode::OK,
        [
            (
                AXUM_CONTENT_TYPE,
                HeaderValue::from_static("application/x-protobuf"),
            ),
            (CACHE_CONTROL, HeaderValue::from_static("no-store")),
            (ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*")),
            (
                ACCESS_CONTROL_ALLOW_HEADERS,
                HeaderValue::from_static("content-type"),
            ),
            (
                ACCESS_CONTROL_ALLOW_METHODS,
                HeaderValue::from_static("GET, OPTIONS"),
            ),
        ],
        payload,
    )
        .into_response()
}

async fn handle_local_options() -> Response {
    (
        AxumStatusCode::NO_CONTENT,
        [
            (ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*")),
            (
                ACCESS_CONTROL_ALLOW_HEADERS,
                HeaderValue::from_static("content-type"),
            ),
            (
                ACCESS_CONTROL_ALLOW_METHODS,
                HeaderValue::from_static("GET, POST, OPTIONS"),
            ),
        ],
    )
        .into_response()
}

async fn handle_local_eventbus_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<LocalBridgeState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| local_eventbus_session(socket, state))
}

async fn handle_local_fs_meta(
    State(state): State<Arc<LocalBridgeState>>,
    Query(query): Query<LocalFsQuery>,
) -> Response {
    if let Err(err) = enforce_local_node(&state, query.node_id.as_deref()) {
        return local_json_error(AxumStatusCode::FORBIDDEN, &err.to_string());
    }
    local_json_ok(LocalFsMetaData {
        local_fs_root: state.local_fs_root.to_string_lossy().to_string(),
    })
}

async fn handle_local_fs_list(
    State(state): State<Arc<LocalBridgeState>>,
    Query(query): Query<LocalFsQuery>,
) -> Response {
    if let Err(err) = enforce_local_node(&state, query.node_id.as_deref()) {
        return local_json_error(AxumStatusCode::FORBIDDEN, &err.to_string());
    }
    let requested = query.path.unwrap_or_else(|| "/".to_string());
    let display = match local_fs_display_path(&requested) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let absolute = match local_fs_abs_path(&state.local_fs_root, &display) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    if let Err(err) = assert_path_under_root(&state.local_fs_root, &absolute) {
        return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string());
    }
    let metadata = match fs::metadata(&absolute) {
        Ok(meta) => meta,
        Err(err) => return local_json_error(AxumStatusCode::NOT_FOUND, &err.to_string()),
    };
    if !metadata.is_dir() {
        return local_json_error(
            AxumStatusCode::BAD_REQUEST,
            "list path must resolve to a directory",
        );
    }
    let mut entries = Vec::new();
    let iterator = match fs::read_dir(&absolute) {
        Ok(value) => value,
        Err(err) => {
            return local_json_error(AxumStatusCode::INTERNAL_SERVER_ERROR, &err.to_string())
        }
    };
    for item in iterator {
        let item = match item {
            Ok(value) => value,
            Err(err) => {
                return local_json_error(AxumStatusCode::INTERNAL_SERVER_ERROR, &err.to_string())
            }
        };
        let path = item.path();
        match local_fs_entry(&state.local_fs_root, &path) {
            Ok(entry) => entries.push(entry),
            Err(err) => {
                return local_json_error(AxumStatusCode::INTERNAL_SERVER_ERROR, &err.to_string())
            }
        }
    }
    entries.sort_by(|a, b| {
        let a_kind = a.kind.as_str();
        let b_kind = b.kind.as_str();
        if a_kind != b_kind {
            return if a_kind == "dir" {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            };
        }
        a.name.to_lowercase().cmp(&b.name.to_lowercase())
    });
    local_json_ok(LocalFsListData { entries })
}

async fn handle_local_fs_read(
    State(state): State<Arc<LocalBridgeState>>,
    Query(query): Query<LocalFsQuery>,
) -> Response {
    if let Err(err) = enforce_local_node(&state, query.node_id.as_deref()) {
        return local_json_error(AxumStatusCode::FORBIDDEN, &err.to_string());
    }
    let requested = query.path.unwrap_or_else(|| "/".to_string());
    let display = match local_fs_display_path(&requested) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let absolute = match local_fs_abs_path(&state.local_fs_root, &display) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    if let Err(err) = assert_path_under_root(&state.local_fs_root, &absolute) {
        return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string());
    }
    let bytes = match fs::read(&absolute) {
        Ok(value) => value,
        Err(err) => return local_json_error(AxumStatusCode::NOT_FOUND, &err.to_string()),
    };
    let content = match String::from_utf8(bytes) {
        Ok(value) => value,
        Err(_) => {
            return local_json_error(AxumStatusCode::BAD_REQUEST, "file content is not utf-8")
        }
    };
    local_json_ok(LocalFsReadData { content })
}

async fn handle_local_fs_write(
    State(state): State<Arc<LocalBridgeState>>,
    Json(body): Json<LocalFsWriteRequest>,
) -> Response {
    if let Err(err) = enforce_local_node(&state, body.node_id.as_deref()) {
        return local_json_error(AxumStatusCode::FORBIDDEN, &err.to_string());
    }
    let display = match local_fs_display_path(&body.path) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let absolute = match local_fs_abs_path(&state.local_fs_root, &display) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    if let Err(err) = assert_path_under_root(&state.local_fs_root, &absolute) {
        return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string());
    }
    if let Some(parent) = absolute.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            return local_json_error(AxumStatusCode::INTERNAL_SERVER_ERROR, &err.to_string());
        }
    }
    if let Err(err) = fs::write(&absolute, body.content.as_bytes()) {
        return local_json_error(AxumStatusCode::INTERNAL_SERVER_ERROR, &err.to_string());
    }
    local_json_ok(LocalFsMetaData {
        local_fs_root: state.local_fs_root.to_string_lossy().to_string(),
    })
}

async fn handle_local_fs_mkdir(
    State(state): State<Arc<LocalBridgeState>>,
    Json(body): Json<LocalFsPathRequest>,
) -> Response {
    if let Err(err) = enforce_local_node(&state, body.node_id.as_deref()) {
        return local_json_error(AxumStatusCode::FORBIDDEN, &err.to_string());
    }
    let display = match local_fs_display_path(&body.path) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let absolute = match local_fs_abs_path(&state.local_fs_root, &display) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    if let Err(err) = assert_path_under_root(&state.local_fs_root, &absolute) {
        return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string());
    }
    if let Err(err) = fs::create_dir_all(&absolute) {
        return local_json_error(AxumStatusCode::INTERNAL_SERVER_ERROR, &err.to_string());
    }
    local_json_ok(LocalFsMetaData {
        local_fs_root: state.local_fs_root.to_string_lossy().to_string(),
    })
}

async fn handle_local_fs_delete(
    State(state): State<Arc<LocalBridgeState>>,
    Json(body): Json<LocalFsPathRequest>,
) -> Response {
    if let Err(err) = enforce_local_node(&state, body.node_id.as_deref()) {
        return local_json_error(AxumStatusCode::FORBIDDEN, &err.to_string());
    }
    let display = match local_fs_display_path(&body.path) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let absolute = match local_fs_abs_path(&state.local_fs_root, &display) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    if let Err(err) = assert_path_under_root(&state.local_fs_root, &absolute) {
        return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string());
    }
    let metadata = match fs::metadata(&absolute) {
        Ok(meta) => meta,
        Err(err) => return local_json_error(AxumStatusCode::NOT_FOUND, &err.to_string()),
    };
    let result = if metadata.is_dir() {
        fs::remove_dir_all(&absolute)
    } else {
        fs::remove_file(&absolute)
    };
    if let Err(err) = result {
        return local_json_error(AxumStatusCode::INTERNAL_SERVER_ERROR, &err.to_string());
    }
    local_json_ok(LocalFsMetaData {
        local_fs_root: state.local_fs_root.to_string_lossy().to_string(),
    })
}

async fn handle_local_fs_move(
    State(state): State<Arc<LocalBridgeState>>,
    Json(body): Json<LocalFsMoveRequest>,
) -> Response {
    if let Err(err) = enforce_local_node(&state, body.node_id.as_deref()) {
        return local_json_error(AxumStatusCode::FORBIDDEN, &err.to_string());
    }
    let from_display = match local_fs_display_path(&body.from) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let to_display = match local_fs_display_path(&body.to) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let from_absolute = match local_fs_abs_path(&state.local_fs_root, &from_display) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let to_absolute = match local_fs_abs_path(&state.local_fs_root, &to_display) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    if let Err(err) = assert_path_under_root(&state.local_fs_root, &from_absolute) {
        return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string());
    }
    if let Err(err) = assert_path_under_root(&state.local_fs_root, &to_absolute) {
        return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string());
    }
    if let Some(parent) = to_absolute.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            return local_json_error(AxumStatusCode::INTERNAL_SERVER_ERROR, &err.to_string());
        }
    }
    if let Err(err) = fs::rename(&from_absolute, &to_absolute) {
        return local_json_error(AxumStatusCode::INTERNAL_SERVER_ERROR, &err.to_string());
    }
    local_json_ok(LocalFsMetaData {
        local_fs_root: state.local_fs_root.to_string_lossy().to_string(),
    })
}

async fn handle_local_fs_copy(
    State(state): State<Arc<LocalBridgeState>>,
    Json(body): Json<LocalFsMoveRequest>,
) -> Response {
    if let Err(err) = enforce_local_node(&state, body.node_id.as_deref()) {
        return local_json_error(AxumStatusCode::FORBIDDEN, &err.to_string());
    }
    let from_display = match local_fs_display_path(&body.from) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let to_display = match local_fs_display_path(&body.to) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let from_absolute = match local_fs_abs_path(&state.local_fs_root, &from_display) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let to_absolute = match local_fs_abs_path(&state.local_fs_root, &to_display) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    if let Err(err) = assert_path_under_root(&state.local_fs_root, &from_absolute) {
        return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string());
    }
    if let Err(err) = assert_path_under_root(&state.local_fs_root, &to_absolute) {
        return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string());
    }
    if let Err(err) = local_fs_copy_recursive(&from_absolute, &to_absolute) {
        return local_json_error(AxumStatusCode::INTERNAL_SERVER_ERROR, &err.to_string());
    }
    local_json_ok(LocalFsMetaData {
        local_fs_root: state.local_fs_root.to_string_lossy().to_string(),
    })
}

async fn handle_local_fs_archive(
    State(state): State<Arc<LocalBridgeState>>,
    Json(body): Json<LocalFsArchiveRequest>,
) -> Response {
    if let Err(err) = enforce_local_node(&state, body.node_id.as_deref()) {
        return local_json_error(AxumStatusCode::FORBIDDEN, &err.to_string());
    }
    let display = match local_fs_display_path(&body.path) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let absolute = match local_fs_abs_path(&state.local_fs_root, &display) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    if let Err(err) = assert_path_under_root(&state.local_fs_root, &absolute) {
        return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string());
    }
    let archive = local_fs_archive_path(&absolute, body.format.as_deref());
    if let Err(err) = assert_path_under_root(&state.local_fs_root, &archive) {
        return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string());
    }
    let parent = match absolute.parent() {
        Some(path) => path,
        None => {
            return local_json_error(AxumStatusCode::BAD_REQUEST, "invalid archive source path")
        }
    };
    let source_name = match absolute.file_name() {
        Some(name) => name.to_string_lossy().to_string(),
        None => {
            return local_json_error(AxumStatusCode::BAD_REQUEST, "invalid archive source path")
        }
    };
    let status = Command::new("tar")
        .arg("-czf")
        .arg(&archive)
        .arg("-C")
        .arg(parent)
        .arg(source_name)
        .status();
    match status {
        Ok(result) if result.success() => {}
        Ok(result) => {
            return local_json_error(
                AxumStatusCode::INTERNAL_SERVER_ERROR,
                &format!("tar failed with status {result}"),
            )
        }
        Err(err) => {
            return local_json_error(AxumStatusCode::INTERNAL_SERVER_ERROR, &err.to_string())
        }
    }
    let archive_display = match archive.strip_prefix(&state.local_fs_root) {
        Ok(rel) if rel.as_os_str().is_empty() => "/".to_string(),
        Ok(rel) => format!("/{}", rel.to_string_lossy()),
        Err(_) => archive.to_string_lossy().to_string(),
    };
    local_json_ok(LocalFsArchiveData {
        archive_path: archive_display,
    })
}

async fn handle_local_fs_extract(
    State(state): State<Arc<LocalBridgeState>>,
    Json(body): Json<LocalFsPathRequest>,
) -> Response {
    if let Err(err) = enforce_local_node(&state, body.node_id.as_deref()) {
        return local_json_error(AxumStatusCode::FORBIDDEN, &err.to_string());
    }
    let display = match local_fs_display_path(&body.path) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let archive = match local_fs_abs_path(&state.local_fs_root, &display) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    if let Err(err) = assert_path_under_root(&state.local_fs_root, &archive) {
        return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string());
    }
    let parent = match archive.parent() {
        Some(path) => path,
        None => return local_json_error(AxumStatusCode::BAD_REQUEST, "invalid archive path"),
    };
    let status = Command::new("tar")
        .arg("-xf")
        .arg(&archive)
        .arg("-C")
        .arg(parent)
        .status();
    match status {
        Ok(result) if result.success() => {}
        Ok(result) => {
            return local_json_error(
                AxumStatusCode::INTERNAL_SERVER_ERROR,
                &format!("extract failed with status {result}"),
            )
        }
        Err(err) => {
            return local_json_error(AxumStatusCode::INTERNAL_SERVER_ERROR, &err.to_string())
        }
    }
    local_json_ok(LocalFsMetaData {
        local_fs_root: state.local_fs_root.to_string_lossy().to_string(),
    })
}

fn normalize_credential_type(value: Option<&str>) -> String {
    let trimmed = value.unwrap_or("").trim();
    if trimmed.is_empty() {
        "secret".to_string()
    } else {
        trimmed.to_string()
    }
}

fn local_credential_store_name(body: &LocalCredentialStoreRequest) -> Option<String> {
    let name = body.name.as_deref().unwrap_or("").trim();
    if !name.is_empty() {
        return Some(name.to_string());
    }
    if let Some(raw) = body.entry_id.as_deref() {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    None
}

fn find_integration_token_entry<'a>(
    entries: &'a [LocalCredentialEntry],
    integration_id: &str,
) -> Option<&'a LocalCredentialEntry> {
    let expected_name = format!("integration/{integration_id}/token");
    entries.iter().find(|entry| {
        entry.name.eq_ignore_ascii_case(&expected_name)
            || entry.entry_id.eq_ignore_ascii_case(&expected_name)
    })
}

fn find_credential_secret_by_name(cfg: &ManagerConfig, name: &str) -> Option<String> {
    let target = name.trim();
    if target.is_empty() {
        return None;
    }
    cfg.credentials
        .iter()
        .find(|entry| {
            entry.name.eq_ignore_ascii_case(target) || entry.entry_id.eq_ignore_ascii_case(target)
        })
        .map(|entry| entry.secret.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn google_oauth_client_id() -> Option<String> {
    let from_env = std::env::var(GOOGLE_OAUTH_CLIENT_ID_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if from_env.is_some() {
        return from_env;
    }
    if let Ok(secrets) = load_google_oauth_secrets_from_nv() {
        let value = secrets.client_id.trim().to_string();
        if !value.is_empty() {
            return Some(value);
        }
    }
    let cfg = load_config().ok()?;
    find_credential_secret_by_name(&cfg, "google/oauth/client_id")
}

fn google_oauth_client_secret() -> Option<String> {
    let from_env = std::env::var(GOOGLE_OAUTH_CLIENT_SECRET_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if from_env.is_some() {
        return from_env;
    }
    if let Ok(secrets) = load_google_oauth_secrets_from_nv() {
        let value = secrets.client_secret.trim().to_string();
        if !value.is_empty() {
            return Some(value);
        }
    }
    let cfg = load_config().ok()?;
    find_credential_secret_by_name(&cfg, "google/oauth/client_secret")
}

async fn handle_local_credentials_status(State(state): State<Arc<LocalBridgeState>>) -> Response {
    let _guard = state.credentials_lock.lock().await;
    let cfg = load_config().unwrap_or_else(|_| default_manager_config());
    local_json_ok(LocalCredentialsStatusData {
        installed: true,
        locked: false,
        count: cfg.credentials.len() as u64,
        backend: "tpm".to_string(),
    })
}

async fn handle_local_credentials_list(State(state): State<Arc<LocalBridgeState>>) -> Response {
    let _guard = state.credentials_lock.lock().await;
    let mut cfg = load_config().unwrap_or_else(|_| default_manager_config());
    cfg.credentials
        .sort_by(|a, b| b.updated_unix_ms.cmp(&a.updated_unix_ms));
    let count = cfg.credentials.len() as u64;
    local_json_ok(LocalCredentialsListData {
        entries: cfg.credentials,
        count,
        locked: false,
    })
}

async fn handle_local_credentials_store(
    State(state): State<Arc<LocalBridgeState>>,
    Json(body): Json<LocalCredentialStoreRequest>,
) -> Response {
    let name = match local_credential_store_name(&body) {
        Some(value) => value,
        None => {
            return local_json_error(AxumStatusCode::BAD_REQUEST, "credential name is required")
        }
    };
    let mut entry_id = body.entry_id.unwrap_or_default().trim().to_string();
    let secret = body.secret.unwrap_or_default().trim().to_string();
    if secret.is_empty() {
        return local_json_error(AxumStatusCode::BAD_REQUEST, "credential secret is required");
    }
    if entry_id.is_empty() {
        entry_id = name.clone();
    }

    let now = now_unix_ms();
    let _guard = state.credentials_lock.lock().await;
    let mut cfg = load_config().unwrap_or_else(|_| default_manager_config());
    let mut updated = false;
    for entry in &mut cfg.credentials {
        if entry.entry_id.eq_ignore_ascii_case(&entry_id) || entry.name.eq_ignore_ascii_case(&name)
        {
            entry.credential_type = normalize_credential_type(body.credential_type.as_deref());
            entry.name = name.clone();
            entry.username = body.username.clone().unwrap_or_default().trim().to_string();
            entry.secret = secret.clone();
            entry.url = body.url.clone().unwrap_or_default().trim().to_string();
            entry.note = body.note.clone().unwrap_or_default().trim().to_string();
            entry.tags = body.tags.clone().unwrap_or_default().trim().to_string();
            entry.folder = body.folder.clone().unwrap_or_default().trim().to_string();
            entry.updated_unix_ms = now;
            updated = true;
            break;
        }
    }
    if !updated {
        cfg.credentials.push(LocalCredentialEntry {
            entry_id,
            credential_type: normalize_credential_type(body.credential_type.as_deref()),
            name,
            username: body.username.unwrap_or_default().trim().to_string(),
            secret,
            url: body.url.unwrap_or_default().trim().to_string(),
            note: body.note.unwrap_or_default().trim().to_string(),
            tags: body.tags.unwrap_or_default().trim().to_string(),
            folder: body.folder.unwrap_or_default().trim().to_string(),
            created_unix_ms: now,
            updated_unix_ms: now,
        });
    }
    if let Err(err) = save_config(&cfg) {
        return local_json_error(AxumStatusCode::INTERNAL_SERVER_ERROR, &err.to_string());
    }
    local_json_ok(LocalFsEmptyData {})
}

async fn handle_local_credentials_delete(
    State(state): State<Arc<LocalBridgeState>>,
    Json(body): Json<LocalCredentialDeleteRequest>,
) -> Response {
    let entry_id = body.entry_id.unwrap_or_default().trim().to_string();
    if entry_id.is_empty() {
        return local_json_error(AxumStatusCode::BAD_REQUEST, "entryId is required");
    }
    let _guard = state.credentials_lock.lock().await;
    let mut cfg = load_config().unwrap_or_else(|_| default_manager_config());
    let initial_len = cfg.credentials.len();
    cfg.credentials
        .retain(|entry| !entry.entry_id.eq_ignore_ascii_case(&entry_id));
    if cfg.credentials.len() == initial_len {
        return local_json_error(AxumStatusCode::NOT_FOUND, "credential not found");
    }
    if let Err(err) = save_config(&cfg) {
        return local_json_error(AxumStatusCode::INTERNAL_SERVER_ERROR, &err.to_string());
    }
    local_json_ok(LocalFsEmptyData {})
}

async fn handle_local_credentials_lock(State(state): State<Arc<LocalBridgeState>>) -> Response {
    let _guard = state.credentials_lock.lock().await;
    let cfg = load_config().unwrap_or_else(|_| default_manager_config());
    local_json_ok(LocalCredentialsStatusData {
        installed: true,
        locked: false,
        count: cfg.credentials.len() as u64,
        backend: "tpm".to_string(),
    })
}

async fn handle_local_credentials_unlock(
    State(state): State<Arc<LocalBridgeState>>,
    Json(_body): Json<LocalCredentialUnlockRequest>,
) -> Response {
    let _guard = state.credentials_lock.lock().await;
    let cfg = load_config().unwrap_or_else(|_| default_manager_config());
    local_json_ok(LocalCredentialsStatusData {
        installed: true,
        locked: false,
        count: cfg.credentials.len() as u64,
        backend: "tpm".to_string(),
    })
}

async fn handle_local_credentials_google_oauth_store(
    State(state): State<Arc<LocalBridgeState>>,
    Json(body): Json<LocalGoogleOauthCredentialStoreRequest>,
) -> Response {
    let client_id = body.client_id.trim().to_string();
    let client_secret = body.client_secret.trim().to_string();
    if client_id.len() < 16 || client_secret.len() < 12 {
        return local_json_error(
            AxumStatusCode::BAD_REQUEST,
            "google oauth client_id/client_secret are missing or invalid",
        );
    }
    let _guard = state.credentials_lock.lock().await;
    let secrets = GoogleOauthStoredSecrets {
        client_id,
        client_secret,
    };
    if let Err(err) = save_google_oauth_secrets_to_nv(&secrets) {
        return local_json_error(AxumStatusCode::INTERNAL_SERVER_ERROR, &err.to_string());
    }
    local_json_ok(LocalFsEmptyData {})
}

async fn handle_local_credentials_integration_token(
    State(state): State<Arc<LocalBridgeState>>,
    Query(query): Query<LocalCredentialTokenQuery>,
) -> Response {
    let integration_id = query
        .integration_id
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    if integration_id.is_empty() {
        return local_json_error(AxumStatusCode::BAD_REQUEST, "integration_id is required");
    }
    let _guard = state.credentials_lock.lock().await;
    let cfg = load_config().unwrap_or_else(|_| default_manager_config());
    if let Some(entry) = find_integration_token_entry(&cfg.credentials, &integration_id) {
        return local_json_ok(LocalCredentialTokenData {
            integration_id,
            token: entry.secret.clone(),
        });
    }
    local_json_ok(LocalCredentialTokenData {
        integration_id,
        token: String::new(),
    })
}

async fn handle_local_mcp_start(
    State(state): State<Arc<LocalBridgeState>>,
    Json(body): Json<LocalMcpStartRequest>,
) -> Response {
    if let Err(err) = enforce_local_node(&state, body.node_id.as_deref()) {
        return local_json_error(AxumStatusCode::FORBIDDEN, &err.to_string());
    }
    let integration_id = body.integration_id.trim().to_ascii_lowercase();
    if integration_id.is_empty() {
        return local_json_error(AxumStatusCode::BAD_REQUEST, "integration_id is required");
    }
    let token = body.token.trim().to_string();
    if token.len() < 8 {
        return local_json_error(
            AxumStatusCode::BAD_REQUEST,
            "integration token is missing or invalid",
        );
    }
    let image = match mcp_image_for(&integration_id) {
        Some(value) => value,
        None => {
            return local_json_error(
                AxumStatusCode::BAD_REQUEST,
                &format!("no MCP image mapping for integration {}", integration_id),
            )
        }
    };
    let container_name = mcp_container_name(&integration_id);
    let _ = Command::new("docker")
        .args(["rm", "-f", &container_name])
        .output();
    let token_env = mcp_token_env_for(&integration_id);
    let nats_url = std::env::var(EVENTBUS_NATS_URL_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| EVENTBUS_NATS_URL_DEFAULT.to_string());
    let mut args = vec![
        "run".to_string(),
        "-d".to_string(),
        "--name".to_string(),
        container_name.clone(),
        "--restart".to_string(),
        "unless-stopped".to_string(),
        "-e".to_string(),
        format!("{}={}", token_env, token),
        "-e".to_string(),
        format!("EDGERUN_INTEGRATION_ID={}", integration_id),
        "-e".to_string(),
        format!("EDGERUN_EVENTBUS_NATS_URL={}", nats_url),
        "-e".to_string(),
        format!(
            "EDGERUN_EVENTBUS_TOPIC_INBOUND={}",
            integration_topic_for(&integration_id, "inbound")
        ),
        "-e".to_string(),
        format!(
            "EDGERUN_EVENTBUS_TOPIC_OUTBOUND={}",
            integration_topic_for(&integration_id, "outbound")
        ),
        "-e".to_string(),
        format!(
            "EDGERUN_EVENTBUS_TOPIC_ERRORS={}",
            integration_topic_for(&integration_id, "errors")
        ),
        image.clone(),
    ];
    args.extend(mcp_container_command_for(&integration_id));
    let output = match Command::new("docker").args(args.drain(..)).output() {
        Ok(value) => value,
        Err(err) => {
            return local_json_error(AxumStatusCode::INTERNAL_SERVER_ERROR, &err.to_string())
        }
    };
    if !output.status.success() {
        return local_json_error(
            AxumStatusCode::INTERNAL_SERVER_ERROR,
            &format!(
                "failed to start MCP container {}: {}",
                container_name,
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        );
    }
    if let Err(err) = sync_opencode_mcp_config() {
        return local_json_error(
            AxumStatusCode::INTERNAL_SERVER_ERROR,
            &format!("MCP started but opencode config sync failed: {err}"),
        );
    }
    local_json_ok(LocalMcpStatusData {
        integration_id,
        container_name,
        running: true,
        status: "running".to_string(),
    })
}

async fn handle_local_mcp_stop(
    State(state): State<Arc<LocalBridgeState>>,
    Json(body): Json<LocalMcpStopRequest>,
) -> Response {
    if let Err(err) = enforce_local_node(&state, body.node_id.as_deref()) {
        return local_json_error(AxumStatusCode::FORBIDDEN, &err.to_string());
    }
    let integration_id = body.integration_id.trim().to_ascii_lowercase();
    if integration_id.is_empty() {
        return local_json_error(AxumStatusCode::BAD_REQUEST, "integration_id is required");
    }
    let container_name = mcp_container_name(&integration_id);
    let output = match Command::new("docker")
        .args(["rm", "-f", &container_name])
        .output()
    {
        Ok(value) => value,
        Err(err) => {
            return local_json_error(AxumStatusCode::INTERNAL_SERVER_ERROR, &err.to_string())
        }
    };
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.to_ascii_lowercase().contains("no such container") {
            return local_json_error(
                AxumStatusCode::INTERNAL_SERVER_ERROR,
                &format!(
                    "failed to stop MCP container {}: {}",
                    container_name,
                    stderr.trim()
                ),
            );
        }
    }
    if let Err(err) = sync_opencode_mcp_config() {
        return local_json_error(
            AxumStatusCode::INTERNAL_SERVER_ERROR,
            &format!("MCP stopped but opencode config sync failed: {err}"),
        );
    }
    local_json_ok(LocalMcpStatusData {
        integration_id,
        container_name,
        running: false,
        status: "stopped".to_string(),
    })
}

async fn handle_local_mcp_status(
    State(state): State<Arc<LocalBridgeState>>,
    Query(query): Query<LocalMcpStatusQuery>,
) -> Response {
    if let Err(err) = enforce_local_node(&state, query.node_id.as_deref()) {
        return local_json_error(AxumStatusCode::FORBIDDEN, &err.to_string());
    }
    let integration_id = query.integration_id.trim().to_ascii_lowercase();
    if integration_id.is_empty() {
        return local_json_error(AxumStatusCode::BAD_REQUEST, "integration_id is required");
    }
    let container_name = mcp_container_name(&integration_id);
    let output = match Command::new("docker")
        .args([
            "inspect",
            "-f",
            "{{.State.Running}}\t{{.State.Status}}",
            &container_name,
        ])
        .output()
    {
        Ok(value) => value,
        Err(err) => {
            return local_json_error(AxumStatusCode::INTERNAL_SERVER_ERROR, &err.to_string())
        }
    };
    if !output.status.success() {
        return local_json_ok(LocalMcpStatusData {
            integration_id,
            container_name,
            running: false,
            status: "not_found".to_string(),
        });
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let cols: Vec<&str> = stdout.split('\t').collect();
    let running = cols
        .first()
        .map(|value| value.trim().eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let status = cols
        .get(1)
        .map(|value| value.trim().to_string())
        .unwrap_or_else(|| {
            if running {
                "running".to_string()
            } else {
                "stopped".to_string()
            }
        });
    local_json_ok(LocalMcpStatusData {
        integration_id,
        container_name,
        running,
        status,
    })
}

async fn handle_local_mcp_preflight(
    State(state): State<Arc<LocalBridgeState>>,
    Query(query): Query<LocalMcpPreflightQuery>,
) -> Response {
    if let Err(err) = enforce_local_node(&state, query.node_id.as_deref()) {
        return local_json_error(AxumStatusCode::FORBIDDEN, &err.to_string());
    }
    let integration_id = query.integration_id.trim().to_ascii_lowercase();
    if integration_id.is_empty() {
        return local_json_error(AxumStatusCode::BAD_REQUEST, "integration_id is required");
    }
    let container_name = mcp_container_name(&integration_id);
    let image = mcp_image_for(&integration_id).unwrap_or_default();
    let token_env = mcp_token_env_for(&integration_id).to_string();
    local_json_ok(LocalMcpPreflightData {
        integration_id,
        container_name,
        token_env,
        image: image.clone(),
        image_resolved: !image.trim().is_empty(),
    })
}

async fn handle_local_beeper_verify(Json(body): Json<LocalBeeperVerifyRequest>) -> Response {
    let token = match beeper_token_from(body.token.as_deref().unwrap_or_default()) {
        Ok(value) => value,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let payload = match beeper_api_verify_token(&token).await {
        Ok(value) => value,
        Err(err) => return local_json_error(AxumStatusCode::BAD_GATEWAY, &err.to_string()),
    };
    let account_count = payload["items"]
        .as_array()
        .map(|items| items.len())
        .or_else(|| payload.as_array().map(|items| items.len()))
        .unwrap_or(0);
    local_json_value(
        AxumStatusCode::OK,
        sonic_rs::json!({
            "ok": true,
            "accounts": payload,
            "account_count": account_count,
            "api_base": beeper_api_base(),
        }),
    )
}

async fn handle_local_beeper_chats(Query(query): Query<LocalBeeperChatsQuery>) -> Response {
    let token = match beeper_token_from(&query.token) {
        Ok(value) => value,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let payload = match beeper_api_get_chats(&token, limit).await {
        Ok(value) => value,
        Err(err) => return local_json_error(AxumStatusCode::BAD_GATEWAY, &err.to_string()),
    };
    local_json_value(
        AxumStatusCode::OK,
        sonic_rs::json!({
            "ok": true,
            "items": payload["items"],
        }),
    )
}

async fn handle_local_beeper_messages(Query(query): Query<LocalBeeperMessagesQuery>) -> Response {
    let token = match beeper_token_from(&query.token) {
        Ok(value) => value,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let chat_id = match query.chat_id.as_deref().map(str::trim) {
        Some(value) if !value.is_empty() => value.to_string(),
        _ => return local_json_error(AxumStatusCode::BAD_REQUEST, "chat_id is required"),
    };
    let limit = query.limit.unwrap_or(40).clamp(1, 200);
    let payload = match beeper_api_get_chat_messages(&token, &chat_id, limit).await {
        Ok(value) => value,
        Err(err) => return local_json_error(AxumStatusCode::BAD_GATEWAY, &err.to_string()),
    };
    local_json_value(
        AxumStatusCode::OK,
        sonic_rs::json!({
            "ok": true,
            "items": payload["items"],
        }),
    )
}

async fn handle_local_beeper_imported(Query(query): Query<LocalBeeperImportedQuery>) -> Response {
    let limit_threads = query.limit_threads.unwrap_or(100).clamp(1, 500);
    let limit_messages = query.limit_messages.unwrap_or(120).clamp(1, 1000);
    let items = match load_imported_fb_threads(limit_threads, limit_messages) {
        Ok(value) => value,
        Err(err) => return local_json_error(AxumStatusCode::BAD_GATEWAY, &err.to_string()),
    };
    local_json_value(
        AxumStatusCode::OK,
        sonic_rs::json!({
            "ok": true,
            "items": items,
            "count": items.len(),
            "source_dir": beeper_import_dir(),
        }),
    )
}

async fn handle_local_beeper_media(Query(query): Query<LocalBeeperMediaQuery>) -> Response {
    let path = match resolve_beeper_media_path(&query.uri) {
        Ok(value) => value,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let bytes = match fs::read(&path) {
        Ok(value) => value,
        Err(err) => {
            return local_json_error(
                AxumStatusCode::NOT_FOUND,
                &format!("failed to read media {}: {err}", path.display()),
            )
        }
    };
    let mut response = (AxumStatusCode::OK, bytes).into_response();
    if let Ok(value) = HeaderValue::from_str(beeper_media_content_type(&path)) {
        response.headers_mut().insert(AXUM_CONTENT_TYPE, value);
    }
    if let Ok(value) = HeaderValue::from_str("private, max-age=3600") {
        response.headers_mut().insert(CACHE_CONTROL, value);
    }
    response
}

async fn handle_local_beeper_send(Json(body): Json<LocalBeeperSendRequest>) -> Response {
    let token = match beeper_token_from(&body.token) {
        Ok(value) => value,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let chat_id = match body.chat_id.as_deref().map(str::trim) {
        Some(value) if !value.is_empty() => value.to_string(),
        _ => return local_json_error(AxumStatusCode::BAD_REQUEST, "chat_id is required"),
    };
    let text = match body.text.as_deref().map(str::trim) {
        Some(value) if !value.is_empty() => value.to_string(),
        _ => return local_json_error(AxumStatusCode::BAD_REQUEST, "text is required"),
    };
    let payload = match beeper_api_send_chat_message(&token, &chat_id, &text).await {
        Ok(value) => value,
        Err(err) => return local_json_error(AxumStatusCode::BAD_GATEWAY, &err.to_string()),
    };
    local_json_value(
        AxumStatusCode::OK,
        sonic_rs::json!({
            "ok": true,
            "item": payload,
        }),
    )
}

async fn handle_local_tailscale_devices(
    Json(body): Json<LocalTailscaleDevicesRequest>,
) -> Response {
    let api_key = match tailscale_api_key_from(body.api_key.as_deref().unwrap_or_default()) {
        Ok(value) => value,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let tailnet = match tailscale_tailnet_from(body.tailnet.as_deref().unwrap_or_default()) {
        Ok(value) => value,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let payload = match tailscale_api_get_devices(&api_key, &tailnet).await {
        Ok(value) => value,
        Err(err) => return local_json_error(AxumStatusCode::BAD_GATEWAY, &err.to_string()),
    };
    local_json_value(
        AxumStatusCode::OK,
        sonic_rs::json!({
            "ok": true,
            "devices": payload["devices"].as_array().cloned().unwrap_or_default(),
        }),
    )
}

fn parse_onvif_name_hint(payload: &str) -> Option<String> {
    let marker = "onvif://www.onvif.org/name/";
    let payload_lower = payload.to_ascii_lowercase();
    let marker_index = payload_lower.find(marker)?;
    let source = &payload[marker_index + marker.len()..];
    let mut value = String::new();
    for ch in source.chars() {
        if ch.is_whitespace() || ch == '<' || ch == '>' || ch == '"' || ch == '\'' {
            break;
        }
        value.push(ch);
    }
    let decoded = value.replace("%20", " ").replace('+', " ").trim().to_string();
    if decoded.is_empty() {
        return None;
    }
    Some(decoded.chars().take(120).collect())
}

fn extract_http_urls(payload: &str) -> Vec<String> {
    payload
        .split(|ch: char| ch.is_whitespace() || ch == '<' || ch == '>' || ch == '"' || ch == '\'')
        .filter_map(|token| {
            let candidate = token.trim_matches(|ch: char| {
                matches!(ch, ',' | ';' | ')' | '(' | '[' | ']' | '{' | '}' | '\\')
            });
            if candidate.starts_with("http://") || candidate.starts_with("https://") {
                Some(candidate.to_string())
            } else {
                None
            }
        })
        .collect()
}

fn normalize_onvif_candidate_url(raw: &str) -> Option<String> {
    let value = raw.trim();
    if value.is_empty() {
        return None;
    }
    let candidate = if value.starts_with("http://") || value.starts_with("https://") {
        value.to_string()
    } else {
        format!("http://{value}")
    };
    let parsed = reqwest::Url::parse(&candidate).ok()?;
    Some(parsed.to_string())
}

fn onvif_host_from_url(url: &str) -> Option<String> {
    let parsed = reqwest::Url::parse(url).ok()?;
    parsed.host_str().map(|value| value.to_string())
}

fn is_likely_onvif_service_url(url: &str) -> bool {
    let parsed = match reqwest::Url::parse(url) {
        Ok(value) => value,
        Err(_) => return false,
    };
    let host = parsed.host_str().unwrap_or_default().to_ascii_lowercase();
    if host.is_empty() || host == "www.onvif.org" || host.ends_with(".onvif.org") {
        return false;
    }
    let path = parsed.path().to_ascii_lowercase();
    path.contains("device_service") || path.contains("/onvif/")
}

async fn discover_onvif_ws(wait_ms: u64) -> Result<Vec<sonic_rs::Value>> {
    let socket = tokio::net::UdpSocket::bind("0.0.0.0:0")
        .await
        .context("bind onvif discovery socket")?;
    let message_id = format!("uuid:edgerun-{}-{}", now_unix_ms(), std::process::id());
    let probe = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
<s:Envelope xmlns:s=\"http://www.w3.org/2003/05/soap-envelope\" xmlns:a=\"http://schemas.xmlsoap.org/ws/2004/08/addressing\" xmlns:d=\"http://schemas.xmlsoap.org/ws/2005/04/discovery\" xmlns:dn=\"http://www.onvif.org/ver10/network/wsdl\">\
  <s:Header>\
    <a:Action>http://schemas.xmlsoap.org/ws/2005/04/discovery/Probe</a:Action>\
    <a:MessageID>{}</a:MessageID>\
    <a:To>urn:schemas-xmlsoap-org:ws:2005:04:discovery</a:To>\
  </s:Header>\
  <s:Body>\
    <d:Probe>\
      <d:Types>dn:NetworkVideoTransmitter</d:Types>\
    </d:Probe>\
  </s:Body>\
</s:Envelope>",
        message_id
    );
    socket
        .send_to(probe.as_bytes(), "239.255.255.250:3702")
        .await
        .context("send onvif discovery probe")?;

    let wait_duration = Duration::from_millis(wait_ms.clamp(250, 5000));
    let start = tokio::time::Instant::now();
    let mut seen = std::collections::HashSet::new();
    let mut items = Vec::new();
    let mut buffer = vec![0u8; 8192];

    while start.elapsed() < wait_duration {
        let remaining = wait_duration.saturating_sub(start.elapsed());
        let received = tokio::time::timeout(remaining, socket.recv_from(&mut buffer)).await;
        let (size, source_addr) = match received {
            Ok(Ok(value)) => value,
            Ok(Err(_)) => continue,
            Err(_) => break,
        };

        let payload = String::from_utf8_lossy(&buffer[..size]);
        let name_hint = parse_onvif_name_hint(&payload);
        let mut urls: Vec<String> = extract_http_urls(&payload)
            .into_iter()
            .filter_map(|item| normalize_onvif_candidate_url(&item))
            .filter(|item| is_likely_onvif_service_url(item))
            .collect();

        if urls.is_empty() {
            urls.push(format!("http://{}/onvif/device_service", source_addr.ip()));
        }

        for url in urls {
            let key = url.to_ascii_lowercase();
            if !seen.insert(key) {
                continue;
            }
            let ip = onvif_host_from_url(&url).unwrap_or_else(|| source_addr.ip().to_string());
            let name = name_hint.clone().unwrap_or_else(|| ip.clone());
            items.push(sonic_rs::json!({
                "name": name,
                "ip": ip,
                "url": url,
                "source": "ws-discovery"
            }));
        }
    }

    Ok(items)
}

async fn handle_local_onvif_discover(Query(query): Query<LocalOnvifDiscoverQuery>) -> Response {
    let wait_ms = query.wait_ms.unwrap_or(1400).clamp(250, 5000);
    let items = match discover_onvif_ws(wait_ms).await {
        Ok(value) => value,
        Err(err) => return local_json_error(AxumStatusCode::BAD_GATEWAY, &err.to_string()),
    };
    local_json_value(
        AxumStatusCode::OK,
        sonic_rs::json!({
            "ok": true,
            "waitMs": wait_ms,
            "items": items
        }),
    )
}

async fn handle_local_google_messages(Query(query): Query<LocalGoogleMessagesQuery>) -> Response {
    let token = match google_token_from(&query.token) {
        Ok(value) => value,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let max_results = query.max_results.or(query.limit).unwrap_or(50).clamp(1, 500);
    let mut url = format!(
        "https://gmail.googleapis.com/gmail/v1/users/me/messages?maxResults={max_results}&fields=messages(id,threadId),resultSizeEstimate"
    );
    if let Some(after) = query.after {
        url.push_str(&format!("&q=after:{after}"));
    }
    let payload = match google_api_get_json(&token, &url).await {
        Ok(value) => value,
        Err(err) => {
            let detail = err.to_string();
            if detail.to_ascii_lowercase().contains("insufficient authentication scopes") {
                return local_json_error(
                    AxumStatusCode::FORBIDDEN,
                    "Google Photos scope missing. Reconnect Google with Photos consent (prompt=consent) and retry.",
                );
            }
            return local_json_error(AxumStatusCode::BAD_GATEWAY, &detail);
        }
    };
    local_json_value(
        AxumStatusCode::OK,
        sonic_rs::json!({
            "ok": true,
            "messages": payload["messages"].as_array().cloned().unwrap_or_default(),
            "resultSizeEstimate": payload["resultSizeEstimate"].as_u64().unwrap_or_default(),
        }),
    )
}

async fn handle_local_google_message(
    AxumPath(id): AxumPath<String>,
    Query(query): Query<LocalGoogleMessageQuery>,
) -> Response {
    let token = match google_token_from(&query.token) {
        Ok(value) => value,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let message_id = id.trim();
    if message_id.is_empty() {
        return local_json_error(AxumStatusCode::BAD_REQUEST, "message id is required");
    }
    let url = format!(
        "https://gmail.googleapis.com/gmail/v1/users/me/messages/{}?format=full",
        message_id
    );
    let payload = match google_api_get_json(&token, &url).await {
        Ok(value) => value,
        Err(err) => {
            let detail = err.to_string();
            if detail
                .to_ascii_lowercase()
                .contains("insufficient authentication scopes")
            {
                return local_json_error(
                    AxumStatusCode::FORBIDDEN,
                    "Google Photos scope missing. Reconnect Google with Photos consent and retry.",
                );
            }
            return local_json_error(AxumStatusCode::BAD_GATEWAY, &detail);
        }
    };
    let gmail_payload = &payload["payload"];
    let (body, html) = extract_gmail_text(gmail_payload);
    local_json_value(
        AxumStatusCode::OK,
        sonic_rs::json!({
            "ok": true,
            "id": payload["id"].as_str().unwrap_or_default(),
            "threadId": payload["threadId"].as_str().unwrap_or_default(),
            "labelIds": payload["labelIds"].as_array().cloned().unwrap_or_default(),
            "snippet": payload["snippet"].as_str().unwrap_or_default(),
            "subject": gmail_header(gmail_payload, "Subject"),
            "from": gmail_header(gmail_payload, "From"),
            "to": gmail_header(gmail_payload, "To"),
            "date": gmail_header(gmail_payload, "Date"),
            "body": body,
            "html": html,
        }),
    )
}

async fn handle_local_google_events(Query(query): Query<LocalGoogleEventsQuery>) -> Response {
    let token = match google_token_from(&query.token) {
        Ok(value) => value,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let limit = query.limit.unwrap_or(10).clamp(1, 100);
    let url = format!(
        "https://www.googleapis.com/calendar/v3/calendars/primary/events?singleEvents=true&orderBy=startTime&maxResults={limit}"
    );
    let payload = match google_api_get_json(&token, &url).await {
        Ok(value) => value,
        Err(err) => return local_json_error(AxumStatusCode::BAD_GATEWAY, &err.to_string()),
    };
    local_json_value(
        AxumStatusCode::OK,
        sonic_rs::json!({
            "ok": true,
            "items": payload["items"].as_array().cloned().unwrap_or_default(),
        }),
    )
}

async fn handle_local_google_contacts(Query(query): Query<LocalGoogleContactsQuery>) -> Response {
    let token = match google_token_from(&query.token) {
        Ok(value) => value,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let limit = query.limit.unwrap_or(100).clamp(1, 500);
    let url = format!(
        "https://people.googleapis.com/v1/people/me/connections?pageSize={limit}&personFields=names,emailAddresses,photos"
    );
    let payload = match google_api_get_json(&token, &url).await {
        Ok(value) => value,
        Err(err) => return local_json_error(AxumStatusCode::BAD_GATEWAY, &err.to_string()),
    };
    let items = payload["connections"].as_array().cloned().unwrap_or_default();
    local_json_value(AxumStatusCode::OK, sonic_rs::json!({ "ok": true, "items": items }))
}

async fn handle_local_google_drive_files(
    Query(query): Query<LocalGoogleDriveFilesQuery>,
) -> Response {
    let token = match google_token_from(&query.token) {
        Ok(value) => value,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let parent_id = query.parent_id.as_deref().unwrap_or("root").trim();
    let page_size = query.page_size.unwrap_or(200).clamp(1, 1000);
    let parent_filter = format!("'{}'+in+parents+and+trashed+=+false", parent_id.replace('"', ""));
    let url = format!(
        "https://www.googleapis.com/drive/v3/files?pageSize={page_size}&q={}&fields=files(id,name,mimeType,size,modifiedTime)",
        parent_filter
    );
    let payload = match google_api_get_json(&token, &url).await {
        Ok(value) => value,
        Err(err) => return local_json_error(AxumStatusCode::BAD_GATEWAY, &err.to_string()),
    };
    local_json_value(
        AxumStatusCode::OK,
        sonic_rs::json!({
            "ok": true,
            "files": payload["files"].as_array().cloned().unwrap_or_default(),
        }),
    )
}

async fn handle_local_google_drive_file(
    AxumPath(id): AxumPath<String>,
    Query(query): Query<LocalGoogleDriveFileQuery>,
) -> Response {
    let token = match google_token_from(&query.token) {
        Ok(value) => value,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let file_id = id.trim();
    if file_id.is_empty() {
        return local_json_error(AxumStatusCode::BAD_REQUEST, "file id is required");
    }
    let metadata_url = format!(
        "https://www.googleapis.com/drive/v3/files/{}?fields=id,name,mimeType,size,modifiedTime",
        file_id
    );
    let file = match google_api_get_json(&token, &metadata_url).await {
        Ok(value) => value,
        Err(err) => return local_json_error(AxumStatusCode::BAD_GATEWAY, &err.to_string()),
    };
    let mime_type = file["mimeType"].as_str().unwrap_or_default().to_string();
    let mut content = String::new();
    if !mime_type.eq_ignore_ascii_case("application/vnd.google-apps.folder") {
        let content_url = if mime_type.eq_ignore_ascii_case("application/vnd.google-apps.document") {
            format!(
                "https://www.googleapis.com/drive/v3/files/{}/export?mimeType=text/plain",
                file_id
            )
        } else {
            format!(
                "https://www.googleapis.com/drive/v3/files/{}?alt=media",
                file_id
            )
        };
        let bytes = match google_api_get_bytes(&token, &content_url).await {
            Ok(value) => value,
            Err(err) => return local_json_error(AxumStatusCode::BAD_GATEWAY, &err.to_string()),
        };
        content = String::from_utf8(bytes).unwrap_or_default();
    }
    local_json_value(
        AxumStatusCode::OK,
        sonic_rs::json!({
            "ok": true,
            "file": file,
            "content": content,
        }),
    )
}

async fn handle_local_google_photos(Query(query): Query<LocalGooglePhotosQuery>) -> Response {
    let token = match google_token_from(&query.token) {
        Ok(value) => value,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let page_size = query.page_size.unwrap_or(50).clamp(1, 100);
    let url = format!("https://photoslibrary.googleapis.com/v1/mediaItems?pageSize={page_size}");
    let payload = match google_api_get_json(&token, &url).await {
        Ok(value) => value,
        Err(err) => return local_json_error(AxumStatusCode::BAD_GATEWAY, &err.to_string()),
    };
    let mut items = payload["mediaItems"].as_array().cloned().unwrap_or_default();
    if items.is_empty() {
        let client = Client::new();
        let search_url = "https://photoslibrary.googleapis.com/v1/mediaItems:search";
        let search_response = client
            .post(search_url)
            .bearer_auth(&token)
            .header("Accept", "application/json")
            .header(CONTENT_TYPE, "application/json")
            .body(
                sonic_rs::to_vec(&sonic_rs::json!({
                    "pageSize": page_size,
                }))
                .unwrap_or_default(),
            )
            .send()
            .await;
        if let Ok(response) = search_response {
            if response.status().is_success() {
                if let Ok(bytes) = response.bytes().await {
                    if let Ok(search_payload) = sonic_rs::from_slice::<sonic_rs::Value>(&bytes) {
                        items = search_payload["mediaItems"].as_array().cloned().unwrap_or_default();
                    }
                }
            }
        }
    }
    local_json_value(
        AxumStatusCode::OK,
        sonic_rs::json!({
            "ok": true,
            "items": items,
            "hint": if items.is_empty() {
                "No items returned by Google Photos API. This OAuth client may only access app-created media unless additional API access is approved."
            } else {
                ""
            },
        }),
    )
}

async fn handle_local_google_refresh(Json(body): Json<LocalGoogleRefreshRequest>) -> Response {
    let refresh_token = body.refresh_token.trim();
    if refresh_token.len() < 12 {
        return local_json_error(
            AxumStatusCode::BAD_REQUEST,
            "google refresh_token is missing or invalid",
        );
    }
    let client_id = google_oauth_client_id();
    let client_secret = google_oauth_client_secret();
    let (Some(client_id), Some(client_secret)) = (client_id, client_secret) else {
        return local_json_error(
            AxumStatusCode::NOT_IMPLEMENTED,
            &format!(
                "google refresh is not configured (set {} and {} or store in hwvault as google/oauth/client_id and google/oauth/client_secret)",
                GOOGLE_OAUTH_CLIENT_ID_ENV, GOOGLE_OAUTH_CLIENT_SECRET_ENV
            ),
        );
    };
    let client = Client::new();
    let response = match client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await
    {
        Ok(value) => value,
        Err(err) => return local_json_error(AxumStatusCode::BAD_GATEWAY, &err.to_string()),
    };
    let status = response.status();
    let bytes = match response.bytes().await {
        Ok(value) => value,
        Err(err) => return local_json_error(AxumStatusCode::BAD_GATEWAY, &err.to_string()),
    };
    if !status.is_success() {
        return local_json_error(
            AxumStatusCode::BAD_GATEWAY,
            &format!(
                "google token refresh failed ({}): {}",
                status,
                String::from_utf8_lossy(&bytes)
            ),
        );
    }
    let payload: sonic_rs::Value = match sonic_rs::from_slice(&bytes) {
        Ok(value) => value,
        Err(err) => return local_json_error(AxumStatusCode::BAD_GATEWAY, &err.to_string()),
    };
    let access_token = payload["access_token"].as_str().unwrap_or_default();
    if access_token.is_empty() {
        return local_json_error(
            AxumStatusCode::BAD_GATEWAY,
            "google token refresh response is missing access_token",
        );
    }
    let expires_in = payload["expires_in"].as_i64().unwrap_or(3600).max(60) as u64;
    let expires_at = now_unix_ms() + (expires_in * 1000);
    local_json_value(
        AxumStatusCode::OK,
        sonic_rs::json!({
            "ok": true,
            "access_token": access_token,
            "expires_at": expires_at,
        }),
    )
}

async fn handle_local_google_oauth_start(Query(query): Query<LocalGoogleOauthStartQuery>) -> Response {
    let client_id = google_oauth_client_id();
    let Some(client_id) = client_id else {
        return google_oauth_redirect_with_result(
            &sanitize_return_to(query.return_to.as_deref()),
            false,
            &format!(
                "google oauth is not configured (set {} or store google/oauth/client_id in hwvault)",
                GOOGLE_OAUTH_CLIENT_ID_ENV
            ),
            "",
            "",
        );
    };

    let return_to = sanitize_return_to(query.return_to.as_deref());
    let scopes = google_oauth_scopes_for(query.integration_id.as_deref());
    let state = encode_google_oauth_state(&return_to);
    let redirect_uri = google_oauth_redirect_uri();
    let mut url = match reqwest::Url::parse("https://accounts.google.com/o/oauth2/v2/auth") {
        Ok(value) => value,
        Err(err) => {
            return google_oauth_redirect_with_result(
                &return_to,
                false,
                &format!("failed to build google oauth url: {err}"),
                "",
                "",
            )
        }
    };
    url.query_pairs_mut()
        .append_pair("client_id", &client_id)
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", &scopes)
        .append_pair("access_type", "offline")
        .append_pair("include_granted_scopes", "true")
        .append_pair("prompt", "consent")
        .append_pair("state", &state);

    let mut response = AxumStatusCode::FOUND.into_response();
    if let Ok(value) = HeaderValue::from_str(url.as_str()) {
        response.headers_mut().insert(LOCATION, value);
    }
    response
}

async fn handle_local_google_oauth_callback(Query(query): Query<LocalGoogleOauthCallbackQuery>) -> Response {
    let return_to = decode_google_oauth_state(query.state.as_deref());

    if let Some(error) = query.error.as_deref().map(str::trim).filter(|value| !value.is_empty()) {
        return google_oauth_redirect_with_result(&return_to, false, error, "", "");
    }

    let code = query.code.as_deref().map(str::trim).unwrap_or_default();
    if code.is_empty() {
        return google_oauth_redirect_with_result(&return_to, false, "google oauth callback missing code", "", "");
    }

    let client_id = google_oauth_client_id();
    let client_secret = google_oauth_client_secret();
    let (Some(client_id), Some(client_secret)) = (client_id, client_secret) else {
        return google_oauth_redirect_with_result(
            &return_to,
            false,
            &format!(
                "google oauth callback is not configured (set {} and {} or store in hwvault as google/oauth/client_id and google/oauth/client_secret)",
                GOOGLE_OAUTH_CLIENT_ID_ENV,
                GOOGLE_OAUTH_CLIENT_SECRET_ENV
            ),
            "",
            "",
        );
    };

    let redirect_uri = google_oauth_redirect_uri();
    let client = Client::new();
    let response = match client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("code", code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect_uri.as_str()),
        ])
        .send()
        .await
    {
        Ok(value) => value,
        Err(err) => {
            return google_oauth_redirect_with_result(&return_to, false, &format!("google token exchange failed: {err}"), "", "")
        }
    };
    let status = response.status();
    let bytes = match response.bytes().await {
        Ok(value) => value,
        Err(err) => {
            return google_oauth_redirect_with_result(&return_to, false, &format!("failed to read token response: {err}"), "", "")
        }
    };
    if !status.is_success() {
        return google_oauth_redirect_with_result(
            &return_to,
            false,
            &format!("google token exchange failed ({}): {}", status, String::from_utf8_lossy(&bytes)),
            "",
            "",
        );
    }
    let payload: sonic_rs::Value = match sonic_rs::from_slice(&bytes) {
        Ok(value) => value,
        Err(err) => {
            return google_oauth_redirect_with_result(&return_to, false, &format!("failed to parse token response: {err}"), "", "")
        }
    };
    let access_token = payload["access_token"].as_str().unwrap_or_default();
    let refresh_token = payload["refresh_token"].as_str().unwrap_or_default();
    if access_token.is_empty() {
        return google_oauth_redirect_with_result(&return_to, false, "google token response missing access_token", "", "");
    }
    google_oauth_redirect_with_result(&return_to, true, "oauth complete", access_token, refresh_token)
}

async fn handle_local_assistant(Json(body): Json<LocalAssistantRequest>) -> Response {
    let prompt = body.message.trim().to_string();
    if prompt.is_empty() {
        return local_json_error(AxumStatusCode::BAD_REQUEST, "assistant message is required");
    }
    let provider = body
        .provider
        .as_deref()
        .unwrap_or("opencode")
        .trim()
        .to_ascii_lowercase();
    if provider != "opencode" {
        return local_json_error(AxumStatusCode::BAD_REQUEST, "unsupported provider");
    }
    if let Err(err) = require_opencode_cli_container() {
        return local_json_error(AxumStatusCode::SERVICE_UNAVAILABLE, &err.to_string());
    }

    let requested_session_id = body
        .session_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let requested_thread_id = body
        .thread_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let session_id = requested_thread_id
        .clone()
        .or_else(|| requested_session_id.clone());

    let mut exec_args = vec![
        "30s".to_string(),
        "docker".to_string(),
        "exec".to_string(),
        OPENCODE_CLI_CONTAINER_NAME.to_string(),
        "opencode".to_string(),
        "run".to_string(),
        "--format".to_string(),
        "json".to_string(),
    ];
    if let Some(resume) = session_id.as_deref() {
        exec_args.push("--session".to_string());
        exec_args.push(resume.to_string());
    }
    exec_args.push(prompt.clone());
    let exec_output = Command::new("timeout").args(exec_args).output();

    let exec_output = match exec_output {
        Ok(value) => value,
        Err(err) => {
            return local_json_error(AxumStatusCode::INTERNAL_SERVER_ERROR, &err.to_string())
        }
    };
    if !exec_output.status.success() {
        return local_json_error(
            AxumStatusCode::INTERNAL_SERVER_ERROR,
            &format!(
                "opencode run failed: {}",
                String::from_utf8_lossy(&exec_output.stderr).trim()
            ),
        );
    }

    let stdout = String::from_utf8_lossy(&exec_output.stdout).to_string();
    let discovered_session_id = parse_opencode_session_id(&stdout);
    let resolved_thread_id = discovered_session_id
        .clone()
        .or_else(|| session_id.clone())
        .unwrap_or_default();
    let resolved_session_id = if resolved_thread_id.is_empty() {
        requested_session_id.unwrap_or_default()
    } else {
        resolved_thread_id.clone()
    };
    let message = parse_opencode_assistant_text(&stdout);
    let response = LocalAssistantResponse {
        ok: true,
        error: String::new(),
        message: if message.is_empty() {
            "OpenCode returned no output.".to_string()
        } else {
            message
        },
        actions: Vec::new(),
        status_events: vec![LocalAssistantStatusEvent {
            event_type: "phase".to_string(),
            label: "done".to_string(),
            detail: "Response ready.".to_string(),
        }],
        session_id: resolved_session_id,
        thread_id: resolved_thread_id,
    };
    with_local_cors_headers(
        (
            AxumStatusCode::OK,
            [(
                AXUM_CONTENT_TYPE,
                HeaderValue::from_static("application/json; charset=utf-8"),
            )],
            sonic_rs::to_string(&response)
                .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode failed\"}".to_string()),
        )
            .into_response(),
    )
}

fn parse_opencode_session_id(output: &str) -> Option<String> {
    for line in output.lines() {
        let raw = line.trim();
        if raw.is_empty() {
            continue;
        }
        let event: OpenCodeRunEvent = match sonic_rs::from_str(raw) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let session_id = event
            .session_id
            .or(event.session_id_alt)
            .or_else(|| event.part.as_ref().and_then(|part| part.session_id.clone()))
            .or_else(|| event.part.as_ref().and_then(|part| part.session_id_alt.clone()))
            .unwrap_or_default();
        let normalized = session_id.trim();
        if !normalized.is_empty() {
            return Some(normalized.to_string());
        }
    }
    None
}

fn parse_opencode_assistant_text(output: &str) -> String {
    let mut chunks: Vec<String> = Vec::new();
    for line in output.lines() {
        let raw = line.trim();
        if raw.is_empty() {
            continue;
        }
        let event: OpenCodeRunEvent = match sonic_rs::from_str(raw) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if event.event_type != "text" {
            continue;
        }
        let Some(part) = event.part else {
            continue;
        };
        if part.event_type != "text" {
            continue;
        }
        let text = part.text.unwrap_or_default();
        let normalized = text.trim();
        if !normalized.is_empty() {
            chunks.push(normalized.to_string());
        }
    }
    chunks.join("\n")
}

async fn local_eventbus_session(socket: WebSocket, state: Arc<LocalBridgeState>) {
    let mut rx = state.tx.subscribe();
    let socket = Arc::new(Mutex::new(socket));
    let sender_socket = Arc::clone(&socket);
    let outbound = tokio::spawn(async move {
        loop {
            let event = match rx.recv().await {
                Ok(event) => event,
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            };
            let bytes = event.encode_to_vec();
            let mut ws = sender_socket.lock().await;
            if ws.send(WsMessage::Binary(bytes.into())).await.is_err() {
                break;
            }
        }
    });

    let connected = local_bridge_envelope("local.bridge.connected", "node-manager");
    {
        let mut ws = socket.lock().await;
        let _ = ws
            .send(WsMessage::Binary(connected.encode_to_vec().into()))
            .await;
    }

    let mut inbound_open = true;
    while inbound_open {
        let next = {
            let mut ws = socket.lock().await;
            ws.next().await
        };
        let Some(Ok(message)) = next else {
            break;
        };
        match message {
            WsMessage::Binary(bytes) => {
                if let Ok(envelope) = LocalEventEnvelopeV1::decode(bytes.as_ref()) {
                    let _ = state.tx.send(envelope);
                }
            }
            WsMessage::Text(text) => {
                if text.trim() == "ping" {
                    let pong = local_bridge_envelope("local.bridge.pong", "node-manager");
                    let mut ws = socket.lock().await;
                    let _ = ws
                        .send(WsMessage::Binary(pong.encode_to_vec().into()))
                        .await;
                }
            }
            WsMessage::Close(_) => inbound_open = false,
            WsMessage::Ping(payload) => {
                let mut ws = socket.lock().await;
                if ws.send(WsMessage::Pong(payload)).await.is_err() {
                    break;
                }
            }
            WsMessage::Pong(_) => {}
        }
    }
    outbound.abort();
}

fn spawn_local_docker_events(state: Arc<LocalBridgeState>) {
    tokio::spawn(async move {
        loop {
            if let Err(err) = run_local_docker_event_stream(Arc::clone(&state)).await {
                eprintln!("local_docker_event_stream_error={err}");
            }
            sleep(Duration::from_secs(2)).await;
        }
    });
}

async fn run_local_docker_event_stream(state: Arc<LocalBridgeState>) -> Result<()> {
    let mut child = TokioCommand::new("docker")
        .args([
            "events",
            "--format",
            "{{.TimeNano}}\t{{.Type}}\t{{.Action}}\t{{.Actor.ID}}\t{{.Actor.Attributes.name}}",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| "failed to start docker events stream".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("docker events stream missing stdout"))?;
    let mut lines = BufReader::new(stdout).lines();
    while let Some(line) = lines
        .next_line()
        .await
        .with_context(|| "failed to read docker event line".to_string())?
    {
        let cols: Vec<&str> = line.split('\t').collect();
        let time_nano = cols.first().copied().unwrap_or_default().trim();
        let event_type = cols.get(1).copied().unwrap_or_default().trim().to_string();
        let action = cols.get(2).copied().unwrap_or_default().trim().to_string();
        let container_id = cols.get(3).copied().unwrap_or_default().trim().to_string();
        let container_name = cols.get(4).copied().unwrap_or_default().trim().to_string();
        let ts_unix_ms = time_nano
            .parse::<u64>()
            .map(|value| value / 1_000_000)
            .unwrap_or_else(|_| now_unix_ms());
        let message = format!(
            "{} {} {}",
            if event_type.is_empty() {
                "docker"
            } else {
                &event_type
            },
            if action.is_empty() { "event" } else { &action },
            if container_name.is_empty() {
                if container_id.is_empty() {
                    "unknown".to_string()
                } else {
                    container_id.clone()
                }
            } else {
                container_name.clone()
            }
        );
        let payload = LocalDockerEventPayload {
            event_type,
            action,
            container_id,
            container_name,
            message,
            ts_unix_ms,
        };
        let envelope_payload = LocalBridgeEventPayloadEnvelope {
            payload: sonic_rs::to_value(&payload)
                .with_context(|| "failed to convert docker event payload value".to_string())?,
            meta: sonic_rs::json!({ "source": "node-manager", "kind": "docker.event" }),
        };
        let mut envelope = local_bridge_envelope(LOCAL_DOCKER_EVENTS_TOPIC, "node-manager");
        envelope.payload = sonic_rs::to_vec(&envelope_payload)
            .with_context(|| "failed to encode local docker event payload".to_string())?;
        let _ = state.tx.send(envelope);
    }
    let status = child
        .wait()
        .await
        .with_context(|| "failed to wait docker event stream process".to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("docker events stream exited with status {status}"))
    }
}

fn run_command_capture(bin: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(bin)
        .args(args)
        .output()
        .with_context(|| format!("failed to run command: {} {}", bin, args.join(" ")))?;
    if !output.status.success() {
        return Err(anyhow!(
            "command failed: {} {} ({})",
            bin,
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

async fn cmd_run(local_bridge_listen: &str) -> Result<()> {
    let signer = load_tpm_signer()?;
    let pid1 = std::process::id() == 1;
    let boot_policy = if pid1 {
        Some(enforce_boot_policy()?)
    } else {
        None
    };

    let mut cfg = load_config().unwrap_or_else(|_| default_manager_config());

    if cfg.heartbeat_secs == 0 {
        cfg.heartbeat_secs = 15;
    }
    cfg.rpc_url = DEFAULT_RPC_URL.to_string();
    if cfg.worker_max_concurrency == 0 {
        cfg.worker_max_concurrency = DEFAULT_WORKER_MAX_CONCURRENCY;
    }
    if cfg.worker_mem_bytes == 0 {
        cfg.worker_mem_bytes = DEFAULT_WORKER_MEM_BYTES;
    }

    let client = Client::new();
    let cloud_enabled = owner_context_available(&cfg, boot_policy.as_ref());
    if cloud_enabled {
        bootstrap_api_state(&client, &signer, &mut cfg, boot_policy.as_ref()).await?;
        ensure_runtime_image_ready(&client, &mut cfg, &signer).await?;
    } else {
        println!("mode=local-unbonded");
        println!("status=cloud-bootstrap-skipped");
    }
    ensure_worker_running(&cfg, &signer.public_key_b64url)?;
    save_config(&cfg)?;
    start_local_bridge(local_bridge_listen, &signer.public_key_b64url)?;

    println!("manager=starting");
    println!("backend=tpm");
    println!("device_pubkey_b64url={}", signer.public_key_b64url);
    println!("api_base={}", cfg.api_base);
    println!("rpc_url={}", cfg.rpc_url);
    println!("worker_max_concurrency={}", cfg.worker_max_concurrency);
    println!("worker_mem_bytes={}", cfg.worker_mem_bytes);
    println!("local_bridge_listen={local_bridge_listen}");
    println!("local_bridge_eventbus_path={LOCAL_BRIDGE_EVENTBUS_PATH}");
    println!("cloud_enabled={cloud_enabled}");
    if let Some(image_ref) = cfg.runtime_image_ref.as_deref() {
        println!("runtime_image_ref={image_ref}");
    }

    loop {
        if cloud_enabled && !cfg.runtime_image_pulled {
            match ensure_runtime_image_ready(&client, &mut cfg, &signer).await {
                Ok(()) => {
                    let _ = save_config(&cfg);
                }
                Err(err) => {
                    eprintln!("runtime_image_error={err}");
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
            }
        }
        if let Err(err) = ensure_worker_running(&cfg, &signer.public_key_b64url) {
            eprintln!("worker_error={err}");
        }
        if cloud_enabled {
            if let Err(err) = send_heartbeat(&client, &cfg, &signer.public_key_b64url).await {
                eprintln!("heartbeat_error={err}");
            }
        }
        sleep(Duration::from_secs(cfg.heartbeat_secs)).await;
    }
}

fn enforce_boot_policy() -> Result<BootPolicy> {
    if setup_mode_enabled()? {
        auto_import_secure_boot_keys()?;
        return Err(anyhow!(
            "UEFI SetupMode detected; EdgeRun Secure Boot keys were enrolled, reboot required"
        ));
    }

    if !secure_boot_enabled()? {
        return Err(anyhow!("secure boot is disabled; refusing PID 1 startup"));
    }

    let cmdline = fs::read_to_string("/proc/cmdline").context("failed to read /proc/cmdline")?;
    if !cmdline
        .split_ascii_whitespace()
        .any(|p| p == REQUIRED_CMDLINE_LOCK_TOKEN)
    {
        return Err(anyhow!(
            "missing required cmdline token `{REQUIRED_CMDLINE_LOCK_TOKEN}`"
        ));
    }

    for forbidden in ["init=", "rdinit=", "systemd.unit=", "edgerun.insecure="] {
        if cmdline
            .split_ascii_whitespace()
            .any(|p| p.starts_with(forbidden))
        {
            return Err(anyhow!(
                "forbidden cmdline token prefix detected: {forbidden}"
            ));
        }
    }

    let api = cmdline_arg(&cmdline, "api_base").unwrap_or_else(|| DEFAULT_API_BASE.to_string());
    if api.trim_end_matches('/') != DEFAULT_API_BASE {
        return Err(anyhow!(
            "api_base must be locked to {DEFAULT_API_BASE}, got {api}"
        ));
    }

    Ok(BootPolicy {
        owner_pubkey: cmdline_arg(&cmdline, "owner_pubkey"),
    })
}

fn secure_boot_enabled() -> Result<bool> {
    read_efi_bool_var("SecureBoot")
        .and_then(|v| v.ok_or_else(|| anyhow!("SecureBoot efivar not found")))
}

fn setup_mode_enabled() -> Result<bool> {
    Ok(read_efi_bool_var("SetupMode")?.unwrap_or(false))
}

fn read_efi_bool_var(var_prefix: &str) -> Result<Option<bool>> {
    let efivars = Path::new("/sys/firmware/efi/efivars");
    if !efivars.exists() {
        return Err(anyhow!("efivars path missing: {}", efivars.display()));
    }

    for entry in fs::read_dir(efivars).context("failed to list efivars")? {
        let entry = entry.context("failed to read efivars entry")?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with(&format!("{var_prefix}-")) {
            continue;
        }
        let data = fs::read(entry.path())
            .with_context(|| format!("failed to read {var_prefix} efivar"))?;
        if data.len() < 5 {
            return Err(anyhow!("invalid {var_prefix} efivar payload"));
        }
        return Ok(Some(data[4] == 1));
    }
    Ok(None)
}

fn auto_import_secure_boot_keys() -> Result<()> {
    let cert_path = if Path::new(EDGE_SECUREBOOT_CERT_DER_PATH).exists() {
        EDGE_SECUREBOOT_CERT_DER_PATH
    } else {
        EDGE_SECUREBOOT_CERT_PEM_PATH
    };
    if !Path::new(EFI_UPDATEVAR_BIN).exists() {
        return Err(anyhow!(
            "SetupMode detected but efi-updatevar is unavailable at {EFI_UPDATEVAR_BIN}"
        ));
    }
    if !Path::new(cert_path).exists() {
        return Err(anyhow!(
            "SetupMode detected but EdgeRun cert is missing at {cert_path}"
        ));
    }

    for var in ["db", "KEK", "PK"] {
        let esl_path = format!("/etc/edgerun/secureboot/{var}.esl");
        let mut command = Command::new(EFI_UPDATEVAR_BIN);
        if Path::new(&esl_path).exists() {
            command.arg("-f").arg(&esl_path);
        } else {
            command.arg("-e").arg("-c").arg(cert_path);
        }
        let output = command
            .arg(var)
            .output()
            .with_context(|| format!("failed to execute efi-updatevar for {var}"))?;
        if !output.status.success() {
            return Err(anyhow!(
                "efi-updatevar failed for {var} with status {} stderr: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
    }
    Ok(())
}

fn cmdline_arg(cmdline: &str, key: &str) -> Option<String> {
    cmdline.split_ascii_whitespace().find_map(|part| {
        part.strip_prefix(&format!("{key}="))
            .map(ToString::to_string)
    })
}

fn owner_context_available(cfg: &ManagerConfig, policy: Option<&BootPolicy>) -> bool {
    cfg.owner_pubkey.is_some() || policy.and_then(|p| p.owner_pubkey.as_ref()).is_some()
}

async fn bootstrap_api_state(
    client: &Client,
    signer: &DeviceSigner,
    cfg: &mut ManagerConfig,
    policy: Option<&BootPolicy>,
) -> Result<()> {
    cfg.api_base = cfg.api_base.trim_end_matches('/').to_string();
    if cfg.api_base.is_empty() {
        cfg.api_base = DEFAULT_API_BASE.to_string();
    }

    if cfg.api_base != DEFAULT_API_BASE {
        return Err(anyhow!(
            "api_base must be {DEFAULT_API_BASE}, got {}",
            cfg.api_base
        ));
    }

    if !cfg.bonded {
        let owner_pubkey = cfg
            .owner_pubkey
            .clone()
            .or_else(|| policy.and_then(|p| p.owner_pubkey.clone()))
            .ok_or_else(|| anyhow!("owner_pubkey is required for first-boot bonding"))?;

        register_device(
            client,
            &cfg.api_base,
            &signer.public_key_b64url,
            &owner_pubkey,
        )
        .await?;
        call_ok_json(
            client,
            &format!("{}/v1/node/bond", cfg.api_base),
            &BondRequest {
                owner_pubkey: &owner_pubkey,
                device_pubkey_b64url: &signer.public_key_b64url,
            },
        )
        .await?;

        cfg.owner_pubkey = Some(owner_pubkey);
        cfg.bonded = true;
        println!("status=bonded");
        println!(
            "onboarding_url={}/register?device={}&owner={}",
            cfg.api_base,
            signer.public_key_b64url,
            cfg.owner_pubkey.as_deref().unwrap_or_default()
        );
    }

    let owner_pubkey = cfg
        .owner_pubkey
        .as_deref()
        .ok_or_else(|| anyhow!("owner_pubkey missing after bonding"))?;

    if !cfg.node_initialized {
        call_ok_json(
            client,
            &format!("{}/v1/node/init", cfg.api_base),
            &NodeInitRequest {
                owner_pubkey,
                device_pubkey_b64url: &signer.public_key_b64url,
                rpc_url: &cfg.rpc_url,
            },
        )
        .await?;
        cfg.node_initialized = true;
        println!("status=node-initialized");
    }

    Ok(())
}

fn read_worker_pid() -> Option<u32> {
    let raw = fs::read_to_string(WORKER_PID_FILE).ok()?;
    raw.trim().parse::<u32>().ok()
}

fn pid_alive(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
}

fn ensure_worker_running(cfg: &ManagerConfig, worker_pubkey: &str) -> Result<()> {
    if !Path::new(DEFAULT_WORKER_BIN).exists() {
        if Path::new(DEFAULT_CRUN_BIN).exists() {
            if !Path::new(WORKER_RUNTIME_MARKER_FILE).exists() {
                fs::write(WORKER_RUNTIME_MARKER_FILE, "crun-ready\n")
                    .context("failed to write worker runtime marker")?;
                println!("status=worker-runtime=crun-ready");
            }
            return Ok(());
        }
        return Err(anyhow!(
            "worker runtime missing: expected {} or {}",
            DEFAULT_WORKER_BIN,
            DEFAULT_CRUN_BIN
        ));
    }

    if let Some(pid) = read_worker_pid() {
        if pid_alive(pid) {
            return Ok(());
        }
    }

    fs::create_dir_all("/run/edgerun").context("failed to create /run/edgerun")?;
    let mut log_file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/run/edgerun/edgerun-worker.log")
        .context("failed to open worker log file")?;
    writeln!(log_file, "=== starting edgerun-worker ===").ok();
    let log_file_err = log_file
        .try_clone()
        .context("failed to clone worker log fd")?;

    let child = Command::new(DEFAULT_WORKER_BIN)
        .env("EDGERUN_WORKER_PUBKEY", worker_pubkey)
        .env("EDGERUN_SCHEDULER_URL", &cfg.api_base)
        .env(
            "EDGERUN_WORKER_MAX_CONCURRENT",
            cfg.worker_max_concurrency.to_string(),
        )
        .env("EDGERUN_WORKER_MEM_BYTES", cfg.worker_mem_bytes.to_string())
        .stdout(Stdio::from(log_file))
        .stderr(Stdio::from(log_file_err))
        .spawn()
        .context("failed to spawn edgerun-worker")?;

    let pid = child.id();
    fs::write(WORKER_PID_FILE, pid.to_string()).context("failed to write worker pid file")?;
    println!("status=worker-started pid={pid} pubkey={worker_pubkey}");
    Ok(())
}

async fn register_device(
    client: &Client,
    api_base: &str,
    device_pubkey_b64url: &str,
    owner_pubkey: &str,
) -> Result<()> {
    if owner_pubkey.trim().is_empty() {
        return Err(anyhow!("owner_pubkey is required"));
    }

    let challenge_url = format!("{api_base}/v1/device/challenge");
    let challenge_resp = client
        .post(&challenge_url)
        .send()
        .await
        .with_context(|| format!("challenge request failed: {challenge_url}"))?
        .error_for_status()
        .context("challenge endpoint returned error status")?;
    let challenge: DeviceChallengeResponse =
        parse_json_response(challenge_resp, "failed to decode challenge response").await?;

    let handshake_url = format!("{api_base}/v1/device/handshake");
    let handshake_resp = post_json(
        client,
        &handshake_url,
        &DeviceHandshakeRequest {
            owner_pubkey,
            nonce_b64url: &challenge.nonce_b64url,
        },
    )?
    .send()
    .await
    .with_context(|| format!("handshake request failed: {handshake_url}"))?
    .error_for_status()
    .context("handshake endpoint returned error status")?;
    let resp: ApiResponse =
        parse_json_response(handshake_resp, "failed to decode handshake response").await?;

    if !resp.ok {
        return Err(anyhow!(
            "device handshake rejected: {}",
            resp.error.unwrap_or_else(|| "unknown error".to_string())
        ));
    }

    println!("status=registered");
    println!("device_pubkey_b64url={device_pubkey_b64url}");
    println!("owner_pubkey={owner_pubkey}");
    Ok(())
}

async fn ensure_runtime_image_ready(
    client: &Client,
    cfg: &mut ManagerConfig,
    signer: &DeviceSigner,
) -> Result<()> {
    if cfg.runtime_image_pulled {
        return Ok(());
    }
    let image_ref = request_runtime_image_ref(client, cfg, signer).await?;
    pull_runtime_image(&image_ref)?;
    cfg.runtime_image_ref = Some(image_ref.clone());
    cfg.runtime_image_pulled = true;
    println!("status=runtime-image-ready image_ref={image_ref}");
    Ok(())
}

async fn request_runtime_image_ref(
    client: &Client,
    cfg: &mut ManagerConfig,
    signer: &DeviceSigner,
) -> Result<String> {
    let owner_pubkey = cfg
        .owner_pubkey
        .as_deref()
        .ok_or_else(|| anyhow!("owner_pubkey missing for runtime image request"))?
        .to_string();
    let request_issued_at_unix_s = now_unix_s()?;
    let request_nonce_b64url = random_token_b64url(24);
    let request_canonical = runtime_image_request_canonical_message(
        &owner_pubkey,
        &signer.public_key_b64url,
        &cfg.rpc_url,
        &request_nonce_b64url,
        request_issued_at_unix_s,
    );
    let request_signature_b64url = signer.sign_b64url(request_canonical.as_bytes());
    let payload = RuntimeImageRequest {
        owner_pubkey: &owner_pubkey,
        device_pubkey_b64url: &signer.public_key_b64url,
        rpc_url: &cfg.rpc_url,
        request_nonce_b64url: &request_nonce_b64url,
        request_issued_at_unix_s,
        request_signature_b64url: &request_signature_b64url,
    };
    let candidate_paths = ["/v1/node/runtime/image-tag", "/v1/node/image-tag"];
    for path in candidate_paths {
        let url = format!("{}{}", cfg.api_base, path);
        let resp = post_json(client, &url, &payload)?
            .send()
            .await
            .with_context(|| format!("runtime image request failed: {url}"))?;
        if !resp.status().is_success() {
            continue;
        }
        let data: RuntimeImageResponse =
            parse_json_response(resp, &format!("invalid runtime image response from {url}"))
                .await?;
        if !data.ok {
            if let Some(err) = data.error {
                eprintln!("runtime_image_api_error={err}");
            }
            continue;
        }
        if let Some(image_ref) = data.image_ref.as_deref().filter(|s| !s.trim().is_empty()) {
            let image_ref = image_ref.to_string();
            verify_runtime_image_policy_response(
                cfg,
                &owner_pubkey,
                &signer.public_key_b64url,
                &request_nonce_b64url,
                &data,
                &image_ref,
            )?;
            println!("runtime_image_ref_requested={image_ref}");
            return Ok(image_ref);
        }
        if let Some(tag) = data.image_tag.as_deref().filter(|s| !s.trim().is_empty()) {
            let image_ref = format!("ghcr.io/edgerun/worker:{tag}");
            verify_runtime_image_policy_response(
                cfg,
                &owner_pubkey,
                &signer.public_key_b64url,
                &request_nonce_b64url,
                &data,
                &image_ref,
            )?;
            println!("runtime_image_tag_requested={tag}");
            return Ok(image_ref);
        }
    }
    Err(anyhow!(
        "runtime image tag endpoint unavailable or returned no image reference"
    ))
}

fn pull_runtime_image(image_ref: &str) -> Result<()> {
    let runners: &[(&str, &[&str])] = &[
        ("nerdctl", &["pull"]),
        ("ctr", &["images", "pull"]),
        ("podman", &["pull"]),
        ("docker", &["pull"]),
        ("skopeo", &["copy", "--insecure-policy"]),
    ];

    for (bin, args) in runners {
        if !command_in_path(bin) {
            continue;
        }
        let status = if *bin == "skopeo" {
            Command::new(bin)
                .args(*args)
                .arg(format!("docker://{image_ref}"))
                .arg(format!("containers-storage:{image_ref}"))
                .status()
        } else {
            Command::new(bin).args(*args).arg(image_ref).status()
        }
        .with_context(|| format!("failed launching image pull via {bin}"))?;
        if status.success() {
            println!("runtime_image_pull_via={bin}");
            return Ok(());
        }
    }

    Err(anyhow!(
        "no supported image pull tool succeeded (tried nerdctl/ctr/podman/docker/skopeo)"
    ))
}

fn now_unix_s() -> Result<u64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock before unix epoch")?
        .as_secs())
}

fn runtime_image_request_canonical_message(
    owner_pubkey: &str,
    device_pubkey_b64url: &str,
    rpc_url: &str,
    request_nonce_b64url: &str,
    request_issued_at_unix_s: u64,
) -> String {
    format!(
        "{RUNTIME_IMAGE_REQUEST_SIGNING_CONTEXT}\nowner_pubkey={owner_pubkey}\ndevice_pubkey_b64url={device_pubkey_b64url}\nrpc_url={rpc_url}\nrequest_nonce_b64url={request_nonce_b64url}\nrequest_issued_at_unix_s={request_issued_at_unix_s}"
    )
}

struct RuntimeImagePolicyCanonicalInput<'a> {
    owner_pubkey: &'a str,
    device_pubkey_b64url: &'a str,
    rpc_url: &'a str,
    request_nonce_b64url: &'a str,
    image_ref: &'a str,
    issued_at_unix_s: u64,
    valid_until_unix_s: u64,
    rollback_index: u64,
}

fn runtime_image_policy_canonical_message(input: &RuntimeImagePolicyCanonicalInput<'_>) -> String {
    format!(
        "{RUNTIME_IMAGE_POLICY_SIGNING_CONTEXT}\nowner_pubkey={}\ndevice_pubkey_b64url={}\nrpc_url={}\nrequest_nonce_b64url={}\nimage_ref={}\nissued_at_unix_s={}\nvalid_until_unix_s={}\nrollback_index={}",
        input.owner_pubkey,
        input.device_pubkey_b64url,
        input.rpc_url,
        input.request_nonce_b64url,
        input.image_ref,
        input.issued_at_unix_s,
        input.valid_until_unix_s,
        input.rollback_index
    )
}

fn parse_ed25519_signature_b64url(signature_b64url: &str) -> Result<Signature> {
    let bytes = URL_SAFE_NO_PAD
        .decode(signature_b64url.trim().as_bytes())
        .context("invalid boot policy signature encoding")?;
    let arr: [u8; 64] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow!("boot policy signature must decode to 64 bytes"))?;
    Ok(Signature::from_bytes(&arr))
}

fn load_boot_policy_verifying_key() -> Result<VerifyingKey> {
    let raw = std::env::var(BOOT_POLICY_VERIFY_KEY_B64URL_ENV).with_context(|| {
        format!("missing {BOOT_POLICY_VERIFY_KEY_B64URL_ENV} for fail-closed boot policy verify")
    })?;
    let bytes = URL_SAFE_NO_PAD
        .decode(raw.trim().as_bytes())
        .with_context(|| format!("invalid base64url in {BOOT_POLICY_VERIFY_KEY_B64URL_ENV}"))?;
    let arr: [u8; 32] = bytes.as_slice().try_into().map_err(|_| {
        anyhow!("{BOOT_POLICY_VERIFY_KEY_B64URL_ENV} must decode to 32-byte ed25519 pubkey")
    })?;
    VerifyingKey::from_bytes(&arr).context("invalid ed25519 verifying key bytes")
}

fn verify_runtime_image_policy_response(
    cfg: &mut ManagerConfig,
    owner_pubkey: &str,
    device_pubkey_b64url: &str,
    request_nonce_b64url: &str,
    response: &RuntimeImageResponse,
    image_ref: &str,
) -> Result<()> {
    let response_nonce = response
        .request_nonce_b64url
        .as_deref()
        .ok_or_else(|| anyhow!("runtime policy response missing request_nonce_b64url"))?;
    if response_nonce != request_nonce_b64url {
        return Err(anyhow!(
            "runtime policy nonce mismatch (expected {request_nonce_b64url}, got {response_nonce})"
        ));
    }
    let issued_at_unix_s = response
        .issued_at_unix_s
        .ok_or_else(|| anyhow!("runtime policy response missing issued_at_unix_s"))?;
    let valid_until_unix_s = response
        .valid_until_unix_s
        .ok_or_else(|| anyhow!("runtime policy response missing valid_until_unix_s"))?;
    if valid_until_unix_s <= issued_at_unix_s {
        return Err(anyhow!(
            "runtime policy validity window is invalid ({issued_at_unix_s}..{valid_until_unix_s})"
        ));
    }
    let now = now_unix_s()?;
    if valid_until_unix_s <= now {
        return Err(anyhow!(
            "runtime policy expired at {valid_until_unix_s}, now={now}"
        ));
    }
    let rollback_index = response
        .rollback_index
        .ok_or_else(|| anyhow!("runtime policy response missing rollback_index"))?;
    if rollback_index < cfg.runtime_policy_rollback_index {
        return Err(anyhow!(
            "runtime policy rollback detected (stored={}, got={rollback_index})",
            cfg.runtime_policy_rollback_index
        ));
    }
    let canonical = runtime_image_policy_canonical_message(&RuntimeImagePolicyCanonicalInput {
        owner_pubkey,
        device_pubkey_b64url,
        rpc_url: &cfg.rpc_url,
        request_nonce_b64url,
        image_ref,
        issued_at_unix_s,
        valid_until_unix_s,
        rollback_index,
    });
    let signature_b64url = response
        .signature_b64url
        .as_deref()
        .ok_or_else(|| anyhow!("runtime policy response missing signature_b64url"))?;
    let signature = parse_ed25519_signature_b64url(signature_b64url)?;
    let verifying_key = load_boot_policy_verifying_key()?;
    verifying_key
        .verify(canonical.as_bytes(), &signature)
        .context("runtime policy signature verification failed")?;
    if let Some(signing_key_id) = response.signing_key_id.as_deref() {
        println!("runtime_policy_signing_key_id={signing_key_id}");
    }
    cfg.runtime_policy_rollback_index = rollback_index;
    Ok(())
}

fn command_in_path(cmd: &str) -> bool {
    Command::new("sh")
        .arg("-lc")
        .arg(format!("command -v {cmd} >/dev/null 2>&1"))
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

async fn send_heartbeat(
    client: &Client,
    cfg: &ManagerConfig,
    device_pubkey_b64url: &str,
) -> Result<()> {
    let owner_pubkey = cfg
        .owner_pubkey
        .as_deref()
        .ok_or_else(|| anyhow!("owner_pubkey missing for heartbeat"))?;

    let url = format!("{}/v1/node/heartbeat", cfg.api_base);
    let resp = post_json(
        client,
        &url,
        &NodeHeartbeatRequest {
            owner_pubkey,
            device_pubkey_b64url,
            rpc_url: &cfg.rpc_url,
            version: env!("CARGO_PKG_VERSION"),
            pid1: std::process::id() == 1,
        },
    )?
    .send()
    .await
    .with_context(|| format!("heartbeat request failed: {url}"))?;

    if resp.status() != StatusCode::OK {
        return Err(anyhow!("heartbeat rejected with status {}", resp.status()));
    }

    println!("heartbeat_ok=true");
    Ok(())
}

async fn call_ok_json<T: Serialize>(client: &Client, url: &str, payload: &T) -> Result<()> {
    let resp = post_json(client, url, payload)?
        .send()
        .await
        .with_context(|| format!("request failed: {url}"))?
        .error_for_status()
        .with_context(|| format!("endpoint returned error status: {url}"))?;
    let resp: ApiResponse =
        parse_json_response(resp, &format!("failed to decode endpoint response: {url}")).await?;

    if !resp.ok {
        return Err(anyhow!(
            "endpoint rejected request at {url}: {}",
            resp.error.unwrap_or_else(|| "unknown error".to_string())
        ));
    }
    Ok(())
}

fn post_json<T: Serialize>(
    client: &Client,
    url: &str,
    payload: &T,
) -> Result<reqwest::RequestBuilder> {
    let body = sonic_rs::to_vec(payload).context("failed to encode json request body")?;
    Ok(client
        .post(url)
        .header(CONTENT_TYPE, "application/json")
        .body(body))
}

async fn parse_json_response<T: DeserializeOwned>(
    resp: reqwest::Response,
    context: &str,
) -> Result<T> {
    let payload = resp
        .bytes()
        .await
        .with_context(|| format!("{context}: failed to read response body"))?;
    sonic_rs::from_slice(&payload).with_context(|| context.to_string())
}

async fn post_protobuf<Req, Resp>(client: &Client, url: &str, payload: &Req) -> Result<Resp>
where
    Req: Message,
    Resp: Message + Default,
{
    let mut body = Vec::with_capacity(payload.encoded_len());
    payload
        .encode(&mut body)
        .context("failed to encode protobuf request body")?;
    let resp = client
        .post(url)
        .header(CONTENT_TYPE, "application/protobuf")
        .body(body)
        .send()
        .await
        .with_context(|| format!("protobuf request failed: {url}"))?;
    if !resp.status().is_success() {
        return Err(anyhow!(
            "endpoint returned error status {}: {url}",
            resp.status()
        ));
    }
    let bytes = resp
        .bytes()
        .await
        .with_context(|| format!("failed to read protobuf response body: {url}"))?;
    Resp::decode(bytes.as_ref())
        .with_context(|| format!("failed to decode protobuf response: {url}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn opencode_mcp_sync_preserves_user_entries() {
        let existing = r#"{
          "$schema": "https://opencode.ai/config.json",
          "mcp": {
            "context7": {
              "type": "remote",
              "url": "https://mcp.context7.com/mcp"
            },
            "edgerun-github": {
              "command": "docker",
              "args": ["exec", "-i", "edgerun-mcp-github", "server", "stdio"],
              "enabled": false
            }
          },
          "theme": "dark"
        }"#;
        let next = apply_managed_opencode_mcp_entries(existing, &["github".to_string()])
            .expect("sync should succeed");
        let parsed: Value = sonic_rs::from_str(&next).expect("synced config should parse");

        assert!(parsed["mcp"].get("context7").is_object());
        assert!(parsed["mcp"].get("edgerun-github").is_object());
        assert_eq!(
            parsed
                .get("theme")
                .and_then(|value| value.as_str())
                .unwrap_or(""),
            "dark"
        );
    }

    #[test]
    fn opencode_mcp_sync_removes_managed_entries_when_not_running() {
        let existing = r#"{
          "mcp": {
            "edgerun-github": {
              "command": "docker",
              "args": ["exec", "-i", "edgerun-mcp-github", "server", "stdio"],
              "enabled": true
            },
            "context7": {
              "type": "remote",
              "url": "https://mcp.context7.com/mcp"
            }
          }
        }"#;
        let next =
            apply_managed_opencode_mcp_entries(existing, &[]).expect("sync should succeed");
        let parsed: Value = sonic_rs::from_str(&next).expect("synced config should parse");

        assert!(!parsed["mcp"].get("edgerun-github").is_object());
        assert!(parsed["mcp"].get("context7").is_object());
        assert_eq!(
            parsed["$schema"].as_str().unwrap_or_default(),
            OPENCODE_MCP_SCHEMA_URL
        );
    }

    #[test]
    fn mcp_image_for_google_integrations_has_defaults() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        let key = "EDGERUN_MCP_GOOGLE_MESSAGES_IMAGE";
        let previous = std::env::var(key).ok();
        unsafe {
            std::env::remove_var(key);
        }

        assert_eq!(
            mcp_image_for("google_messages").as_deref(),
            Some(MCP_IMAGE_GOOGLE_MESSAGES_DEFAULT)
        );
        assert_eq!(mcp_image_for("gvoice").as_deref(), Some(MCP_IMAGE_GVOICE_DEFAULT));
        assert_eq!(
            mcp_image_for("googlechat").as_deref(),
            Some(MCP_IMAGE_GOOGLECHAT_DEFAULT)
        );

        match previous {
            Some(value) => unsafe {
                std::env::set_var(key, value);
            },
            None => unsafe {
                std::env::remove_var(key);
            },
        }
    }

    #[test]
    fn mcp_image_for_prefers_env_override() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        let key = "EDGERUN_MCP_GOOGLE_MESSAGES_IMAGE";
        let previous = std::env::var(key).ok();
        unsafe {
            std::env::set_var(key, "example.com/custom/gmessages:stable");
        }
        let resolved = mcp_image_for("google_messages");

        match previous {
            Some(value) => unsafe {
                std::env::set_var(key, value);
            },
            None => unsafe {
                std::env::remove_var(key);
            },
        }

        assert_eq!(
            resolved.as_deref(),
            Some("example.com/custom/gmessages:stable")
        );
    }
}
