//! DeploymentContract Client
//!
//! Minimal JSON-RPC client using edgerun-http and bincode.

use std::sync::{Arc, mpsc};
use crate::solana_types::{Pubkey, AccountMeta, Instruction};
use crate::signers::Signer;
use edgerun_http::HttpClient;
use edgerun_json::{json, JsonValue};

use crate::error::SolanaError;
use crate::types::{Deployment, DeploymentStatus};
use crate::deployment_program_id;

const SYSTEM_PROGRAM_ID: Pubkey = Pubkey::new_from_array([0u8; 32]);

fn make_instruction(
    program_id: Pubkey,
    variant: u8,
    data: &[u8],
    accounts: Vec<AccountMeta>,
) -> Instruction {
    let mut bytes = vec![variant];
    bytes.extend_from_slice(data);
    Instruction {
        program_id,
        accounts,
        data: bytes,
    }
}

pub struct DeploymentClient {
    http: HttpClient,
    program_id: Pubkey,
    rpc_url: String,
    runtime: Arc<HttpRuntime>,
}

impl DeploymentClient {
    pub fn new(rpc_url: &str) -> Result<Self, SolanaError> {
        let rt = HttpRuntime::new();
        Ok(Self {
            http: HttpClient::new().no_redirects(),
            program_id: deployment_program_id(),
            rpc_url: rpc_url.to_string(),
            runtime: Arc::new(rt),
        })
    }

    fn rpc_call(&self, payload: JsonValue) -> Result<JsonValue, SolanaError> {
        let json_str = payload.to_json_string()
            .map_err(|e| SolanaError::Rpc(e.to_string()))?;
        let rpc_url = self.rpc_url.clone();
        let http = self.http.clone();

        let (tx, rx) = mpsc::channel();

        self.runtime.spawn(async move {
            let body = http.post_json(&rpc_url, &json_str).await;
            let _ = tx.send(body);
        });

        let resp = rx.recv().map_err(|e| SolanaError::Rpc(e.to_string()))?
            .map_err(|e| SolanaError::Rpc(e.to_string()))?;
        let body = String::from_utf8(resp.body().to_vec())
            .map_err(|e| SolanaError::Rpc(e.to_string()))?;
        let parsed = edgerun_json::parse_json(&body)
            .map_err(|e| SolanaError::Rpc(e.to_string()))?;
        if parsed.get("error").is_some() {
            return Err(SolanaError::Rpc(
                parsed["error"].to_json_string().unwrap_or_default(),
            ));
        }
        Ok(parsed)
    }

    pub fn get_deployment(&self, deployment_pubkey: &Pubkey) -> Result<Deployment, SolanaError> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getAccountInfo",
            "params": [deployment_pubkey.to_string(), { "encoding": "base64" }]
        });
        let resp = self.rpc_call(payload)?;
        let data = resp["result"]["value"]["data"]
            .as_str()
            .ok_or_else(|| SolanaError::Rpc("no data in response".to_string()))?;
        let bytes = base64_decode(data)?;
        Deployment::try_from_slice(&bytes)
            .map_err(|e| SolanaError::Serialization(e.to_string()))
    }

    pub fn initialize_instruction(
        &self,
        deployment_pubkey: &Pubkey,
        owner_pubkey: &Pubkey,
        name: [u8; 64],
        provider: [u8; 32],
        container_count: u32,
        total_cpu_cores: u32,
        total_memory_bytes: u64,
        total_storage_bytes: u64,
        total_network_mbps: u32,
        deposit: u64,
        burn_rate: u64,
    ) -> Instruction {
        let mut encoded = Vec::new();
        encoded.extend_from_slice(&name);
        encoded.extend_from_slice(&provider);
        encoded.extend_from_slice(&container_count.to_le_bytes());
        encoded.extend_from_slice(&total_cpu_cores.to_le_bytes());
        encoded.extend_from_slice(&total_memory_bytes.to_le_bytes());
        encoded.extend_from_slice(&total_storage_bytes.to_le_bytes());
        encoded.extend_from_slice(&total_network_mbps.to_le_bytes());
        encoded.extend_from_slice(&deposit.to_le_bytes());
        encoded.extend_from_slice(&burn_rate.to_le_bytes());
        make_instruction(
            self.program_id,
            0,
            &encoded,
            vec![
                AccountMeta::new(*deployment_pubkey, false),
                AccountMeta::new(*owner_pubkey, true),
                AccountMeta::new_readonly(SYSTEM_PROGRAM_ID),
            ],
        )
    }

    pub fn report_metrics_instruction(
        &self,
        deployment_pubkey: &Pubkey,
        provider_pubkey: &Pubkey,
        cpu_cores_used: u32,
        memory_bytes_used: u64,
        storage_bytes_used: u64,
        network_bytes_sent: u64,
        container_count: u32,
    ) -> Instruction {
        let mut encoded = Vec::new();
        encoded.extend_from_slice(&cpu_cores_used.to_le_bytes());
        encoded.extend_from_slice(&memory_bytes_used.to_le_bytes());
        encoded.extend_from_slice(&storage_bytes_used.to_le_bytes());
        encoded.extend_from_slice(&network_bytes_sent.to_le_bytes());
        encoded.extend_from_slice(&container_count.to_le_bytes());
        make_instruction(
            self.program_id,
            8,
            &encoded,
            vec![
                AccountMeta::new(*deployment_pubkey, false),
                AccountMeta::new(*provider_pubkey, true),
            ],
        )
    }

    pub fn start_instruction(&self, deployment_pubkey: &Pubkey, owner_pubkey: &Pubkey) -> Instruction {
        make_instruction(
            self.program_id,
            1,
            &[],
            vec![
                AccountMeta::new(*deployment_pubkey, false),
                AccountMeta::new(*owner_pubkey, true),
            ],
        )
    }

    pub fn stop_instruction(&self, deployment_pubkey: &Pubkey, owner_pubkey: &Pubkey) -> Instruction {
        make_instruction(
            self.program_id,
            4,
            &[],
            vec![
                AccountMeta::new(*deployment_pubkey, false),
                AccountMeta::new(*owner_pubkey, true),
            ],
        )
    }

