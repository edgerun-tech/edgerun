//! DeploymentContract Client
//!
//! Minimal JSON-RPC client using edgerun-http and bincode.

use crate::prelude::*;
use crate::signers::Signer;
use crate::solana_types::{AccountMeta, Instruction, Pubkey};
use edgerun_http::HttpClient;
use edgerun_json::{json, JsonValue};
use std::sync::Arc;

use crate::error::SolanaError;
use crate::try_deployment_program_id;
use crate::types::{Deployment, DeploymentStatus};

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
        Self::new_with_program_id(rpc_url, try_deployment_program_id()?)
    }

    pub fn new_with_program_id(rpc_url: &str, program_id: Pubkey) -> Result<Self, SolanaError> {
        let rt = HttpRuntime::new();
        Ok(Self {
            http: HttpClient::new().no_redirects(),
            program_id,
            rpc_url: rpc_url.to_string(),
            runtime: Arc::new(rt),
        })
    }

    fn rpc_call(&self, payload: JsonValue) -> Result<JsonValue, SolanaError> {
        let json_str = payload
            .to_json_string()
            .map_err(|e| SolanaError::Rpc(e.to_string()))?;
        let rpc_url = self.rpc_url.clone();
        let http = self.http.clone();

        let resp = self
            .runtime
            .block_on(async move { http.post_json(&rpc_url, &json_str).await })
            .map_err(|e| SolanaError::Rpc(e.to_string()))?;
        let body =
            String::from_utf8(resp.body().to_vec()).map_err(|e| SolanaError::Rpc(e.to_string()))?;
        let parsed =
            edgerun_json::parse_json(&body).map_err(|e| SolanaError::Rpc(e.to_string()))?;
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
        let bytes = decode_rpc_account_data(&resp)?;
        Deployment::try_from_slice(&bytes).map_err(|e| SolanaError::Serialization(e.to_string()))
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

    pub fn start_instruction(
        &self,
        deployment_pubkey: &Pubkey,
        owner_pubkey: &Pubkey,
    ) -> Instruction {
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

    pub fn stop_instruction(
        &self,
        deployment_pubkey: &Pubkey,
        owner_pubkey: &Pubkey,
    ) -> Instruction {
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
        _instruction: Instruction,
        _signer_pubkey: &Pubkey,
    ) -> Result<String, SolanaError> {
        Err(SolanaError::Transaction(
            "unsigned Solana transaction submission is not supported".to_string(),
        ))
    }

    pub async fn send_instruction_signed<S: Signer>(
        &self,
        instruction: Instruction,
        signer_pubkey: &Pubkey,
        signer: &S,
    ) -> Result<String, SolanaError> {
        validate_single_signer(signer_pubkey, core::slice::from_ref(&instruction))?;
        let recent_blockhash = self.get_latest_blockhash()?;
        let ix = instruction.clone();
        let msg = serialize_transaction_message(signer_pubkey, &[ix], &recent_blockhash);
        let signature = signer
            .sign(&msg)
            .map_err(|e| SolanaError::Signing(e.to_string()))?;
        let tx_bytes =
            serialize_transaction(signer_pubkey, &[instruction], &recent_blockhash, &signature);
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

    fn get_latest_blockhash(&self) -> Result<Pubkey, SolanaError> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "getLatestBlockhash",
            "params": [{ "commitment": "processed" }]
        });
        let resp = self.rpc_call(payload)?;
        let blockhash = resp["result"]["value"]["blockhash"]
            .as_str()
            .ok_or_else(|| SolanaError::Rpc("missing latest blockhash".to_string()))?;
        blockhash
            .parse::<Pubkey>()
            .map_err(|err| SolanaError::Rpc(format!("invalid latest blockhash: {err}")))
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
            deployment_pubkey,
            provider_pubkey,
            cpu_cores_used,
            memory_bytes_used,
            storage_bytes_used,
            network_bytes_sent,
            container_count,
        );
        self.send_instruction_signed(instruction, provider_pubkey, signer)
            .await
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

    fn block_on<F>(&self, f: F) -> F::Output
    where
        F: std::future::Future,
    {
        self.rt.block_on(f)
    }
}

pub fn serialize_transaction(
    signer: &Pubkey,
    instructions: &[Instruction],
    recent_blockhash: &Pubkey,
    signature: &[u8; 64],
) -> Vec<u8> {
    let mut buf = Vec::new();
    encode_shortvec_len(1, &mut buf);
    buf.extend_from_slice(signature);
    buf.extend_from_slice(&serialize_transaction_message(
        signer,
        instructions,
        recent_blockhash,
    ));
    buf
}

pub fn serialize_transaction_message(
    signer: &Pubkey,
    instructions: &[Instruction],
    recent_blockhash: &Pubkey,
) -> Vec<u8> {
    let mut buf = Vec::new();
    let (account_keys, readonly_unsigned_count) = compile_account_keys(signer, instructions);

    buf.extend_from_slice(&[1, 0, readonly_unsigned_count]);
    encode_shortvec_len(account_keys.len(), &mut buf);
    for key in &account_keys {
        buf.extend_from_slice(key.as_bytes());
    }
    buf.extend_from_slice(recent_blockhash.as_bytes());
    encode_shortvec_len(instructions.len(), &mut buf);
    for ix in instructions {
        let prog_idx = account_index(&account_keys, ix.program_id);
        buf.push(prog_idx);
        encode_shortvec_len(ix.accounts.len(), &mut buf);
        for meta in &ix.accounts {
            buf.push(account_index(&account_keys, meta.pubkey()));
        }
        encode_shortvec_len(ix.data.len(), &mut buf);
        buf.extend_from_slice(&ix.data);
    }
    buf
}

