use anyhow::{Context, Result};
use edgerun_proto::edgerun::v0::{
    common::{IdentityRef, NodeRef},
    stream::{CommandEnvelope, CommandType},
    trust::DelegationRecord,
};
use prost_types::Timestamp;
use std::sync::{Arc, Mutex};
use wasmtime::*;

#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub node_identity: IdentityRef,
    pub app_instance_id: [u8; 16],
    pub delegation_chain: Vec<DelegationRecord>,
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

impl WasmRuntime {
    pub fn new(wasm_path: &str, verbose: bool) -> Result<Self> {
        let engine = Engine::default();
        let module =
            Module::from_file(&engine, wasm_path).context("Failed to load WASM module")?;

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
                let safe_read = |p: i32, l: i32| -> String {
                    let start = p as usize;
                    let end = start.saturating_add(l as usize);
                    if end > mem_data.len() {
                        String::from("<oob>")
                    } else {
                        String::from_utf8_lossy(&mem_data[start..end]).to_string()
                    }
                };
                let target_str = safe_read(target_ptr, target_len);
                let payload_str = safe_read(payload_ptr, payload_len);

                let host = caller.data_mut();
                let cmd = build_command(
                    host,
                    CommandType::StoreObject,
                    payload_str.into_bytes(),
                    target_str,
                );

                host.pending_commands.lock().unwrap().push(cmd);
                0
            },
        )?;

        linker.func_wrap(
            "env",
            "read_blob",
            |mut caller: Caller<'_, HostState>, hash_ptr: i32, hash_len: i32, _dst_ptr: i32| -> i32 {
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
                let cmd = build_command(
                    host,
                    CommandType::FetchObject,
                    hash,
                    String::new(),
                );

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
                let cmd = build_command(
                    host,
                    CommandType::StoreObject,
                    payload,
                    String::new(),
                );

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

        Ok(Self {
            engine,
            module,
            linker,
            output,
            verbose,
        })
    }

    pub fn run_with_context(&mut self, input: &str, context: ExecutionContext) -> Result<Vec<u8>> {
        self.output.lock().unwrap().clear();

        let pending_commands = Arc::new(Mutex::new(Vec::new()));
        let pending_events = Arc::new(Mutex::new(Vec::new()));

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

        if self.verbose {
            let cmds = pending_commands.lock().unwrap();
            if !cmds.is_empty() {
                eprintln!("WASM produced {} pending command(s)", cmds.len());
                for (i, cmd) in cmds.iter().enumerate() {
                    eprintln!(
                        "  cmd[{}]: type={:?}, id_len={}",
                        i,
                        CommandType::from_i32(cmd.command_type),
                        cmd.command_id.len()
                    );
                }
            }
        }

        let out = self.output.lock().unwrap();
        if out.is_empty() {
            return Ok(Vec::new());
        }

        if out.len() < 10 {
            return Ok(out.clone());
        }

        let ct_len = u32::from_le_bytes([out[2], out[3], out[4], out[5]]) as usize;
        let body_offset = 6 + ct_len;

        if body_offset + 4 > out.len() {
            return Ok(out.clone());
        }

        let body_len = u32::from_le_bytes([
            out[body_offset],
            out[body_offset + 1],
            out[body_offset + 2],
            out[body_offset + 3],
        ]) as usize;
        let body_start = body_offset + 4;
        let body_end = body_start.saturating_add(body_len).min(out.len());

        let body = out[body_start..body_end].to_vec();
        Ok(body)
    }

    pub fn run(&mut self, input: &str) -> Result<Vec<u8>> {
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
        .unwrap_or_default();
    let delegation_chain = ctx
        .map(|c| c.delegation_chain.clone())
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
            Some(
                edgerun_proto::edgerun::v0::stream::command_envelope::Payload::InlinePayload(
                    payload,
                ),
            )
        },
        delegation_chain,
        requested_assurance: None,
        command_metadata: None,
        signature: None,
    }
}