pub fn send_instruction_sync(
        &self,
        instruction: Instruction,
        signer_pubkey: &Pubkey,
    ) -> Result<String, SolanaError> {
        let tx_bytes = serialize_transaction(signer_pubkey, &[instruction], &[0u8; 64]);
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "sendTransaction",
            "params": [
                base64_encode(&tx_bytes),
                { "encoding": "base64", "preflightCommitment": "processed" }
            ]
        });
        let resp = self.rpc_call(payload)?;
        resp["result"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| SolanaError::Rpc("no result in response".to_string()))
    }

    pub async fn send_instruction_signed<S: Signer>(
        &self,
        instruction: Instruction,
        signer_pubkey: &Pubkey,
        signer: &S,
    ) -> Result<String, SolanaError> {
        let ix = instruction.clone();
        let msg = serialize_transaction_message(signer_pubkey, &[ix]);
        let signature = signer.sign(&msg).map_err(|e| SolanaError::Signing(e.to_string()))?;
        let tx_bytes = serialize_transaction(signer_pubkey, &[instruction], &signature);
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "sendTransaction",
            "params": [
                base64_encode(&tx_bytes),
                { "encoding": "base64", "preflightCommitment": "processed" }
            ]
        });
        let resp = self.rpc_call(payload)?;
        resp["result"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| SolanaError::Rpc("no result in response".to_string()))
    }

    pub async fn post_report_signed<S: Signer>(
        &self,
        deployment_pubkey: &Pubkey,
        provider_pubkey: &Pubkey,
        signer: &S,
        cpu_cores_used: u32,
        memory_bytes_used: u64,
        storage_bytes_used: u64,
        network_bytes_sent: u64,
        container_count: u32,
    ) -> Result<String, SolanaError> {
        let instruction = self.report_metrics_instruction(
            deployment_pubkey, provider_pubkey, cpu_cores_used,
            memory_bytes_used, storage_bytes_used, network_bytes_sent, container_count,
        );
        self.send_instruction_signed(instruction, provider_pubkey, signer).await
    }

    pub fn calculate_burn_rate(
        cpu_cores: u32,
        memory_bytes: u64,
        storage_bytes: u64,
        network_mbits: u32,
    ) -> u64 {
        use crate::types::pricing;
        let cpu_hour = cpu_cores as u64 * pricing::CORE_HOUR;
        let memory_gib = memory_bytes.div_ceil(1024 * 1024 * 1024);
        let memory_hour = memory_gib * pricing::RAM_GIB_HOUR;
        let storage_gib = storage_bytes.div_ceil(1024 * 1024 * 1024);
        let storage_hour = storage_gib * pricing::STORAGE_GIB_HOUR;
        let network_hour = network_mbits as u64 * pricing::NETWORK_MBIT_HOUR;
        (cpu_hour + memory_hour + storage_hour + network_hour) / 3600
    }

    pub fn get_remaining(&self, deployment: &Deployment) -> u64 {
        deployment.deposit.saturating_sub(deployment.spent)
    }

    pub fn is_active(&self, deployment: &Deployment) -> bool {
        matches!(deployment.status, DeploymentStatus::Running)
    }
}

// Dedicated runtime for HTTP calls
struct HttpRuntime {
    rt: edgerun_rt::Runtime,
}

impl HttpRuntime {
    fn new() -> Self {
        Self {
            rt: edgerun_rt::Builder::new_multi_thread()
                .build()
                .expect("runtime"),
        }
    }

    fn spawn<F>(&self, f: F)
    where
        F: std::future::Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.rt.spawn(f);
    }
}

