use crate::app_principal::AppKeyPair;
use crate::wasm_host::encrypted_envelope_compat::{
    looks_like_encrypted_envelope, validate_encrypted_envelope,
};
use anyhow::{Context, Result};
use edgerun_core::protocol::Timestamp;
use edgerun_core::protocol::{
    common::{EncryptedEnvelope, IdentityRef, NodeRef},
    stream::{
        AddBootstrapNodePayload, AddReachabilityHintPayload, AppIntent, CommandEnvelope,
        CommandType, CreateIdentityPayload, ImportIdentityPayload, QueryNodeStatePayload,
    },
    trust::DelegationRecord,
};
use edgerun_crypto::p256::elliptic_curve::sec1::ToEncodedPoint;
use std::sync::{Arc, Mutex};
// TODO: Replace with real wasmtime when feature-gated
mod wasmtime_mock;
use wasmtime_mock::*;

#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub node_identity: IdentityRef,
    pub app_instance_id: [u8; 16],
    pub delegation_chain: Vec<DelegationRecord>,
    pub app_key: Arc<AppKeyPair>,
}

pub struct WasmRuntime {
    engine: Engine,
    module: Module,
    linker: Linker<HostState>,
    output: Arc<Mutex<Vec<u8>>>,
    verbose: bool,
}

pub struct HostState {
    pub output: Arc<Mutex<Vec<u8>>>,
    pub context: Option<ExecutionContext>,
    pub pending_commands: Arc<Mutex<Vec<CommandEnvelope>>>,
    pub pending_events: Arc<Mutex<Vec<Vec<u8>>>>,
    pub next_command_seq: u64,
}

pub struct WasmRunResult {
    pub ui_bytes: Vec<u8>,
    pub pending_commands: Vec<CommandEnvelope>,
}

impl WasmRuntime {
    pub fn new(wasm_path: &str, verbose: bool) -> Result<Self> {
        let engine = Engine::default();
        let module = Module::from_file(&engine, wasm_path).context("Failed to load WASM module")?;

        let output = Arc::new(Mutex::new(Vec::new()));

        let mut linker = Linker::new(&engine);

        let output_clone = output.clone();
        linker.func_wrap(
            "env",
            "write_output",
            move |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| {
                let mem = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .expect("No memory export");
                let data = mem.data(&caller);
                let start = ptr as usize;
                let end = start.saturating_add(len as usize);
                if end > data.len() {
                    return;
                }
                let slice = &data[start..end];
                let mut out = output_clone.lock().unwrap();
                out.clear();
                out.extend_from_slice(slice);
            },
        )?;

        linker.func_wrap(
            "env",
            "send_message",
            |mut caller: Caller<'_, HostState>,
             target_ptr: i32,
             target_len: i32,
             payload_ptr: i32,
             payload_len: i32|
             -> i32 {
                let mem = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .expect("No memory export");
                let mem_data = mem.data(&caller);
                let safe_read_str = |p: i32, l: i32| -> String {
                    let start = p as usize;
                    let end = start.saturating_add(l as usize);
                    if end > mem_data.len() {
                        String::from("<oob>")
                    } else {
                        String::from_utf8_lossy(&mem_data[start..end]).to_string()
                    }
                };
                let safe_read_bytes = |p: i32, l: i32| -> Vec<u8> {
                    let start = p as usize;
                    let end = start.saturating_add(l as usize);
                    if end > mem_data.len() {
                        Vec::new()
                    } else {
                        mem_data[start..end].to_vec()
                    }
                };
                let payload_str = safe_read_str(payload_ptr, payload_len);
                let payload_bytes = safe_read_bytes(payload_ptr, payload_len);

                let host = caller.data_mut();

                if let Some(cmd) = interpret_send_message(&payload_str) {
                    host.pending_commands.lock().unwrap().push(cmd);
                } else {
                    let env = match encrypted_envelope_compat::EncryptedEnvelope::decode(
                        payload_bytes.as_slice(),
                    ) {
                        Ok(e) => e,
                        Err(_) => {
                            eprintln!("send_message REJECTED: not a valid EncryptedEnvelope");
                            return -1;
                        }
                    };

                    if let Err(reason) = validate_encrypted_envelope(&env) {
                        eprintln!("send_message REJECTED: {}", reason);
                        return -1;
                    }

                    let cmd = build_command(
                        host,
                        CommandType::StoreObject,
                        native_app_encode(&env),
                        String::new(),
                    );
                    host.pending_commands.lock().unwrap().push(cmd);
                }

                0
            },
        )?;

        linker.func_wrap(
            "env",
            "read_blob",
            |mut caller: Caller<'_, HostState>,
             hash_ptr: i32,
             hash_len: i32,
             _dst_ptr: i32|
             -> i32 {
                let mem = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .expect("No memory export");
                let mem_data = mem.data(&caller);
                let start = hash_ptr as usize;
                let end = start.saturating_add(hash_len as usize);
                let hash = if end <= mem_data.len() {
                    mem_data[start..end].to_vec()
                } else {
                    Vec::new()
                };

                let host = caller.data_mut();
                let cmd = build_command(host, CommandType::FetchObject, hash, String::new());

                host.pending_commands.lock().unwrap().push(cmd);
                0
            },
        )?;

        linker.func_wrap(
            "env",
            "write_blob",
            |mut caller: Caller<'_, HostState>, ptr: i32, len: i32, _hash_out_ptr: i32| -> i32 {
                let mem = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .expect("No memory export");
                let mem_data = mem.data(&caller);
                let start = ptr as usize;
                let end = start.saturating_add(len as usize);
                let payload = if end <= mem_data.len() {
                    mem_data[start..end].to_vec()
                } else {
                    Vec::new()
                };

                let host = caller.data_mut();
                let cmd = build_command(host, CommandType::StoreObject, payload, String::new());

                host.pending_commands.lock().unwrap().push(cmd);
                0
            },
        )?;

