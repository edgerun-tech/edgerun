//! ProviderRegistry Client
//!
//! Minimal JSON-RPC client using edgerun-http and edgerun-json.

use crate::prelude::*;
use crate::signers::Signer;
use crate::solana_types::{AccountMeta, Instruction, Pubkey};
use edgerun_http::{HttpClient, HttpVersion};
use edgerun_json::{JsonValue, json};
use std::sync::Arc;

use crate::error::SolanaError;
use crate::try_provider_registry_program_id;
use crate::types::{Provider, ProviderStatus, collateral};

const SYSTEM_PROGRAM_ID: Pubkey = Pubkey::new_from_array([0u8; 32]);
const PROVIDER_ACCOUNT_SIZE: u64 = 128;

pub struct ProviderClient {
    http: HttpClient,
    program_id: Pubkey,
    rpc_url: String,
    runtime: Arc<HttpRuntime>,
}

impl ProviderClient {
    pub fn new(rpc_url: &str) -> Result<Self, SolanaError> {
        Self::new_with_program_id(rpc_url, try_provider_registry_program_id()?)
    }

    pub fn new_with_program_id(rpc_url: &str, program_id: Pubkey) -> Result<Self, SolanaError> {
        let rt = HttpRuntime::new();
        Ok(Self {
            http: HttpClient::new().version(HttpVersion::Http1).no_redirects(),
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

    pub fn get_provider(&self, provider_pubkey: &Pubkey) -> Result<Provider, SolanaError> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getAccountInfo",
            "params": [provider_pubkey.to_string(), { "encoding": "base64", "commitment": "confirmed" }]
        });
        let resp = self.rpc_call(payload)?;
        let bytes = decode_rpc_account_data(&resp)?;
        Provider::try_from_slice(&bytes).map_err(|e| SolanaError::Serialization(e.to_string()))
    }