pub(crate) fn validate_single_signer(
    signer: &Pubkey,
    instructions: &[Instruction],
) -> Result<(), SolanaError> {
    for ix in instructions {
        for meta in &ix.accounts {
            if meta.is_signer() && meta.pubkey() != *signer {
                return Err(SolanaError::Transaction(
                    "instruction requires a signer not provided by this client".to_string(),
                ));
            }
        }
    }
    Ok(())
}

fn compile_account_keys(signer: &Pubkey, instructions: &[Instruction]) -> (Vec<Pubkey>, u8) {
    let mut writable_unsigned = Vec::new();
    let mut readonly_unsigned = Vec::new();

    for ix in instructions {
        for meta in &ix.accounts {
            let key = meta.pubkey();
            if key == *signer {
                continue;
            }
            if meta.is_readonly() {
                push_unique(&mut readonly_unsigned, key);
            } else {
                push_unique(&mut writable_unsigned, key);
            }
        }
        push_unique(&mut readonly_unsigned, ix.program_id);
    }

    let readonly_unsigned_count = readonly_unsigned.len() as u8;
    let mut account_keys =
        Vec::with_capacity(1 + writable_unsigned.len() + readonly_unsigned.len());
    account_keys.push(*signer);
    account_keys.extend(writable_unsigned);
    account_keys.extend(readonly_unsigned);
    (account_keys, readonly_unsigned_count)
}

fn push_unique(keys: &mut Vec<Pubkey>, key: Pubkey) {
    if !keys.contains(&key) {
        keys.push(key);
    }
}

fn account_index(account_keys: &[Pubkey], key: Pubkey) -> u8 {
    account_keys
        .iter()
        .position(|candidate| *candidate == key)
        .expect("account key must be compiled before instruction encoding") as u8
}

fn encode_shortvec_len(mut value: usize, out: &mut Vec<u8>) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value == 0 {
            out.push(byte);
            break;
        }
        byte |= 0x80;
        out.push(byte);
    }
}

pub fn base64_encode(data: &[u8]) -> String {
    edgerun_encoding::base64::standard_encode(data)
}

fn base64_decode(input: &str) -> Result<Vec<u8>, SolanaError> {
    edgerun_encoding::base64::standard_decode(input)
        .map_err(|err| SolanaError::Rpc(err.to_string()))
}

fn decode_rpc_account_data(resp: &JsonValue) -> Result<Vec<u8>, SolanaError> {
    let data = if !resp["result"]["value"]["data"].is_null() {
        &resp["result"]["value"]["data"]
    } else {
        &resp["result"]["data"]
    };

    if let Some(encoded) = data.as_str() {
        return base64_decode(encoded);
    }

    let values = data
        .as_array()
        .ok_or_else(|| SolanaError::Rpc("no data in response".to_string()))?;

    if let Some(encoded) = values.first().and_then(JsonValue::as_str) {
        return base64_decode(encoded);
    }

    values
        .iter()
        .map(|value| {
            value
                .as_u64()
                .and_then(|n| u8::try_from(n).ok())
                .ok_or_else(|| SolanaError::Rpc("account data byte is not u8".to_string()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_message_uses_solana_wire_shape() {
        let signer = Pubkey::new_from_array([1u8; 32]);
        let account = Pubkey::new_from_array([2u8; 32]);
        let program = Pubkey::new_from_array([3u8; 32]);
        let blockhash = Pubkey::new_from_array([4u8; 32]);
        let instruction = Instruction {
            program_id: program,
            accounts: vec![
                AccountMeta::new(account, false),
                AccountMeta::new(signer, true),
            ],
            data: vec![9, 8, 7],
        };

        let msg = serialize_transaction_message(&signer, &[instruction], &blockhash);

        assert_eq!(&msg[0..3], &[1, 0, 1]);
        assert_eq!(msg[3], 3);
        assert_eq!(&msg[4..36], signer.as_bytes());
        assert_eq!(&msg[36..68], account.as_bytes());
        assert_eq!(&msg[68..100], program.as_bytes());
        assert_eq!(&msg[100..132], blockhash.as_bytes());
        assert_eq!(msg[132], 1);
        assert_eq!(msg[133], 2);
        assert_eq!(msg[134], 2);
        assert_eq!(&msg[135..137], &[1, 0]);
        assert_eq!(msg[137], 3);
        assert_eq!(&msg[138..141], &[9, 8, 7]);
    }

    #[test]
    fn legacy_transaction_prefixes_signature_count_and_message() {
        let signer = Pubkey::new_from_array([1u8; 32]);
        let program = Pubkey::new_from_array([3u8; 32]);
        let blockhash = Pubkey::new_from_array([4u8; 32]);
        let signature = [5u8; 64];
        let instruction = Instruction {
            program_id: program,
            accounts: vec![AccountMeta::new(signer, true)],
            data: vec![1],
        };

        let tx = serialize_transaction(&signer, &[instruction.clone()], &blockhash, &signature);
        let msg = serialize_transaction_message(&signer, &[instruction], &blockhash);

        assert_eq!(tx[0], 1);
        assert_eq!(&tx[1..65], &signature);
        assert_eq!(&tx[65..], msg.as_slice());
    }
}