        linker.func_wrap(
            "env",
            "poll_event",
            |mut caller: Caller<'_, HostState>, buf_ptr: i32, buf_len: i32| -> i32 {
                let event = {
                    let host = caller.data();
                    host.pending_events.lock().unwrap().pop()
                };
                if let Some(event) = event {
                    let len = event.len();
                    let mem = caller
                        .get_export("memory")
                        .and_then(|e| e.into_memory())
                        .expect("No memory export");
                    let data = mem.data_mut(&mut caller);
                    let start = buf_ptr as usize;
                    let end = start.saturating_add(buf_len as usize);
                    if end <= data.len() && len <= buf_len as usize {
                        data[start..start + len].copy_from_slice(&event);
                        len as i32
                    } else {
                        -2
                    }
                } else {
                    -1
                }
            },
        )?;

        linker.func_wrap(
            "env",
            "request_user_presence",
            |mut caller: Caller<'_, HostState>,
             reason_ptr: i32,
             reason_len: i32,
             session_ptr: i32,
             session_len: i32,
             ttl_seconds: u32,
             out_token_ptr: i32,
             _out_token_len: i32|
             -> i32 {
                let mem = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .expect("No memory export");
                let mem_data = mem.data(&caller);
                let safe_read = |p: i32, l: i32| -> Vec<u8> {
                    let start = p as usize;
                    let end = start.saturating_add(l as usize);
                    if end > mem_data.len() {
                        Vec::new()
                    } else {
                        mem_data[start..end].to_vec()
                    }
                };
                let reason =
                    String::from_utf8_lossy(&safe_read(reason_ptr, reason_len)).to_string();
                let session_id = safe_read(session_ptr, session_len);

                let app_id = {
                    let host = caller.data();
                    host.context
                        .as_ref()
                        .map(|c| c.app_key.app_id.clone())
                        .unwrap_or_default()
                };

                {
                    let host = caller.data_mut();
                    let payload_bytes =
                        native_app_encode(&edgerun_core::protocol::RequestUserPresencePayload {
                            payload_version: 1,
                            reason,
                            session_id: session_id.clone(),
                            ttl_seconds,
                        });

                    let cmd = build_command(
                        host,
                        CommandType::RequestUserPresence,
                        payload_bytes,
                        String::new(),
                    );
                    host.pending_commands.lock().unwrap().push(cmd);
                }

                let mut token = [0u8; 32];
                edgerun_crypto::fill_random(&mut token);
                let out_token = out_token_ptr as usize;
                let data = mem.data_mut(&mut caller);
                if out_token + 32 <= data.len() {
                    data[out_token..out_token + 32].copy_from_slice(&token);
                }

                let expires_at = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_micros() as i64
                    + (ttl_seconds as i64 * 1_000_000);

                let event_bytes =
                    native_app_encode(&edgerun_core::protocol::UserPresenceGrantedPayload {
                        payload_version: 1,
                        presence_token: token.to_vec(),
                        app_id,
                        session_id,
                        expires_at,
                    });
                {
                    let host = caller.data_mut();
                    host.pending_events.lock().unwrap().push(event_bytes);
                }

                0
            },
        )?;

        linker.func_wrap(
            "env",
            "request_signature",
            |mut caller: Caller<'_, HostState>,
             payload_ptr: i32,
             payload_len: i32,
             action_ptr: i32,
             action_len: i32,
             human_ptr: i32,
             human_len: i32,
             session_ptr: i32,
             session_len: i32,
             token_ptr: i32,
             token_len: i32,
             out_sig_ptr: i32,
             _out_sig_len: i32|
             -> i32 {
                let mem = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .expect("No memory export");
                let mem_data = mem.data(&caller);
                let safe_read = |p: i32, l: i32| -> Vec<u8> {
                    let start = p as usize;
                    let end = start.saturating_add(l as usize);
                    if end > mem_data.len() {
                        Vec::new()
                    } else {
                        mem_data[start..end].to_vec()
                    }
                };
                let payload = safe_read(payload_ptr, payload_len);
                let action =
                    String::from_utf8_lossy(&safe_read(action_ptr, action_len)).to_string();
                let human_readable =
                    String::from_utf8_lossy(&safe_read(human_ptr, human_len)).to_string();
                let session_id = safe_read(session_ptr, session_len);
                let presence_token = safe_read(token_ptr, token_len);

                let (app_id, verifying_key_bytes, signature) = {
                    let host = caller.data();
                    let app_id = host
                        .context
                        .as_ref()
                        .map(|c| c.app_key.app_id.clone())
                        .unwrap_or_default();
                    let verifying_key_bytes = host
                        .context
                        .as_ref()
                        .map(|c| {
                            c.app_key
                                .verifying_key
                                .to_encoded_point(false)
                                .as_bytes()
                                .to_vec()
                        })
                        .unwrap_or_default();
                    let signature = host
                        .context
                        .as_ref()
                        .and_then(|c| c.app_key.sign_intent(&payload).ok())
                        .map(|intent| intent.signature)
                        .unwrap_or_default();
                    (app_id, verifying_key_bytes, signature)
                };

                let cmd_payload =
                    native_app_encode(&edgerun_core::protocol::RequestSignaturePayload {
                        payload_version: 1,
                        payload: payload.clone(),
                        human_readable: human_readable.clone(),
                        action: action.clone(),
                        session_id: session_id.clone(),
                        presence_token,
                    });

                {
                    let host = caller.data_mut();
                    let cmd = build_command(
                        host,
                        CommandType::RequestSignature,
                        cmd_payload,
                        String::new(),
                    );
                    host.pending_commands.lock().unwrap().push(cmd);
                }

                let out_sig = out_sig_ptr as usize;
                let data = mem.data_mut(&mut caller);
                let sig_len = signature.len().min(64);
                if out_sig + sig_len <= data.len() {
                    data[out_sig..out_sig + sig_len].copy_from_slice(&signature[..sig_len]);
                }

                let event_bytes =
                    native_app_encode(&edgerun_core::protocol::SignatureResponsePayload {
                        payload_version: 1,
                        app_id,
                        payload,
                        signature,
                        signing_key: verifying_key_bytes,
                        session_id,
                    });
                {
                    let host = caller.data_mut();
                    host.pending_events.lock().unwrap().push(event_bytes);
                }

                0
            },
        )?;

        Ok(Self {
            engine,
            module,
            linker,
            output,
            verbose,
        })
    }

    pub fn run_with_context(&mut self, input: &str, context: ExecutionContext) -> Result<Vec<u8>> {
        let result = self.run_with_events(input, context, Vec::new())?;
        Ok(result.ui_bytes)
    }

    pub fn run_with_events(
        &mut self,
        input: &str,
        context: ExecutionContext,
        events: Vec<Vec<u8>>,
    ) -> Result<WasmRunResult> {
        self.output.lock().unwrap().clear();

        let pending_commands = Arc::new(Mutex::new(Vec::new()));
        let pending_events = Arc::new(Mutex::new(events));

        let host = HostState {
            output: self.output.clone(),
            context: Some(context),
            pending_commands: pending_commands.clone(),
            pending_events: pending_events.clone(),
            next_command_seq: 0,
        };

        let mut store = Store::new(&self.engine, host);
        let instance = self.linker.instantiate(&mut store, &self.module)?;

        let run_func = instance
            .get_typed_func::<(i32, i32), i32>(&mut store, "run")
            .context("Failed to find 'run' function")?;

        let input_bytes = input.as_bytes();
        let input_len = input_bytes.len() as i32;

        if input_len > 0 {
            let memory = instance
                .get_memory(&mut store, "memory")
                .context("No memory export")?;

            let offset = memory.data_size(&mut store) as i32;
            memory.grow(&mut store, 1)?;

            let mem_data = memory.data_mut(&mut store);
            let start = offset as usize;
            let end = start + input_bytes.len();
            if end <= mem_data.len() {
                mem_data[start..end].copy_from_slice(input_bytes);
                run_func.call(&mut store, (offset, input_len))?;
            }
        } else {
            run_func.call(&mut store, (0, 0))?;
        }

        let cmds = pending_commands.lock().unwrap().clone();

        if self.verbose && !cmds.is_empty() {
            eprintln!("WASM produced {} pending command(s)", cmds.len());
            for (i, cmd) in cmds.iter().enumerate() {
                eprintln!(
                    "  cmd[{}]: type={:?}, id_len={}",
                    i,
                    edgerun_core::protocol::enum_from_i32::<CommandType>(cmd.command_type),
                    cmd.command_id.len()
                );
            }
        }

        let out = self.output.lock().unwrap().clone();
        let ui_bytes = extract_ui_bytes(&out);

        Ok(WasmRunResult {
            ui_bytes,
            pending_commands: cmds,
        })
    }

    pub fn run(&mut self, input: &str) -> Result<Vec<u8>> {
        let default_app_key = AppKeyPair::generate(b"default-app".to_vec())?;
        self.run_with_context(
            input,
            ExecutionContext {
                node_identity: IdentityRef {
                    identity_id: Vec::new(),
                    identity_kind: None,
                    key_hint: None,
                },
                app_instance_id: [0u8; 16],
                delegation_chain: Vec::new(),
                app_key: Arc::new(default_app_key),
            },
        )
    }
}

