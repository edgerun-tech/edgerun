use std::collections::BTreeMap;
use std::collections::HashMap;
use std::error::Error;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use codex_core::Provider;
use codex_core::api::AuthProvider;
use codex_core::api::RetryConfig;
use codex_core::protocol::models::ContentItem;
use codex_core::protocol::models::ResponseItem;
use edgerun_crypto::Ed25519SigningKey;
use edgerun_http::HeaderMap;
use edgerun_http::HeaderValue;
use edgerun_http::header::AUTHORIZATION;
use edgerun_json::Value;
use edgerun_work::*;

mod pipeline;
mod repo_workspace;
mod ui_stream;

use crate::pipeline::PipelineObserver;

const CONTACT_CARD_MAGIC: &[u8] = b"EDGERUN-CHAT-CONTACT1";
const CONTACT_CARD_DOMAIN: &[u8] = b"edgerun:v1:work:chat-contact-card";
const DEFAULT_LISTEN: &str = "127.0.0.1:8787";
const DEFAULT_MODEL: &str = "local";
const DEFAULT_BASE_URL: &str = "http://127.0.0.1:5000/v1";

#[derive(Debug)]
struct HostAuth {
    bearer_token: Option<String>,
    account_id: Option<String>,
}

impl AuthProvider for HostAuth {
    fn add_auth_headers(&self, headers: &mut HeaderMap) {
        if let Some(token) = self.bearer_token.as_deref() {
            let bearer = format!("Bearer {token}");
            if let Ok(value) = HeaderValue::from_str(&bearer) {
                headers.insert(AUTHORIZATION, value);
            }
        }
        if let Some(account_id) = self.account_id.as_deref()
            && let Ok(value) = HeaderValue::from_str(account_id)
        {
            headers.insert("ChatGPT-Account-ID", value);
        }
    }
}

#[derive(Debug)]
struct Config {
    listen: String,
    model: String,
    base_url: String,
    api_key: Option<String>,
    ui_stream_stdout: bool,
    ui_stream_path: Option<PathBuf>,
    repo_root: PathBuf,
    agent_seed: [u8; 32],
    executor_seed: [u8; 32],
    print_contact: bool,
    mock_response: Option<String>,
}

struct UiStreamSink {
    writer: Option<Box<dyn Write>>,
}

impl UiStreamSink {
    fn open(stdout: bool, path: Option<&PathBuf>) -> Result<Self, Box<dyn Error>> {
        if stdout {
            return Ok(Self {
                writer: Some(Box::new(std::io::stdout())),
            });
        }
        let Some(path) = path else {
            return Ok(Self { writer: None });
        };
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        Ok(Self {
            writer: Some(Box::new(file)),
        })
    }

    fn emit_patch(&mut self, patch: Vec<u8>) -> Result<(), Box<dyn Error>> {
        let Some(writer) = self.writer.as_mut() else {
            return Ok(());
        };
        writer.write_all(&[ui_stream::MessageType::Patch as u8])?;
        writer.write_all(&patch)?;
        writer.flush()?;
        Ok(())
    }

    fn status(&mut self, text: &str) -> Result<(), Box<dyn Error>> {
        self.emit_patch(ui_stream::agent::status(text))
    }

    fn progress(&mut self, value: f32) -> Result<(), Box<dyn Error>> {
        self.emit_patch(ui_stream::agent::progress(value))
    }

    fn assistant_draft(&mut self, text: &str) -> Result<(), Box<dyn Error>> {
        self.emit_patch(ui_stream::agent::assistant_draft(text))
    }

    fn tool_call(&mut self, name: &str, detail: &str) -> Result<(), Box<dyn Error>> {
        self.emit_patch(ui_stream::agent::tool_call(name, detail))
    }
}

impl pipeline::PipelineObserver for UiStreamSink {
    fn stage_started(
        &mut self,
        stage: &pipeline::PipelineStage,
        index: usize,
        total: usize,
    ) -> Result<(), pipeline::BoxError> {
        let progress = (index as f32) / (total.max(1) as f32);
        self.status(&format!("pipeline: {}", stage.name))?;
        self.progress(progress)?;
        self.tool_call(stage.name, "started")?;
        Ok(())
    }

