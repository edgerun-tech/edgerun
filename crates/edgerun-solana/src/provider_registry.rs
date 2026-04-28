//! ProviderRegistry Client
//!
//! Minimal JSON-RPC client using edgerun-http and edgerun-json.

use crate::prelude::*;
use crate::signers::Signer;
use crate::solana_types::{AccountMeta, Instruction, Pubkey};
use edgerun_http::HttpClient;
use edgerun_json::{JsonValue, json};
use std::sync::Arc;

use crate::error::SolanaError;
use crate::provider_registry_program_id;
use crate::types::{Provider, ProviderStatus, collateral};

#[derive(Debug, Clone, serde::Serialize)]
#[allow(dead_code)]
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
    Attest {
        uptime_seconds: u32,
    },
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
        Provider::try_from_slice(&bytes).map_err(|e| SolanaError::Serialization(e.to_string()))
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
        let encoded = edgerun_json::to_vec(data).unwrap();
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

    pub async fn send_instruction_signed<S: Signer>(
        &self,
        instruction: Instruction,
        signer_pubkey: &Pubkey,
        signer: &S,
    ) -> Result<String, SolanaError> {
        let msg =
            crate::deployment::serialize_transaction_message(signer_pubkey, &[instruction.clone()]);
        let signature = signer
            .sign(&msg)
            .map_err(|e| SolanaError::Signing(e.to_string()))?;
        let tx_bytes =
            crate::deployment::serialize_transaction(signer_pubkey, &[instruction], &signature);
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "sendTransaction",
            "params": [
                crate::deployment::base64_encode(&tx_bytes),
                { "encoding": "base64", "preflightCommitment": "processed" }
            ]
        });
        let resp = self.rpc_call(payload)?;
        resp["result"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| SolanaError::Rpc("no result in response".to_string()))
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

    fn block_on<F>(&self, f: F) -> F::Output
    where
        F: std::future::Future,
    {
        self.rt.block_on(f)
    }
}

fn base64_decode(input: &str) -> Result<Vec<u8>, SolanaError> {
    edgerun_encoding::base64::standard_decode(input)
        .map_err(|err| SolanaError::Rpc(err.to_string()))
}