fn now_ts() -> Timestamp {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    Timestamp {
        seconds: secs,
        nanos: 0,
    }
}

fn build_command(
    host: &mut HostState,
    command_type: CommandType,
    payload: Vec<u8>,
    _target_hint: String,
) -> CommandEnvelope {
    let seq = host.next_command_seq;
    host.next_command_seq += 1;

    let ctx = host.context.as_ref();
    let issuer = ctx
        .map(|c| c.node_identity.clone())
        .unwrap_or_else(|| IdentityRef {
            identity_id: Vec::new(),
            identity_kind: None,
            key_hint: None,
        });
    let delegation_chain = ctx.map(|c| c.delegation_chain.clone()).unwrap_or_default();

    let app_intent_bytes = ctx
        .and_then(|c| c.app_key.sign_intent(&payload).ok())
        .map(|intent| native_app_encode(&intent))
        .unwrap_or_default();

    CommandEnvelope {
        envelope_version: 1,
        command_id: format!("wasm-cmd-{}", seq).into_bytes(),
        target_node: Some(NodeRef {
            node_id: issuer.identity_id.clone(),
        }),
        issuer: Some(issuer),
        command_type: command_type as i32,
        command_version: 1,
        issued_at: Some(now_ts()),
        not_before: None,
        expires_at: None,
        idempotency_key: format!("idem-{}", seq).into_bytes(),
        payload: if payload.is_empty() {
            None
        } else {
            Some(edgerun_core::protocol::command_envelope::Payload::InlinePayload(payload))
        },
        delegation_chain,
        requested_assurance: None,
        command_metadata: None,
        signature: None,
        app_intent: app_intent_bytes,
    }
}

