//! ProviderRegistry Client
//!
//! Minimal JSON-RPC client using edgerun-http and bincode.

use std::sync::{Arc, mpsc};
use crate::solana_types::{Pubkey, AccountMeta, Instruction};
use edgerun_http::HttpClient;
use edgerun_json::{json, JsonValue};

use crate::error::SolanaError;
use crate::types::{Provider, ProviderStatus, collateral};
use crate::provider_registry_program_id;

#[derive(Debug, Clone, serde::Serialize)]
enum ProviderRegistryInstruction {
    Initialize,
    Register {
        cpu_cores: u32,
        memory_bytes: u64,
        storage_bytes: u64,
        network_mbits: u32,
    },
    UpdateCollateral(u64),
    Pause,
    Resume,
    Attest { uptime_seconds: u32 },
}

pub struct ProviderClient {
    http: HttpClient,
    program_id: Pubkey,
    rpc_url: String,
    runtime: Arc<HttpRuntime>,
}

impl ProviderClient {
    pub fn new(rpc_url: &str) -> Result<Self, SolanaError> {
        let rt = HttpRuntime::new();
        Ok(Self {
            http: HttpClient::new().no_redirects(),
            program_id: provider_registry_program_id(),
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

    pub fn get_provider(&self, provider_pubkey: &Pubkey) -> Result<Provider, SolanaError> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getAccountInfo",
            "params": [provider_pubkey.to_string(), { "encoding": "base64" }]
        });
        let resp = self.rpc_call(payload)?;
        let data = resp["result"]["value"]["data"]
            .as_str()
            .ok_or_else(|| SolanaError::Rpc("no data in response".to_string()))?;
        let bytes = base64_decode(data)?;
        Provider::try_from_slice(&bytes)
            .map_err(|e| SolanaError::Serialization(e.to_string()))
    }

    pub fn find_active_providers(&self) -> Result<Vec<Pubkey>, SolanaError> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "getProgramAccounts",
            "params": [
                self.program_id.to_string(),
                { "encoding": "base64", "filters": [{ "dataSize": 200 }] }
            ]
        });
        let resp = self.rpc_call(payload)?;
        let accounts = resp["result"]
            .as_array()
            .ok_or_else(|| SolanaError::Rpc("no accounts".to_string()))?;
        let mut pubkeys = Vec::new();
        for account in accounts {
            if let Some(pubkey_str) = account["pubkey"].as_str() {
                if let Ok(pk) = pubkey_str.parse::<Pubkey>() {
                    pubkeys.push(pk);
                }
            }
        }
        Ok(pubkeys)
    }

    fn make_instruction<T: serde::Serialize>(
        &self,
        variant: u8,
        data: &T,
        accounts: Vec<AccountMeta>,
    ) -> Instruction {
        let encoded = bincode::serialize(data).unwrap();
        let mut bytes = vec![variant];
        bytes.extend(encoded);
        Instruction {
            program_id: self.program_id,
            accounts,
            data: bytes,
        }
    }

    pub fn register_instruction(
        &self,
        provider_pubkey: &Pubkey,
        authority_pubkey: &Pubkey,
        cpu_cores: u32,
        memory_bytes: u64,
        storage_bytes: u64,
        network_mbits: u32,
    ) -> Instruction {
        self.make_instruction(
            1,
            &ProviderRegistryInstruction::Register {
                cpu_cores,
                memory_bytes,
                storage_bytes,
                network_mbits,
            },
            vec![
                AccountMeta::new(*provider_pubkey, false),
                AccountMeta::new(*authority_pubkey, true),
            ],
        )
    }

    pub fn initialize_instruction(
        &self,
        provider_pubkey: &Pubkey,
        authority_pubkey: &Pubkey,
    ) -> Instruction {
        self.make_instruction(
            0,
            &ProviderRegistryInstruction::Initialize,
            vec![
                AccountMeta::new(*provider_pubkey, false),
                AccountMeta::new(*authority_pubkey, true),
            ],
        )
    }

    pub fn pause_instruction(
        &self,
        provider_pubkey: &Pubkey,
        authority_pubkey: &Pubkey,
    ) -> Instruction {
        self.make_instruction(
            3,
            &ProviderRegistryInstruction::Pause,
            vec![
                AccountMeta::new(*provider_pubkey, false),
                AccountMeta::new(*authority_pubkey, true),
            ],
        )
    }

    pub fn resume_instruction(
        &self,
        provider_pubkey: &Pubkey,
        authority_pubkey: &Pubkey,
    ) -> Instruction {
        self.make_instruction(
            4,
            &ProviderRegistryInstruction::Resume,
            vec![
                AccountMeta::new(*provider_pubkey, false),
                AccountMeta::new(*authority_pubkey, true),
            ],
        )
    }

    pub fn attest_instruction(
        &self,
        provider_pubkey: &Pubkey,
        authority_pubkey: &Pubkey,
        uptime_seconds: u32,
    ) -> Instruction {
        self.make_instruction(
            5,
            &ProviderRegistryInstruction::Attest { uptime_seconds },
            vec![
                AccountMeta::new(*provider_pubkey, false),
                AccountMeta::new(*authority_pubkey, true),
            ],
        )
    }

    pub fn calculate_minimum_collateral(
        cpu_cores: u32,
        memory_bytes: u64,
        storage_bytes: u64,
        network_mbits: u32,
    ) -> u64 {
        collateral::calculate_minimum(cpu_cores, memory_bytes, storage_bytes, network_mbits)
    }

    pub fn select_best_provider<'a>(
        &self,
        providers: &'a [Provider],
        required_cpu: u32,
        required_memory: u64,
    ) -> Option<&'a Provider> {
        if providers.is_empty() {
            return None;
        }
        providers
            .iter()
            .filter(|p| {
                p.status == ProviderStatus::Active
                    && p.cpu_cores >= required_cpu
                    && p.memory_bytes >= required_memory
            })
            .max_by_key(|p| p.uptime_percent.saturating_sub(p.slash_count * 100))
    }
}

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