    pub fn find_active_providers(&self) -> Result<Vec<Pubkey>, SolanaError> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "getProgramAccounts",
            "params": [
                self.program_id.to_string(),
                { "encoding": "base64", "commitment": "confirmed", "filters": [{ "dataSize": 128 }] }
            ]
        });
        let resp = self.rpc_call(payload)?;
        let accounts = resp["result"]
            .as_array()
            .ok_or_else(|| SolanaError::Rpc("no accounts".to_string()))?;
        let mut pubkeys = Vec::new();
        for account in accounts {
            if let Some(pubkey) = active_provider_pubkey(account)? {
                pubkeys.push(pubkey);
            }
        }
        Ok(pubkeys)
    }

    fn make_instruction(
        &self,
        variant: u8,
        data: &[u8],
        accounts: Vec<AccountMeta>,
    ) -> Instruction {
        let mut bytes = vec![variant];
        bytes.extend_from_slice(data);
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
        let stake_amount = Self::calculate_minimum_collateral(
            cpu_cores,
            memory_bytes,
            storage_bytes,
            network_mbits,
        );
        let mut data = Vec::with_capacity(32);
        data.extend_from_slice(&cpu_cores.to_le_bytes());
        data.extend_from_slice(&stake_amount.to_le_bytes());
        data.extend_from_slice(&memory_bytes.to_le_bytes());
        data.extend_from_slice(&storage_bytes.to_le_bytes());
        data.extend_from_slice(&network_mbits.to_le_bytes());
        self.make_instruction(
            1,
            &data,
            vec![
                AccountMeta::new(*provider_pubkey, false),
                AccountMeta::new(*authority_pubkey, true),
                AccountMeta::new_readonly(SYSTEM_PROGRAM_ID),
            ],
        )
    }

    pub fn provider_address_with_seed(
        &self,
        authority_pubkey: &Pubkey,
        seed: &str,
    ) -> Result<Pubkey, SolanaError> {
        crate::deployment::create_address_with_seed(authority_pubkey, seed, &self.program_id)
    }

    pub async fn register_with_seed_signed<S: Signer>(
        &self,
        authority_pubkey: &Pubkey,
        seed: &str,
        signer: &S,
        cpu_cores: u32,
        memory_bytes: u64,
        storage_bytes: u64,
        network_mbits: u32,
    ) -> Result<(Pubkey, String), SolanaError> {
        let provider_pubkey = self.provider_address_with_seed(authority_pubkey, seed)?;
        let rent_lamports = self.minimum_balance_for_rent_exemption(PROVIDER_ACCOUNT_SIZE)?;
        let create_ix = crate::deployment::create_account_with_seed_instruction(
            authority_pubkey,
            &provider_pubkey,
            authority_pubkey,
            seed,
            rent_lamports,
            PROVIDER_ACCOUNT_SIZE,
            &self.program_id,
        )?;
        let register_ix = self.register_instruction(
            &provider_pubkey,
            authority_pubkey,
            cpu_cores,
            memory_bytes,
            storage_bytes,
            network_mbits,
        );
        let tx = self
            .send_instructions_signed(&[create_ix, register_ix], authority_pubkey, signer)
            .await?;
        Ok((provider_pubkey, tx))
    }

    pub fn initialize_instruction(
        &self,
        provider_pubkey: &Pubkey,
        authority_pubkey: &Pubkey,
    ) -> Instruction {
        self.make_instruction(
            0,
            &[],
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
            2,
            &[],
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
            3,
            &[],
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
        uptime_percent_bps: u32,
    ) -> Instruction {
        let data = uptime_percent_bps.to_le_bytes();
        self.make_instruction(
            5,
            &data,
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
        self.send_instructions_signed(core::slice::from_ref(&instruction), signer_pubkey, signer)
            .await
    }

    pub async fn send_instructions_signed<S: Signer>(
        &self,
        instructions: &[Instruction],
        signer_pubkey: &Pubkey,
        signer: &S,
    ) -> Result<String, SolanaError> {
        crate::deployment::validate_single_signer(signer_pubkey, instructions)?;
        let recent_blockhash = self.get_latest_blockhash()?;
        let msg = crate::deployment::serialize_transaction_message(
            signer_pubkey,
            instructions,
            &recent_blockhash,
        );
        let signature = signer
            .sign(&msg)
            .map_err(|e| SolanaError::Signing(e.to_string()))?;
        let tx_bytes = crate::deployment::serialize_transaction(
            signer_pubkey,
            instructions,
            &recent_blockhash,
            &signature,
        );
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

    pub fn minimum_balance_for_rent_exemption(&self, size: u64) -> Result<u64, SolanaError> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "getMinimumBalanceForRentExemption",
            "params": [size]
        });
        let resp = self.rpc_call(payload)?;
        resp["result"]
            .as_u64()
            .ok_or_else(|| SolanaError::Rpc("missing rent exemption result".to_string()))
    }

    #[cfg(not(target_os = "none"))]
    pub fn confirm_transaction(&self, signature: &str) -> Result<(), SolanaError> {
        const ATTEMPTS: usize = 30;
        const SLEEP_MS: u64 = 500;

        for _ in 0..ATTEMPTS {
            let payload = json!({
                "jsonrpc": "2.0",
                "id": 5,
                "method": "getSignatureStatuses",
                "params": [[signature], { "searchTransactionHistory": true }]
            });
            let resp = self.rpc_call(payload)?;
            let statuses = resp["result"]["value"]
                .as_array()
                .ok_or_else(|| SolanaError::Rpc("missing signature status array".to_string()))?;
            if let Some(status) = statuses.first() {
                if status.is_null() {
                    std::thread::sleep(std::time::Duration::from_millis(SLEEP_MS));
                    continue;
                }
                if !status["err"].is_null() {
                    return Err(SolanaError::Transaction(format!(
                        "transaction {signature} failed: {}",
                        status["err"].to_json_string().unwrap_or_default()
                    )));
                }
                if matches!(
                    status["confirmationStatus"].as_str(),
                    Some("confirmed" | "finalized")
                ) {
                    return Ok(());
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(SLEEP_MS));
        }

        Err(SolanaError::Transaction(format!(
            "transaction {signature} was not confirmed after {} seconds",
            (ATTEMPTS as u64 * SLEEP_MS) / 1_000
        )))
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

fn decode_rpc_account_data(resp: &JsonValue) -> Result<Vec<u8>, SolanaError> {
    if resp["result"]["value"].is_null() {
        return Err(SolanaError::AccountNotFound("provider account".to_string()));
    }

    let data = if !resp["result"]["value"]["data"].is_null() {
        &resp["result"]["value"]["data"]
    } else {
        &resp["result"]["data"]
    };

    decode_rpc_account_data_value(data)
}

fn decode_rpc_account_data_value(data: &JsonValue) -> Result<Vec<u8>, SolanaError> {
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

fn active_provider_pubkey(account: &JsonValue) -> Result<Option<Pubkey>, SolanaError> {
    let pubkey = account["pubkey"]
        .as_str()
        .ok_or_else(|| SolanaError::Rpc("program account missing pubkey".to_string()))?
        .parse::<Pubkey>()
        .map_err(|err| SolanaError::Rpc(format!("invalid program account pubkey: {err}")))?;

    let data = &account["account"]["data"];
    if data.is_null() {
        return Err(SolanaError::Rpc(
            "program account missing account.data".to_string(),
        ));
    }

    let bytes = decode_rpc_account_data_value(data)?;
    let provider =
        Provider::try_from_slice(&bytes).map_err(|e| SolanaError::Serialization(e.to_string()))?;
    Ok((provider.status == ProviderStatus::Active).then_some(pubkey))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client() -> ProviderClient {
        ProviderClient::new_with_program_id(
            "http://127.0.0.1:8899",
            crate::provider_registry_program_id(),
        )
        .unwrap()
    }

    #[test]
    fn register_instruction_uses_binary_provider_layout() {
        let provider = Pubkey::new_from_array([1u8; 32]);
        let authority = Pubkey::new_from_array([2u8; 32]);
        let ix = client().register_instruction(
            &provider,
            &authority,
            4,
            8 * 1024 * 1024 * 1024,
            16 * 1024 * 1024 * 1024,
            100,
        );

        assert_eq!(ix.data.len(), 33);
        assert_eq!(ix.accounts.len(), 3);
        assert_eq!(ix.data[0], 1);
        assert_eq!(u32::from_le_bytes(ix.data[1..5].try_into().unwrap()), 4);
        assert_eq!(
            u64::from_le_bytes(ix.data[5..13].try_into().unwrap()),
            ProviderClient::calculate_minimum_collateral(
                4,
                8 * 1024 * 1024 * 1024,
                16 * 1024 * 1024 * 1024,
                100,
            )
        );
        assert_eq!(
            u64::from_le_bytes(ix.data[13..21].try_into().unwrap()),
            8 * 1024 * 1024 * 1024
        );
        assert_eq!(
            u64::from_le_bytes(ix.data[21..29].try_into().unwrap()),
            16 * 1024 * 1024 * 1024
        );
        assert_eq!(u32::from_le_bytes(ix.data[29..33].try_into().unwrap()), 100);
    }

    #[test]
    fn provider_control_variants_match_program() {
        let provider = Pubkey::new_from_array([1u8; 32]);
        let authority = Pubkey::new_from_array([2u8; 32]);
        let client = client();

        assert_eq!(
            client.pause_instruction(&provider, &authority).data,
            vec![2]
        );
        assert_eq!(
            client.resume_instruction(&provider, &authority).data,
            vec![3]
        );
        assert_eq!(
            client.attest_instruction(&provider, &authority, 9_999).data,
            vec![5, 15, 39, 0, 0]
        );
    }

    #[test]
    fn program_account_filter_decodes_only_active_providers() {
        let active_pubkey = Pubkey::new_from_array([9u8; 32]);
        let paused_pubkey = Pubkey::new_from_array([8u8; 32]);
        let active = provider_program_account(active_pubkey, ProviderStatus::Active);
        let paused = provider_program_account(paused_pubkey, ProviderStatus::Paused);

        assert_eq!(
            active_provider_pubkey(&active).unwrap(),
            Some(active_pubkey)
        );
        assert_eq!(active_provider_pubkey(&paused).unwrap(), None);
    }

    #[test]
    fn get_account_info_null_value_is_account_not_found() {
        let resp = json!({
            "jsonrpc": "2.0",
            "result": { "value": null },
            "id": 1
        });

        match decode_rpc_account_data(&resp) {
            Err(SolanaError::AccountNotFound(label)) => assert_eq!(label, "provider account"),
            other => panic!("unexpected decode result: {other:?}"),
        }
    }

    #[test]
    fn get_account_info_decodes_base64_data_array() {
        let encoded = edgerun_encoding::base64::standard_encode(&[9, 8, 7, 6]);
        let resp = json!({
            "jsonrpc": "2.0",
            "result": { "value": { "data": [encoded, "base64"] } },
            "id": 1
        });

        assert_eq!(decode_rpc_account_data(&resp).unwrap(), vec![9, 8, 7, 6]);
    }

    fn provider_program_account(pubkey: Pubkey, status: ProviderStatus) -> JsonValue {
        let mut data = [0u8; 128];
        data[0..32].copy_from_slice(&[7u8; 32]);
        data[32..40].copy_from_slice(&1000u64.to_le_bytes());
        data[40..44].copy_from_slice(&4u32.to_le_bytes());
        data[44..52].copy_from_slice(&(8 * 1024 * 1024 * 1024u64).to_le_bytes());
        data[52..60].copy_from_slice(&(16 * 1024 * 1024 * 1024u64).to_le_bytes());
        data[60..64].copy_from_slice(&100u32.to_le_bytes());
        data[80] = status as u8;

        let data_json = data.iter().map(u8::to_string).collect::<Vec<_>>().join(",");
        edgerun_json::parse_json(&format!(
            r#"{{"pubkey":"{}","account":{{"data":[{}]}}}}"#,
            pubkey, data_json
        ))
        .unwrap()
    }
}