pub fn build_bootstrap_command(host: &mut HostState, action: &str) -> Option<CommandEnvelope> {
    match action {
        "generate_identity" => {
            let payload = CreateIdentityPayload {
                payload_version: 1,
                label: String::from("primary"),
                key_algorithm: 1,
            };
            Some(build_command(
                host,
                CommandType::CreateIdentity,
                native_app_encode(&payload),
                String::new(),
            ))
        }
        "import_identity" => {
            let payload = ImportIdentityPayload {
                payload_version: 1,
                label: String::from("imported"),
                key_algorithm: 1,
                public_key: Vec::new(),
                encrypted_private_key: Vec::new(),
                source: String::from("file"),
            };
            Some(build_command(
                host,
                CommandType::ImportIdentity,
                native_app_encode(&payload),
                String::new(),
            ))
        }
        "add_controller" => {
            let payload = AddBootstrapNodePayload {
                payload_version: 1,
                node: None,
                address: String::from("local"),
                transport_class: 0,
                label: String::from("controller"),
            };
            Some(build_command(
                host,
                CommandType::AddController,
                native_app_encode(&payload),
                String::new(),
            ))
        }
        "add_bootstrap_peer" => {
            let payload = AddBootstrapNodePayload {
                payload_version: 1,
                node: None,
                address: String::from("quic://peer.local:4242"),
                transport_class: 3,
                label: String::from("bootstrap-peer"),
            };
            Some(build_command(
                host,
                CommandType::AddBootstrapNode,
                native_app_encode(&payload),
                String::new(),
            ))
        }
        "publish_reachability" => {
            let payload = AddReachabilityHintPayload {
                payload_version: 1,
                address: String::from("quic://self.local:4242"),
                transport_class: 3,
                directness: 1,
            };
            Some(build_command(
                host,
                CommandType::AddReachabilityHint,
                native_app_encode(&payload),
                String::new(),
            ))
        }
        "query_node_state" => {
            let payload = QueryNodeStatePayload {
                payload_version: 1,
                query_kind: String::from("all"),
            };
            Some(build_command(
                host,
                CommandType::QueryNodeState,
                native_app_encode(&payload),
                String::new(),
            ))
        }
        _ => None,
    }
}