pub fn serialize_transaction(signer: &Pubkey, instructions: &[Instruction], signature: &[u8; 64]) -> Vec<u8> {
    use std::io::Write;
    let mut buf = Vec::new();
    buf.write_all(&(1u32).to_le_bytes()).unwrap();
    buf.write_all(signature).unwrap();
    buf.write_all(&[0u8; 32]).unwrap(); // recent blockhash
    let mut account_keys: Vec<Pubkey> = vec![*signer];
    for ix in instructions {
        for meta in &ix.accounts {
            if !account_keys.contains(&meta.pubkey()) {
                account_keys.push(meta.pubkey());
            }
        }
        if !account_keys.contains(&ix.program_id) {
            account_keys.push(ix.program_id);
        }
    }
    buf.write_all(&(account_keys.len() as u32).to_le_bytes()).unwrap();
    for key in &account_keys {
        buf.write_all(key.as_bytes()).unwrap();
    }
    buf.write_all(&(instructions.len() as u32).to_le_bytes()).unwrap();
    for ix in instructions {
        let prog_idx = account_keys.iter().position(|k| k == &ix.program_id).unwrap() as u8;
        buf.write_all(&[prog_idx]).unwrap();
        buf.write_all(&[ix.accounts.len() as u8]).unwrap();
        for meta in &ix.accounts {
            let idx = account_keys.iter().position(|k| k == &meta.pubkey()).unwrap() as u8;
            buf.write_all(&[idx]).unwrap();
        }
        buf.write_all(&(ix.data.len() as u16).to_le_bytes()).unwrap();
        buf.write_all(&ix.data).unwrap();
    }
    buf
}

pub fn serialize_transaction_message(signer: &Pubkey, instructions: &[Instruction]) -> Vec<u8> {
    use std::io::Write;
    let mut buf = Vec::new();
    buf.write_all(&(1u32).to_le_bytes()).unwrap();
    buf.write_all(&[0u8; 32]).unwrap();
    let mut account_keys: Vec<Pubkey> = vec![*signer];
    for ix in instructions {
        for meta in &ix.accounts {
            if !account_keys.contains(&meta.pubkey()) {
                account_keys.push(meta.pubkey());
            }
        }
        if !account_keys.contains(&ix.program_id) {
            account_keys.push(ix.program_id);
        }
    }
    buf.write_all(&(account_keys.len() as u32).to_le_bytes()).unwrap();
    for key in &account_keys {
        buf.write_all(key.as_bytes()).unwrap();
    }
    buf.write_all(&(instructions.len() as u32).to_le_bytes()).unwrap();
    for ix in instructions {
        let prog_idx = account_keys.iter().position(|k| k == &ix.program_id).unwrap() as u8;
        buf.write_all(&[prog_idx]).unwrap();
        buf.write_all(&[ix.accounts.len() as u8]).unwrap();
        for meta in &ix.accounts {
            let idx = account_keys.iter().position(|k| k == &meta.pubkey()).unwrap() as u8;
            buf.write_all(&[idx]).unwrap();
        }
        buf.write_all(&(ix.data.len() as u16).to_le_bytes()).unwrap();
        buf.write_all(&ix.data).unwrap();
    }
    buf
}

pub fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        result.push(ALPHABET[(b0 >> 2) as usize] as char);
        result.push(ALPHABET[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        result.push(if chunk.len() > 1 {
            ALPHABET[(((b1 & 0x0F) << 2) | (b2 >> 6)) as usize] as char
        } else { '=' });
        result.push(if chunk.len() > 2 {
            ALPHABET[(b2 & 0x3F) as usize] as char
        } else { '=' });
    }
    result
}

fn base64_decode(input: &str) -> Result<Vec<u8>, SolanaError> {
    fn decode_char(b: u8) -> Result<u8, SolanaError> {
        match b {
            b'A'..=b'Z' => Ok(b - b'A'),
            b'a'..=b'z' => Ok(b - b'a' + 26),
            b'0'..=b'9' => Ok(b - b'0' + 52),
            b'+' => Ok(62),
            b'/' => Ok(63),
            _ => Err(SolanaError::Rpc(format!("invalid base64 char: {}", b as char))),
        }
    }
    let input = input.trim_end_matches('=');
    let mut result = Vec::with_capacity(input.len() * 3 / 4);
    for chunk in input.as_bytes().chunks(4) {
        if chunk.len() < 4 { break; }
        let c0 = decode_char(chunk[0])?;
        let c1 = decode_char(chunk[1])?;
        result.push(c0 << 2 | c1 >> 4);
        let c2 = if chunk.len() >= 3 && chunk[2] != b'=' {
            let c = decode_char(chunk[2])?;
            result.push(c1 << 4 | c >> 2);
            Some(c)
        } else { None };
        if chunk.len() >= 4 && chunk[3] != b'=' {
            let c3 = decode_char(chunk[3])?;
            result.push((c2.unwrap_or(0)) << 6 | c3);
        }
    }
    Ok(result)
}