    fn stage_finished(
        &mut self,
        stage: &pipeline::PipelineStage,
        index: usize,
        total: usize,
        output: &str,
    ) -> Result<(), pipeline::BoxError> {
        let progress = ((index + 1) as f32) / (total.max(1) as f32);
        self.progress(progress)?;
        self.tool_call(stage.output_name, output)?;
        Ok(())
    }

    fn repo_revealed(&mut self, request: &str, found: bool) -> Result<(), pipeline::BoxError> {
        self.tool_call("repo_reveal", &format!("{} {}", request, if found { "ok" } else { "missing" }))?;
        Ok(())
    }

    fn repo_edited(&mut self, path: &str, ok: bool) -> Result<(), pipeline::BoxError> {
        self.tool_call("repo_edit", &format!("{} {}", path, if ok { "memory" } else { "failed" }))?;
        Ok(())
    }

    fn repo_written(&mut self, path: &str, ok: bool) -> Result<(), pipeline::BoxError> {
        self.tool_call("repo_writeback", &format!("{} {}", path, if ok { "written" } else { "failed" }))?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct PeerThreadState {
    next_sequence: u64,
    previous_message_hash: Hash,
    history: Vec<ResponseItem>,
}

impl Default for PeerThreadState {
    fn default() -> Self {
        Self {
            next_sequence: 1,
            previous_message_hash: [0u8; 32],
            history: Vec::new(),
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let config = parse_config()?;
    let mut repo_workspace = repo_workspace::RepoWorkspace::load(&config.repo_root)?;
    let agent_key = Ed25519SigningKey::from_bytes(&config.agent_seed);
    let agent = node_identity_from_key(&agent_key, NODE_ROLE_MESSAGE);
    let executor_key = Ed25519SigningKey::from_bytes(&config.executor_seed);
    let mut executor = ProgramIoService::new(executor_key, NativeProcessAdapter::new());

    if config.print_contact {
        println!("{}", hex(&contact_card(&agent_key, &agent)?));
        return Ok(());
    }

    let runtime = edgerun_tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let client = match config.mock_response {
        Some(_) => None,
        None => Some(codex_core::ModelClient::new_native(
            config.model.clone(),
            provider(&config),
            auth_provider(&config)?,
        )),
    };
    let mut ui_sink = UiStreamSink::open(config.ui_stream_stdout, config.ui_stream_path.as_ref())?;
    ui_sink.status("codex-host starting")?;
    ui_sink.progress(0.0)?;
    ui_sink.emit_patch(ui_stream::agent::run_button("Run pipeline"))?;

    let hub = WebSocketWorkHub::bind(&config.listen)?;
    eprintln!("codex-host listening on ws://{}", hub.listen_addr());
    eprintln!("codex model {}", config.model);
    eprintln!("codex model base_url {}", config.base_url);
    eprintln!(
        "repo workspace {} files {} bytes root {}",
        repo_workspace.file_count(),
        repo_workspace.total_bytes(),
        repo_workspace.root().display()
    );
    if config.ui_stream_stdout {
        eprintln!("ui_stream patch output stdout");
    } else if let Some(path) = &config.ui_stream_path {
        eprintln!("ui_stream patch output {}", path.display());
    }
    eprintln!("codex agent node {}", hex(&agent.node_id));
    eprintln!("host executor node {}", hex(&executor.node_id()));
    eprintln!("import contact card in frontend/chat.html:");
    eprintln!("{}", hex(&contact_card(&agent_key, &agent)?));
    ui_sink.tool_call(
        "repo workspace",
        &format!("{} files indexed in memory", repo_workspace.file_count()),
    )?;
    ui_sink.status("codex-host ready")?;

    let mut threads = BTreeMap::<NodeId, PeerThreadState>::new();
    loop {
        for envelope in hub.drain_envelopes() {
            if envelope.to == agent.node_id {
                if let Err(error) = handle_chat_envelope(
                    &runtime,
                    client.as_ref(),
                    config.mock_response.as_deref(),
                    &hub,
                    &agent_key,
                    &agent,
                    &mut repo_workspace,
                    &mut threads,
                    &mut ui_sink,
                    envelope,
                ) {
                    let _ = ui_sink.status("chat envelope failed");
                    eprintln!("chat envelope failed: {error}");
                }
            } else if envelope.to == executor.node_id() {
                if let Err(error) = handle_executor_envelope(&hub, &mut executor, &mut ui_sink, envelope) {
                    let _ = ui_sink.status("executor envelope failed");
                    eprintln!("executor envelope failed: {error}");
                }
            }
        }
        thread::sleep(Duration::from_millis(25));
    }
}


fn is_commit_command(value: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "commit" | "/commit" | "commit repo" | "writeback" | "/writeback"
    )
}

fn commit_pending_repo_changes(
    repo_workspace: &mut repo_workspace::RepoWorkspace,
    ui_sink: &mut UiStreamSink,
) -> Result<String, Box<dyn Error>> {
    let changed = repo_workspace.changed_files();
    if changed.is_empty() {
        return Ok("No pending in-memory repo changes to commit.".to_string());
    }

    let mut out = String::from("Committed pending in-memory repo changes to disk:\n");
    for path in changed {
        let ok = repo_workspace.write_back(&path).is_ok();
        ui_sink.repo_written(&path, ok)?;
        out.push_str("- ");
        out.push_str(&path);
        out.push_str(if ok { " written\n" } else { " write failed\n" });
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
fn handle_chat_envelope(
    runtime: &edgerun_tokio::runtime::Runtime,
    client: Option<&codex_core::ModelClient>,
    mock_response: Option<&str>,
    hub: &WebSocketWorkHub,
    agent_key: &Ed25519SigningKey,
    agent: &NodeIdentity,
    repo_workspace: &mut repo_workspace::RepoWorkspace,
    threads: &mut BTreeMap<NodeId, PeerThreadState>,
    ui_sink: &mut UiStreamSink,
    envelope: ChannelEnvelope,
) -> Result<(), Box<dyn Error>> {
    let WorkPacket::NetworkMessage(message) = envelope.packet else {
        return Ok(());
    };
    if message.work_type != WORK_TYPE_MESSAGE_DELIVER || message.department != DEPARTMENT_MESSAGE {
        return Ok(());
    }
    let sender = sender_identity_from_message(&message)?;
    if !verify_network_message(&message, &sender) {
        return Err("message signature failed".into());
    }
    let plaintext = unseal_message_from_recipient_payload(agent_key, agent, &message.payload)
        .map_err(|error| format!("message unseal failed: {error:?}"))?;
    let text = String::from_utf8_lossy(&plaintext).trim().to_string();
    if text.is_empty() {
        return Ok(());
    }

    ui_sink.status("pipeline")?;
    ui_sink.progress(0.0)?;
    ui_sink.assistant_draft("")?;

    let thread_state = threads.entry(message.from).or_default();
    thread_state.previous_message_hash = message.message_id;
    thread_state.next_sequence = thread_state
        .next_sequence
        .max(message.sequence.saturating_add(1));
    thread_state.history.push(user_item(text.clone()));

    if is_commit_command(&text) {
        let reply = commit_pending_repo_changes(repo_workspace, ui_sink)?;
        thread_state.history.push(assistant_item(reply.clone()));
        let response = signed_chat_response(
            agent_key,
            agent,
            &sender,
            message.via_relay,
            envelope.channel_id,
            envelope.route_hash,
            thread_state.next_sequence,
            thread_state.previous_message_hash,
            reply.as_bytes(),
        )?;
        thread_state.previous_message_hash = response.message_id;
        thread_state.next_sequence = thread_state.next_sequence.saturating_add(1);
        hub.send_envelope_to(sender.node_id, &response.envelope)?;
        ui_sink.assistant_draft(&reply)?;
        ui_sink.status("ready")?;
        return Ok(());
    }

    let repo_context = repo_workspace.context_for_request(&text);
    ui_sink.tool_call("repo index", "using persistent in-memory repo state")?;

    let pipeline_output = if mock_response.is_some() {
        pipeline::run_mock_pipeline(&text, ui_sink)?
    } else {
        let client = client.ok_or("codex model client unavailable")?;
        pipeline::run_pipeline(
            runtime,
            client,
            &text,
            &thread_state.history,
            &repo_context,
            repo_workspace,
            ui_sink,
        )?
    };
    let reply = pipeline_output.final_reply;
    ui_sink.assistant_draft(&reply)?;
    ui_sink.status("finalizing")?;
    thread_state.history.push(assistant_item(reply.clone()));

    let response = signed_chat_response(
        agent_key,
        agent,
        &sender,
        message.via_relay,
        envelope.channel_id,
        envelope.route_hash,
        thread_state.next_sequence,
        thread_state.previous_message_hash,
        reply.as_bytes(),
    )?;
    thread_state.previous_message_hash = response.message_id;
    thread_state.next_sequence = thread_state.next_sequence.saturating_add(1);
    hub.send_envelope_to(sender.node_id, &response.envelope)?;
    ui_sink.progress(1.0)?;
    ui_sink.status("ready")?;
    Ok(())
}

fn handle_executor_envelope(
    hub: &WebSocketWorkHub,
    executor: &mut ProgramIoService<NativeProcessAdapter>,
    ui_sink: &mut UiStreamSink,
    envelope: ChannelEnvelope,
) -> Result<(), Box<dyn Error>> {
    let from = envelope.from;
    let channel_id = envelope.channel_id;
    let route_hash = envelope.route_hash;
    ui_sink.tool_call("host executor", "packet received")?;
    let response = executor.handle_packet(envelope.packet, now_unix_ms());
    let Some(packet) = response.packet else {
        ui_sink.tool_call("host executor", "no response packet")?;
        return Ok(());
    };
    let encoded = encode_work_packet_once(&packet)
        .map_err(|error| format!("program event packet encode failed: {error:?}"))?;
    let response_envelope = ChannelEnvelope {
        abi_version: WORK_WIRE_ABI_VERSION,
        channel_id,
        from: executor.node_id(),
        to: from,
        route_hash,
        packet_hash: encoded.hash,
        packet,
    };
    hub.send_envelope_to(from, &response_envelope)?;
    ui_sink.tool_call("host executor", "event sent")?;
    Ok(())
}

struct SignedChatResponse {
    message_id: Hash,
    envelope: ChannelEnvelope,
}

#[allow(clippy::too_many_arguments)]
fn signed_chat_response(
    agent_key: &Ed25519SigningKey,
    agent: &NodeIdentity,
    recipient: &NodeIdentity,
    via_relay: NodeId,
    channel_id: ChannelId,
    route_hash: Hash,
    sequence: u64,
    previous_message_hash: Hash,
    plaintext: &[u8],
) -> Result<SignedChatResponse, Box<dyn Error>> {
    let sealed_payload = seal_message_for_recipient(agent, recipient, plaintext)
        .map_err(|error| format!("message seal failed: {error:?}"))?;
    let payload_hash = blake3_hash(&sealed_payload);
    let message_id =
        simple_network_message_id(&agent.node_id, &recipient.node_id, sequence, &payload_hash);
    let message = sign_network_message_payload(
        agent_key,
        message_id,
        previous_message_hash,
        agent.node_id,
        recipient.node_id,
        via_relay,
        DEPARTMENT_MESSAGE,
        WORK_TYPE_MESSAGE_DELIVER,
        sequence,
        sealed_payload,
    );
    let packet = WorkPacket::NetworkMessage(message);
    let encoded = encode_work_packet_once(&packet)
        .map_err(|error| format!("chat packet encode failed: {error:?}"))?;
    Ok(SignedChatResponse {
        message_id,
        envelope: ChannelEnvelope {
            abi_version: WORK_WIRE_ABI_VERSION,
            channel_id,
            from: agent.node_id,
            to: recipient.node_id,
            route_hash,
            packet_hash: encoded.hash,
            packet,
        },
    })
}

fn sender_identity_from_message(message: &NetworkMessage) -> Result<NodeIdentity, Box<dyn Error>> {
    if message.signature.public_key.len() != 32 {
        return Err("sender public key missing".into());
    }
    let mut public_key = [0u8; 32];
    public_key.copy_from_slice(&message.signature.public_key);
    Ok(NodeIdentity {
        node_id: message.from,
        role: NODE_ROLE_MESSAGE,
        public_key,
    })
}

fn user_item(text: String) -> ResponseItem {
    ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputText { text }],
        phase: None,
    }
}

fn assistant_item(text: String) -> ResponseItem {
    ResponseItem::Message {
        id: None,
        role: "assistant".to_string(),
        content: vec![ContentItem::OutputText { text }],
        phase: None,
    }
}

fn provider(config: &Config) -> Provider {
    let mut headers = HeaderMap::new();
    headers.insert(
        "version",
        HeaderValue::from_static(env!("CARGO_PKG_VERSION")),
    );
    Provider {
        name: provider_name(&config.base_url).to_string(),
        base_url: config.base_url.clone(),
        query_params: None::<HashMap<String, String>>,
        headers,
        retry: RetryConfig {
            max_attempts: 1,
            base_delay: Duration::from_millis(200),
            retry_429: false,
            retry_5xx: false,
            retry_transport: false,
        },
        stream_idle_timeout: Duration::from_secs(60),
    }
}

fn provider_name(base_url: &str) -> &'static str {
    if base_url.contains("chatgpt.com") || base_url.contains("openai.com") {
        "OpenAI"
    } else {
        "OpenAI-compatible-local"
    }
}

fn auth_provider(config: &Config) -> Result<Arc<HostAuth>, Box<dyn Error>> {
    if let Some(api_key) = config.api_key.as_deref().filter(|value| !value.is_empty()) {
        return Ok(Arc::new(HostAuth {
            bearer_token: Some(api_key.to_string()),
            account_id: None,
        }));
    }

    if config.base_url.contains("chatgpt.com") {
        return read_chatgpt_auth();
    }

    Ok(Arc::new(HostAuth {
        bearer_token: None,
        account_id: None,
    }))
}

fn codex_home() -> PathBuf {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex")))
        .expect("CODEX_HOME or HOME must be set")
}

fn read_chatgpt_auth() -> Result<Arc<HostAuth>, Box<dyn Error>> {
    let auth_path = codex_home().join("auth.json");
    let auth: Value = edgerun_json::from_slice(&std::fs::read(&auth_path)?)?;
    let access_token = auth
        .get("tokens")
        .and_then(|tokens| tokens.get("access_token"))
        .and_then(Value::as_str)
        .filter(|token| !token.is_empty())
        .ok_or_else(|| format!("missing tokens.access_token in {}", auth_path.display()))?
        .to_string();
    let account_id = auth
        .get("tokens")
        .and_then(|tokens| tokens.get("account_id"))
        .and_then(Value::as_str)
        .filter(|account_id| !account_id.is_empty())
        .map(ToString::to_string);
    Ok(Arc::new(HostAuth {
        bearer_token: Some(access_token),
        account_id,
    }))
}

fn contact_card(
    key: &Ed25519SigningKey,
    identity: &NodeIdentity,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let preimage = contact_card_preimage(&identity.node_id, &identity.public_key);
    let signature = sign_ed25519(key, &preimage);
    if signature.signature.len() != 64 {
        return Err("contact card signature failed".into());
    }
    let mut out = Vec::with_capacity(CONTACT_CARD_MAGIC.len() + 32 + 32 + 64);
    out.extend_from_slice(CONTACT_CARD_MAGIC);
    out.extend_from_slice(&identity.node_id);
    out.extend_from_slice(&identity.public_key);
    out.extend_from_slice(&signature.signature);
    Ok(out)
}

fn contact_card_preimage(node_id: &[u8; 32], public_key: &[u8; 32]) -> Vec<u8> {
    let mut preimage = CONTACT_CARD_DOMAIN.to_vec();
    preimage.extend_from_slice(node_id);
    preimage.extend_from_slice(public_key);
    preimage
}

fn parse_config() -> Result<Config, Box<dyn Error>> {
    let mut config = Config {
        listen: std::env::var("CODEX_HOST_LISTEN").unwrap_or_else(|_| DEFAULT_LISTEN.to_string()),
        model: std::env::var("CODEX_HOST_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string()),
        base_url: std::env::var("CODEX_HOST_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string()),
        api_key: std::env::var("CODEX_HOST_API_KEY").ok(),
        ui_stream_stdout: env_bool("CODEX_HOST_UI_STREAM_STDOUT"),
        ui_stream_path: std::env::var_os("CODEX_HOST_UI_STREAM_PATH").map(PathBuf::from),
        repo_root: std::env::var_os("CODEX_HOST_REPO_ROOT")
            .map(PathBuf::from)
            .unwrap_or(std::env::current_dir()?),
        agent_seed: seed_from_env("CODEX_HOST_AGENT_SEED_HEX", 201)?,
        executor_seed: seed_from_env("CODEX_HOST_EXECUTOR_SEED_HEX", 202)?,
        print_contact: false,
        mock_response: std::env::var("CODEX_HOST_MOCK_RESPONSE").ok(),
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--listen" => config.listen = args.next().ok_or("--listen requires an address")?,
            "--model" => config.model = args.next().ok_or("--model requires a model")?,
            "--base-url" => config.base_url = args.next().ok_or("--base-url requires a URL")?,
            "--api-key" => config.api_key = Some(args.next().ok_or("--api-key requires a value")?),
            "--repo-root" => config.repo_root = PathBuf::from(args.next().ok_or("--repo-root requires a path")?),
            "--ui-stream-stdout" => config.ui_stream_stdout = true,
            "--ui-stream-path" => {
                config.ui_stream_path = Some(PathBuf::from(args.next().ok_or("--ui-stream-path requires a path")?))
            }
            "--agent-seed-hex" => {
                config.agent_seed =
                    parse_seed(&args.next().ok_or("--agent-seed-hex requires hex")?)?
            }
            "--executor-seed-hex" => {
                config.executor_seed =
                    parse_seed(&args.next().ok_or("--executor-seed-hex requires hex")?)?
            }
            "--print-contact" => config.print_contact = true,
            "--mock-response" => {
                config.mock_response = Some(args.next().ok_or("--mock-response requires text")?)
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}").into()),
        }
    }
    Ok(config)
}

fn print_help() {
    println!(
        "Usage: codex-host [--listen ADDR] [--model MODEL] [--base-url URL] [--api-key KEY] [--repo-root PATH] [--ui-stream-stdout|--ui-stream-path PATH] [--print-contact] [--mock-response TEXT]"
    );
    println!(
        "Env: CODEX_HOST_LISTEN CODEX_HOST_MODEL CODEX_HOST_BASE_URL CODEX_HOST_API_KEY CODEX_HOST_REPO_ROOT CODEX_HOST_UI_STREAM_STDOUT CODEX_HOST_UI_STREAM_PATH CODEX_HOST_AGENT_SEED_HEX CODEX_HOST_EXECUTOR_SEED_HEX CODEX_HOST_MOCK_RESPONSE"
    );
    println!("Default base URL: {DEFAULT_BASE_URL}");
    println!("Repository files are indexed into an in-memory workspace from --repo-root/current directory. Native mode should prefer --ui-stream-stdout and read raw ui_stream MessageType.patch bytes from the child stdout pipe; logs are written to stderr.");
}

fn env_bool(name: &str) -> bool {
    matches!(
        std::env::var(name).as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes") | Ok("YES") | Ok("on") | Ok("ON")
    )
}

fn seed_from_env(name: &str, fallback_byte: u8) -> Result<[u8; 32], Box<dyn Error>> {
    match std::env::var(name) {
        Ok(value) => parse_seed(&value),
        Err(_) => Ok([fallback_byte; 32]),
    }
}

fn parse_seed(value: &str) -> Result<[u8; 32], Box<dyn Error>> {
    let bytes = hex_bytes(value.trim().trim_start_matches("0x"))?;
    if bytes.len() != 32 {
        return Err("seed must be exactly 32 bytes of hex".into());
    }
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&bytes);
    Ok(seed)
}

fn hex_bytes(value: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    if !value.len().is_multiple_of(2) {
        return Err("hex length must be even".into());
    }
    let mut out = Vec::with_capacity(value.len() / 2);
    let mut index = 0;
    while index < value.len() {
        out.push(u8::from_str_radix(&value[index..index + 2], 16)?);
        index += 2;
    }
    Ok(out)
}

fn hex(bytes: &[u8]) -> String {
    const TABLE: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(TABLE[(byte >> 4) as usize] as char);
        out.push(TABLE[(byte & 0x0f) as usize] as char);
    }
    out
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}