fn extract_ui_bytes(raw_output: &[u8]) -> Vec<u8> {
    if raw_output.is_empty() {
        return Vec::new();
    }

    if raw_output.len() < 10 {
        return raw_output.to_vec();
    }

    let ct_len =
        u32::from_le_bytes([raw_output[2], raw_output[3], raw_output[4], raw_output[5]]) as usize;
    let body_offset = 6 + ct_len;

    if body_offset + 4 > raw_output.len() {
        return raw_output.to_vec();
    }

    let body_len = u32::from_le_bytes([
        raw_output[body_offset],
        raw_output[body_offset + 1],
        raw_output[body_offset + 2],
        raw_output[body_offset + 3],
    ]) as usize;
    let body_start = body_offset + 4;
    let body_end = body_start.saturating_add(body_len).min(raw_output.len());

    raw_output[body_start..body_end].to_vec()
}

fn interpret_send_message(payload: &str) -> Option<CommandEnvelope> {
    let cmd_type = match payload {
        "create_identity" => Some(CommandType::CreateIdentity),
        "import_identity" => Some(CommandType::ImportIdentity),
        "add_controller" => Some(CommandType::AddController),
        "add_bootstrap_peer" => Some(CommandType::AddBootstrapNode),
        "add_reachability" => Some(CommandType::AddReachabilityHint),
        "query_node_state" => Some(CommandType::QueryNodeState),
        _ => None,
    };

    cmd_type.map(|ct| {
        let payload_bytes = match ct {
            CommandType::CreateIdentity => native_app_encode(&CreateIdentityPayload {
                payload_version: 1,
                label: String::from("primary"),
                key_algorithm: 1,
            }),
            CommandType::ImportIdentity => native_app_encode(&ImportIdentityPayload {
                payload_version: 1,
                label: String::from("imported"),
                key_algorithm: 1,
                public_key: Vec::new(),
                encrypted_private_key: Vec::new(),
                source: String::from("file"),
            }),
            CommandType::AddController => native_app_encode(&AddBootstrapNodePayload {
                payload_version: 1,
                node: None,
                address: String::from("local"),
                transport_class: 0,
                label: String::from("controller"),
            }),
            CommandType::AddBootstrapNode => native_app_encode(&AddBootstrapNodePayload {
                payload_version: 1,
                node: None,
                address: String::from("quic://peer.local:4242"),
                transport_class: 3,
                label: String::from("bootstrap-peer"),
            }),
            CommandType::AddReachabilityHint => native_app_encode(&AddReachabilityHintPayload {
                payload_version: 1,
                address: String::from("quic://self.local:4242"),
                transport_class: 3,
                directness: 1,
            }),
            CommandType::QueryNodeState => native_app_encode(&QueryNodeStatePayload {
                payload_version: 1,
                query_kind: String::from("all"),
            }),
            _ => Vec::new(),
        };

        let app_intent = AppIntent {
            app_id: b"settings-app".to_vec(),
            payload: payload_bytes.clone(),
            signature: Vec::new(),
        };

        CommandEnvelope {
            envelope_version: 1,
            command_id: format!("settings-cmd-{}", payload).into_bytes(),
            target_node: Some(NodeRef {
                node_id: Vec::new(),
            }),
            issuer: None,
            command_type: cmd_type.unwrap() as i32,
            command_version: 1,
            issued_at: Some(now_ts()),
            not_before: None,
            expires_at: None,
            idempotency_key: format!("idem-{}", payload).into_bytes(),
            payload: if payload_bytes.is_empty() {
                None
            } else {
                Some(
                    edgerun_core::protocol::command_envelope::Payload::InlinePayload(payload_bytes),
                )
            },
            delegation_chain: Vec::new(),
            requested_assurance: None,
            command_metadata: None,
            signature: None,
            app_intent: native_app_encode(&app_intent),
        }
    })
}

fn native_app_encode<T>(_value: &T) -> Vec<u8> {
    b"edgerun-app-native-v0".to_vec()
}

pub mod encrypted_envelope_compat {
    #[derive(Clone, Debug)]
    pub struct EncryptedEnvelope {
        pub ciphertext: Vec<u8>,
    }

    impl EncryptedEnvelope {
        pub fn decode(_bytes: &[u8]) -> Result<Self, &'static str> {
            Err("native encrypted envelope decode not wired yet")
        }
    }

    pub fn decrypt_envelope(
        _envelope: &EncryptedEnvelope,
        _key: &[u8],
    ) -> Result<Vec<u8>, &'static str> {
        Err("native encrypted envelope decrypt not wired yet")
    }

    pub fn encrypt_envelope(
        payload: &[u8],
        _key: &[u8],
    ) -> Result<EncryptedEnvelope, &'static str> {
        Ok(EncryptedEnvelope {
            ciphertext: payload.to_vec(),
        })
    }
}
